package config

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/viper"
	"github.com/zalando/go-keyring"
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

	return &Config{v: v, useLocalConfig: useLocalConfig}, nil
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

// GetAPIKey returns api_key from system keyring only (never from config file, to avoid exposing secrets on disk).
func (c *Config) GetAPIKey() (string, error) {
	token, err := keyring.Get(ServiceName, KeyUser)
	if err == nil && token != "" {
		return token, nil
	}
	return "", fmt.Errorf("no API key found: run 'prke register' or 'prke set prke_key <key>'")
}

// SetAPIKey stores api_key in system keyring only. Config file is not used for the token.
func (c *Config) SetAPIKey(apiKey string) error {
	return keyring.Set(ServiceName, KeyUser, apiKey)
}

// DeleteAPIKey removes api_key from system keyring only.
func (c *Config) DeleteAPIKey() error {
	return keyring.Delete(ServiceName, KeyUser)
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
