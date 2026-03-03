package com.promptkeeper.sdk.sse

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.primitiveContent

/**
 * Parses Server-Sent Events (SSE) format: lines like "data: ...", separated by blank lines.
 */
internal object SSEParser {

    private val json = Json { ignoreUnknownKeys = true }

    /**
     * Parses a single SSE message block (ended by \n\n). Returns list of event data strings.
     */
    fun parse(block: String): List<String> {
        val result = mutableListOf<String>()
        var data = StringBuilder()
        for (line in block.split("\n")) {
            val trimmed = line.trim()
            when {
                trimmed.startsWith("data:") -> data.append(trimmed.removePrefix("data:").trim()).append("\n")
                trimmed.isEmpty() -> {
                    val d = data.toString().trim()
                    if (d.isNotEmpty()) result.add(d)
                    data = StringBuilder()
                }
            }
        }
        val d = data.toString().trim()
        if (d.isNotEmpty()) result.add(d)
        return result
    }

    /**
     * If [data] is JSON with an "error" field, returns that message; otherwise null.
     */
    fun parseErrorPayload(data: String): String? {
        return try {
            val obj = json.parseToJsonElement(data).jsonObject
            obj["error"]?.jsonPrimitive?.primitiveContent
        } catch (_: Exception) {
            null
        }
    }
}
