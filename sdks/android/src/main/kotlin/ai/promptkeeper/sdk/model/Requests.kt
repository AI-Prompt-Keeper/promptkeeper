package ai.promptkeeper.sdk.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
internal data class PutKeyRequest(
    @SerialName("raw_secret") val rawSecret: String,
    val provider: String
)

@Serializable
internal data class PutPromptRequest(
    val name: String,
    @SerialName("raw_secret") val rawSecret: String,
    val provider: String? = null,
    @SerialName("preferred_model") val preferredModel: String? = null
)

@Serializable
internal data class ExecuteRequest(
    @SerialName("function_id") val functionId: String,
    val variables: Map<String, kotlinx.serialization.json.JsonElement> = emptyMap(),
    val provider: String? = null,
    val model: String? = null
)

/** Request body for POST /v1/execute-raw. Backend requires prompt and provider. */
@Serializable
internal data class ExecuteRawRequest(
    val prompt: String,
    val provider: String,
    val model: String? = null,
    val variables: Map<String, kotlinx.serialization.json.JsonElement> = emptyMap()
)
