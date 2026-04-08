package config

import (
	"fmt"
	"os"
	"strings"

	"github.com/zalando/go-keyring"
)

// e2eBearerToken returns a bearer token when PKRE_E2E=1 and PKRE_E2E_CLIENT_KEY are set.
// Used by integration tests when OS keychain cannot store keys for a temp HOME (e.g. CI).
func e2eBearerToken() string {
	if strings.TrimSpace(os.Getenv("PKRE_E2E")) != "1" {
		return ""
	}
	return strings.TrimSpace(os.Getenv("PKRE_E2E_CLIENT_KEY"))
}

// CLIExeName is the binary name shown in error hints ("prke" or "promptkeeper"); set from cmd/root init.
var CLIExeName = "prke"

// SetPersonalWorkspaceAndSessionUser records the signup-default workspace id and user id (for session scoping).
func (c *Config) SetPersonalWorkspaceAndSessionUser(personalWorkspaceID, userID string) error {
	if c.state == nil {
		c.state = &PrkeState{}
	}
	c.state.PersonalWorkspaceID = strings.TrimSpace(personalWorkspaceID)
	c.state.SessionUserID = strings.TrimSpace(userID)
	return savePrkeState(c.home, c.state)
}

// SetCurrentWorkspaceID sets the active workspace for CLI commands (persisted).
func (c *Config) SetCurrentWorkspaceID(id string) error {
	if c.state == nil {
		c.state = &PrkeState{}
	}
	c.state.CurrentWorkspaceID = strings.TrimSpace(id)
	return savePrkeState(c.home, c.state)
}

// CurrentWorkspaceID returns the persisted active workspace UUID, or empty if unset (legacy mode).
func (c *Config) CurrentWorkspaceID() string {
	if c.state == nil {
		return ""
	}
	return c.state.CurrentWorkspaceID
}

// PersonalWorkspaceID is the workspace whose slug is {user_id}-personal (session token targets this workspace on the server for execute/put).
func (c *Config) PersonalWorkspaceID() string {
	if c.state == nil {
		return ""
	}
	return c.state.PersonalWorkspaceID
}

// SessionUserID is the logged-in user id (UUID) used to locate the personal workspace slug.
func (c *Config) SessionUserID() string {
	if c.state == nil {
		return ""
	}
	return c.state.SessionUserID
}

// SetSessionToken stores the login session in the vault (or prints if storage is unavailable).
func (c *Config) SetSessionToken(token string) {
	token = strings.TrimSpace(token)
	if token == "" {
		return
	}
	setKeyringSecret(os.Stderr, os.Stdout, vaultUserSession, token, "Session token")
}

// GetSessionToken returns the stored session token if present.
func (c *Config) GetSessionToken() string {
	v, _, err := getKeyringSecret(vaultUserSession)
	if err != nil || v == "" {
		return ""
	}
	return v
}

// ClearSessionToken removes the session from the vault.
func (c *Config) ClearSessionToken() error {
	return deleteKeyringSecret(vaultUserSession)
}

// ClearLegacyAPIKey removes the pre-workspace single keyring entry.
func (c *Config) ClearLegacyAPIKey() error {
	if !SecureStorageSupported() {
		return nil
	}
	err := keyring.Delete(ServiceName, KeyUser)
	if err != nil && err != keyring.ErrNotFound {
		return err
	}
	return nil
}

// LooksLikeSessionToken is true for 64 hex chars (login session shape).
func LooksLikeSessionToken(s string) bool {
	s = strings.TrimSpace(s)
	if len(s) != 64 {
		return false
	}
	for _, r := range s {
		switch {
		case r >= '0' && r <= '9', r >= 'a' && r <= 'f', r >= 'A' && r <= 'F':
		default:
			return false
		}
	}
	return true
}

// SetWorkspaceManagementKey stores a pk_mgt_live_ key for a workspace.
func (c *Config) SetWorkspaceManagementKey(workspaceID, key string) {
	workspaceID = strings.TrimSpace(workspaceID)
	key = strings.TrimSpace(key)
	if workspaceID == "" || key == "" {
		return
	}
	setKeyringSecret(os.Stderr, os.Stdout, vaultUserMgt(workspaceID), key, "Management API key (pk_mgt_live_...)")
}

