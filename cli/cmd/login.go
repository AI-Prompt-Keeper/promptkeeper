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
	"github.com/promptkeeper/cli/internal/validate"
	"github.com/spf13/cobra"
)

var loginCmd = &cobra.Command{
	Use:   "login [email] [password]",
	Short: "Sign in and store a session token",
	Long:  "POST /v1/auth/login with your email and password. On success, stores the session token and the new management key for your default workspace (per backend). Sets the active workspace from default_workspace_id. If email or password is omitted, an interactive form will guide you.",
	Args:  cobra.MaximumNArgs(2),
	RunE:  runLogin,
}

func init() {
	rootCmd.AddCommand(loginCmd)
}

func runLogin(_ *cobra.Command, args []string) error {
	email := ""
	password := ""
	if len(args) >= 1 {
		first := strings.TrimSpace(args[0])
		if len(args) == 1 {
			if err := validate.ValidateEmail(first); err == nil {
				email = first
			} else {
				password = first
			}
		} else {
			email = first
		}
	}
	if len(args) >= 2 {
		password = strings.TrimSpace(args[1])
	}

	if len(args) < 2 || strings.TrimSpace(email) == "" || strings.TrimSpace(password) == "" {
		if err := ui.FormLogin(&email, &password); err != nil {
			return err
		}
		email = strings.TrimSpace(email)
		password = strings.TrimSpace(password)
	}

	if err := validate.ValidateEmail(email); err != nil {
		PrintFriendlyError(os.Stderr,
			"Invalid email.",
			err.Error(),
			[]string{bin() + " login you@example.com YourSecurePassword123"})
		return err
	}
	if err := validate.ValidatePassword(password); err != nil {
		PrintFriendlyError(os.Stderr,
			"Invalid password.",
			err.Error(),
			[]string{bin() + " login you@example.com YourSecurePassword123", "# Use at least 12 characters."})
		return err
	}

	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Cannot load config (e.g. home directory).")
		return err
	}

	baseURL := cfg.BaseURL()
	if rootDebug {
		fmt.Fprintln(os.Stderr, ui.DebugLine("base URL: %s", baseURL))
	}
	client := api.NewClient(baseURL, "")
	if rootDebug {
		client.DebugLog = os.Stderr
	}

	resp, err := client.Login(email, password)
	if err != nil {
		hint := loginErrorHint(err)
		PrintAPIError(os.Stderr, err.Error(), usererr.MergeAPIHint(err, hint))
		return err
	}

	if resp.Token != "" {
		cfg.SetSessionToken(resp.Token)
	}
	if resp.User.ID != "" && resp.DefaultWorkspaceID != "" {
		if err := cfg.SetPersonalWorkspaceAndSessionUser(resp.DefaultWorkspaceID, resp.User.ID); err != nil {
			fmt.Fprintln(os.Stderr, ui.WarningBlock("Warning", "Could not save workspace state: "+err.Error()))
		}
		if err := cfg.SetCurrentWorkspaceID(resp.DefaultWorkspaceID); err != nil {
			fmt.Fprintln(os.Stderr, ui.WarningBlock("Warning", "Could not set active workspace: "+err.Error()))
		}
		if resp.APIKey != "" {
			cfg.SetWorkspaceManagementKey(resp.DefaultWorkspaceID, resp.APIKey)
		}
	}

	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	if err := enc.Encode(resp); err != nil {
		PrintAPIError(os.Stderr, "Failed to print response: "+err.Error(), "")
		return err
	}

	if resp.Token != "" {
		fmt.Fprintln(os.Stdout)
		fmt.Fprintln(os.Stdout, ui.WarningBlock("⚠️  IMPORTANT: Session token",
			"Treat it like a password. Saved to the OS secure store when available.",
			"Expires at: "+resp.ExpiresAt))
	}
	ws := cfg.CurrentWorkspaceID()
	if ws != "" && cfg.GetWorkspaceManagementKey(ws) == "" && resp.APIKey == "" {
		fmt.Fprintln(os.Stderr)
		fmt.Fprintln(os.Stderr, ui.WarningBlock("Next step",
			"No management key was returned or stored.",
			"Mint one for this workspace:",
			bin()+" workspace mint-mgt"))
	}

	return nil
}

func loginErrorHint(err error) string {
	if err == nil {
		return ""
	}
	s := strings.ToLower(err.Error())
	if strings.Contains(s, "401") || strings.Contains(s, "invalid email or password") {
		return "Check your email and password. If you do not have an account yet, run '" + bin() + " register' first."
	}
	if strings.Contains(s, "500") || strings.Contains(s, "login failed") {
		return "The server could not complete login. Try again later or check backend logs."
	}
	return "Check that the backend is reachable and DATABASE_URL is configured for login."
}
