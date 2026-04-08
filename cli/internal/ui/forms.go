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
			),
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
		),
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
			),
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
		),
	).Run()
}

// FormSetAPIKey collects the API key.
func FormSetAPIKey(key *string) error {
	return huh.NewForm(
		huh.NewGroup(
			huh.NewInput().
				Title("API key").
				Password(true).
				Value(key).
				Validate(func(s string) error {
					if len(s) == 0 {
						return errors.New("API key is required")
					}
					return nil
				}),
		),
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
		),
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
		),
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

// FormWorkspacePick lists workspaces in a select field (arrow keys, Enter to confirm).
func FormWorkspacePick(workspaces []WorkspacePick, selectedID *string) error {
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
	return huh.NewForm(
		huh.NewGroup(
			huh.NewSelect[string]().
				Title("Which workspace do you want to rename?").
				Description("↑/↓ to move, Enter to select.").
				Options(opts...).
				Value(selectedID).
				Validate(func(s string) error {
					if strings.TrimSpace(s) == "" {
						return errors.New("please select a workspace")
					}
					return nil
				}),
		).Title("Rename workspace").
			Description("No active workspace is set — pick one to rename."),
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
		),
	).Run()
}
