package config

import (
	"fmt"
	"os"
	"strings"

	"github.com/zalando/go-keyring"
)

// GetDefaultKeyAlias returns the active alias name for key resolution for this workspace, if any.
func (c *Config) GetDefaultKeyAlias(workspaceID string) string {
	workspaceID = strings.TrimSpace(workspaceID)
	if c.state == nil || c.state.DefaultKeyAlias == nil {
		return ""
	}
	return strings.TrimSpace(c.state.DefaultKeyAlias[workspaceID])
}

// SetDefaultKeyAlias records which alias name is preferred for API calls for this workspace.
func (c *Config) SetDefaultKeyAlias(workspaceID, alias string) error {
	if c.state == nil {
		c.state = &PrkeState{}
	}
	if c.state.DefaultKeyAlias == nil {
		c.state.DefaultKeyAlias = map[string]string{}
	}
	workspaceID = strings.TrimSpace(workspaceID)
	alias = strings.TrimSpace(alias)
	if workspaceID == "" {
		return fmt.Errorf("workspace id is required")
	}
	if alias == "" {
		delete(c.state.DefaultKeyAlias, workspaceID)
	} else {
		c.state.DefaultKeyAlias[workspaceID] = alias
	}
	return savePrkeState(c.home, c.state)
}

// SetAliasKey stores a client key under a CLI-only alias for a workspace.
func (c *Config) SetAliasKey(workspaceID, alias, key string) {
	workspaceID = strings.TrimSpace(workspaceID)
	alias = strings.TrimSpace(alias)
	key = strings.TrimSpace(key)
	if workspaceID == "" || alias == "" || key == "" {
		return
	}
	setKeyringSecret(os.Stderr, os.Stdout, vaultUserAlias(workspaceID, alias), key, "Client key (alias "+alias+")")
}

// GetAliasKey returns a key stored under an alias, if present.
func (c *Config) GetAliasKey(workspaceID, alias string) string {
	workspaceID = strings.TrimSpace(workspaceID)
	alias = strings.TrimSpace(alias)
	if workspaceID == "" || alias == "" {
		return ""
	}
	v, ok, err := getKeyringSecret(vaultUserAlias(workspaceID, alias))
	if err != nil || !ok {
		return ""
	}
	return v
}

// RenameAliasKey renames an alias (keyring entry + default pointer if needed).
func (c *Config) RenameAliasKey(workspaceID, oldAlias, newAlias string) error {
	workspaceID = strings.TrimSpace(workspaceID)
	oldAlias = strings.TrimSpace(oldAlias)
	newAlias = strings.TrimSpace(newAlias)
	if workspaceID == "" || oldAlias == "" || newAlias == "" {
		return fmt.Errorf("workspace id and both alias names are required")
	}
	if oldAlias == newAlias {
		return nil
	}
	k := c.GetAliasKey(workspaceID, oldAlias)
	if k == "" {
		return fmt.Errorf("alias %q not found for this workspace", oldAlias)
	}
	c.SetAliasKey(workspaceID, newAlias, k)
	if SecureStorageSupported() {
		_ = keyring.Delete(ServiceName, vaultUserAlias(workspaceID, oldAlias))
	}
	if c.GetDefaultKeyAlias(workspaceID) == oldAlias {
		_ = c.SetDefaultKeyAlias(workspaceID, newAlias)
	}
	return nil
}
