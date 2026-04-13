package config

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"gopkg.in/yaml.v3"
)

// PrkeState is persisted to ~/.prke-state.yaml (not secrets — workspace selection only).
type PrkeState struct {
	CurrentWorkspaceID  string `yaml:"current_workspace_id,omitempty"`
	PersonalWorkspaceID string `yaml:"personal_workspace_id,omitempty"`
	SessionUserID       string `yaml:"session_user_id,omitempty"`
	// DefaultKeyAlias maps workspace_id -> alias name to use first when resolving keys (CLI-only).
	DefaultKeyAlias map[string]string `yaml:"default_key_alias_by_workspace,omitempty"`
}

func statePath(home string) string {
	return filepath.Join(home, ".prke-state.yaml")
}

func loadPrkeState(home string) (*PrkeState, error) {
	path := statePath(home)
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return &PrkeState{}, nil
		}
		return nil, err
	}
	var s PrkeState
	if err := yaml.Unmarshal(data, &s); err != nil {
		return nil, fmt.Errorf("parse state file: %w", err)
	}
	s.CurrentWorkspaceID = strings.TrimSpace(s.CurrentWorkspaceID)
	s.PersonalWorkspaceID = strings.TrimSpace(s.PersonalWorkspaceID)
	s.SessionUserID = strings.TrimSpace(s.SessionUserID)
	if s.DefaultKeyAlias == nil {
		s.DefaultKeyAlias = map[string]string{}
	}
	return &s, nil
}

func savePrkeState(home string, s *PrkeState) error {
	if s == nil {
		s = &PrkeState{}
	}
	path := statePath(home)
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0700); err != nil {
		return err
	}
	data, err := yaml.Marshal(s)
	if err != nil {
		return err
	}
	tmp := path + ".tmp"
	if err := os.WriteFile(tmp, data, 0600); err != nil {
		return err
	}
	return os.Rename(tmp, path)
}
