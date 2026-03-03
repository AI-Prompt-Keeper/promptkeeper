//
//  StreamReader.swift
//  PromptKeeper
//
//  Reads URLSession.AsyncBytes into SSE message blocks (text separated by blank lines).
//

import Foundation

enum StreamReader {
    /// Consume `bytes` and invoke `onBlock` for each SSE block (string ending with "\n\n").
    /// If `onBlock` throws, the error is propagated.
    static func readSSEBlocks(
        from bytes: URLSession.AsyncBytes,
        onBlock: (String) async throws -> Void
    ) async throws {
        var buffer = [UInt8]()
        let newline = UInt8(ascii: "\n")
        let carriageReturn = UInt8(ascii: "\r")

        for try await byte in bytes {
            buffer.append(byte)
            let len = buffer.count
            let isDoubleNewline = (len >= 2 && buffer[len - 1] == newline && buffer[len - 2] == newline)
                || (len >= 4 && buffer[len - 1] == newline && buffer[len - 2] == carriageReturn
                    && buffer[len - 3] == newline && buffer[len - 4] == carriageReturn)
            if isDoubleNewline {
                let block = String(bytes: buffer, encoding: .utf8) ?? ""
                buffer = []
                try await onBlock(block)
            }
        }
        if !buffer.isEmpty {
            let block = String(bytes: buffer, encoding: .utf8) ?? ""
            try await onBlock(block)
        }
    }
}
