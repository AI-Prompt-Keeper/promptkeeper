package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/promptkeeper/cli/internal/api"
	"github.com/promptkeeper/cli/internal/config"
	"github.com/promptkeeper/cli/internal/ui"
	"github.com/promptkeeper/cli/internal/usererr"
	"github.com/spf13/cobra"
)

var workspaceCmd = &cobra.Command{
	Use:   "workspace",
	Short: "List, create, and switch workspaces",
	Long:  "Manage Prompt Keeper workspaces (GET/POST /v1/workspaces). Execution and vault operations use the active workspace and keys stored per workspace in the OS secure store.",
}

var workspaceListJSON bool

var workspaceListCmd = &cobra.Command{
	Use:   "list",
	Short: "List workspaces you belong to",
	RunE:  runWorkspaceList,
}

var workspaceSwitchMint bool

var workspaceSwitchCmd = &cobra.Command{
	Use:   "switch <name_or_id>",
	Short: "Set the active workspace (UUID, name, or \"default\" for your personal workspace)",
	Args:  cobra.ExactArgs(1),
	RunE: func(_ *cobra.Command, args []string) error {
		return doWorkspaceSwitch(args[0])
	},
}

var workspaceCreateCmd = &cobra.Command{
	Use:   "create <name>",
	Short: "Create a workspace (returns a new management key once)",
	Args:  cobra.ExactArgs(1),
	RunE:  runWorkspaceCreate,
}

var workspaceMintMgtCmd = &cobra.Command{
	Use:   "mint-mgt [label]",
	Short: "Mint a new management API key for the active workspace",
	Long:  "Calls POST /v1/workspaces/:id/mgt-key. Requires a session (after login) or an existing management key. The new key is stored in the vault for the active workspace.",
	Args:  cobra.MaximumNArgs(1),
	RunE:  runWorkspaceMintMgt,
}

var workspaceCurrentCmd = &cobra.Command{
	Use:   "current",
	Short: "Show the active workspace id and personal workspace id (from state)",
	RunE:  runWorkspaceCurrent,
}

var workspaceEditCmd = &cobra.Command{
	Use:   "edit <name_or_id> <new_name>",
	Short: "Rename a workspace (not allowed for the signup default workspace)",
	Args:  cobra.ExactArgs(2),
	RunE:  runWorkspaceEdit,
}

var workspaceDeleteCmd = &cobra.Command{
	Use:   "delete <name_or_id>",
	Short: "Delete a workspace (not allowed for the signup default or last workspace)",
	Args:  cobra.ExactArgs(1),
	RunE:  runWorkspaceDelete,
}

// Root-level alias for tests / short usage.
var mintMgtRootCmd = &cobra.Command{
	Use:   "mint-mgt [label]",
	Short: "Mint a management key for the active workspace (same as workspace mint-mgt)",
	Args:  cobra.MaximumNArgs(1),
	RunE:  runWorkspaceMintMgt,
}

func init() {
	rootCmd.AddCommand(workspaceCmd)
	rootCmd.AddCommand(mintMgtRootCmd)
	workspaceCmd.AddCommand(workspaceListCmd, workspaceSwitchCmd, workspaceCreateCmd, workspaceMintMgtCmd, workspaceCurrentCmd, workspaceEditCmd, workspaceDeleteCmd)
	workspaceListCmd.Flags().BoolVar(&workspaceListJSON, "json", false, "Print JSON (default: names only, one per line)")
	workspaceSwitchCmd.Flags().BoolVar(&workspaceSwitchMint, "mint", false, "After switching, mint a management key if none is stored locally")
}

func runWorkspaceList(_ *cobra.Command, _ []string) error {
	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Cannot load config.")
		return err
	}
	token, err := cfg.AuthSessionOrManagement()
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	baseURL := cfg.BaseURL()
	if rootDebug {
		fmt.Fprintln(os.Stderr, ui.DebugLine("base URL: %s", baseURL))
	}
	client := api.NewClient(baseURL, token)
	if rootDebug {
		client.DebugLog = os.Stderr
	}
	resp, err := client.ListWorkspaces()
	if err != nil {
		usererr.PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	if workspaceListJSON {
		enc := json.NewEncoder(os.Stdout)
		enc.SetIndent("", "  ")
		return enc.Encode(resp)
	}
	for _, w := range resp.Workspaces {
		fmt.Fprintln(os.Stdout, w.Name)
	}
	return nil
}

func resolveWorkspaceSpecifier(cfg *config.Config, client *api.Client, raw string) (string, error) {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return "", fmt.Errorf("workspace is required")
	}
	if strings.EqualFold(raw, "default") {
		if p := cfg.PersonalWorkspaceID(); p != "" {
			return p, nil
		}
		id, err := api.FindWorkspaceIDByName(client, "Personal")
		if err == nil {
			return id, nil
		}
		return "", fmt.Errorf("could not resolve default workspace; run login or register first")
	}
	if isLikelyUUID(raw) {
		return raw, nil
	}
	return api.FindWorkspaceIDByName(client, raw)
}

