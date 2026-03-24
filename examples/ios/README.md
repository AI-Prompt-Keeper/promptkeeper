# PromptKeeper iOS Example

Example iOS app that integrates the PromptKeeper Swift SDK. It demonstrates:

- **Add prompt** (management key `pk_mgt_live_…`): Stores a named prompt template via `setPrompt`.
- **Execute prompt** (execution key `pk_exe_live_…`): Lists titles with `GET /v1/list-prompts`, then runs `exec(functionId: …)` and streams the response. Raw inline prompts are not used.

## Requirements

- Xcode 15+ (Swift 5.9+).
- iOS 15+ deployment target (see project settings).

## Setup

1. **Open the project**: Open `examples/ios/PromptKeeperExample.xcodeproj` (or the workspace if you use one).

2. **API keys** (in `Keys.swift`):
   - **Management** (`pk_mgt_live_…`): From registration; required only to store prompts in this demo. Do not ship management keys in production clients.
   - **Execution** (`pk_exe_live_…`): Mint with `POST /v1/auth/api-tokens` using the management key (see `backend/README.md`).
   - **Provider keys**: Register OpenAI / Anthropic / Gemini keys with PromptKeeper (e.g. via CLI) so the backend can call the LLM. The example does not embed provider secrets.

3. Run on Simulator or device (**⌘R**). Use **Add prompt** to store a template, then **Execute prompt** to list and run stored prompts.

## Project layout

- `Services/ExecService.swift` — `setPrompt` (management), `listPrompts` + `exec` streaming (execution).
- `Services/UserFacingError.swift` — Maps `PromptKeeperError` and network errors to alert copy.
- `Views/AddPromptView.swift` — Store prompt form.
- `Views/ExecutePromptView.swift` — List prompts, run `exec`, show streamed output.

## License

See the repository root license.
