# PromptKeeper Example (iOS)

A minimal SwiftUI iOS app that demonstrates integration with the [PromptKeeper](../../sdks/ios) SDK. It uses the library via **Swift Package Manager** (local package reference) and showcases:

- **Text exec** — streaming text from **OpenAI**, **Gemini**, or **Anthropic** using an **inline prompt** (no stored function; uses `execPrompt`).

## Requirements

- Xcode 15+ / Swift 5.9+
- iOS 15+
- Prompt Keeper API key (see below)

## Setup

1. **Open the project**  
   Open `PromptKeeperExample.xcodeproj` in Xcode. The app depends on the PromptKeeper package at `../../sdks/ios` (relative to this folder). If you opened the repo at the root, the local package should resolve automatically.

2. **Obtain a Prompt Keeper API key**  
   Get your API key via the Prompt Keeper CLI (e.g. `prke login` or your backend’s registration flow). **Do not store plaintext API keys in source code or commit them.** This example uses in-memory configuration only; for production, use Keychain or a secure backend.

3. **Configure provider keys on the server**  
   Before running the app, store your OpenAI, Gemini, and/or Anthropic API keys on the Prompt Keeper server (e.g. via the CLI or your backend). The example app only calls `execPrompt`; it does not persist or send raw provider keys.

4. **Run the app**  
   Select an iOS Simulator or device and run (⌘R). Use the **Config** tab to enter your Prompt Keeper API key (session only). Use **Text Query** to choose a provider (radio buttons) and run an inline prompt via `execPrompt`.

## Project layout

- `PromptKeeperExample/` — SwiftUI app target
  - `Keys.swift` — Stub for API key (see comments; do not commit real keys).
  - `Services/ExecService.swift` — Uses PromptKeeper `execPrompt` (inline prompt) and streams the selected provider’s output.
  - `Services/StreamParser.swift` — Parses provider-specific SSE chunks (Foundation only).
  - `Views/TextExecView.swift` — Inline prompt text exec via `execPrompt` with provider selection.

## No persistence

The app does not persist the Prompt Keeper API key or any secrets. Configuration is in-memory for the current run only.