// GetWorkspaceManagementKey returns the management key for a workspace, if stored.
func (c *Config) GetWorkspaceManagementKey(workspaceID string) string {
	workspaceID = strings.TrimSpace(workspaceID)
	if workspaceID == "" {
		return ""
	}
	v, ok, err := getKeyringSecret(vaultUserMgt(workspaceID))
	if err != nil || !ok {
		return ""
	}
	return v
}

// SetWorkspaceExecutionKey stores a pk_exe_live_ key for a workspace.
func (c *Config) SetWorkspaceExecutionKey(workspaceID, key string) {
	workspaceID = strings.TrimSpace(workspaceID)
	key = strings.TrimSpace(key)
	if workspaceID == "" || key == "" {
		return
	}
	setKeyringSecret(os.Stderr, os.Stdout, vaultUserExe(workspaceID), key, "Execution API key (pk_exe_live_...)")
}

// GetWorkspaceExecutionKey returns the execution key for a workspace, if stored.
func (c *Config) GetWorkspaceExecutionKey(workspaceID string) string {
	workspaceID = strings.TrimSpace(workspaceID)
	if workspaceID == "" {
		return ""
	}
	v, ok, err := getKeyringSecret(vaultUserExe(workspaceID))
	if err != nil || !ok {
		return ""
	}
	return v
}

// CanUseSessionForWorkspace is true when the session token (server-side) applies to this workspace (personal/default only).
func (c *Config) CanUseSessionForWorkspace(workspaceID string) bool {
	if c.state == nil {
		return false
	}
	p := strings.TrimSpace(c.state.PersonalWorkspaceID)
	if p == "" || workspaceID == "" {
		return false
	}
	return p == strings.TrimSpace(workspaceID) && c.GetSessionToken() != ""
}

func (c *Config) canUseSessionForWorkspace(workspaceID string) bool {
	return c.CanUseSessionForWorkspace(workspaceID)
}

// legacyAPIKey reads the pre–multi-workspace single keyring entry.
func (c *Config) legacyAPIKey() string {
	if !SecureStorageSupported() {
		return ""
	}
	v, err := keyring.Get(ServiceName, KeyUser)
	if err != nil || v == "" {
		return ""
	}
	return v
}

// AuthTokenForExec prefers execution key, then management, then session (personal only), then legacy.
func (c *Config) AuthTokenForExec() (string, error) {
	if t := e2eBearerToken(); t != "" {
		return t, nil
	}
	ws := c.CurrentWorkspaceID()
	if ws != "" {
		if k := GetSessionOverrideClientKey(ws); k != "" {
			return k, nil
		}
		if da := c.GetDefaultKeyAlias(ws); da != "" {
			if k := c.GetAliasKey(ws, da); k != "" {
				return k, nil
			}
		}
		if k := c.GetWorkspaceExecutionKey(ws); k != "" {
			return k, nil
		}
		if k := c.GetWorkspaceManagementKey(ws); k != "" {
			return k, nil
		}
		if c.canUseSessionForWorkspace(ws) {
			return c.GetSessionToken(), nil
		}
		return "", fmt.Errorf("no API key or session for workspace %s: run '%s workspace mint-mgt' after login, or '%s set prke_key <key>'", ws, CLIExeName, CLIExeName)
	}
	if k := c.legacyAPIKey(); k != "" {
		return k, nil
	}
	if s := c.GetSessionToken(); s != "" {
		return s, nil
	}
	return "", fmt.Errorf("no API key: run '%s register', '%s login', or '%s set prke_key <key>'", CLIExeName, CLIExeName, CLIExeName)
}

// AuthTokenForManagement requires management or session (for personal workspace); execution-only in vault is rejected.
func (c *Config) AuthTokenForManagement() (string, error) {
	if t := e2eBearerToken(); t != "" {
		return t, nil
	}
	ws := c.CurrentWorkspaceID()
	if ws != "" {
		if k := GetSessionOverrideClientKey(ws); k != "" {
			return k, nil
		}
		if da := c.GetDefaultKeyAlias(ws); da != "" {
			if k := c.GetAliasKey(ws, da); k != "" {
				return k, nil
			}
		}
		if k := c.GetWorkspaceManagementKey(ws); k != "" {
			return k, nil
		}
		if c.canUseSessionForWorkspace(ws) {
			if s := c.GetSessionToken(); s != "" {
				return s, nil
			}
		}
		if k := c.GetWorkspaceExecutionKey(ws); k != "" {
			return "", fmt.Errorf("vault has an execution-only key for this workspace; management or session is required for this command (use '%s workspace mint-mgt' or '%s set prke_key <pk_mgt_live_...>')", CLIExeName, CLIExeName)
		}
		return "", fmt.Errorf("no management key for workspace %s: run '%s workspace mint-mgt' (session works only for your personal workspace), or '%s set prke_key <pk_mgt_live_...>'", ws, CLIExeName, CLIExeName)
	}
	if k := c.legacyAPIKey(); k != "" {
		return k, nil
	}
	if s := c.GetSessionToken(); s != "" {
		return s, nil
	}
	return "", fmt.Errorf("no API key: run '%s register', '%s login', or '%s set prke_key <key>'", CLIExeName, CLIExeName, CLIExeName)
}

