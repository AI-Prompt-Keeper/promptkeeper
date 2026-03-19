package cmd

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/promptkeeper/cli/internal/api"
	"github.com/promptkeeper/cli/internal/config"
	"github.com/promptkeeper/cli/internal/ui"
	"github.com/promptkeeper/cli/internal/validate"
	"github.com/spf13/cobra"
)

var execCmd = &cobra.Command{
	Use:   "exec [prompt_title] [key=value...] [--raw-prompt \"...\"] [--provider provider]",
	Short: "Execute a prompt",
	Long: `Executes a stored prompt by function_id, or executes a raw inline prompt with --raw-prompt.

Stored prompt mode:
Example: prke exec my_prompt name=Alice query="What is X?"

Raw prompt mode:
Example: prke exec --raw-prompt "Summarize {{text}}" --provider openai text="Hello world"

Variables are injected into the Handlebars template on the backend.
Streams the LLM response to stdout in real-time. If prompt_title is omitted (stored mode), an interactive form will guide you.`,
	Args: cobra.ArbitraryArgs,
	RunE: runExec,
}

var execProvider, execModel, execRawPrompt string

func init() {
	rootCmd.AddCommand(execCmd)
	execCmd.Flags().StringVar(&execProvider, "provider", "", "Preferred provider (e.g. openai, anthropic)")
	execCmd.Flags().StringVar(&execModel, "model", "", "LLM model override (e.g. gpt-4o, claude-3-5-sonnet)")
	execCmd.Flags().StringVar(&execRawPrompt, "raw-prompt", "", "Execute raw prompt text directly (requires --provider)")
}

func runExec(cmd *cobra.Command, args []string) error {
	dbg := func(format string, a ...interface{}) {
		if rootDebug {
			fmt.Fprintln(os.Stderr, ui.DebugLine(format, a...))
		}
	}

	title := ""
	rawPrompt := strings.TrimSpace(execRawPrompt)
	var varLines string
	if rawPrompt == "" && len(args) >= 1 {
		title = strings.TrimSpace(args[0])
	}
	if rawPrompt == "" && len(args) < 1 {
		if err := ui.FormExec(&title, &varLines); err != nil {
			return err
		}
		title = strings.TrimSpace(title)
		var pairs []string
		for _, line := range strings.Split(varLines, "\n") {
			line = strings.TrimSpace(line)
			if line == "" {
				continue
			}
			idx := strings.IndexRune(line, '=')
			if idx > 0 {
				k := strings.TrimSpace(line[:idx])
				v := strings.TrimSpace(line[idx+1:])
				if k != "" {
					pairs = append(pairs, k+"="+v)
				}
			}
		}
		args = append([]string{title}, pairs...)
	}
	if rawPrompt == "" && title == "" {
		PrintFriendlyError(os.Stderr,
			"exec requires a prompt title.",
			"You did not provide a prompt title (stored mode).",
			[]string{bin() + " exec my_prompt", bin() + " exec --raw-prompt \"Summarize {{text}}\" --provider openai text=hello"})
		return fmt.Errorf("missing prompt title")
	}
	if rawPrompt != "" {
		dbg("exec start: mode=raw")
	} else {
		dbg("exec start: prompt_title=%q", title)
	}

	if rawPrompt == "" {
		if err := validate.ValidatePromptTitle(title); err != nil {
			PrintFriendlyError(os.Stderr,
				"Invalid prompt title.",
				err.Error(),
				[]string{bin() + " exec my_prompt", "# Use letters, numbers, spaces, underscore, or hyphen."})
			return err
		}
	}
	if rawPrompt != "" {
		if err := validate.ValidateInputLength(rawPrompt, "raw prompt"); err != nil {
			PrintFriendlyError(os.Stderr,
				"Invalid raw prompt.",
				err.Error(),
				[]string{bin() + " exec --raw-prompt \"Summarize {{text}}\" --provider openai text=hello"})
			return err
		}
		if strings.TrimSpace(execProvider) == "" {
			PrintFriendlyError(os.Stderr,
				"--provider is required with --raw-prompt.",
				"Raw prompt execution needs an explicit provider (e.g. openai, anthropic, gemini).",
				[]string{bin() + " exec --raw-prompt \"Summarize {{text}}\" --provider openai text=hello"})
			return fmt.Errorf("provider required for raw prompt mode")
		}
	}
	if err := validate.ValidateProvider(execProvider); err != nil {
		PrintFriendlyError(os.Stderr,
			"Invalid provider.",
			err.Error(),
			[]string{bin() + " exec my_prompt --provider openai"})
		return err
	}
	if err := validate.ValidateModel(execModel); err != nil {
		PrintFriendlyError(os.Stderr,
			"Invalid model.",
			err.Error(),
			[]string{bin() + " exec my_prompt --model gpt-4o"})
		return err
	}

	variables := make(map[string]interface{})
	startIdx := 1
	if rawPrompt != "" {
		startIdx = 0
	}
	for i := startIdx; i < len(args); i++ {
		arg := args[i]
		if strings.HasPrefix(arg, "--") {
			continue
		}
		idx := strings.IndexRune(arg, '=')
		if idx <= 0 {
			PrintFriendlyError(os.Stderr,
				"Invalid variable format.",
				fmt.Sprintf("Each variable must be key=value. Got: %q", arg),
				[]string{bin() + " exec my_prompt name=Alice query=\"Your question\"", "# Use key=value for each variable; put values with spaces in quotes."})
			return fmt.Errorf("invalid variable: %s", arg)
		}
		k := strings.TrimSpace(arg[:idx])
		v := strings.TrimSpace(arg[idx+1:])
		if k == "" {
			continue
		}
		variables[k] = v
	}
	if err := validate.ValidateVarMappings(toStringMap(variables)); err != nil {
		PrintFriendlyError(os.Stderr,
			"Invalid variables.",
			err.Error(),
			[]string{bin() + " exec my_prompt name=Alice query=\"Short question\""})
		return err
	}
	dbg("variables: %v", variables)

	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Cannot load config (e.g. home directory).")
		return err
	}
	dbg("config: base_url=%s", cfg.BaseURL())

	token, err := cfg.GetAPIKey()
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Run '"+bin()+" register' or '"+bin()+" set prke_key <key>' first to set your API key.")
		return err
	}
	dbg("api_key: found (%d chars)", len(token))

	client := api.NewClient(cfg.BaseURL(), token)
	var debugOut io.Writer
	if rootDebug {
		debugOut = os.Stderr
	}
	streamWriter := func(data string) error {
		if data != "" {
			os.Stdout.WriteString(data)
			os.Stdout.Sync() // flush for real-time streaming
		}
		return nil
	}
	if rawPrompt != "" {
		err = client.ExecuteRaw(rawPrompt, variables, execProvider, execModel, streamWriter, debugOut)
	} else {
		err = client.Execute(title, variables, execProvider, execModel, streamWriter, debugOut)
	}
	if err != nil {
		if rawPrompt != "" {
			PrintAPIError(os.Stderr, err.Error(), "Check provider key setup ('store key'), backend reachability, and raw prompt/provider validity.")
		} else {
			PrintAPIError(os.Stderr, err.Error(), "Check that the prompt exists, the backend is reachable, and the provider is configured.")
		}
	}
	return err
}

func toStringMap(m map[string]interface{}) map[string]string {
	out := make(map[string]string)
	for k, v := range m {
		switch x := v.(type) {
		case string:
			out[k] = x
		default:
			b, _ := json.Marshal(v)
			out[k] = string(b)
		}
	}
	return out
}
