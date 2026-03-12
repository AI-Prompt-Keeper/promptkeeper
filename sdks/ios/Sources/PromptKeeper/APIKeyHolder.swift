//
//  APIKeyHolder.swift
//  PromptKeeper
//
//  In-memory only storage for the API key. Not persisted across app launches.
//

import Foundation

/// Holds the API key in memory only. No persistence (no UserDefaults, Keychain, or file).
///
/// Marked `@unchecked Sendable` because the stored key is set at init and never mutated;
/// reads are safe from any actor/thread. Not persisted across app launches.
final class APIKeyHolder: @unchecked Sendable {
    private let _apiKey: String

    init(apiKey: String) {
        self._apiKey = apiKey
    }

    /// The in-memory API key. Do not log or expose to the UI.
    var apiKey: String { _apiKey }
}
