package ui

import (
	"fmt"
	"strings"
)

// FriendlyErrorBlock returns a Lip Gloss–styled error block: what failed, why, examples.
func FriendlyErrorBlock(binName, what, why string, examples []string) string {
	lines := []string{
		ErrorLabel.Render("Error: ") + what,
		"",
		HintLabel.Render("Why:   ") + why,
	}
	if len(examples) > 0 {
		lines = append(lines, "")
		lines = append(lines, HintLabel.Render("Example:"))
		for _, ex := range examples {
			lines = append(lines, "  "+ExampleLine.Render(ex))
		}
	}
	return ErrorBox.Render(strings.Join(lines, "\n"))
}

// APIErrorBlock returns a Lip Gloss–styled API/config error with optional hint.
func APIErrorBlock(errMsg, hint string) string {
	lines := []string{ErrorLabel.Render("Error: ") + errMsg}
	if hint != "" {
		lines = append(lines, "", HintLabel.Render("Hint:  ")+hint)
	}
	return ErrorBox.Render(strings.Join(lines, "\n"))
}

// SuccessMessage returns a styled success line (e.g. "success", "API key stored successfully.").
func SuccessMessage(msg string) string {
	return Success.Render("✓ " + msg)
}

// WarningBlock returns a styled warning box (title + body lines).
func WarningBlock(title string, bodyLines ...string) string {
	lines := []string{Subtitle.Copy().MarginBottom(0).Render(title)}
	if len(bodyLines) > 0 {
		lines = append(lines, "")
		for _, l := range bodyLines {
			lines = append(lines, Body.Copy().MarginLeft(0).Render(l))
		}
	}
	return WarningBox.Render(strings.Join(lines, "\n"))
}

// DebugLine returns a styled debug log line (for --debug).
func DebugLine(format string, a ...interface{}) string {
	return ExampleLine.Render(fmt.Sprintf("[debug] "+format, a...))
}
