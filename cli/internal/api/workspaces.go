package api

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
)

// WorkspaceSummary is one row from GET /v1/workspaces.
type WorkspaceSummary struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}

// ListWorkspacesResponse is the JSON body for GET /v1/workspaces.
type ListWorkspacesResponse struct {
	Workspaces []WorkspaceSummary `json:"workspaces"`
}

// ListWorkspaces returns workspaces the caller belongs to.
func (c *Client) ListWorkspaces() (*ListWorkspacesResponse, error) {
	url := c.BaseURL + "/v1/workspaces"
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] GET %s\n", url)
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] response status: %s\n", resp.Status)
	}
	var result map[string]interface{}
	if err := json.Unmarshal(data, &result); err != nil {
		return nil, fmt.Errorf("invalid response: JSON parse failed")
	}
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("list workspaces failed (%d): %s", resp.StatusCode, getErrorMsg(result))
	}
	var out ListWorkspacesResponse
	if err := json.Unmarshal(data, &out); err != nil {
		return nil, fmt.Errorf("invalid list workspaces response: %w", err)
	}
	return &out, nil
}

// WorkspaceTokenMeta is a row in GET /v1/workspaces/:id (no secrets).
type WorkspaceTokenMeta struct {
	ID        string `json:"id"`
	Label     string `json:"label"`
	Scope     string `json:"scope"`
	CreatedAt string `json:"created_at"`
}

// GetWorkspaceResponse is GET /v1/workspaces/:workspace_id.
type GetWorkspaceResponse struct {
	ID        string               `json:"id"`
	Name      string               `json:"name"`
	Slug      string               `json:"slug"`
	APITokens []WorkspaceTokenMeta `json:"api_tokens"`
	Note      string               `json:"note"`
}

// GetWorkspace fetches one workspace including slug (for personal-workspace detection).
func (c *Client) GetWorkspace(workspaceID string) (*GetWorkspaceResponse, error) {
	workspaceID = strings.TrimSpace(workspaceID)
	if workspaceID == "" {
		return nil, fmt.Errorf("workspace id is required")
	}
	url := fmt.Sprintf("%s/v1/workspaces/%s", c.BaseURL, workspaceID)
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] GET %s\n", url)
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] response status: %s\n", resp.Status)
	}
	var result map[string]interface{}
	if err := json.Unmarshal(data, &result); err != nil {
		return nil, fmt.Errorf("invalid response: JSON parse failed")
	}
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("get workspace failed (%d): %s", resp.StatusCode, getErrorMsg(result))
	}
	var out GetWorkspaceResponse
	if err := json.Unmarshal(data, &out); err != nil {
		return nil, fmt.Errorf("invalid get workspace response: %w", err)
	}
	return &out, nil
}

// CreateWorkspaceResponse is POST /v1/workspaces (201).
type CreateWorkspaceResponse struct {
	ID            string `json:"id"`
	Name          string `json:"name"`
	Slug          string `json:"slug"`
	APIKey        string `json:"api_key"`
	APIKeyScope   string `json:"api_key_scope"`
}

// CreateWorkspace creates a workspace and returns a new management key (shown once).
func (c *Client) CreateWorkspace(name string) (*CreateWorkspaceResponse, error) {
	body := map[string]interface{}{
		"name":    strings.TrimSpace(name),
		"surface": surfaceCLI,
	}
	jsonBody, _ := json.Marshal(body)
	url := c.BaseURL + "/v1/workspaces"
	req, err := http.NewRequest(http.MethodPost, url, bytes.NewReader(jsonBody))
	if err != nil {
		return nil, err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] POST %s\n", url)
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] response status: %s\n", resp.Status)
	}
	var result map[string]interface{}
	if err := json.Unmarshal(data, &result); err != nil {
		return nil, fmt.Errorf("invalid response: JSON parse failed")
	}
	if resp.StatusCode != http.StatusCreated {
		return nil, fmt.Errorf("create workspace failed (%d): %s", resp.StatusCode, getErrorMsg(result))
	}
	var out CreateWorkspaceResponse
	if err := json.Unmarshal(data, &out); err != nil {
		return nil, fmt.Errorf("invalid create workspace response: %w", err)
	}
	return &out, nil
}

// MintWorkspaceMgtKeyResponse is POST /v1/workspaces/:id/mgt-key (201).
type MintWorkspaceMgtKeyResponse struct {
	APIKey      string `json:"api_key"`
	APIKeyScope string `json:"api_key_scope"`
	Label       string `json:"label"`
}

