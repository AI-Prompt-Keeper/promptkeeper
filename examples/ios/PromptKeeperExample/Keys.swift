//
//  Keys.swift
//  PromptKeeperExample
//
//  API key stubs for the example app. No persistence; in-memory only for this run.
//

import Foundation

/// How to obtain the Prompt Keeper API key:
/// - Use the Prompt Keeper CLI: run `prke login` (or equivalent) to authenticate,
///   then obtain an API key from your backend or the CLI (e.g. `prke whoami` or your server’s registration flow).
/// - Do not commit real keys. Use environment variables or a secure credential store in production.
///
/// IMPORTANT: Using plaintext keys in code is for example/demo purposes only.
/// Do not store API keys in source code and do not commit them to version control.
/// In production, obtain keys at runtime (e.g. from Keychain, backend, or environment).
enum Keys {
    /// Prompt Keeper API key (obtain via CLI / backend registration).
    static let promptKeeperAPIKey = ""

    /// OpenAI API key (stored on Prompt Keeper server via setKey; this stub is only for local setup instructions).
    static let openAIKeyStub = ""

    /// Google/Gemini API key (stored on Prompt Keeper server via setKey; this stub is only for local setup).
    static let geminiKeyStub = ""
}
