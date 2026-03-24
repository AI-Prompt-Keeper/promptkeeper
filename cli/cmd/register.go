package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/promptkeeper/cli/internal/api"
	"github.com/promptkeeper/cli/internal/config"
	"github.com/promptkeeper/cli/internal/ui"
	"github.com/promptkeeper/cli/internal/validate"
	"github.com/spf13/cobra"
)

var registerCmd = &cobra.Command{
	Use:   "register [email] [password]",
	Short: "Register a new user",
	Long:  "Registers a new user with the Secure AI Gateway. On success, stores the management API key (pk_mgt_live_...) in the system vault. You must persist it somewhere secure—it is returned only once. If email or password are omitted, an interactive form will guide you.",
	Args: cobra.MaximumNArgs(2),
	RunE: runRegister,
}

func init() {
	rootCmd.AddCommand(registerCmd)
}

func runRegister(cmd *cobra.Command, args []string) error {
	email := ""
	password := ""
	if len(args) >= 1 {
		email = strings.TrimSpace(args[0])
	}
	if len(args) >= 2 {
		password = strings.TrimSpace(args[1])
	}
	if len(args) < 2 {
		if err := ui.FormRegister(&email, &password); err != nil {
			return err
		}
		email = strings.TrimSpace(email)
		password = strings.TrimSpace(password)
	}

	if err := validate.ValidateEmail(email); err != nil {
		PrintFriendlyError(os.Stderr,
			"Invalid email.",
			err.Error(),
			[]string{bin() + " register you@example.com YourSecurePassword123"})
		return err
	}
	if err := validate.ValidatePassword(password); err != nil {
		PrintFriendlyError(os.Stderr,
			"Invalid password.",
			err.Error(),
			[]string{bin() + " register you@example.com YourSecurePassword123", "# Use at least 12 characters."})
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
	fmt.Fprintln(os.Stderr, ui.Body.Render("Solving proof-of-work..."))
	resp, err := client.Register(email, password, "")
	if err != nil {
		hint := "Check that the backend is reachable and the email is not already registered."
		if strings.Contains(err.Error(), "proof-of-work") {
			hint = "Get a fresh challenge by running the command again."
		}
		PrintAPIError(os.Stderr, err.Error(), hint)
		return err
	}

	apiKey, _ := resp["api_key"].(string)
	if apiKey != "" {
		if err := cfg.SetAPIKey(apiKey); err != nil {
			fmt.Fprintln(os.Stderr, ui.WarningBlock("Warning", "Could not store API key in vault: "+err.Error()))
		}
	}

	// Pretty-print response once (full JSON including api_key on stdout; not duplicated in API debug logs)
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	if err := enc.Encode(resp); err != nil {
		PrintAPIError(os.Stderr, "Failed to print response: "+err.Error(), "")
		return err
	}

	if apiKey != "" {
		fmt.Fprintln(os.Stdout)
		fmt.Fprintln(os.Stdout, ui.WarningBlock("⚠️  IMPORTANT: Store your API key securely",
			"It is returned only once. The CLI has saved it for you.",
			"Future requests will use this key automatically."))
	}

	return nil
}
