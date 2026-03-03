package com.promptkeeper.sdk

import com.promptkeeper.sdk.model.ExecuteRequest
import com.promptkeeper.sdk.model.PutKeyRequest
import com.promptkeeper.sdk.model.PutKeyResponse
import com.promptkeeper.sdk.model.PutPromptRequest
import com.promptkeeper.sdk.model.PutPromptResponse
import com.promptkeeper.sdk.sse.SSEParser
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.withContext
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.BufferedReader
import java.io.InputStreamReader
import java.util.concurrent.TimeUnit

/**
 * Prompt Keeper SDK for Android/Kotlin.
 *
 * Requires an API key (obtain via registration outside this SDK). The key is kept **in-memory only**
 * for the current app run; it is not persisted.
 *
 * Usage:
 * ```
 * val sdk = PromptKeeper(apiKey = "pk_...")
 * sdk.setKey(rawSecret = "sk-...", provider = "openai")
 * sdk.setPrompt(name = "default", rawSecret = "Hello {{name}}!", provider = "openai")
 * sdk.exec(functionId = "default", variables = mapOf("name" to "Alice")).collect { chunk -> ... }
 * ```
 */
class PromptKeeper internal constructor(
    private val apiKeyHolder: ApiKeyHolder,
    private val baseUrl: String,
    private val client: OkHttpClient
) {

    private val json = Json { ignoreUnknownKeys = true }
    private val jsonMediaType = "application/json; charset=utf-8".toMediaType()

    constructor(apiKey: String) : this(
        apiKeyHolder = ApiKeyHolder(apiKey),
        baseUrl = DEFAULT_BASE_URL.trimEnd('/'),
        client = defaultClient()
    )

    /**
     * Stores a provider API key (e.g. OpenAI, Anthropic) on the server.
     * @param rawSecret Raw API key (e.g. `sk-...`). Sent only to the server; not persisted by the SDK.
     * @param provider Provider name (e.g. `"openai"`, `"anthropic"`).
     * @return Put key response with version_id, created_at, etc.
     */
    suspend fun setKey(rawSecret: String, provider: String): PutKeyResponse = withContext(Dispatchers.IO) {
        val body = json.encodeToString(PutKeyRequest(rawSecret = rawSecret, provider = provider))
        val request = Request.Builder()
            .url("$baseUrl/v1/keys")
            .post(body.toRequestBody(jsonMediaType))
            .setAuth()
            .build()
        executeJson(request, 201) { json.decodeFromString<PutKeyResponse>(it) }
    }

    /**
     * Stores a prompt template for a named function.
     * @param name Function/prompt name (e.g. `"customer_support"`).
     * @param rawSecret Raw prompt template (e.g. Handlebars). Not persisted by the SDK.
     * @param provider Optional default provider (e.g. `"openai"`).
     * @param preferredModel Optional default model (e.g. `"gpt-4o"`, `"claude-3-5-sonnet-20240620"`).
     * @return Put prompt response with version_id, created_at, etc.
     */
    suspend fun setPrompt(
        name: String,
        rawSecret: String,
        provider: String? = null,
        preferredModel: String? = null
    ): PutPromptResponse = withContext(Dispatchers.IO) {
        val body = json.encodeToString(
            PutPromptRequest(
                name = name,
                rawSecret = rawSecret,
                provider = provider,
                preferredModel = preferredModel
            )
        )
        val request = Request.Builder()
            .url("$baseUrl/v1/prompts")
            .post(body.toRequestBody(jsonMediaType))
            .setAuth()
            .build()
        executeJson(request, 201) { json.decodeFromString<PutPromptResponse>(it) }
    }

    /**
     * Executes a function: resolves prompt, injects variables, calls the configured LLM, and streams the response.
     *
     * Each emitted value is the raw SSE `data` payload (always a string). Content type depends on the provider and function:
     * - **Chat/completions**: Usually JSON stream chunks (e.g. OpenAI `choices[].delta.content`, Anthropic text deltas).
     * - **Image generation** (e.g. OpenAI DALL·E): The payload is still a string — JSON containing e.g. `b64_json` (base64 image)
     *   or `url`. Parse the string as JSON and extract image data as needed; decode base64 or fetch the URL for display.
     * - **Video**: Same idea — provider-specific JSON with URL or base64; parse each chunk accordingly.
     *
     * The transport is always text (SSE); binary media are embedded as base64 or URLs inside that text.
     *
     * @param functionId Function identifier (e.g. `"default"`, `"customer_support_reply"`).
     * @param variables Optional map of variable names to string values (Handlebars). Default: empty.
     * @param provider Optional preferred provider (e.g. `"openai"`, `"anthropic"`).
     * @param model Optional model override.
     * @return Flow of SSE data chunks (provider payload strings). On server error, throws [PromptKeeperException.Server].
     */
    fun exec(
        functionId: String,
        variables: Map<String, String> = emptyMap(),
        provider: String? = null,
        model: String? = null
    ): Flow<String> = flow {
        val variablesJson = buildJsonObject {
            variables.forEach { (k, v) -> put(k, JsonPrimitive(v)) }
        }
        val body = json.encodeToString(
            ExecuteRequest(
                functionId = functionId,
                variables = variablesJson,
                provider = provider,
                model = model
            )
        )
        val request = Request.Builder()
            .url("$baseUrl/v1/execute")
            .post(body.toRequestBody(jsonMediaType))
            .setAuth()
            .build()
        withContext(Dispatchers.IO) {
            client.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    throw PromptKeeperException.Http(response.code, response.body?.bytes())
                }
                val bodyStream = response.body?.byteStream() ?: return@use
                BufferedReader(InputStreamReader(bodyStream)).use { reader ->
                    var buffer = StringBuilder()
                    reader.useLines { lines ->
                        for (line in lines) {
                            buffer.append(line).append("\n")
                            if (buffer.endsWith("\n\n")) {
                                for (data in SSEParser.parse(buffer.toString())) {
                                    if (data.isEmpty()) continue
                                    val err = SSEParser.parseErrorPayload(data)
                                    if (err != null) throw PromptKeeperException.Server(err)
                                    emit(data)
                                }
                                buffer = StringBuilder()
                            }
                        }
                    }
                    if (buffer.isNotEmpty()) {
                        for (data in SSEParser.parse(buffer.toString())) {
                            if (data.isEmpty()) continue
                            val err = SSEParser.parseErrorPayload(data)
                            if (err != null) throw PromptKeeperException.Server(err)
                            emit(data)
                        }
                    }
                }
            }
        }
    }

    private fun Request.Builder.setAuth(): Request.Builder {
        addHeader("Authorization", "Bearer ${apiKeyHolder.apiKey}")
        addHeader("X-API-Key", apiKeyHolder.apiKey)
        return this
    }

    private fun <T> executeJson(request: Request, expectedStatus: Int, block: (String) -> T): T {
        client.newCall(request).execute().use { response ->
            val body = response.body?.string() ?: ""
            if (response.code != expectedStatus) {
                throw PromptKeeperException.Http(response.code, body.toByteArray())
            }
            return block(body)
        }
    }

    companion object {
        const val DEFAULT_BASE_URL = "http://localhost:3000"

        @Volatile
        private var defaultInstance: PromptKeeper? = null

        /**
         * Initializes the SDK with an API key (the "init" step). Key is kept in-memory only for the current app run.
         * Use [getInstance] for consecutive API calls, or keep the returned instance.
         * @param apiKey API key (e.g. `pk_...`) obtained via registration outside this SDK.
         * @return The initialized [PromptKeeper] instance.
         */
        @JvmStatic
        fun initialize(apiKey: String): PromptKeeper {
            val instance = PromptKeeper(apiKey)
            defaultInstance = instance
            return instance
        }

        /**
         * Returns the instance set by [initialize], or null if [initialize] was never called.
         */
        @JvmStatic
        fun getInstance(): PromptKeeper? = defaultInstance

        private fun defaultClient(): OkHttpClient = OkHttpClient.Builder()
            .connectTimeout(30, TimeUnit.SECONDS)
            .readTimeout(90, TimeUnit.SECONDS)
            .writeTimeout(30, TimeUnit.SECONDS)
            .build()
    }
}
