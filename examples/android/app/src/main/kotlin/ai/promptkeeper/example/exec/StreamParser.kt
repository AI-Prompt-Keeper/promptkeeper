package ai.promptkeeper.example.exec

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

private val json = Json { ignoreUnknownKeys = true }

/**
 * Parses a streaming text chunk from either:
 * - Backend format: `{"content":"...","provider":"openai"}` (top-level "content").
 * - OpenAI format: `{"choices":[{"delta":{"content":"..."}}]}`.
 * Returns null if neither is present or not valid JSON.
 */
fun parseOpenAITextChunk(data: String): String? {
    return try {
        val obj = json.parseToJsonElement(data).jsonObject
        obj["content"]?.jsonPrimitive?.content
            ?: run {
                val choices = obj["choices"]?.jsonArray ?: return null
                val first = choices.getOrNull(0)?.jsonObject ?: return null
                val delta = first["delta"]?.jsonObject ?: return null
                delta["content"]?.jsonPrimitive?.content
            }
    } catch (_: Exception) {
        null
    }
}
