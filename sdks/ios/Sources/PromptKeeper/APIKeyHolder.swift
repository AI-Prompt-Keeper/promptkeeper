//
//  APIKeyHolder.swift
//  PromptKeeper
//
//  In-memory only storage for the API key. Not persisted across app launches.
//

import Foundation

/// Holds the API key in memory only. No persistence (no UserDefaults, Keychain, or file).
final class APIKeyHolder: @unchecked Sendable {
    private var _apiKey: String
    init(apiKey: String) {
        self._apiKey = apiKey
    }
    var apiKey: String { _apiKey }
}
