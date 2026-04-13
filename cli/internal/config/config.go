package config

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/viper"
)

const (
	ServiceName = "promptkeeper"
	KeyUser     = "api_key"
	BaseURLKey  = "base_url"
	DefaultURL  = "https://api.promptkeeper.ai"
)

// Config manages viper config file and optional system keyring.
// useLocalConfig means both --debug and --use-local-config were passed; then ~/.prke-config.yaml is read/written for base_url.
// When useLocalConfig is false, the config file is not used for base URL.
type Config struct {
	v              *viper.Viper
	useLocalConfig bool
	home           string
	state          *PrkeState
}

// New creates and initializes config. useLocalConfig should be true only when both --debug and --use-local-config are set.
// When true, reads ~/.prke-config.yaml (ignore missing file). Do not set a viper default for base_url so an empty/missing file can fall through to env.
func New(useLocalConfig bool) (*Config, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return nil, fmt.Errorf("cannot get home dir: %w", err)
	}
	configPath := filepath.Join(home, ".prke-config.yaml")

	v := viper.New()
	v.SetConfigFile(configPath)
	v.SetConfigType("yaml")
	if useLocalConfig {
		_ = v.ReadInConfig() // ignore if file does not exist
	}

	st, err := loadPrkeState(home)
	if err != nil {
		return nil, fmt.Errorf("load workspace state: %w", err)
	}

	return &Config{v: v, useLocalConfig: useLocalConfig, home: home, state: st}, nil
}

// BaseURL resolves the API base URL:
//   - If useLocalConfig (both --debug and --use-local-config): use base_url from ~/.prke-config.yaml when it is non-empty after trim.
//   - Otherwise, or when that value is empty: PKRE_BASE_URL env if set, else DefaultURL.
func (c *Config) BaseURL() string {
	if c.useLocalConfig {
		url := strings.TrimSpace(c.v.GetString(BaseURLKey))
		if url != "" {
			return url
		}
	}
	if u := strings.TrimSpace(os.Getenv("PKRE_BASE_URL")); u != "" {
		return u
	}
	return DefaultURL
}

// SetBaseURL stores the base URL in config (only persisted when useLocalConfig).
func (c *Config) SetBaseURL(url string) error {
	c.v.Set(BaseURLKey, url)
	if !c.useLocalConfig {
		return nil
	}
	return c.writeConfig()
}

// GetAPIKey returns a credential suitable for management-style API calls (store, mint execution, workspaces).
func (c *Config) GetAPIKey() (string, error) {
	return c.AuthTokenForManagement()
}

// SetAPIKey stores a client key or session: per-workspace vault when current workspace is set, otherwise legacy single-slot keyring.
func (c *Config) SetAPIKey(apiKey string) error {
	apiKey = strings.TrimSpace(apiKey)
	if apiKey == "" {
		return fmt.Errorf("API key is empty")
	}
	ws := c.CurrentWorkspaceID()
	switch {
	case strings.HasPrefix(apiKey, "pk_exe_live_") && ws != "":
		c.SetWorkspaceExecutionKey(ws, apiKey)
		return nil
	case strings.HasPrefix(apiKey, "pk_mgt_live_") && ws != "":
		c.SetWorkspaceManagementKey(ws, apiKey)
		return nil
	case LooksLikeSessionToken(apiKey):
		c.SetSessionToken(apiKey)
		return nil
	default:
		// Unknown shape: store as legacy (backward compatibility).
		setKeyringSecret(os.Stderr, os.Stdout, KeyUser, apiKey, "API key")
		return nil
	}
}

// DeleteAPIKey removes the legacy single-slot key only (not per-workspace or session entries).
func (c *Config) DeleteAPIKey() error {
	return c.ClearLegacyAPIKey()
}

func (c *Config) writeConfig() error {
	path := c.v.ConfigFileUsed()
	if path == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			return err
		}
		path = filepath.Join(home, ".prke-config.yaml")
		c.v.SetConfigFile(path)
	}
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0700); err != nil {
		return err
	}
	return c.v.WriteConfigAs(path)
}
