//
//  Models.swift
//  PromptKeeper
//
//  Request/response DTOs for Prompt Keeper API.
//

import Foundation

// MARK: - Decoding helpers

private extension KeyedDecodingContainer {
    /// Backend returns `version_id` as JSON number (`i64`); some environments may use a string.
    func decodeVersionIdString(forKey key: Key) throws -> String {
        if let n = try? decode(Int64.self, forKey: key) {
            return String(n)
        }
        if let n = try? decode(Int.self, forKey: key) {
            return String(n)
        }
        return try decode(String.self, forKey: key)
    }
}

// MARK: - Set key

struct PutKeyRequest: Encodable {
    let raw_secret: String
    let provider: String
    let surface: String?
}

/// Response from setKey (store provider API key).
public struct PutKeyResponse: Decodable, Sendable {
    /// Opaque version identifier for the stored key (server sends a numeric id; exposed as string).
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

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        versionId = try c.decodeVersionIdString(forKey: .versionId)
        createdAt = try c.decode(String.self, forKey: .createdAt)
        kmsKeyArn = try c.decodeIfPresent(String.self, forKey: .kmsKeyArn)
        fingerprint = try c.decodeIfPresent(String.self, forKey: .fingerprint)
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
    /// Opaque version identifier for the stored prompt (server sends a numeric id; exposed as string).
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

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        versionId = try c.decodeVersionIdString(forKey: .versionId)
        createdAt = try c.decode(String.self, forKey: .createdAt)
        kmsKeyArn = try c.decodeIfPresent(String.self, forKey: .kmsKeyArn)
        fingerprint = try c.decodeIfPresent(String.self, forKey: .fingerprint)
    }
}

// MARK: - List prompts

/// Response from `GET /v1/list-prompts`.
public struct ListPromptsResponse: Decodable, Sendable {
    /// Stored prompt function names (titles) available for execution.
    public let titles: [String]
}

// MARK: - Execute

struct ExecuteRequest: Encodable {
    let function_id: String
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
