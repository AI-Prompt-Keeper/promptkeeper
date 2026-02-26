package cmd

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/promptkeeper/cli/internal/api"
	"github.com/promptkeeper/cli/internal/config"
	"github.com/promptkeeper/cli/internal/validate"
	"github.com/spf13/cobra"
)

var registerCmd = &cobra.Command{
	Use:   "register <email> <password>",
	Short: "Register a new user",
	Long:  "Registers a new user with the Secure AI Gateway. On success, stores the API key in the system vault. You must store the API key somewhere secure—it is returned only once.",
	Args:  cobra.ExactArgs(2),
	RunE:  runRegister,
}

func init() {
	rootCmd.AddCommand(registerCmd)
}

func runRegister(cmd *cobra.Command, args []string) error {
	email := args[0]
	password := args[1]

	if err := validate.ValidateEmail(email); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}
	if err := validate.ValidatePassword(password); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}

	cfg, err := config.New()
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}

	client := api.NewClient(cfg.BaseURL(), "")
	resp, err := client.Register(email, password, "")
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}

	apiKey, _ := resp["api_key"].(string)
	if apiKey != "" {
		if err := cfg.SetAPIKey(apiKey); err != nil {
			fmt.Fprintf(os.Stderr, "warning: could not store API key in vault: %v\n", err)
		}
	}

	// Pretty-print response
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	if err := enc.Encode(resp); err != nil {
		fmt.Fprintf(os.Stderr, "error encoding response: %v\n", err)
		return err
	}

	if apiKey != "" {
		fmt.Fprintln(os.Stdout)
		fmt.Fprintln(os.Stdout, "┌─────────────────────────────────────────────────────────────────┐")
		fmt.Fprintln(os.Stdout, "│  ⚠️  IMPORTANT: Store your API key securely!                      │")
		fmt.Fprintln(os.Stdout, "│     It is returned only once. The CLI has saved it for you.       │")
		fmt.Fprintln(os.Stdout, "│     Future requests will use this key automatically.             │")
		fmt.Fprintln(os.Stdout, "└─────────────────────────────────────────────────────────────────┘")
	}

	return nil
}
