package store

import (
	"fmt"
	"os"
	"strings"

	"github.com/promptkeeper/cli/internal/api"
	"github.com/promptkeeper/cli/internal/config"
	"github.com/promptkeeper/cli/internal/validate"
	"github.com/spf13/cobra"
)

var keyCmd = &cobra.Command{
	Use:   "key <provider> <api_key>",
	Short: "Store a provider API key",
	Long:  "Stores a provider API key (e.g. OpenAI, Anthropic) in the Secure AI Gateway. Uses envelope encryption. Requires authentication (run 'prke register' or 'prke set prke_key' first).",
	Args:  cobra.ExactArgs(2),
	RunE:  runStoreKey,
}

func init() {
	StoreCmd.AddCommand(keyCmd)
}

func runStoreKey(cmd *cobra.Command, args []string) error {
	provider := strings.TrimSpace(args[0])
	apiKey := strings.TrimSpace(args[1])

	if err := validate.ValidateProvider(provider); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}
	if provider == "" {
		fmt.Fprintf(os.Stderr, "error: provider is required\n")
		return fmt.Errorf("provider required")
	}
	if apiKey == "" {
		fmt.Fprintf(os.Stderr, "error: api_key is required\n")
		return fmt.Errorf("api_key required")
	}
	if err := validate.ValidateInputLength(apiKey, "api_key"); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}

	cfg, err := config.New()
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}

	token, err := cfg.GetAPIKey()
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}

	client := api.NewClient(cfg.BaseURL(), token)
	if err := client.PutKey(provider, apiKey); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}

	fmt.Fprintln(os.Stdout, "success")
	return nil
}
