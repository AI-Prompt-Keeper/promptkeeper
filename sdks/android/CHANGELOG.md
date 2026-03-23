# Changelog — Android SDK

## 1.0.4

- **API surface:** The public SDK supports **stored prompts only** (`setPrompt` + `exec` → `POST /v1/execute`).  
  Inline/raw prompt execution (`POST /v1/execute-raw`) is **not** exposed by this library.