// AuthForMintWorkspaceMgtKey returns a token that may call POST /v1/workspaces/:id/mgt-key (session or existing mgt).
// A session token is accepted for any workspace the user belongs to; legacy single-slot keys are used only when no current workspace is set.
func (c *Config) AuthForMintWorkspaceMgtKey() (string, error) {
	if t := e2eBearerToken(); t != "" {
		return t, nil
	}
	ws := c.CurrentWorkspaceID()
	if ws == "" {
		if k := c.legacyAPIKey(); k != "" {
			return k, nil
		}
		if s := c.GetSessionToken(); s != "" {
			return s, nil
		}
		return "", fmt.Errorf("no current workspace: run '%s workspace list' and '%s workspace switch <id>'", CLIExeName, CLIExeName)
	}
	if k := c.GetWorkspaceManagementKey(ws); k != "" {
		return k, nil
	}
	if s := c.GetSessionToken(); s != "" {
		return s, nil
	}
	if k := c.legacyAPIKey(); k != "" {
		return k, nil
	}
	return "", fmt.Errorf("need a session (login) or management key to mint a workspace key")
}

// AuthSessionOrManagement returns a token for GET/POST /v1/workspaces* (session or management; not execution-only).
//
// Prefer the stored management key for the active workspace before the login session token. A stale session
// from a prior login (still in the keychain) would otherwise be sent first and fail with 401 on the server
// even though register just stored a valid pk_mgt_live_ key for this workspace.
func (c *Config) AuthSessionOrManagement() (string, error) {
	if t := e2eBearerToken(); t != "" {
		return t, nil
	}
	ws := c.CurrentWorkspaceID()
	if ws != "" {
		if k := c.GetWorkspaceManagementKey(ws); k != "" {
			return k, nil
		}
		if c.GetWorkspaceExecutionKey(ws) != "" {
			return "", fmt.Errorf("execution-only key cannot access workspace APIs; use '%s login' or '%s workspace mint-mgt'", CLIExeName, CLIExeName)
		}
	}
	if s := c.GetSessionToken(); s != "" {
		return s, nil
	}
	if k := c.legacyAPIKey(); k != "" {
		return k, nil
	}
	return "", fmt.Errorf("no session or management key: run '%s login' or '%s register'", CLIExeName, CLIExeName)
}

// AuthTokenForWorkspaceList is an alias for workspace listing (same as AuthSessionOrManagement).
func (c *Config) AuthTokenForWorkspaceList() (string, error) {
	return c.AuthSessionOrManagement()
}

// ShouldPromptMintMgtForCurrentWorkspace is true when the active workspace has no stored management key and session cannot satisfy execute/put for it (non-personal).
func (c *Config) ShouldPromptMintMgtForCurrentWorkspace() bool {
	ws := c.CurrentWorkspaceID()
	if ws == "" {
		return false
	}
	if c.GetWorkspaceManagementKey(ws) != "" {
		return false
	}
	if c.canUseSessionForWorkspace(ws) {
		return false
	}
	// Non-personal (or unknown personal): need a minted management key for store/exec, or rely on session only if we had mgt — show hint if user has session or only exe.
	return c.GetSessionToken() != "" || c.GetWorkspaceExecutionKey(ws) != ""
}

// AuthTokenForMintExecutionKey mints an execution key in the workspace bound to the auth token.
// Session tokens are tied to the server's first workspace — only use session when current workspace matches personalWorkspaceID.
func (c *Config) AuthTokenForMintExecutionKey() (string, error) {
	return c.AuthTokenForManagement()
}
