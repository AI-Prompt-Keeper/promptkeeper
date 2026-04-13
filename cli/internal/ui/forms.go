package ui

import (
	"errors"
	"strings"

	"charm.land/huh/v2"
	"github.com/promptkeeper/cli/internal/validate"
)

// FormRegister collects email and optionally password (password optional only when email is pre-filled).
// If email is non-empty, only password is asked.
func FormRegister(email *string, password *string) error {
	if *email != "" {
		// Only ask for password
		return huh.NewForm(
			huh.NewGroup(
				huh.NewInput().
					Title("Password").
					Password(true).
					Value(password).
					Validate(func(s string) error {
						return validate.ValidatePassword(s)
					}),
			).Title("Register").
				Description("Create a Prompt Keeper account and default workspace."),
		).Run()
	}
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("Email").
				Value(email).
				Validate(func(s string) error {
					return validate.ValidateEmail(s)
				}),
			huh.NewInput().
				Title("Password").
				Password(true).
				Value(password).
				Validate(func(s string) error {
					return validate.ValidatePassword(s)
				}),
		).Title("Register").
			Description("Create a Prompt Keeper account and default workspace."),
	).Run()
}

// FormLogin collects email and password for login (same layout as registration).
// If email is non-empty, only password is asked.
func FormLogin(email *string, password *string) error {
	if *email != "" {
		return huh.NewForm(
			huh.NewGroup(
				huh.NewInput().
					Title("Password").
					Password(true).
					Value(password).
					Validate(func(s string) error {
						return validate.ValidatePassword(s)
					}),
			).Title("Sign in").
				Description("Authenticate with your email and password."),
		).Run()
	}
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("Email").
				Value(email).
				Validate(func(s string) error {
					return validate.ValidateEmail(s)
				}),
			huh.NewInput().
				Title("Password").
				Password(true).
				Value(password).
				Validate(func(s string) error {
					return validate.ValidatePassword(s)
				}),
		).Title("Sign in").
			Description("Authenticate with your email and password."),
	).Run()
}

// FormSetAPIKey collects the API key.
func FormSetAPIKey(key *string) error {
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("API key").
				Description("Management key (pk_mgt_live_...), execution key, or 64-char session token.").
				Password(true).
				Value(key).
				Validate(func(s string) error {
					if len(s) == 0 {
						return errors.New("API key is required")
					}
					return nil
				}),
		).Title("Set Prompt Keeper client key").
			Description("Stored in the OS secure store for the active workspace."),
	).Run()
}

// StoreKind is key or prompt.
type StoreKind string

const (
	StoreKindKey    StoreKind = "key"
	StoreKindPrompt StoreKind = "prompt"
)

// FormStoreKind asks user to choose store key or store prompt.
func FormStoreKind(kind *StoreKind) error {
	return huh.NewForm(
		huh.NewGroup(
			huh.NewSelect[StoreKind]().
				Title("What do you want to store?").
				Options(
					huh.NewOption("Provider API key (e.g. OpenAI, Anthropic)", StoreKindKey),
					huh.NewOption("Prompt template", StoreKindPrompt),
				).
				Value(kind),
		).Title("Store").
			Description("Guided flow for provider keys or prompt templates."),
	).Run()
}

// FormStoreKey collects provider and API key for store key.
func FormStoreKey(provider *string, apiKey *string) error {
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("Provider (e.g. openai, anthropic)").
				Value(provider).
				Validate(func(s string) error {
					if s == "" {
						return errors.New("provider is required")
					}
					return validate.ValidateProvider(s)
				}),
			huh.NewInput().
				Title("API key").
				Password(true).
				Value(apiKey).
				Validate(func(s string) error {
					if len(s) == 0 {
						return errors.New("API key is required")
					}
					return validate.ValidateInputLength(s, "api_key")
				}),
		).Title("Store provider API key").
			Description("Sent to the Secure AI Gateway with envelope encryption."),
	).Run()
}

// FormStorePrompt collects title, prompt value (or path), optional provider, optional model.
// Split into two groups to reduce render load and improve responsiveness.
func FormStorePrompt(title *string, promptValue *string, provider *string, model *string) error {
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("Prompt title (function_id)").
				Description("Used as the prompt name when calling exec. Letters, numbers, spaces, underscore, hyphen.").
				Value(title).
				Validate(func(s string) error {
					return validate.ValidatePromptTitle(s)
				}),
			huh.NewText().
				Title("Prompt content or path to file").
				Description("Paste the template text here, or enter a path to a file (e.g. ./prompt.txt).").
				Value(promptValue).
				Lines(8).
				Validate(func(s string) error {
					if len(s) == 0 {
						return errors.New("prompt content or file path is required")
					}
					return nil
				}),
		).
			Title("Prompt template").
			Description("Enter the prompt title and content. You can paste text or type a file path."),
		huh.NewGroup(
			huh.NewInput().
				Title("Provider (optional, e.g. openai)").
				Value(provider),
			huh.NewInput().
				Title("Model (optional, e.g. gpt-4o)").
				Value(model),
		).
			Title("Optional settings").
			Description("Leave blank to use backend defaults."),
	).Run()
}

// WorkspacePick is one workspace row for interactive rename (id + display name).
type WorkspacePick struct {
	ID   string
	Name string
}

// WorkspacePickPurpose selects copy for the workspace picker (rename / switch / delete).
type WorkspacePickPurpose int

const (
	WorkspacePickPurposeRename WorkspacePickPurpose = iota
	WorkspacePickPurposeSwitch
	WorkspacePickPurposeDelete
)

// FormWorkspacePick lists workspaces in a select field (arrow keys, Enter to confirm).
func FormWorkspacePick(workspaces []WorkspacePick, selectedID *string) error {
	return FormWorkspacePickFor(workspaces, selectedID, WorkspacePickPurposeRename)
}

