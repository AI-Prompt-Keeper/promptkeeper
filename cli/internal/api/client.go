package api

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

const surfaceCLI = "cli"

// Client for Prompt Keeper backend API.
type Client struct {
	BaseURL    string
	APIKey     string
	HTTPClient *http.Client
	// DebugLog: when set, request metadata is written here (e.g. os.Stderr for --debug). Response bodies are never logged.
	DebugLog io.Writer
}

// NewClient creates an API client.
func NewClient(baseURL, apiKey string) *Client {
	return &Client{
		BaseURL: strings.TrimSuffix(baseURL, "/"),
		APIKey:  apiKey,
		HTTPClient: &http.Client{
			Timeout: 5 * time.Minute, // allow long streaming responses
		},
	}
}

// authHeaders returns headers with API key for authenticated requests.
func (c *Client) authHeaders() map[string]string {
	h := map[string]string{
		"Content-Type": "application/json",
	}
	if c.APIKey != "" {
		h["Authorization"] = "Bearer " + c.APIKey
		h["X-API-Key"] = c.APIKey
	}
	return h
}

// GetRegisterChallenge fetches the proof-of-work challenge. GET /v1/auth/register-challenge.
func (c *Client) GetRegisterChallenge() (*RegisterChallenge, error) {
	url := c.BaseURL + "/v1/auth/register-challenge"
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return nil, err
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
	if resp.StatusCode != http.StatusOK {
		var result map[string]interface{}
		_ = json.Unmarshal(data, &result)
		return nil, fmt.Errorf("challenge failed (%d): %s", resp.StatusCode, getErrorMsg(result))
	}
	var ch RegisterChallenge
	if err := json.Unmarshal(data, &ch); err != nil {
		return nil, fmt.Errorf("invalid challenge response: %w", err)
	}
	return &ch, nil
}

// Register creates a new user. Obtains a PoW challenge, solves it, then POST /v1/auth/register with PoW headers.
func (c *Client) Register(email, password, name string) (map[string]interface{}, error) {
	ch, err := c.GetRegisterChallenge()
	if err != nil {
		return nil, err
	}
	solution, err := SolvePoW(ch.Nonce, ch.ValidUntil, ch.Difficulty)
	if err != nil {
		return nil, fmt.Errorf("proof-of-work: %w", err)
	}

	body := map[string]interface{}{
		"email":    email,
		"password": password,
		"surface":  surfaceCLI,
	}
	if name != "" {
		body["name"] = name
	}
	jsonBody, _ := json.Marshal(body)
	url := c.BaseURL + "/v1/auth/register"
	req, err := http.NewRequest("POST", url, bytes.NewReader(jsonBody))
	if err != nil {
		return nil, err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	req.Header.Set("X-Pow-Nonce", ch.Nonce)
	req.Header.Set("X-Pow-Solution", solution)
	req.Header.Set("X-Pow-Valid-Until", ch.ValidUntil)

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
		msg := getErrorMsg(result)
		if resp.StatusCode == http.StatusBadRequest && strings.Contains(strings.ToLower(msg), "proof-of-work") {
			return nil, fmt.Errorf("registration failed: proof-of-work invalid or expired. Please try again")
		}
		return nil, fmt.Errorf("register failed (%d): %s", resp.StatusCode, msg)
	}
	return result, nil
}

// LoginUser is the `user` object in POST /v1/auth/login success response.
type LoginUser struct {
	ID    string  `json:"id"`
	Email string  `json:"email"`
	Name  *string `json:"name"`
}

// LoginResponse is the JSON body for successful POST /v1/auth/login (200).
type LoginResponse struct {
	Token              string    `json:"token"`
	ExpiresAt          string    `json:"expires_at"`
	User               LoginUser `json:"user"`
	DefaultWorkspaceID string    `json:"default_workspace_id"`
	APIKey             string    `json:"api_key"`
	APIKeyScope        string    `json:"api_key_scope"`
}

// Login creates a session token. POST /v1/auth/login. Does not send an API key on the request.
func (c *Client) Login(email, password string) (*LoginResponse, error) {
	body := map[string]interface{}{
		"email":    email,
		"password": password,
		"surface":  surfaceCLI,
	}
	jsonBody, _ := json.Marshal(body)
	url := c.BaseURL + "/v1/auth/login"
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
	if resp.StatusCode != http.StatusOK {
		msg := getErrorMsg(result)
		return nil, fmt.Errorf("login failed (%d): %s", resp.StatusCode, msg)
	}
	var out LoginResponse
	if err := json.Unmarshal(data, &out); err != nil {
		return nil, fmt.Errorf("invalid login response: %w", err)
	}
	return &out, nil
}

