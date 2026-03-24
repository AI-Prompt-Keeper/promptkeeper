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

var mintCmd = &cobra.Command{
	Use:   "mint",
	Short: "Mint API tokens (execution-only client keys)",
}

var mintKeyCmd = &cobra.Command{
	Use:   "key [label]",
	Short: "Mint an execution-only client API key (pk_exe_live_...)",
	Long:  "",
	Args:  cobra.MaximumNArgs(1),
	RunE:  runMintKey,
}

func init() {
	mintKeyCmd.Long = "Calls POST /v1/auth/api-tokens using the Prompt Keeper API key from your vault (must be a management key pk_mgt_live_..., or use a session token from login — not an execution-only key).\n\n" +
		"The new key is shown only once. Use it in apps that should only run prompts, not manage provider keys or prompt templates.\n\n" +
		"Example:  " + bin() + " mint key \"CI runner\""
	rootCmd.AddCommand(mintCmd)
	mintCmd.AddCommand(mintKeyCmd)
}

func runMintKey(cmd *cobra.Command, args []string) error {
	label := ""
	if len(args) >= 1 {
		label = strings.TrimSpace(args[0])
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
	token, err := cfg.GetAPIKey()
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Run '"+bin()+" register' or '"+bin()+" set prke_key <management key>' first.")
		return err
	}
	client := api.NewClient(baseURL, token)
	if rootDebug {
		client.DebugLog = os.Stderr
	}
	resp, err := client.MintExecutionToken(label)
	if err != nil {
		usererr.PrintAPIError(os.Stderr, err.Error(), usererr.MergeAPIHint(err,
			"Check that you use a management key (pk_mgt_live_...), not an execution-only key."))
		return err
	}

	out := map[string]interface{}{
		"api_key": resp.APIKey,
		"scope":   resp.Scope,
		"label":   resp.Label,
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	if err := enc.Encode(out); err != nil {
		PrintAPIError(os.Stderr, "Failed to print response: "+err.Error(), "")
		return err
	}
	fmt.Fprintln(os.Stdout)
	fmt.Fprintln(os.Stdout, ui.WarningBlock("⚠️  IMPORTANT: Store this execution key securely",
		"It is shown only once. Prefer a secret manager or environment variable.",
		"Do not commit it to source control. To save it in the CLI vault (replaces your current vault key), run: "+bin()+" set prke_key <key>"))
	return nil
}
