//
//  SSEParser.swift
//  PromptKeeper
//
//  Parses Server-Sent Events (data: ... lines, separated by blank line).
//

import Foundation

struct SSEDataEvent {
    var data: String?
}

enum SSEParser {
    /// Parse a single SSE message block (lines ending with blank line). Returns events (one per "data:" line).
    static func parse(_ block: String) -> [SSEDataEvent] {
        var events: [SSEDataEvent] = []
        var currentData: String?
        let lines = block.split(separator: "\n", omittingEmptySubsequences: false)
        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed.isEmpty {
                if let d = currentData {
                    events.append(SSEDataEvent(data: d))
                    currentData = nil
                }
                continue
            }
            if trimmed.hasPrefix("data:") {
                let payload = String(trimmed.dropFirst(5)).trimmingCharacters(in: .whitespaces)
                currentData = payload.isEmpty ? nil : payload
            }
        }
        if let d = currentData {
            events.append(SSEDataEvent(data: d))
        }
        return events
    }

    /// If `data` is JSON of the form { "error": "..." }, returns the error message; otherwise nil.
    static func parseErrorPayload(_ data: String) -> String? {
        guard let json = data.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: json) as? [String: Any],
              let err = obj["error"] as? String else { return nil }
        return err
    }
}
