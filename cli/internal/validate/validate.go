package validate

import (
	"fmt"
	"net/mail"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"unicode/utf8"
)

// Security limits - assume malicious intent, do not trust user input
const (
	MaxEmailLen     = 320
	MaxPasswordLen  = 256
	MinPasswordLen  = 12
	MaxInputLen     = 64 * 1024 // 64KB for prompts/secrets
	MaxProviderLen  = 64
	MaxPromptTitle  = 256
	MaxVarKeyLen    = 128
	MaxVarValueLen  = 4096
	MaxVarPairs     = 32
	MaxFilePathLen  = 4096
)

// Allowed characters for identifiers (provider, prompt_title)
var safeIdentRe = regexp.MustCompile(`^[a-zA-Z0-9_-]+$`)

// Variable keys may include dots for Handlebars (e.g. user.name)
var varKeyRe = regexp.MustCompile(`^[a-zA-Z0-9_.-]+$`)

// ValidateEmail validates email format and length. Returns error for invalid input.
func ValidateEmail(email string) error {
	email = strings.TrimSpace(email)
	if email == "" {
		return fmt.Errorf("email is required")
	}
	if utf8.RuneCountInString(email) > MaxEmailLen {
		return fmt.Errorf("email exceeds maximum length")
	}
	addr, err := mail.ParseAddress(email)
	if err != nil || addr.Address != email {
		return fmt.Errorf("invalid email format")
	}
	if !strings.Contains(addr.Address, "@") {
		return fmt.Errorf("invalid email: must contain @")
	}
	return nil
}

// ValidatePassword validates password (min 12 chars, reasonable max).
func ValidatePassword(password string) error {
	if utf8.RuneCountInString(password) < MinPasswordLen {
		return fmt.Errorf("password must be at least %d characters", MinPasswordLen)
	}
	if utf8.RuneCountInString(password) > MaxPasswordLen {
		return fmt.Errorf("password exceeds maximum length")
	}
	return nil
}

// ValidateProvider validates provider name.
func ValidateProvider(provider string) error {
	provider = strings.TrimSpace(provider)
	if provider == "" {
		return nil // optional
	}
	if utf8.RuneCountInString(provider) > MaxProviderLen {
		return fmt.Errorf("provider name exceeds maximum length")
	}
	if !safeIdentRe.MatchString(provider) {
		return fmt.Errorf("provider contains invalid characters (use alphanumeric, _, -)")
	}
	return nil
}

// ValidatePromptTitle validates prompt/function name.
func ValidatePromptTitle(title string) error {
	title = strings.TrimSpace(title)
	if title == "" {
		return fmt.Errorf("prompt title is required")
	}
	if utf8.RuneCountInString(title) > MaxPromptTitle {
		return fmt.Errorf("prompt title exceeds maximum length")
	}
	if !safeIdentRe.MatchString(title) {
		return fmt.Errorf("prompt title contains invalid characters (use alphanumeric, _, -)")
	}
	return nil
}

// ValidateVarMappings validates key=value pairs for exec.
func ValidateVarMappings(pairs map[string]string) error {
	if len(pairs) > MaxVarPairs {
		return fmt.Errorf("too many variable pairs (max %d)", MaxVarPairs)
	}
	for k, v := range pairs {
		if utf8.RuneCountInString(k) > MaxVarKeyLen {
			return fmt.Errorf("variable key %q exceeds maximum length", k)
		}
		if utf8.RuneCountInString(v) > MaxVarValueLen {
			return fmt.Errorf("variable value for %q exceeds maximum length", k)
		}
		if !varKeyRe.MatchString(k) {
			return fmt.Errorf("variable key %q contains invalid characters", k)
		}
	}
	return nil
}

// ValidateInputLength validates raw input (prompt, secret) size.
func ValidateInputLength(s string, label string) error {
	if utf8.RuneCountInString(s) > MaxInputLen {
		return fmt.Errorf("%s exceeds maximum length (%d bytes)", label, MaxInputLen)
	}
	return nil
}

// SafeFilePath checks if path is a safe file path (no traversal, within reasonable constraints).
// Returns absolute path and error.
func SafeFilePath(path string) (string, error) {
	path = strings.TrimSpace(path)
	if path == "" {
		return "", fmt.Errorf("file path is required")
	}
	if utf8.RuneCountInString(path) > MaxFilePathLen {
		return "", fmt.Errorf("file path exceeds maximum length")
	}
	// Reject path traversal patterns
	if strings.Contains(path, "..") {
		return "", fmt.Errorf("path traversal not allowed")
	}
	abs, err := filepath.Abs(path)
	if err != nil {
		return "", fmt.Errorf("invalid path: %w", err)
	}
	info, err := os.Stat(abs)
	if err != nil {
		return "", fmt.Errorf("cannot read file: %w", err)
	}
	if info.IsDir() {
		return "", fmt.Errorf("path must be a file, not a directory")
	}
	if info.Size() > int64(MaxInputLen) {
		return "", fmt.Errorf("file too large (max %d bytes)", MaxInputLen)
	}
	return abs, nil
}
