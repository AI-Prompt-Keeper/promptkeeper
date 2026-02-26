package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/promptkeeper/cli/internal/api"
	"github.com/promptkeeper/cli/internal/config"
	"github.com/promptkeeper/cli/internal/validate"
	"github.com/spf13/cobra"
)

var execCmd = &cobra.Command{
	Use:   "exec <prompt_title> [key=value...] [--provider provider]",
	Short: "Execute a prompt",
	Long: `Executes a prompt by function_id. Supports variable substitutions via key=value pairs.
Example: prke exec my_prompt name=Alice query="What is X?"
Variables are injected into the Handlebars template on the backend.
Streams the LLM response to stdout in real-time.`,
	Args: cobra.MinimumNArgs(1),
	RunE: runExec,
}

var execProvider string

func init() {
	rootCmd.AddCommand(execCmd)
	execCmd.Flags().StringVar(&execProvider, "provider", "", "Preferred provider (e.g. openai, anthropic)")
}

func runExec(cmd *cobra.Command, args []string) error {
	title := strings.TrimSpace(args[0])
	if err := validate.ValidatePromptTitle(title); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		return err
	}
	if err := validate.ValidateProvider(execProvider); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
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
			fmt.Fprintf(os.Stderr, "error: invalid variable format %q (use key=value)\n", arg)
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
	return client.Execute(title, variables, execProvider, func(data string) error {
		if data != "" {
			os.Stdout.WriteString(data)
			os.Stdout.Sync() // flush for real-time streaming
		}
		return nil
	})
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
