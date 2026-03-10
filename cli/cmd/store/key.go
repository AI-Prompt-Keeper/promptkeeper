package store

import (
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

var keyCmd = &cobra.Command{
	Use:   "key [provider] [api_key]",
	Short: "Store a provider API key",
	Long:  "Stores a provider API key (e.g. OpenAI, Anthropic) in the Secure AI Gateway. Uses envelope encryption. Requires authentication (run 'prke register' or 'prke set prke_key' first). If arguments are omitted, an interactive form will guide you.",
	Args:  cobra.MaximumNArgs(2),
	RunE:  runStoreKey,
}

func init() {
	StoreCmd.AddCommand(keyCmd)
}

func runStoreKey(cmd *cobra.Command, args []string) error {
	provider := ""
	apiKey := ""
	if len(args) >= 1 {
		provider = strings.TrimSpace(args[0])
	}
	if len(args) >= 2 {
		apiKey = strings.TrimSpace(args[1])
	}
	if len(args) < 2 {
		if err := ui.FormStoreKey(&provider, &apiKey); err != nil {
			return err
		}
		provider = strings.TrimSpace(provider)
		apiKey = strings.TrimSpace(apiKey)
	}

	b := getBin(cmd)
	if err := validate.ValidateProvider(provider); err != nil {
		usererr.PrintFriendlyError(os.Stderr, b,
			"Invalid provider.",
			err.Error(),
			[]string{b + " store key openai sk-your-key", "# Provider must be alphanumeric, underscore, or hyphen (e.g. openai, anthropic)."})
		return err
	}
	if provider == "" {
		usererr.PrintFriendlyError(os.Stderr, b,
			"Provider is required.",
			"The first argument must be the provider name (e.g. openai, anthropic).",
			[]string{b + " store key openai sk-your-openai-key"})
		return fmt.Errorf("provider required")
	}
	if apiKey == "" {
		usererr.PrintFriendlyError(os.Stderr, b,
			"API key is required.",
			"The second argument must be the provider API key to store.",
			[]string{b + " store key openai sk-your-openai-key"})
		return fmt.Errorf("api_key required")
	}
	if err := validate.ValidateInputLength(apiKey, "api_key"); err != nil {
		usererr.PrintFriendlyError(os.Stderr, b,
			"API key is too long.",
			err.Error(),
			[]string{b + " store key openai sk-your-key"})
		return err
	}

	return DoStoreKey(cmd, provider, apiKey)
}

// DoStoreKey performs the API call to store a provider key.
// Caller must have validated provider and apiKey.
func DoStoreKey(cmd *cobra.Command, provider, apiKey string) error {
	useLocalConfig := getUseLocalConfig(cmd)
	cfg, err := config.New(useLocalConfig)
	if err != nil {
		usererr.PrintAPIError(os.Stderr, err.Error(), "Cannot load config (e.g. home directory).")
		return err
	}
	token, err := cfg.GetAPIKey()
	if err != nil {
		b := getBin(cmd)
		usererr.PrintAPIError(os.Stderr, err.Error(), "Run '"+b+" register' or '"+b+" set prke_key <key>' first to set your API key.")
		return err
	}
	baseURL := cfg.BaseURL()
	if getDebug(cmd) {
		fmt.Fprintln(os.Stderr, ui.DebugLine("base URL: %s", baseURL))
	}
	client := api.NewClient(baseURL, token)
	if getDebug(cmd) {
		client.DebugLog = os.Stderr
	}
	if err := client.PutKey(provider, apiKey); err != nil {
		usererr.PrintAPIError(os.Stderr, err.Error(), "Check that the provider is supported and enabled on the backend, and that you are authenticated.")
		return err
	}
	fmt.Fprintln(os.Stdout, ui.SuccessMessage("success"))
	return nil
}
