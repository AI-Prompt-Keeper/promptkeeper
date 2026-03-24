package ai.promptkeeper.example.api

import ai.promptkeeper.sdk.PromptKeeper
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.parseToJsonElement
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

/**
 * GET /v1/list-prompts — not exposed on older SDK versions; implemented here for the example app.
 * See [backend/README.md](https://github.com/AI-Prompt-Keeper/promptkeeper/blob/main/backend/README.md).
 */
object ListPromptsClient {

    private val json = Json { ignoreUnknownKeys = true }

    private val client: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(30, TimeUnit.SECONDS)
        .readTimeout(90, TimeUnit.SECONDS)
        .writeTimeout(30, TimeUnit.SECONDS)
        .build()

    @Serializable
    private data class ListPromptsResponse(
        val titles: List<String> = emptyList()
    )

    suspend fun fetchTitles(apiKey: String): Result<List<String>> = withContext(Dispatchers.IO) {
        runCatching {
            val base = PromptKeeper.DEFAULT_BASE_URL.trimEnd('/')
            val url = "$base/v1/list-prompts?surface=android"
            val request = Request.Builder()
                .url(url)
                .get()
                .addHeader("Authorization", "Bearer $apiKey")
                .addHeader("X-API-Key", apiKey)
                .build()
            client.newCall(request).execute().use { response ->
                val body = response.body?.string().orEmpty()
                if (!response.isSuccessful) {
                    val err = parseErrorJson(body) ?: body.ifBlank { "HTTP ${response.code}" }
                    error(err)
                }
                json.decodeFromString(ListPromptsResponse.serializer(), body).titles.sorted()
            }
        }
    }

    private fun parseErrorJson(body: String): String? {
        return try {
            parseToJsonElement(body).jsonObject["error"]?.jsonPrimitive?.content
        } catch (_: Exception) {
            null
        }
    }
}
