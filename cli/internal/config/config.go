package config

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/spf13/viper"
	"github.com/zalando/go-keyring"
)

const (
	ServiceName = "promptkeeper"
	KeyUser     = "api_key"
	ConfigKey   = "vault_access_token"
	BaseURLKey  = "base_url"
	DefaultURL  = "http://localhost:3000"
)

// Config manages viper config file and optional system keyring.
type Config struct {
	v *viper.Viper
}

// New creates and initializes config. Config file: ~/.pv-config.yaml
func New() (*Config, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return nil, fmt.Errorf("cannot get home dir: %w", err)
	}
	configPath := filepath.Join(home, ".pv-config.yaml")

	v := viper.New()
	v.SetConfigFile(configPath)
	v.SetConfigType("yaml")
	v.SetDefault(BaseURLKey, DefaultURL)
	_ = v.ReadInConfig() // ignore if file does not exist

	return &Config{v: v}, nil
}

// BaseURL returns the API base URL (env PKRE_BASE_URL overrides config).
func (c *Config) BaseURL() string {
	if u := os.Getenv("PKRE_BASE_URL"); u != "" {
		return u
	}
	url := c.v.GetString(BaseURLKey)
	if url == "" {
		return DefaultURL
	}
	return url
}

// SetBaseURL stores the base URL in config.
func (c *Config) SetBaseURL(url string) error {
	c.v.Set(BaseURLKey, url)
	return c.v.WriteConfigAs(c.v.ConfigFileUsed())
}

// GetAPIKey returns api_key from system keyring first, then config file fallback.
func (c *Config) GetAPIKey() (string, error) {
	// Try keyring first (more secure)
	token, err := keyring.Get(ServiceName, KeyUser)
	if err == nil && token != "" {
		return token, nil
	}
	// Fallback to config file
	token = c.v.GetString(ConfigKey)
	if token != "" {
		return token, nil
	}
	return "", fmt.Errorf("no API key found: run 'prke register' or 'prke set prke_key <key>'")
}

// SetAPIKey stores api_key in system keyring and config file (viper).
func (c *Config) SetAPIKey(apiKey string) error {
	_ = keyring.Set(ServiceName, KeyUser, apiKey) // best-effort; may fail on headless
	c.v.Set(ConfigKey, apiKey)
	return c.writeConfig()
}

// DeleteAPIKey removes api_key from keyring and config.
func (c *Config) DeleteAPIKey() error {
	_ = keyring.Delete(ServiceName, KeyUser)
	c.v.Set(ConfigKey, "")
	return c.writeConfig()
}

func (c *Config) writeConfig() error {
	path := c.v.ConfigFileUsed()
	if path == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			return err
		}
		path = filepath.Join(home, ".pv-config.yaml")
		c.v.SetConfigFile(path)
	}
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0700); err != nil {
		return err
	}
	return c.v.WriteConfigAs(path)
}
