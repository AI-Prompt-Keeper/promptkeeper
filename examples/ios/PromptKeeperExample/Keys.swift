//
//  Keys.swift
//  PromptKeeperExample
//
//  API key stubs for the example app. No persistence; keys are compile-time constants for this target.
//

import Foundation

/// How to obtain Prompt Keeper API keys:
/// - Register (e.g. via CLI / backend) to receive a **management** key.
/// - Mint an **execution** key with `POST /v1/auth/api-tokens` using the management key (see `backend/README.md`).
///
/// **Security:** Do not commit real keys. In production, load credentials from Keychain, your backend, or secure config.
///
/// **Management key in this app:** A management key can create prompts and provider keys. **You should not embed a
/// management key in a shipped client app** — it is included here **only for demonstration** so the example can call
/// `setPrompt` without a separate backend. Treat it like a secret you would normally keep server-side.
enum Keys {
    /// Management client API key (`pk_mgt_live_…`). Demo-only in-source; **do not ship in production apps.**
    static let managementAPIKey = ""

    /// Execution client API key (`pk_exe_live_…`). Intended for client-side list + execute; still avoid committing real keys.
    static let executionAPIKey = ""
}
