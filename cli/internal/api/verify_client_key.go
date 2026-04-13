package api

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
)

// VerifyClientKeyResponse is the JSON body for POST /v1/auth/verify-client-key (200).
type VerifyClientKeyResponse struct {
	OK          bool   `json:"ok"`
	WorkspaceID string `json:"workspace_id"`
	Scope       string `json:"scope"`
}

// VerifyClientKey checks that a pk_mgt_live_ / pk_exe_live_ key resolves to the optional workspace.
// Session tokens are not accepted by this endpoint.
func (c *Client) VerifyClientKey(apiKey string, workspaceID *string) (*VerifyClientKeyResponse, error) {
	apiKey = strings.TrimSpace(apiKey)
	if apiKey == "" {
		return nil, fmt.Errorf("api_key is empty")
	}
	body := map[string]interface{}{"api_key": apiKey}
	if workspaceID != nil && strings.TrimSpace(*workspaceID) != "" {
		ws := strings.TrimSpace(*workspaceID)
		body["workspace_id"] = ws
	}
	jsonBody, _ := json.Marshal(body)
	url := c.BaseURL + "/v1/auth/verify-client-key"
	req, err := http.NewRequest(http.MethodPost, url, bytes.NewReader(jsonBody))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")

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
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("verify client key failed (%d): %s", resp.StatusCode, getErrorMsg(result))
	}
	var out VerifyClientKeyResponse
	if err := json.Unmarshal(data, &out); err != nil {
		return nil, fmt.Errorf("invalid verify response: %w", err)
	}
	return &out, nil
}
