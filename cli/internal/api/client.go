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

// Client for Prompt Keeper backend API.
type Client struct {
	BaseURL    string
	APIKey     string
	HTTPClient *http.Client
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

// Register creates a new user. POST /v1/auth/register
func (c *Client) Register(email, password, name string) (map[string]interface{}, error) {
	body := map[string]interface{}{
		"email":    email,
		"password": password,
	}
	if name != "" {
		body["name"] = name
	}
	jsonBody, _ := json.Marshal(body)
	req, err := http.NewRequest("POST", c.BaseURL+"/v1/auth/register", bytes.NewReader(jsonBody))
	if err != nil {
		return nil, err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
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
	var result map[string]interface{}
	if err := json.Unmarshal(data, &result); err != nil {
		return nil, fmt.Errorf("invalid response: %s", string(data))
	}
	if resp.StatusCode != http.StatusCreated {
		msg := getErrorMsg(result)
		return nil, fmt.Errorf("register failed (%d): %s", resp.StatusCode, msg)
	}
	return result, nil
}

// PutKey stores a provider API key. POST /v1/keys
func (c *Client) PutKey(provider, rawSecret string) error {
	body := map[string]string{
		"raw_secret": rawSecret,
		"provider":   provider,
	}
	return c.putJSON("/v1/keys", body)
}

// PutPrompt stores a prompt template. POST /v1/prompts
func (c *Client) PutPrompt(name, rawSecret, provider string) error {
	body := map[string]interface{}{
		"name":       name,
		"raw_secret": rawSecret,
	}
	if provider != "" {
		body["provider"] = provider
	}
	return c.putPromptBody(body)
}

func (c *Client) putJSON(path string, body interface{}) error {
	jsonBody, _ := json.Marshal(body)
	req, err := http.NewRequest("POST", c.BaseURL+path, bytes.NewReader(jsonBody))
	if err != nil {
		return err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	data, _ := io.ReadAll(resp.Body)
	var result map[string]interface{}
	_ = json.Unmarshal(data, &result)
	if resp.StatusCode != http.StatusCreated && resp.StatusCode != http.StatusOK {
		msg := getErrorMsg(result)
		return fmt.Errorf("%s: %s", resp.Status, msg)
	}
	return nil
}

func (c *Client) putPromptBody(body map[string]interface{}) error {
	jsonBody, _ := json.Marshal(body)
	req, err := http.NewRequest("POST", c.BaseURL+"/v1/prompts", bytes.NewReader(jsonBody))
	if err != nil {
		return err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	data, _ := io.ReadAll(resp.Body)
	var result map[string]interface{}
	_ = json.Unmarshal(data, &result)
	if resp.StatusCode != http.StatusCreated && resp.StatusCode != http.StatusOK {
		msg := getErrorMsg(result)
		return fmt.Errorf("%s: %s", resp.Status, msg)
	}
	return nil
}

// Execute runs the execute endpoint with streaming. POST /v1/execute
// StreamWriter is called for each SSE data chunk. For streaming, extract content from provider chunks.
func (c *Client) Execute(functionID string, variables map[string]interface{}, provider string, streamWriter func(data string) error) error {
	body := map[string]interface{}{
		"function_id": functionID,
		"variables":   variables,
	}
	if provider != "" {
		body["provider"] = provider
	}
	jsonBody, _ := json.Marshal(body)
	req, err := http.NewRequest("POST", c.BaseURL+"/v1/execute", bytes.NewReader(jsonBody))
	if err != nil {
		return err
	}
	for k, v := range c.authHeaders() {
		req.Header.Set(k, v)
	}
	req.Header.Set("Accept", "text/event-stream")
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		data, _ := io.ReadAll(resp.Body)
		var result map[string]interface{}
		_ = json.Unmarshal(data, &result)
		msg := getErrorMsg(result)
		return fmt.Errorf("%s: %s", resp.Status, msg)
	}
	return parseSSEStream(resp.Body, streamWriter)
}

// parseSSEStream reads SSE events and calls streamWriter for each data payload.
// Extracts content from OpenAI/Anthropic-style chunks and errors.
func parseSSEStream(r io.Reader, streamWriter func(data string) error) error {
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
				return fmt.Errorf("%s", errMsg)
			}
			// Extract content from provider chunks (OpenAI/Anthropic format)
			content := extractContent(parsed)
			if content != "" && streamWriter != nil {
				if err := streamWriter(content); err != nil {
					return err
				}
			}
		}
	}
	return scanner.Err()
}

func extractContent(parsed map[string]interface{}) string {
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
