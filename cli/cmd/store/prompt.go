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

var storePromptModel string

var promptCmd = &cobra.Command{
	Use:   "prompt [prompt_title] [prompt_value|file_path] [provider]",
	Short: "Store a prompt template",
	Long:  "Stores a prompt template. The second argument can be the prompt text itself or a path to a file. Optional provider sets the default. Use --model for preferred LLM model. If arguments are omitted, an interactive form will guide you.",
	Args:  cobra.MaximumNArgs(3),
	RunE:  runStorePrompt,
}

func init() {
	StoreCmd.AddCommand(promptCmd)
	promptCmd.Flags().StringVar(&storePromptModel, "model", "", "Preferred LLM model for this prompt (e.g. gpt-4o, claude-3-5-sonnet)")
}

func runStorePrompt(cmd *cobra.Command, args []string) error {
	title := ""
	promptInput := ""
	provider := ""
	model := storePromptModel
	if len(args) >= 1 {
		title = strings.TrimSpace(args[0])
	}
	if len(args) >= 2 {
		promptInput = strings.TrimSpace(args[1])
	}
	if len(args) >= 3 {
		provider = strings.TrimSpace(args[2])
	}
	if len(args) < 2 {
		if err := ui.FormStorePrompt(&title, &promptInput, &provider, &model); err != nil {
			return err
		}
		title = strings.TrimSpace(title)
		promptInput = strings.TrimSpace(promptInput)
		provider = strings.TrimSpace(provider)
		model = strings.TrimSpace(model)
	} else {
		model = storePromptModel
	}
	b := getBin(cmd)

	if err := validate.ValidatePromptTitle(title); err != nil {
		usererr.PrintFriendlyError(os.Stderr, b,
			"Invalid prompt title.",
			err.Error(),
			[]string{b + " store prompt my_prompt \"Hello {{name}}!\"", "# Use letters, numbers, spaces, underscore, or hyphen."})
		return err
	}
	if err := validate.ValidateProvider(provider); err != nil {
		usererr.PrintFriendlyError(os.Stderr, b,
			"Invalid provider.",
			err.Error(),
			[]string{b + " store prompt my_prompt \"Your prompt\" openai"})
		return err
	}
	if err := validate.ValidateModel(model); err != nil {
		usererr.PrintFriendlyError(os.Stderr, b,
			"Invalid model.",
			err.Error(),
			[]string{b + " store prompt my_prompt \"Your prompt\" --model gpt-4o"})
		return err
	}

	var promptValue string
	if path, err := validate.SafeFilePath(promptInput); err == nil {
		data, err := os.ReadFile(path)
		if err != nil {
			usererr.PrintAPIError(os.Stderr, "Cannot read file: "+err.Error(), "Use a path to an existing file under the current directory, or pass the prompt text as the second argument.")
			return err
		}
		promptValue = string(data)
	} else {
		promptValue = promptInput
	}

	if err := validate.ValidateInputLength(promptValue, "prompt"); err != nil {
		usererr.PrintFriendlyError(os.Stderr, b,
			"Prompt content is too long.",
			err.Error(),
			[]string{b + " store prompt my_prompt \"Short prompt\"", "# Or use a file: " + b + " store prompt my_prompt ./prompt.txt"})
		return err
	}

	return DoStorePrompt(cmd, title, promptValue, provider, model)
}

// DoStorePrompt performs the API call to store a prompt. Caller must have resolved promptInput to promptValue (file read if path).
func DoStorePrompt(cmd *cobra.Command, title, promptValue, provider, model string) error {
	b := getBin(cmd)
	useLocalConfig := getUseLocalConfig(cmd)
	cfg, err := config.New(useLocalConfig)
	if err != nil {
		usererr.PrintAPIError(os.Stderr, err.Error(), "Cannot load config (e.g. home directory).")
		return err
	}
	token, err := cfg.GetAPIKey()
	if err != nil {
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
	if err := client.PutPrompt(title, promptValue, provider, model); err != nil {
		usererr.PrintAPIError(os.Stderr, err.Error(), "Check that the provider is supported and that you are authenticated.")
		return err
	}
	fmt.Fprintln(os.Stdout, ui.SuccessMessage("success"))
	return nil
}
