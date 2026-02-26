package cmd

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/promptkeeper/cli/cmd/store"
	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "prke",
	Short: "Prompt Keeper CLI — test the Secure AI Gateway",
	Long: `prke (promptkeeper) is a minimalist CLI for testing the Secure AI Gateway.
Use 'prke register' to create a user, 'prke set prke_key' to configure your API key,
and 'prke exec' to run prompts.`,
}

func init() {
	if len(os.Args) > 0 {
		if base := strings.ToLower(filepath.Base(os.Args[0])); strings.Contains(base, "promptkeeper") {
			rootCmd.Use = "promptkeeper"
		}
	}
	rootCmd.CompletionOptions.DisableDefaultCmd = true
	rootCmd.SilenceUsage = true
	rootCmd.SilenceErrors = true
	rootCmd.AddCommand(store.StoreCmd)
}

func Execute() error {
	return rootCmd.Execute()
}
