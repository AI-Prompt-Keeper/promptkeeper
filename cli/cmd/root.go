package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/promptkeeper/cli/cmd/store"
	"github.com/promptkeeper/cli/internal/config"
	"github.com/promptkeeper/cli/internal/ui"
	"github.com/spf13/cobra"
)

var (
	rootDebug          bool
	rootUseLocalConfig bool
)

var rootCmd = &cobra.Command{
	Use:   "prke",
	Short: "Prompt Keeper CLI — test the Secure AI Gateway",
	Long: `prke (promptkeeper) is a minimalist CLI for testing the Secure AI Gateway.
Use 'prke register' to create a user (management key for your default workspace),
'prke login' to sign in (session stored; mint a workspace management key if needed),
'prke workspace' to list/switch workspaces, 'prke set prke_key' to store keys for the active workspace,
'mint key' for execution-only keys, and 'prke exec' to run stored prompts.
By default, .prke-config.yaml is not read; use --debug and --use-local-config together to load it.`,
	RunE: runRoot,
}

func init() {
	if len(os.Args) > 0 {
		if base := strings.ToLower(filepath.Base(os.Args[0])); strings.Contains(base, "promptkeeper") {
			rootCmd.Use = "promptkeeper"
		}
	}
	config.CLIExeName = rootCmd.Use
	rootCmd.PersistentFlags().BoolVar(&rootDebug, "debug", false, "Enable debug logging")
	rootCmd.PersistentFlags().BoolVar(&rootUseLocalConfig, "use-local-config", false, "Read ~/.prke-config.yaml (only has effect when used with --debug)")
	rootCmd.CompletionOptions.DisableDefaultCmd = true
	rootCmd.SilenceUsage = true
	// Do not SilenceErrors: Cobra must print validation errors (e.g. wrong number of args) so user sees why the command failed
	rootCmd.AddCommand(store.StoreCmd)
}

func runRoot(c *cobra.Command, args []string) error {
	fmt.Println(ui.DefaultHelp(c.Use))
	return nil
}

func Execute() error {
	return rootCmd.Execute()
}
