package api

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

const surfaceCLI = "cli"

// Client for Prompt Keeper backend API.
type Client struct {
	BaseURL    string
	APIKey     string
	HTTPClient *http.Client
	// DebugLog: when set, request URL and response body are written here (e.g. os.Stderr for --debug).
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
		fmt.Fprintf(c.DebugLog, "[debug] response body: %s\n", string(data))
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
		logBody := map[string]interface{}{"email": email, "password": "[REDACTED]", "surface": surfaceCLI}
		if name != "" {
			logBody["name"] = name
		}
		logJSON, _ := json.Marshal(logBody)
		fmt.Fprintf(c.DebugLog, "[debug] request body: %s\n", string(logJSON))
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
		fmt.Fprintf(c.DebugLog, "[debug] response body: %s\n", string(data))
	}
	var result map[string]interface{}
	if err := json.Unmarshal(data, &result); err != nil {
		return nil, fmt.Errorf("invalid response: %s", string(data))
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

// PutKey stores a provider API key. POST /v1/keys
func (c *Client) PutKey(provider, rawSecret string) error {
	body := map[string]string{
		"raw_secret": rawSecret,
		"provider":   provider,
		"surface":    surfaceCLI,
	}
	return c.putJSON("/v1/keys", body, map[string]string{
		"provider":   provider,
		"raw_secret": "[REDACTED]",
		"surface":    surfaceCLI,
	})
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

// putJSON sends POST with body. For debug logging, bodyForLog can be a redacted copy of body (e.g. redact raw_secret).
func (c *Client) putJSON(path string, body interface{}, bodyForLog map[string]string) error {
	jsonBody, _ := json.Marshal(body)
	url := c.BaseURL + path
	req, err := http.NewRequest("POST", url, bytes.NewReader(jsonBody))
	if err != nil {
		return err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	if c.DebugLog != nil && bodyForLog != nil {
		logBody, _ := json.Marshal(bodyForLog)
		fmt.Fprintf(c.DebugLog, "[debug] POST %s\n", url)
		fmt.Fprintf(c.DebugLog, "[debug] request body: %s\n", string(logBody))
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	data, _ := io.ReadAll(resp.Body)
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] response status: %s\n", resp.Status)
		fmt.Fprintf(c.DebugLog, "[debug] response body: %s\n", string(data))
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
		logBody := make(map[string]interface{})
		for k, v := range body {
			if k == "raw_secret" {
				logBody[k] = "[REDACTED]"
			} else {
				logBody[k] = v
			}
		}
		logJSON, _ := json.Marshal(logBody)
		fmt.Fprintf(c.DebugLog, "[debug] POST %s\n", url)
		fmt.Fprintf(c.DebugLog, "[debug] request body: %s\n", string(logJSON))
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	data, _ := io.ReadAll(resp.Body)
	if c.DebugLog != nil {
		fmt.Fprintf(c.DebugLog, "[debug] response status: %s\n", resp.Status)
		fmt.Fprintf(c.DebugLog, "[debug] response body: %s\n", string(data))
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
	return c.executeStream("/v1/execute", body, streamWriter, debugLog)
}

// ExecuteRaw runs inline/raw prompt execution with streaming. POST /v1/execute-raw.
func (c *Client) ExecuteRaw(prompt string, variables map[string]interface{}, provider, model string, streamWriter func(data string) error, debugLog io.Writer) error {
	body := map[string]interface{}{
		"prompt":    prompt,
		"provider":  provider,
		"variables": variables,
		"surface":   surfaceCLI,
	}
	if model != "" {
		body["model"] = model
	}
	return c.executeStream("/v1/execute-raw", body, streamWriter, debugLog)
}

// executeStream calls an SSE execute endpoint and parses streaming chunks.
func (c *Client) executeStream(path string, body map[string]interface{}, streamWriter func(data string) error, debugLog io.Writer) error {
	jsonBody, _ := json.Marshal(body)
	url := c.BaseURL + path
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
		fmt.Fprintf(debugLog, "[debug] body: %s\n", string(jsonBody))
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
			fmt.Fprintf(debugLog, "[debug] non-200 body: %s\n", string(data))
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
func parseSSEStream(r io.Reader, streamWriter func(data string) error, debugLog io.Writer) error {
	scanner := bufio.NewScanner(r)
	scanner.Buffer(make([]byte, 64*1024), 1024*1024)
	eventNum := 0
	for scanner.Scan() {
		line := scanner.Bytes()
		if debugLog != nil && bytes.HasPrefix(line, []byte("data: ")) {
			eventNum++
			fmt.Fprintf(debugLog, "[debug] SSE event #%d (raw): %s\n", eventNum, string(line))
		}
		if bytes.HasPrefix(line, []byte("data: ")) {
			data := bytes.TrimSpace(line[6:])
			if len(data) == 0 {
				continue
			}
			if bytes.Equal(data, []byte("[DONE]")) {
				if debugLog != nil {
					fmt.Fprintf(debugLog, "[debug] SSE [DONE]\n")
				}
				continue
			}
			var parsed map[string]interface{}
			if err := json.Unmarshal(data, &parsed); err != nil {
				if debugLog != nil {
					fmt.Fprintf(debugLog, "[debug] SSE parse error: %v (raw: %s)\n", err, string(data))
				}
				continue
			}
			if errMsg, ok := parsed["error"].(string); ok && errMsg != "" {
				if debugLog != nil {
					fmt.Fprintf(debugLog, "[debug] SSE error event: %s\n", errMsg)
				}
				if details, ok := parsed["details"].(string); ok && details != "" {
					return fmt.Errorf("%s (details: %s)", errMsg, details)
				}
				return fmt.Errorf("%s", errMsg)
			}
			// Extract content from provider chunks (OpenAI/Anthropic format)
			content := extractContent(parsed)
			if content != "" && streamWriter != nil {
				if debugLog != nil {
					fmt.Fprintf(debugLog, "[debug] content chunk (%d bytes) -> stdout\n", len(content))
				}
				if err := streamWriter(content); err != nil {
					if debugLog != nil {
						fmt.Fprintf(debugLog, "[debug] streamWriter error: %v\n", err)
					}
					return err
				}
			} else if debugLog != nil && len(parsed) > 0 {
				// Log when we got a valid event but no content extracted (unknown format)
				fmt.Fprintf(debugLog, "[debug] event with no content extracted, keys: %v\n", mapKeys(parsed))
			}
		}
	}
	err := scanner.Err()
	if debugLog != nil {
		fmt.Fprintf(debugLog, "[debug] scanner finished: events=%d, err=%v\n", eventNum, err)
	}
	return err
}

func mapKeys(m map[string]interface{}) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	return keys
}

func extractContent(parsed map[string]interface{}) string {
	// Backend wraps chunks as {"content": "...", "provider": "..."}
	if c, ok := parsed["content"].(string); ok && c != "" {
		return c
	}
	// Raw provider format (OpenAI/Anthropic)
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
