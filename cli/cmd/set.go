package cmd

import (
	"fmt"
	"os"
	"strings"

	"github.com/promptkeeper/cli/internal/config"
	"github.com/spf13/cobra"
)

var setCmd = &cobra.Command{
	Use:   "set",
	Short: "Set configuration values",
	Long:  "Store configuration in the system vault and ~/.pv-config.yaml",
}

var setPkreKeyCmd = &cobra.Command{
	Use:   "prke_key <key>",
	Short: "Store API key for subsequent requests",
	Long:  "Stores the API key in the system vault. The CLI will use this key for all authenticated requests (store key, store prompt, exec).",
	Args:  cobra.ExactArgs(1),
	RunE:  runSetPkreKey,
}

func init() {
	rootCmd.AddCommand(setCmd)
	setCmd.AddCommand(setPkreKeyCmd)
}

func runSetPkreKey(cmd *cobra.Command, args []string) error {
	key := strings.TrimSpace(args[0])
	if key == "" {
		fmt.Fprintf(os.Stderr, "error: API key cannot be empty\n")
		return fmt.Errorf("invalid api key")
	}

	cfg, err := config.New()
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}

	if err := cfg.SetAPIKey(key); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}

	fmt.Fprintln(os.Stdout, "API key stored successfully.")
	return nil
}
