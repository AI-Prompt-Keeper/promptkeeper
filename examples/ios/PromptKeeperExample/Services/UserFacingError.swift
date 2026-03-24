//
//  UserFacingError.swift
//  PromptKeeperExample
//
//  Maps SDK and network errors to short explanations for alerts.
//

import Foundation
import PromptKeeper

private struct BackendErrorBody: Decodable {
    let error: String
}

enum UserFacingError {
    static func message(for error: Error) -> String {
        if let exec = error as? ExecError {
            return exec.errorDescription ?? String(describing: error)
        }
        if let pk = error as? PromptKeeperError {
            switch pk {
            case .httpStatus(let code, let body):
                if let decoded = try? JSONDecoder().decode(BackendErrorBody.self, from: body) {
                    return mapHttp(code: code, serverMessage: decoded.error)
                }
                if let str = String(data: body, encoding: .utf8), !str.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    return mapHttp(code: code, serverMessage: str)
                }
                return mapHttp(code: code, serverMessage: nil)
            case .serverError(let msg):
                return "The provider or server reported: \(msg)"
            }
        }
        if let urlError = error as? URLError {
            switch urlError.code {
            case .notConnectedToInternet:
                return "No internet connection. Check your network and try again."
            case .timedOut:
                return "The request timed out. Try again."
            case .cannotFindHost, .cannotConnectToHost:
                return "Could not reach the server. Check the API URL and your connection."
            default:
                return urlError.localizedDescription
            }
        }
        return error.localizedDescription
    }

    private static func mapHttp(code: Int, serverMessage: String?) -> String {
        let suffix = serverMessage.map { " \($0)" } ?? ""
        switch code {
        case 401:
            return "Authentication failed. Check that your API key is correct and not expired.\(suffix)"
        case 403:
            return "This action isn’t allowed with this API key. Storing prompts requires a management key; execution keys can only list and run prompts.\(suffix)"
        case 404:
            return "The server could not find that resource.\(suffix)"
        case 429:
            return "Too many requests. Please wait and try again.\(suffix)"
        case 500...599:
            return "The server had a problem. Try again later.\(suffix)"
        default:
            return "Request failed (HTTP \(code)).\(suffix)"
        }
    }
}
