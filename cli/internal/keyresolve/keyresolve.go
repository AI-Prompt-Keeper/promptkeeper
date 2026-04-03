package keyresolve

import (
	"fmt"
	"io"
	"strings"

	"github.com/promptkeeper/cli/internal/api"
	"github.com/promptkeeper/cli/internal/config"
)

// ResolveClientKeyForCommand returns a bearer/API token for the given key option (alias, pk_*, or session when allowed).
func ResolveClientKeyForCommand(cfg *config.Config, baseURL, keyOpt, workspaceID string, debug io.Writer) (string, error) {
	keyOpt = strings.TrimSpace(keyOpt)
	if keyOpt == "" {
		return "", nil
	}
	if workspaceID == "" {
		return "", fmt.Errorf("no active workspace: run '%s workspace switch ...' first", config.CLIExeName)
	}
	if k := config.GetSessionOverrideClientKey(workspaceID); k != "" && k == keyOpt {
		return k, nil
	}
	if k := cfg.GetAliasKey(workspaceID, keyOpt); k != "" {
		return k, nil
	}
	if strings.HasPrefix(keyOpt, "pk_mgt_live_") || strings.HasPrefix(keyOpt, "pk_exe_live_") {
		client := api.NewClient(baseURL, "")
		if debug != nil {
			client.DebugLog = debug
		}
		ws := workspaceID
		_, err := client.VerifyClientKey(keyOpt, &ws)
		if err != nil {
			return "", err
		}
		return keyOpt, nil
	}
	if config.LooksLikeSessionToken(keyOpt) {
		if cfg.GetSessionToken() == keyOpt && cfg.CanUseSessionForWorkspace(workspaceID) {
			return keyOpt, nil
		}
		return "", fmt.Errorf("session token cannot be used for this workspace; use a client API key or login from this device")
	}
	return "", fmt.Errorf("unknown key or alias %q", keyOpt)
}
