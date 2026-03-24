package usererr

import (
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/promptkeeper/cli/internal/ui"
)

// PrintFriendlyError writes a Lip Gloss–styled error block: what failed, why, and examples.
// binName is the CLI name (e.g. "prke" or "promptkeeper") for use in examples.
func PrintFriendlyError(w io.Writer, binName, what, why string, examples []string) {
	fmt.Fprintln(w)
	fmt.Fprintln(w, ui.FriendlyErrorBlock(binName, what, why, examples))
	fmt.Fprintln(w)
}

// PrintAPIError writes a Lip Gloss–styled API/config error and an optional hint.
func PrintAPIError(w io.Writer, errMsg, hint string) {
	fmt.Fprintln(w)
	fmt.Fprintln(w, ui.APIErrorBlock(errMsg, hint))
	fmt.Fprintln(w)
}

// MergeAPIHint appends automatic guidance for known HTTP/API errors (e.g. execution-only scope on 403)
// to an optional caller-provided hint. Caller hint is first when both are non-empty.
func MergeAPIHint(err error, hint string) string {
	if err == nil {
		return hint
	}
	s := err.Error()
	auto := ""
	if strings.Contains(s, "403") {
		lower := strings.ToLower(s)
		if strings.Contains(lower, "execution-only") {
			auto = "This key can only run prompts (POST /v1/execute). Use a management API key (pk_mgt_live_...) from registration, or a session token from login, for vault, prompts, and minting tokens."
		}
	}
	if auto == "" {
		return hint
	}
	if hint == "" {
		return auto
	}
	return hint + " " + auto
}

// Stderr returns os.Stderr for convenience.
func Stderr() io.Writer { return os.Stderr }
