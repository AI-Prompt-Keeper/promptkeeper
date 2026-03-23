# PromptKeeper iOS Example

Example iOS app that integrates the PromptKeeper Swift SDK. It demonstrates:

- **Text (OpenAI, Gemini, Anthropic)**: Registers a stored prompt `text` with template `{{prompt}}`, then runs `exec(functionId: "text", variables: ["prompt": ...])` and streams the response.
- **Image (Gemini)**: Runs `exec` with a stored image prompt and displays base64 or URL output.

## Requirements

- Xcode 15+ (Swift 5.9+).
- iOS 17+ deployment target (adjust in project settings if needed).

## Setup

1. **Open the project**: Open `examples/ios/PromptKeeperExample.xcodeproj` (or the workspace if you use one).

2. **API keys**:
   - **PromptKeeper API key** (`pk_...`): Obtain via your registration flow or CLI.
   - **Provider keys**: Register OpenAI, Gemini, and/or Anthropic keys with PromptKeeper (e.g. via CLI) so the backend can call the LLM. The example does not embed provider secrets.

3. Run on Simulator or device (**⌘R**). Use the **Config** tab for the Prompt Keeper API key. Use **Text Query** to pick a provider and run against the stored `text` prompt.

## Project layout

- `Services/ExecService.swift` — `setPrompt` for `text` / `exec` streaming with provider-specific parsers.
- `Views/TextExecView.swift` — Text streaming via stored prompt `text`.

## License

See the repository root license.
