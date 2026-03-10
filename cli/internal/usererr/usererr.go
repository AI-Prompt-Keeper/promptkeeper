package usererr

import (
	"fmt"
	"io"
	"os"

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

// Stderr returns os.Stderr for convenience.
func Stderr() io.Writer { return os.Stderr }