// PutKey stores a provider API key. POST /v1/keys
func (c *Client) PutKey(provider, rawSecret string) error {
	body := map[string]string{
		"raw_secret": rawSecret,
		"provider":   provider,
		"surface":    surfaceCLI,
	}
	return c.putJSON("/v1/keys", body)
}

// PutPrompt stores a prompt template. POST /v1/prompts
func (c *Client) PutPrompt(name, rawSecret, provider, model string) error {
	body := map[string]interface{}{
		"name":       name,
		"raw_secret": rawSecret,
		"surface":    surfaceCLI,
	}
	if provider != "" {
		body["provider"] = provider
	}
	if model != "" {
		body["preferred_model"] = model
	}
	return c.putPromptBody(body)
}

func (c *Client) putJSON(path string, body interface{}) error {
	jsonBody, _ := json.Marshal(body)
	url := c.BaseURL + path
	req, err := http.NewRequest("POST", url, bytes.NewReader(jsonBody))
	if err != nil {
		return err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] POST %s\n", url)
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
	var result map[string]interface{}
	_ = json.Unmarshal(data, &result)
	if resp.StatusCode != http.StatusCreated && resp.StatusCode != http.StatusOK {
		return apiError(resp.Status, getErrorMsg(result))
	}
	return nil
}

func (c *Client) putPromptBody(body map[string]interface{}) error {
	jsonBody, _ := json.Marshal(body)
	url := c.BaseURL + "/v1/prompts"
	req, err := http.NewRequest("POST", url, bytes.NewReader(jsonBody))
	if err != nil {
		return err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] POST %s\n", url)
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
	var result map[string]interface{}
	_ = json.Unmarshal(data, &result)
	if resp.StatusCode != http.StatusCreated && resp.StatusCode != http.StatusOK {
		return apiError(resp.Status, getErrorMsg(result))
	}
	return nil
}

// apiError builds an error from status and message.
func apiError(status string, msg string) error {
	return fmt.Errorf("%s: %s", status, msg)
}

// MintExecutionTokenResponse is the JSON body for POST /v1/auth/api-tokens (201).
type MintExecutionTokenResponse struct {
	APIKey string `json:"api_key"`
	Scope  string `json:"scope"`
	Label  string `json:"label"`
}

// MintExecutionToken creates an execution-only client API key (pk_exe_live_...). POST /v1/auth/api-tokens.
// Requires a management API key or session token — not an execution-only key.
func (c *Client) MintExecutionToken(label string) (*MintExecutionTokenResponse, error) {
	body := map[string]interface{}{"surface": surfaceCLI}
	t := strings.TrimSpace(label)
	if t != "" {
		body["label"] = t
	}
	jsonBody, _ := json.Marshal(body)
	url := c.BaseURL + "/v1/auth/api-tokens"
	req, err := http.NewRequest("POST", url, bytes.NewReader(jsonBody))
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
		return nil, apiError(resp.Status, getErrorMsg(result))
	}
	var out MintExecutionTokenResponse
	if err := json.Unmarshal(data, &out); err != nil {
		return nil, fmt.Errorf("invalid mint response: %w", err)
	}
	return &out, nil
}

// ListPromptsResponse is the JSON body for GET /v1/list-prompts (200).
type ListPromptsResponse struct {
	Titles []string `json:"titles"`
}

// ListPrompts returns stored prompt titles (production deployments). GET /v1/list-prompts?surface=cli.
func (c *Client) ListPrompts() (*ListPromptsResponse, error) {
	reqURL := fmt.Sprintf("%s/v1/list-prompts?surface=%s", c.BaseURL, url.QueryEscape(surfaceCLI))
	req, err := http.NewRequest(http.MethodGet, reqURL, nil)
	if err != nil {
		return nil, err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] GET %s\n", reqURL)
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
	if resp.StatusCode != http.StatusOK {
		var result map[string]interface{}
		_ = json.Unmarshal(data, &result)
		return nil, apiError(resp.Status, getErrorMsg(result))
	}
	var out ListPromptsResponse
	if err := json.Unmarshal(data, &out); err != nil {
		return nil, fmt.Errorf("invalid list-prompts response: %w", err)
	}
	return &out, nil
}

// Execute runs the stored-prompt execute endpoint with streaming. POST /v1/execute.
func (c *Client) Execute(functionID string, variables map[string]interface{}, provider, model string, streamWriter func(data string) error, debugLog io.Writer) error {
	body := map[string]interface{}{
		"function_id": functionID,
		"variables":   variables,
		"surface":     surfaceCLI,
	}
	if provider != "" {
		body["provider"] = provider
	}
	if model != "" {
		body["model"] = model
	}
	return c.executeStream(body, streamWriter, debugLog)
}

