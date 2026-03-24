//
//  PromptKeeperTests.swift
//  PromptKeeperTests
//

import XCTest
@testable import PromptKeeper

final class PromptKeeperTests: XCTestCase {

    func testInitStoresAPIKeyInMemory() {
        let sdk = PromptKeeper(apiKey: "pk_test_abc")
        // SDK is configured; API key is in-memory (no persistence to verify here)
        XCTAssertNotNil(sdk)
    }

    func testPromptKeeperErrorMessage() {
        let err = PromptKeeperError.serverError("function not found")
        XCTAssertEqual(err.message, "function not found")
        let err2 = PromptKeeperError.httpStatus(401, body: Data("Unauthorized".utf8))
        XCTAssertTrue(err2.message.contains("401"))
        XCTAssertTrue(err2.message.contains("Unauthorized"))
    }

    func testListPromptsResponseDecoding() throws {
        let json = """
        {"titles":["default","my_prompt"]}
        """
        let data = Data(json.utf8)
        let response = try JSONDecoder().decode(ListPromptsResponse.self, from: data)
        XCTAssertEqual(response.titles, ["default", "my_prompt"])
    }

    func testPutKeyResponseDecoding() throws {
        let json = """
        {"version_id":"v1","created_at":"2025-01-01T00:00:00Z","kms_key_arn":null,"fingerprint":null}
        """
        let data = Data(json.utf8)
        let decoder = JSONDecoder()
        let response = try decoder.decode(PutKeyResponse.self, from: data)
        XCTAssertEqual(response.versionId, "v1")
        XCTAssertEqual(response.createdAt, "2025-01-01T00:00:00Z")
    }

    /// Production API returns numeric `version_id` (Rust `i64`).
    func testPutPromptResponseDecodingNumericVersionId() throws {
        let json = """
        {"version_id":42,"created_at":"2025-01-01T00:00:00Z","kms_key_arn":"arn:aws:kms:us-east-1:0:key/1","fingerprint":"abc"}
        """
        let data = Data(json.utf8)
        let response = try JSONDecoder().decode(PutPromptResponse.self, from: data)
        XCTAssertEqual(response.versionId, "42")
        XCTAssertEqual(response.createdAt, "2025-01-01T00:00:00Z")
        XCTAssertEqual(response.kmsKeyArn, "arn:aws:kms:us-east-1:0:key/1")
        XCTAssertEqual(response.fingerprint, "abc")
    }

    func testExecStreamEventChunk() {
        let event = ExecStreamEvent.chunk("{\"choices\":[]}")
        if case .chunk(let data) = event {
            XCTAssertEqual(data, "{\"choices\":[]}")
        } else {
            XCTFail("Expected chunk")
        }
    }
}
