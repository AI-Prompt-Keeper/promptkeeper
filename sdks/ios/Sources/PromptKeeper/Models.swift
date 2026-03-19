//
//  Models.swift
//  PromptKeeper
//
//  Request/response DTOs for Prompt Keeper API.
//

import Foundation

// MARK: - Set key

struct PutKeyRequest: Encodable {
    let raw_secret: String
    let provider: String
    let surface: String?
}

/// Response from setKey (store provider API key).
public struct PutKeyResponse: Decodable, Sendable {
    /// Opaque version identifier for the stored key.
    public let versionId: String
    /// ISO 8601 timestamp when the key was created.
    public let createdAt: String
    /// KMS key ARN if server-side encryption is used.
    public let kmsKeyArn: String?
    /// Key fingerprint for verification, if provided by the server.
    public let fingerprint: String?

    enum CodingKeys: String, CodingKey {
        case versionId = "version_id"
        case createdAt = "created_at"
        case kmsKeyArn = "kms_key_arn"
        case fingerprint
    }
}

// MARK: - Set prompt

struct PutPromptRequest: Encodable {
    let name: String
    let raw_secret: String
    let provider: String?
    let preferred_model: String?
    let surface: String?
}

/// Response from setPrompt (store prompt template).
public struct PutPromptResponse: Decodable, Sendable {
    /// Opaque version identifier for the stored prompt.
    public let versionId: String
    /// ISO 8601 timestamp when the prompt was created.
    public let createdAt: String
    /// KMS key ARN if server-side encryption is used.
    public let kmsKeyArn: String?
    /// Content fingerprint for verification, if provided by the server.
    public let fingerprint: String?

    enum CodingKeys: String, CodingKey {
        case versionId = "version_id"
        case createdAt = "created_at"
        case kmsKeyArn = "kms_key_arn"
        case fingerprint
    }
}

// MARK: - Execute

struct ExecuteRequest: Encodable {
    let function_id: String
    let variables: [String: String]
    let provider: String?
    let model: String?
    let surface: String?
}

/// Request body for raw prompt execution (POST /v1/execute-raw).
struct ExecutePromptRequest: Encodable {
    let prompt: String
    let variables: [String: String]
    let provider: String?
    let model: String?
    let surface: String?
}

/// One item from the exec SSE stream.
public enum ExecStreamEvent: Sendable {
    /// A data chunk from the LLM stream (provider-specific payload).
    case chunk(String)
}

// MARK: - Errors

/// Errors thrown by the Prompt Keeper SDK.
public enum PromptKeeperError: Error, Sendable {
    /// HTTP request failed with the given status code and response body.
    case httpStatus(Int, body: Data)
    /// Server returned an error payload (e.g. from the execute stream).
    case serverError(String)

    /// Human-readable error message suitable for logging or display.
    public var message: String {
        switch self {
        case .httpStatus(let code, let body):
            if let str = String(data: body, encoding: .utf8), !str.isEmpty {
                return "HTTP \(code): \(str)"
            }
            return "HTTP \(code)"
        case .serverError(let msg):
            return msg
        }
    }
}