// executeStream calls POST /v1/execute (SSE) and parses streaming chunks.
func (c *Client) executeStream(body map[string]interface{}, streamWriter func(data string) error, debugLog io.Writer) error {
	jsonBody, _ := json.Marshal(body)
	url := c.BaseURL + "/v1/execute"
	req, err := http.NewRequest("POST", url, bytes.NewReader(jsonBody))
	if err != nil {
		return err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	req.Header.Set("Accept", "text/event-stream")

	if debugLog != nil {
		fmt.Fprintf(debugLog, "[debug] POST %s\n", url)
		fmt.Fprintf(debugLog, "[debug] auth: Bearer %s...\n", maskToken(c.APIKey))
	}

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		if debugLog != nil {
			fmt.Fprintf(debugLog, "[debug] HTTP error: %v\n", err)
		}
		return err
	}
	defer resp.Body.Close()

	if debugLog != nil {
		fmt.Fprintf(debugLog, "[debug] status: %s\n", resp.Status)
		for k, v := range resp.Header {
			fmt.Fprintf(debugLog, "[debug] header %s: %s\n", k, strings.Join(v, ", "))
		}
	}

	if resp.StatusCode != http.StatusOK {
		data, _ := io.ReadAll(resp.Body)
		if debugLog != nil {
			fmt.Fprintf(debugLog, "[debug] non-200: %s\n", resp.Status)
		}
		var result map[string]interface{}
		_ = json.Unmarshal(data, &result)
		return apiError(resp.Status, getErrorMsg(result))
	}

	return parseSSEStream(resp.Body, streamWriter, debugLog)
}

func maskToken(s string) string {
	if len(s) <= 8 {
		return "***"
	}
	return s[:4] + "..." + s[len(s)-4:]
}

// parseSSEStream reads SSE events and calls streamWriter for each data payload.
// Extracts content from OpenAI/Anthropic-style chunks and errors.
// Does not log response stream contents to debugLog (response bodies are never logged).
func parseSSEStream(r io.Reader, streamWriter func(data string) error, debugLog io.Writer) error {
	scanner := bufio.NewScanner(r)
	scanner.Buffer(make([]byte, 64*1024), 1024*1024)
	for scanner.Scan() {
		line := scanner.Bytes()
		if bytes.HasPrefix(line, []byte("data: ")) {
			data := bytes.TrimSpace(line[6:])
			if len(data) == 0 {
				continue
			}
			if bytes.Equal(data, []byte("[DONE]")) {
				continue
			}
			var parsed map[string]interface{}
			if err := json.Unmarshal(data, &parsed); err != nil {
				continue
			}
			if errMsg, ok := parsed["error"].(string); ok && errMsg != "" {
				if details, ok := parsed["details"].(string); ok && details != "" {
					return fmt.Errorf("%s (details: %s)", errMsg, details)
				}
				return fmt.Errorf("%s", errMsg)
			}
			// Extract content from provider chunks (OpenAI/Anthropic format)
			content := extractContent(parsed)
			if content != "" && streamWriter != nil {
				if err := streamWriter(content); err != nil {
					if debugLog != nil {
						fmt.Fprintf(debugLog, "[debug] streamWriter error: %v\n", err)
					}
					return err
				}
			}
		}
	}
	err := scanner.Err()
	if debugLog != nil {
		fmt.Fprintf(debugLog, "[debug] SSE stream read finished: err=%v\n", err)
	}
	return err
}

func extractContent(parsed map[string]interface{}) string {
	// Backend wraps chunks as {"content": "...", "provider": "..."}
	if c, ok := parsed["content"].(string); ok && c != "" {
		return c
	}
	// OpenAI-style provider JSON (choices[].delta)
	choices, ok := parsed["choices"].([]interface{})
	if !ok || len(choices) == 0 {
		return ""
	}
	first, ok := choices[0].(map[string]interface{})
	if !ok {
		return ""
	}
	if delta, ok := first["delta"].(map[string]interface{}); ok {
		if c, ok := delta["content"].(string); ok {
			return c
		}
	}
	if msg, ok := first["message"].(map[string]interface{}); ok {
		if c, ok := msg["content"].(string); ok {
			return c
		}
	}
	return ""
}

func getErrorMsg(m map[string]interface{}) string {
	if m == nil {
		return "unknown error"
	}
	if e, ok := m["error"].(string); ok {
		return e
	}
	return "unknown error"
}
