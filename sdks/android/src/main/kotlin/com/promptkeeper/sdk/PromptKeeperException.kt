package com.promptkeeper.sdk

/**
 * Base exception for SDK errors.
 */
sealed class PromptKeeperException(message: String, cause: Throwable? = null) : Exception(message, cause) {

    /** HTTP error: non-success status code. */
    data class Http(val statusCode: Int, val body: ByteArray?) : PromptKeeperException("HTTP $statusCode") {
        override fun equals(other: Any?): Boolean =
            other is Http && statusCode == other.statusCode && body.contentEquals(other.body)
        override fun hashCode(): Int = 31 * statusCode + (body?.contentHashCode() ?: 0)
    }

    /** Server returned an error in SSE or JSON (e.g. `{ "error": "..." }`). */
    data class Server(val message: String) : PromptKeeperException(message)

    /** Network or I/O error. */
    data class Network(override val cause: Throwable) : PromptKeeperException(cause.message ?: "Network error", cause)
}
