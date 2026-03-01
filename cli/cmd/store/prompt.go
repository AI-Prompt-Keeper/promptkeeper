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

var storePromptModel string

var promptCmd = &cobra.Command{
	Use:   "prompt <prompt_title> <prompt_value|file_path> [provider]",
	Short: "Store a prompt template",
	Long:  "Stores a prompt template. The second argument can be the prompt text itself or a path to a file containing the prompt. Optional provider sets the default for this function. Use --model to set the preferred LLM model (e.g. gpt-4o, claude-3-5-sonnet-20240620).",
	Args:  cobra.MinimumNArgs(2),
	RunE:  runStorePrompt,
}

func init() {
	StoreCmd.AddCommand(promptCmd)
	promptCmd.Flags().StringVar(&storePromptModel, "model", "", "Preferred LLM model for this prompt (e.g. gpt-4o, claude-3-5-sonnet)")
}

func runStorePrompt(cmd *cobra.Command, args []string) error {
	title := strings.TrimSpace(args[0])
	promptInput := strings.TrimSpace(args[1])
	provider := ""
	if len(args) >= 3 {
		provider = strings.TrimSpace(args[2])
	}

	if err := validate.ValidatePromptTitle(title); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}
	if err := validate.ValidateProvider(provider); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}
	if err := validate.ValidateModel(storePromptModel); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}

	var promptValue string
	if path, err := validate.SafeFilePath(promptInput); err == nil {
		data, err := os.ReadFile(path)
		if err != nil {
			fmt.Fprintf(os.Stderr, "error: cannot read file: %v\n", err)
			return err
		}
		promptValue = string(data)
	} else {
		promptValue = promptInput
	}

	if err := validate.ValidateInputLength(promptValue, "prompt"); err != nil {
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
	if err := client.PutPrompt(title, promptValue, provider, storePromptModel); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}

	fmt.Fprintln(os.Stdout, "success")
	return nil
}
