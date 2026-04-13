package store

import (
	"io"
	"os"
	"strings"

	"github.com/promptkeeper/cli/internal/config"
	"github.com/promptkeeper/cli/internal/keyresolve"
	"github.com/spf13/cobra"
)

// storeClientKeyFlag is shared by "store key" and "store prompt" --key.
var storeClientKeyFlag string

func resolveAuthToken(cmd *cobra.Command, cfg *config.Config, clientKeyOpt string) (string, error) {
	if strings.TrimSpace(clientKeyOpt) == "" {
		return cfg.GetAPIKey()
	}
	var dbg io.Writer
	if getDebug(cmd) {
		dbg = os.Stderr
	}
	return keyresolve.ResolveClientKeyForCommand(cfg, cfg.BaseURL(), clientKeyOpt, cfg.CurrentWorkspaceID(), dbg)
}
