package cmd

import (
	"github.com/promptkeeper/cli/internal/usererr"
)

// bin returns the root command name (prke or promptkeeper) for use in examples.
func bin() string {
	if rootCmd != nil {
		return rootCmd.Use
	}
	return "prke"
}

// PrintFriendlyError writes a user-friendly error block: what failed, why, and examples.
func PrintFriendlyError(w interface{ Write([]byte) (int, error) }, what, why string, examples []string) {
	usererr.PrintFriendlyError(w, bin(), what, why, examples)
}

// PrintAPIError writes the API/config error and an optional hint.
func PrintAPIError(w interface{ Write([]byte) (int, error) }, errMsg, hint string) {
	usererr.PrintAPIError(w, errMsg, hint)
}
