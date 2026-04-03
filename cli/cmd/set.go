package cmd

import (
	"fmt"
	"os"
	"strings"

	"github.com/promptkeeper/cli/internal/config"
	"github.com/promptkeeper/cli/internal/ui"
	"github.com/spf13/cobra"
)

var setCmd = &cobra.Command{
	Use:   "set",
	Short: "Set configuration values",
	Long:  "Store the API key in the system vault (keyring) only. Config file is not used for the token.",
}

var setPkreKeyCmd = &cobra.Command{
	Use:   "prke_key [key]",
	Short: "Store API key for subsequent requests",
	Long:  "Stores the Prompt Keeper client key in the OS secure store for the **active workspace** (see `workspace switch`). Accepts management keys (pk_mgt_live_...), execution keys (pk_exe_live_...), or a login session token (64 hex). If no active workspace is set, keys are stored in a legacy single slot. If key is omitted, an interactive form will prompt you.",
	Args:  cobra.MaximumNArgs(1),
	RunE:  runSetPkreKey,
}

func init() {
	rootCmd.AddCommand(setCmd)
	setCmd.AddCommand(setPkreKeyCmd)
}

func runSetPkreKey(cmd *cobra.Command, args []string) error {
	key := ""
	if len(args) >= 1 {
		key = strings.TrimSpace(args[0])
	}
	if key == "" {
		if err := ui.FormSetAPIKey(&key); err != nil {
			return err
		}
		key = strings.TrimSpace(key)
	}
	if key == "" {
		PrintFriendlyError(os.Stderr,
			"API key cannot be empty.",
			"The key is required so the CLI can authenticate with the backend.",
			[]string{bin() + " set prke_key pk_your_api_key_here"})
		return fmt.Errorf("invalid api key")
	}

	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Cannot load config (e.g. home directory).")
		return err
	}

	if err := cfg.SetAPIKey(key); err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Check that the config file path is writable.")
		return err
	}

	fmt.Fprintln(os.Stdout, ui.SuccessMessage("API key stored successfully."))
	return nil
}
