package config

import (
	"strings"
	"sync"
)

// Process-local only: when OS secure storage is unavailable, `use <raw key>` stores the key here for the current process.
var (
	sessionOverrideMu sync.Mutex
	sessionOverride   = map[string]string{} // workspace_id -> client key material
)

// SetSessionOverrideClientKey stores a raw client key for this process only (no disk).
func SetSessionOverrideClientKey(workspaceID, clientKey string) {
	sessionOverrideMu.Lock()
	defer sessionOverrideMu.Unlock()
	workspaceID = strings.TrimSpace(workspaceID)
	clientKey = strings.TrimSpace(clientKey)
	if workspaceID == "" || clientKey == "" {
		delete(sessionOverride, workspaceID)
		return
	}
	sessionOverride[workspaceID] = clientKey
}

// GetSessionOverrideClientKey returns a process-local override key if set.
func GetSessionOverrideClientKey(workspaceID string) string {
	sessionOverrideMu.Lock()
	defer sessionOverrideMu.Unlock()
	return sessionOverride[strings.TrimSpace(workspaceID)]
}
