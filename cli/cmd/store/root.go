package store

import (
	"os"
	"strings"

	"github.com/promptkeeper/cli/internal/ui"
	"github.com/promptkeeper/cli/internal/usererr"
	"github.com/promptkeeper/cli/internal/validate"
	"github.com/spf13/cobra"
)

var StoreCmd = &cobra.Command{
	Use:   "store",
	Short: "Store secrets (keys, prompts)",
	Long:  "Store provider API keys or prompt templates in the Secure AI Gateway. Run without a subcommand to be guided through the choice and steps.",
	RunE:  runStoreGuided,
}

// getUseLocalConfig reads --debug and --use-local-config from root; config file is used only when both are set.
func getUseLocalConfig(cmd *cobra.Command) bool {
	root := cmd.Root()
	debug, _ := root.PersistentFlags().GetBool("debug")
	useLocal, _ := root.PersistentFlags().GetBool("use-local-config")
	return debug && useLocal
}

// getDebug returns whether --debug was set on the root command.
func getDebug(cmd *cobra.Command) bool {
	debug, _ := cmd.Root().PersistentFlags().GetBool("debug")
	return debug
}

// getBin returns the CLI name (prke or promptkeeper) for use in error examples.
func getBin(cmd *cobra.Command) string {
	return cmd.Root().Use
}

func runStoreGuided(cmd *cobra.Command, args []string) error {
	var kind ui.StoreKind
	if err := ui.FormStoreKind(&kind); err != nil {
		return err
	}
	switch kind {
	case ui.StoreKindKey:
		var provider, apiKey string
		if err := ui.FormStoreKey(&provider, &apiKey); err != nil {
			return err
		}
		provider = strings.TrimSpace(provider)
		apiKey = strings.TrimSpace(apiKey)
		return DoStoreKey(cmd, provider, apiKey, "")
	case ui.StoreKindPrompt:
		var title, promptInput, provider, model string
		if err := ui.FormStorePrompt(&title, &promptInput, &provider, &model); err != nil {
			return err
		}
		title = strings.TrimSpace(title)
		promptInput = strings.TrimSpace(promptInput)
		provider = strings.TrimSpace(provider)
		model = strings.TrimSpace(model)
		var promptValue string
		if path, err := validate.SafeFilePath(promptInput); err == nil {
			data, err := os.ReadFile(path)
			if err != nil {
				usererr.PrintAPIError(os.Stderr, "Cannot read file: "+err.Error(), "Use a path to an existing file.")
				return err
			}
			promptValue = string(data)
		} else {
			promptValue = promptInput
		}
		return DoStorePrompt(cmd, title, promptValue, provider, model, "")
	default:
		return nil
	}
}