func doWorkspaceSwitch(raw string) error {
	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Cannot load config.")
		return err
	}
	token, err := cfg.AuthSessionOrManagement()
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	baseURL := cfg.BaseURL()
	if rootDebug {
		fmt.Fprintln(os.Stderr, ui.DebugLine("base URL: %s", baseURL))
	}
	client := api.NewClient(baseURL, token)
	if rootDebug {
		client.DebugLog = os.Stderr
	}
	id, err := resolveWorkspaceSpecifier(cfg, client, raw)
	if err != nil {
		PrintFriendlyError(os.Stderr, "Unknown workspace.", err.Error(), []string{bin() + " workspace list"})
		return err
	}
	if err := cfg.SetCurrentWorkspaceID(id); err != nil {
		return err
	}
	fmt.Fprintln(os.Stdout, ui.SuccessMessage("Active workspace set to "+id+"."))
	if cfg.ShouldPromptMintMgtForCurrentWorkspace() {
		fmt.Fprintln(os.Stdout)
		fmt.Fprintln(os.Stdout, ui.WarningBlock("Next step",
			"No management key is stored for this workspace (existing keys cannot be retrieved from the server).",
			"Mint one:",
			bin()+" workspace mint-mgt"))
		if workspaceSwitchMint {
			return runWorkspaceMintMgt(workspaceMintMgtCmd, []string{})
		}
	}
	return nil
}

func runWorkspaceCreate(_ *cobra.Command, args []string) error {
	name := strings.TrimSpace(args[0])
	if name == "" {
		return fmt.Errorf("name is required")
	}
	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Cannot load config.")
		return err
	}
	token, err := cfg.AuthSessionOrManagement()
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	baseURL := cfg.BaseURL()
	if rootDebug {
		fmt.Fprintln(os.Stderr, ui.DebugLine("base URL: %s", baseURL))
	}
	client := api.NewClient(baseURL, token)
	if rootDebug {
		client.DebugLog = os.Stderr
	}
	resp, err := client.CreateWorkspace(name)
	if err != nil {
		usererr.PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	if resp.APIKey != "" {
		_ = cfg.SetCurrentWorkspaceID(resp.ID)
		cfg.SetWorkspaceManagementKey(resp.ID, resp.APIKey)
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	if err := enc.Encode(resp); err != nil {
		return err
	}
	fmt.Fprintln(os.Stdout)
	fmt.Fprintln(os.Stdout, ui.WarningBlock("⚠️  IMPORTANT: Management key",
		"Shown once. Stored in the vault for workspace "+resp.ID+".",
		"Active workspace is set to this new workspace."))
	return nil
}

func runWorkspaceMintMgt(_ *cobra.Command, args []string) error {
	label := "CLI"
	if len(args) >= 1 {
		label = strings.TrimSpace(args[0])
	}
	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Cannot load config.")
		return err
	}
	ws := cfg.CurrentWorkspaceID()
	if ws == "" {
		PrintAPIError(os.Stderr, "no active workspace", bin()+" workspace list && "+bin()+" workspace switch <name_or_id>")
		return fmt.Errorf("no active workspace")
	}
	token, err := cfg.AuthForMintWorkspaceMgtKey()
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	baseURL := cfg.BaseURL()
	if rootDebug {
		fmt.Fprintln(os.Stderr, ui.DebugLine("base URL: %s", baseURL))
	}
	client := api.NewClient(baseURL, token)
	if rootDebug {
		client.DebugLog = os.Stderr
	}
	out, err := client.MintWorkspaceManagementKey(ws, label)
	if err != nil {
		usererr.PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	if out.APIKey != "" {
		cfg.SetWorkspaceManagementKey(ws, out.APIKey)
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	if err := enc.Encode(out); err != nil {
		return err
	}
	fmt.Fprintln(os.Stdout)
	fmt.Fprintln(os.Stdout, ui.WarningBlock("⚠️  IMPORTANT: Management key",
		"Shown once. Saved to the vault for the active workspace when storage is available."))
	return nil
}

func runWorkspaceCurrent(_ *cobra.Command, _ []string) error {
	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Cannot load config.")
		return err
	}
	out := map[string]string{
		"current_workspace_id":  cfg.CurrentWorkspaceID(),
		"personal_workspace_id": cfg.PersonalWorkspaceID(),
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	return enc.Encode(out)
}

func runWorkspaceEdit(_ *cobra.Command, args []string) error {
	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	token, err := cfg.AuthSessionOrManagement()
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	baseURL := cfg.BaseURL()
	client := api.NewClient(baseURL, token)
	if rootDebug {
		client.DebugLog = os.Stderr
	}
	wsID, err := resolveWorkspaceSpecifier(cfg, client, args[0])
	if err != nil {
		return err
	}
	newName := strings.TrimSpace(args[1])
	out, err := client.UpdateWorkspace(wsID, newName)
	if err != nil {
		usererr.PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	return enc.Encode(out)
}

func runWorkspaceDelete(_ *cobra.Command, args []string) error {
	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	token, err := cfg.AuthSessionOrManagement()
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	baseURL := cfg.BaseURL()
	client := api.NewClient(baseURL, token)
	if rootDebug {
		client.DebugLog = os.Stderr
	}
	wsID, err := resolveWorkspaceSpecifier(cfg, client, args[0])
	if err != nil {
		return err
	}
	cur := cfg.CurrentWorkspaceID()
	if err := client.DeleteWorkspace(wsID); err != nil {
		usererr.PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	fmt.Fprintln(os.Stdout, ui.SuccessMessage("Workspace deleted."))
	if cur == wsID {
		p := cfg.PersonalWorkspaceID()
		if p != "" {
			_ = cfg.SetCurrentWorkspaceID(p)
			fmt.Fprintln(os.Stdout, ui.Body.Render("Switched active workspace to personal/default ("+p+")."))
		}
	}
	return nil
}

func isLikelyUUID(s string) bool {
	s = strings.TrimSpace(s)
	if len(s) != 36 {
		return false
	}
	for i, c := range s {
		switch i {
		case 8, 13, 18, 23:
			if c != '-' {
				return false
			}
		default:
			if c >= '0' && c <= '9' || c >= 'a' && c <= 'f' || c >= 'A' && c <= 'F' {
				continue
			}
			return false
		}
	}
	return true
}
