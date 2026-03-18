//
//  StreamParser.swift
//  PromptKeeperExample
//
//  Parses provider-specific SSE chunks from PromptKeeper exec stream (Foundation only; no manual networking).
//

import Foundation

/// OpenAI chat completion stream chunk (choices[].delta.content).
struct OpenAIStreamParser {
    /// Extracts the first delta content string from an OpenAI SSE data line.
    /// Returns nil if the chunk has no content or parsing fails.
    static func parseDeltaContent(from chunk: String) -> String? {
        guard let data = chunk.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return nil }

        // Backend envelope: { "content": "...", "provider": "openai" }
        if let content = json["content"] as? String, !content.isEmpty {
            return content
        }

        // OpenAI streaming payload: { "choices":[{"delta":{"content":"..."}}] }
        guard let choices = json["choices"] as? [[String: Any]],
              let first = choices.first,
              let delta = first["delta"] as? [String: Any],
              let content = delta["content"] as? String, !content.isEmpty else { return nil }
        return content
    }
}

/// Gemini response chunk (e.g. text or inline image data).
struct GeminiStreamParser {
    /// Extracts text from a Gemini-style chunk. Adapt key paths if your backend returns a different structure.
    static func parseText(from chunk: String) -> String? {
        guard let data = chunk.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return nil }
        // Backend envelope: { "content": "...", "provider": "gemini" }
        if let content = json["content"] as? String, !content.isEmpty {
            return content
        }
        if let text = json["text"] as? String, !text.isEmpty { return text }
        if let candidates = json["candidates"] as? [[String: Any]],
           let content = candidates.first?["content"] as? [String: Any],
           let parts = content["parts"] as? [[String: Any]] {
            return parts.compactMap { $0["text"] as? String }.joined()
        }
        return nil
    }

    /// Extracts base64 image data from a Gemini-style chunk (e.g. image generation response).
    static func parseInlineImageBase64(from chunk: String) -> Data? {
        guard let data = chunk.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return nil }
        // Backend envelope for images: { "b64_json": "...", "provider": "gemini" }
        if let b64 = json["b64_json"] as? String, !b64.isEmpty, let decoded = Data(base64Encoded: b64) {
            return decoded
        }
        if let b64 = json["inlineData"] as? [String: Any], let dataBase64 = b64["data"] as? String {
            return Data(base64Encoded: dataBase64)
        }
        if let candidates = json["candidates"] as? [[String: Any]],
           let content = candidates.first?["content"] as? [String: Any],
           let parts = content["parts"] as? [[String: Any]] {
            for part in parts {
                if let inlineData = part["inlineData"] as? [String: Any], let dataBase64 = inlineData["data"] as? String,
                   let d = Data(base64Encoded: dataBase64) { return d }
            }
        }
        return nil
    }
}

/// Anthropic streaming text chunk.
///
/// The backend typically emits an envelope:
/// `{ "content": "...", "provider": "anthropic" }`
///
/// If that envelope is not present, we also attempt to parse a common Anthropic payload shape:
/// `{ "delta": { "text": "..." } }`.
struct AnthropicStreamParser {
    static func parseText(from chunk: String) -> String? {
        guard let data = chunk.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return nil }

        // Backend envelope: { "content": "...", "provider": "anthropic" }
        if let content = json["content"] as? String, !content.isEmpty {
            return content
        }

        // Common streaming payload: { "delta": { "text": "..." } }
        if let delta = json["delta"] as? [String: Any], let text = delta["text"] as? String, !text.isEmpty {
            return text
        }

        // Fallback keys seen in some payloads.
        if let text = json["text"] as? String, !text.isEmpty {
            return text
        }

        return nil
    }
}
