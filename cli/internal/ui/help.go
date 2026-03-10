package ui

import (
	"fmt"
	"strings"
)

// DefaultHelp returns a Lip Gloss–styled help string for the root command.
// binName is the CLI name (e.g. "prke" or "promptkeeper").
func DefaultHelp(binName string) string {
	title := Title.Render("◆ " + binName + " — Prompt Keeper")
	sub := Subtitle.Render("Secure AI Gateway CLI")
	desc := Body.Render("Register, store API keys and prompts, and run prompts with streaming output.")
	usage := Subtitle.Render("Usage")
	usages := []string{
		Cmd.Render(binName+" register") + " [email] [password]  " + Body.Copy().MarginLeft(0).Render("Create account (interactive if args omitted)"),
		Cmd.Render(binName+" set prke_key") + " [key]           " + Body.Copy().MarginLeft(0).Render("Store API key (interactive if omitted)"),
		Cmd.Render(binName+" store") + "                      " + Body.Copy().MarginLeft(0).Render("Store a key or prompt (guided)"),
		Cmd.Render(binName+" store key") + " [provider] [key] " + Body.Copy().MarginLeft(0).Render("Store provider API key"),
		Cmd.Render(binName+" store prompt") + " <title> [value] " + Body.Copy().MarginLeft(0).Render("Store prompt template"),
		Cmd.Render(binName+" exec") + " [prompt_title] ...   " + Body.Copy().MarginLeft(0).Render("Run a prompt (interactive if omitted)"),
	}
	usageBlock := Body.Render(strings.Join(usages, "\n"))
	flags := Subtitle.Render("Flags")
	flagsBlock := Body.Render("  --debug                 Enable debug logging\n  --use-local-config      Read ~/.prke-config.yaml (only with --debug)")
	help := Subtitle.Render("Help")
	helpBlock := Body.Render(fmt.Sprintf("  %s --help", binName))
	out := strings.Join([]string{title, sub, desc, "", usage, usageBlock, "", flags, flagsBlock, "", help, helpBlock}, "\n")
	return Box.Render(out)
}