// MintWorkspaceManagementKey mints a new management key for the workspace.
func (c *Client) MintWorkspaceManagementKey(workspaceID, label string) (*MintWorkspaceMgtKeyResponse, error) {
	workspaceID = strings.TrimSpace(workspaceID)
	if workspaceID == "" {
		return nil, fmt.Errorf("workspace id is required")
	}
	l := strings.TrimSpace(label)
	if l == "" {
		l = "Workspace management"
	}
	body := map[string]interface{}{
		"label":   l,
		"surface": surfaceCLI,
	}
	jsonBody, _ := json.Marshal(body)
	url := fmt.Sprintf("%s/v1/workspaces/%s/mgt-key", c.BaseURL, workspaceID)
	req, err := http.NewRequest(http.MethodPost, url, bytes.NewReader(jsonBody))
	if err != nil {
		return nil, err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] POST %s\n", url)
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] response status: %s\n", resp.Status)
	}
	var result map[string]interface{}
	if err := json.Unmarshal(data, &result); err != nil {
		return nil, fmt.Errorf("invalid response: JSON parse failed")
	}
	if resp.StatusCode != http.StatusCreated {
		return nil, fmt.Errorf("mint workspace key failed (%d): %s", resp.StatusCode, getErrorMsg(result))
	}
	var out MintWorkspaceMgtKeyResponse
	if err := json.Unmarshal(data, &out); err != nil {
		return nil, fmt.Errorf("invalid mint workspace key response: %w", err)
	}
	return &out, nil
}

// FindPersonalWorkspaceID returns the workspace whose slug is "{userUUID}-personal" (signup default).
func FindPersonalWorkspaceID(c *Client, userUUID string) (string, error) {
	userUUID = strings.TrimSpace(strings.ToLower(userUUID))
	if userUUID == "" {
		return "", fmt.Errorf("user id is required")
	}
	wantSlug := userUUID + "-personal"
	list, err := c.ListWorkspaces()
	if err != nil {
		return "", err
	}
	for _, w := range list.Workspaces {
		det, err := c.GetWorkspace(w.ID)
		if err != nil {
			continue
		}
		if strings.EqualFold(det.Slug, wantSlug) {
			return w.ID, nil
		}
	}
	return "", fmt.Errorf("could not find personal workspace (slug %s); run workspace list and switch to the correct id", wantSlug)
}

// FindWorkspaceIDByName returns the first workspace id whose name matches (case-insensitive).
func FindWorkspaceIDByName(c *Client, name string) (string, error) {
	want := strings.TrimSpace(strings.ToLower(name))
	if want == "" {
		return "", fmt.Errorf("workspace name is required")
	}
	list, err := c.ListWorkspaces()
	if err != nil {
		return "", err
	}
	for _, w := range list.Workspaces {
		if strings.ToLower(strings.TrimSpace(w.Name)) == want {
			return w.ID, nil
		}
	}
	return "", fmt.Errorf("no workspace named %q", name)
}

// UpdateWorkspace renames a workspace. PATCH /v1/workspaces/:id.
func (c *Client) UpdateWorkspace(workspaceID, newName string) (map[string]interface{}, error) {
	workspaceID = strings.TrimSpace(workspaceID)
	if workspaceID == "" {
		return nil, fmt.Errorf("workspace id is required")
	}
	body := map[string]interface{}{"name": strings.TrimSpace(newName)}
	jsonBody, _ := json.Marshal(body)
	url := fmt.Sprintf("%s/v1/workspaces/%s", c.BaseURL, workspaceID)
	req, err := http.NewRequest(http.MethodPatch, url, bytes.NewReader(jsonBody))
	if err != nil {
		return nil, err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] PATCH %s\n", url)
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] response status: %s\n", resp.Status)
	}
	var result map[string]interface{}
	if err := json.Unmarshal(data, &result); err != nil {
		return nil, fmt.Errorf("invalid response: JSON parse failed")
	}
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("update workspace failed (%d): %s", resp.StatusCode, getErrorMsg(result))
	}
	return result, nil
}

// DeleteWorkspace removes a workspace. DELETE /v1/workspaces/:id.
func (c *Client) DeleteWorkspace(workspaceID string) error {
	workspaceID = strings.TrimSpace(workspaceID)
	if workspaceID == "" {
		return fmt.Errorf("workspace id is required")
	}
	url := fmt.Sprintf("%s/v1/workspaces/%s", c.BaseURL, workspaceID)
	req, err := http.NewRequest(http.MethodDelete, url, nil)
	if err != nil {
		return err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] DELETE %s\n", url)
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	data, _ := io.ReadAll(resp.Body)
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] response status: %s\n", resp.Status)
	}
	if resp.StatusCode == http.StatusNoContent || resp.StatusCode == http.StatusOK {
		return nil
	}
	var result map[string]interface{}
	_ = json.Unmarshal(data, &result)
	return fmt.Errorf("delete workspace failed (%d): %s", resp.StatusCode, getErrorMsg(result))
}
