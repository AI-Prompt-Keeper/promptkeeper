package cmd

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/promptkeeper/cli/internal/api"
	"github.com/promptkeeper/cli/internal/config"
	"github.com/promptkeeper/cli/internal/ui"
	"github.com/promptkeeper/cli/internal/usererr"
	"github.com/spf13/cobra"
)

var listCmd = &cobra.Command{
	Use:   "list",
	Short: "List resources from the gateway",
}

var listPromptsCmd = &cobra.Command{
	Use:   "prompts",
	Short: "List stored prompt titles (production deployments)",
	Long:  "Calls GET /v1/list-prompts. Returns sorted function names for your workspace (and global scope). Accepts management or execution client API keys.",
	RunE:  runListPrompts,
}

func init() {
	rootCmd.AddCommand(listCmd)
	listCmd.AddCommand(listPromptsCmd)
}

func runListPrompts(_ *cobra.Command, _ []string) error {
	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Cannot load config (e.g. home directory).")
		return err
	}
	baseURL := cfg.BaseURL()
	if rootDebug {
		fmt.Fprintln(os.Stderr, ui.DebugLine("base URL: %s", baseURL))
	}
	token, err := cfg.GetAPIKey()
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Run '"+bin()+" register' or '"+bin()+" set prke_key <key>' first.")
		return err
	}
	client := api.NewClient(baseURL, token)
	if rootDebug {
		client.DebugLog = os.Stderr
	}
	resp, err := client.ListPrompts()
	if err != nil {
		usererr.PrintAPIError(os.Stderr, err.Error(), usererr.MergeAPIHint(err,
			"Check that the backend is reachable and you are authenticated."))
		return err
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	if err := enc.Encode(resp); err != nil {
		PrintAPIError(os.Stderr, "Failed to print response: "+err.Error(), "")
		return err
	}
	return nil
}