// FormWorkspacePickFor lists workspaces with flow-specific titles (rename, switch, or delete).
func FormWorkspacePickFor(workspaces []WorkspacePick, selectedID *string, purpose WorkspacePickPurpose) error {
	if len(workspaces) == 0 {
		return errors.New("no workspaces to choose from")
	}
	opts := make([]huh.Option[string], 0, len(workspaces))
	for _, w := range workspaces {
		label := strings.TrimSpace(w.Name)
		if label == "" {
			label = w.ID
		}
		opts = append(opts, huh.NewOption(label, w.ID))
	}
	var formTitle, formDesc, selectTitle string
	switch purpose {
	case WorkspacePickPurposeRename:
		formTitle = "Rename workspace"
		formDesc = "No active workspace is set — pick one to rename."
		selectTitle = "Which workspace do you want to rename?"
	case WorkspacePickPurposeSwitch:
		formTitle = "Switch workspace"
		formDesc = "Choose the workspace to make active for CLI commands."
		selectTitle = "Which workspace do you want to switch to?"
	case WorkspacePickPurposeDelete:
		formTitle = "Delete workspace"
		formDesc = "This cannot be undone. Pick the workspace to remove."
		selectTitle = "Which workspace do you want to delete?"
	default:
		formTitle = "Workspace"
		formDesc = "Pick a workspace."
		selectTitle = "Workspace"
	}
	return huh.NewForm(
		huh.NewGroup(
			huh.NewSelect[string]().
				Title(selectTitle).
				Description("↑/↓ to move, Enter to select.").
				Options(opts...).
				Value(selectedID).
				Validate(func(s string) error {
					if strings.TrimSpace(s) == "" {
						return errors.New("please select a workspace")
					}
					return nil
				}),
		).Title(formTitle).
			Description(formDesc),
	).Run()
}

// FormWorkspaceCreateName asks for the name of a new workspace (same huh styling as rename flows).
func FormWorkspaceCreateName(name *string) error {
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("Workspace name").
				Description("Choose a display name. You will receive a management API key once (shown and stored locally).").
				Value(name).
				Validate(func(s string) error {
					return validate.ValidateWorkspaceName(s)
				}),
		).Title("Create workspace").
			Description("Adds a new workspace to your account (POST /v1/workspaces)."),
	).Run()
}

// FormWorkspaceNewName asks for the new display name; currentLabel is shown as context (e.g. current name).
func FormWorkspaceNewName(currentLabel, workspaceID string, newName *string) error {
	desc := strings.TrimSpace(currentLabel)
	if desc == "" {
		desc = workspaceID
	}
	sub := "Current: " + desc
	if strings.TrimSpace(workspaceID) != "" {
		sub += "\nID: " + strings.TrimSpace(workspaceID)
	}
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("New workspace name").
				Description(sub).
				Value(newName).
				Validate(func(s string) error {
					return validate.ValidateWorkspaceName(s)
				}),
		).Title("Rename workspace").
			Description("Enter the new display name for this workspace."),
	).Run()
}

// FormExec collects prompt title and optional variables (as key=value lines).
func FormExec(promptTitle *string, varLines *string) error {
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("Prompt title").
				Value(promptTitle).
				Validate(func(s string) error {
					return validate.ValidatePromptTitle(s)
				}),
			huh.NewText().
				Title("Variables (optional, one per line: key=value)").
				Value(varLines),
		).Title("Run prompt").
			Description("Streams the model response to stdout (GET/execute flow)."),
	).Run()
}

// FormKeyAliasRename collects old and new alias names (interactive key alias rename).
func FormKeyAliasRename(oldAlias, newAlias *string) error {
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("Current alias").
				Value(oldAlias).
				Validate(func(s string) error {
					return validate.ValidateKeyAlias(s)
				}),
			huh.NewInput().
				Title("New alias").
				Value(newAlias).
				Validate(func(s string) error {
					return validate.ValidateKeyAlias(s)
				}),
		).Title("Rename key alias").
			Description("Per-workspace aliases for stored client keys (CLI-only)."),
	).Run()
}

// FormKeyAliasNewAlias asks only for the new alias (when old alias was given on the command line).
func FormKeyAliasNewAlias(newAlias *string) error {
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("New alias").
				Value(newAlias).
				Validate(func(s string) error {
					return validate.ValidateKeyAlias(s)
				}),
		).Title("Rename key alias").
			Description("Choose the new alias name."),
	).Run()
}

// FormUseClientKey collects an alias or raw pk_* key for prke use.
func FormUseClientKey(aliasOrKey *string) error {
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("Alias or client key").
				Description("An existing alias, or pk_mgt_live_... / pk_exe_live_... (verified for the active workspace).").
				Value(aliasOrKey).
				Validate(func(s string) error {
					if strings.TrimSpace(s) == "" {
						return errors.New("value is required")
					}
					return nil
				}),
		).Title("Default client key").
			Description("Sets the default key for the active workspace."),
	).Run()
}

// FormMintExecutionKeyLabel collects an optional label for a minted execution key.
func FormMintExecutionKeyLabel(label *string) error {
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("Label (optional)").
				Description("Shown in workspace token lists. Leave blank to use the server default.").
				Value(label),
		).Title("Mint execution key").
			Description("Creates an execution-only client key (pk_exe_live_...) for the active workspace."),
	).Run()
}

// FormMintManagementKeyLabel collects an optional label for POST .../mgt-key.
func FormMintManagementKeyLabel(label *string) error {
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("Label (optional)").
				Description("Shown in workspace token lists. Leave blank to use \"CLI\" as default.").
				Value(label),
		).Title("Mint management key").
			Description("Creates a new management key for the active workspace."),
	).Run()
}
