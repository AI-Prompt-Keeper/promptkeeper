package com.promptkeeper.sdk

/**
 * Holds the API key in memory only. Not persisted (no SharedPreferences, DataStore, or file).
 * Key is valid only for the current app process.
 */
internal class ApiKeyHolder(apiKey: String) {
    @Volatile
    private var key: String = apiKey

    val apiKey: String get() = key
}
