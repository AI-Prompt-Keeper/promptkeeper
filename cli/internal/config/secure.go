package config

import (
	"fmt"
	"io"
	"runtime"

	"github.com/promptkeeper/cli/internal/ui"
	"github.com/zalando/go-keyring"
)

// SecureStorageSupported is true for macOS and Linux where we attempt OS keychain/secret-service.
func SecureStorageSupported() bool {
	switch runtime.GOOS {
	case "darwin", "linux":
		return true
	default:
		return false
	}
}

// SecureStorageUnsupportedMessage explains why we fall back to printing secrets.
func SecureStorageUnsupportedMessage() string {
	return fmt.Sprintf("This OS (%s) is not supported for OS secure storage in this CLI build (macOS and Linux only). Secrets will be printed to the terminal — copy them to a password manager.", runtime.GOOS)
}

func vaultUserMgt(workspaceID string) string {
	return "ws:" + workspaceID + ":mgt"
}

func vaultUserExe(workspaceID string) string {
	return "ws:" + workspaceID + ":exe"
}

const vaultUserSession = "session"

// vaultUserAlias stores a client key under a CLI-only alias for a workspace.
func vaultUserAlias(workspaceID, alias string) string {
	return "ws:" + workspaceID + ":alias:" + alias
}

// setKeyringSecret stores value in the keyring, or prints to wOut with a styled warning on failure / unsupported OS.
func setKeyringSecret(wErr, wOut io.Writer, user, value, what string) {
	if !SecureStorageSupported() {
		fmt.Fprintln(wErr, ui.WarningBlock("No OS secure storage",
			SecureStorageUnsupportedMessage(),
			what+":"))
		fmt.Fprintln(wOut, value)
		return
	}
	if err := keyring.Set(ServiceName, user, value); err != nil {
		fmt.Fprintln(wErr, ui.WarningBlock("Could not store in OS secure storage",
			err.Error(),
			what+" (copy from the line below):",
			"On Linux, ensure a secret service (e.g. gnome-keyring) is available."))
		fmt.Fprintln(wOut, value)
	}
}

func getKeyringSecret(user string) (string, bool, error) {
	if !SecureStorageSupported() {
		return "", false, nil
	}
	v, err := keyring.Get(ServiceName, user)
	if err != nil {
		if err == keyring.ErrNotFound {
			return "", false, nil
		}
		return "", false, err
	}
	if v == "" {
		return "", false, nil
	}
	return v, true, nil
}

func deleteKeyringSecret(user string) error {
	if !SecureStorageSupported() {
		return nil
	}
	err := keyring.Delete(ServiceName, user)
	if err != nil && err != keyring.ErrNotFound {
		return err
	}
	return nil
}
