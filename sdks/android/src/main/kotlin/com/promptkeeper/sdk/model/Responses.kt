package com.promptkeeper.sdk.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/** Response from POST /v1/keys (store provider API key). */
@Serializable
data class PutKeyResponse(
    @SerialName("version_id") val versionId: Long,
    @SerialName("created_at") val createdAt: String,
    @SerialName("kms_key_arn") val kmsKeyArn: String,
    val fingerprint: String
)

/** Response from POST /v1/prompts (store prompt template). */
@Serializable
data class PutPromptResponse(
    @SerialName("version_id") val versionId: Long,
    @SerialName("created_at") val createdAt: String,
    @SerialName("kms_key_arn") val kmsKeyArn: String,
    val fingerprint: String
)
