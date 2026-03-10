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
	Use:   "exec [prompt_title] [key=value...]",
	Short: "Execute a prompt",
	Long: `Executes a prompt by function_id. Supports variable substitutions via key=value pairs.
Example: prke exec my_prompt name=Alice query="What is X?"
Variables are injected into the Handlebars template on the backend.
Streams the LLM response to stdout in real-time. If prompt_title is omitted, an interactive form will guide you.`,
	Args: cobra.ArbitraryArgs,
	RunE: runExec,
}

var execProvider, execModel string

func init() {
	rootCmd.AddCommand(execCmd)
	execCmd.Flags().StringVar(&execProvider, "provider", "", "Preferred provider (e.g. openai, anthropic)")
	execCmd.Flags().StringVar(&execModel, "model", "", "LLM model override (e.g. gpt-4o, claude-3-5-sonnet)")
}

func runExec(cmd *cobra.Command, args []string) error {
	dbg := func(format string, a ...interface{}) {
		if rootDebug {
			fmt.Fprintln(os.Stderr, ui.DebugLine(format, a...))
		}
	}

	title := ""
	var varLines string
	if len(args) >= 1 {
		title = strings.TrimSpace(args[0])
	}
	if len(args) < 1 {
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
	if title == "" {
		PrintFriendlyError(os.Stderr,
			"exec requires a prompt title.",
			"You did not provide a prompt title.",
			[]string{bin() + " exec my_prompt"})
		return fmt.Errorf("missing prompt title")
	}
	dbg("exec start: prompt_title=%q", title)

	if err := validate.ValidatePromptTitle(title); err != nil {
		PrintFriendlyError(os.Stderr,
			"Invalid prompt title.",
			err.Error(),
			[]string{bin() + " exec my_prompt", "# Use letters, numbers, spaces, underscore, or hyphen."})
		return err
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
	for i := 1; i < len(args); i++ {
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
	err = client.Execute(title, variables, execProvider, execModel, func(data string) error {
		if data != "" {
			os.Stdout.WriteString(data)
			os.Stdout.Sync() // flush for real-time streaming
		}
		return nil
	}, debugOut)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Check that the prompt exists, the backend is reachable, and the provider is configured.")
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
