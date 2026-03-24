package ai.promptkeeper.example

import ai.promptkeeper.sdk.PromptKeeperException
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

private val json = Json { ignoreUnknownKeys = true }

fun Throwable.toUserMessage(): String = when (this) {
    is PromptKeeperException.Server -> detailMessage
    is PromptKeeperException.Http -> formatHttp(this)
    is PromptKeeperException.Network -> cause?.message ?: message ?: "Network error. Check connectivity."
    else -> message ?: "Something went wrong."
}

private fun formatHttp(e: PromptKeeperException.Http): String {
    val bodyStr = e.body?.toString(Charsets.UTF_8).orEmpty()
    val serverMsg = try {
        json.parseToJsonElement(bodyStr).jsonObject["error"]?.jsonPrimitive?.content
    } catch (_: Exception) {
        null
    }
    val detail = serverMsg ?: bodyStr.ifBlank { null } ?: "HTTP ${e.statusCode}"
    val prefix = when (e.statusCode) {
        401 -> "Sign-in failed or API key is invalid."
        403 -> "This key is not allowed to do that. "
        404 -> "Not found. "
        409 -> "Conflict. "
        422 -> "Invalid request. "
        429 -> "Too many requests. "
        500, 502, 503 -> "Server error. "
        else -> ""
    }
    return "$prefix$detail".trim()
}
