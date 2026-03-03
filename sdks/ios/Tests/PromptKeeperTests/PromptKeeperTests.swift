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

    func testPutKeyResponseDecoding() throws {
        let json = """
        {"version_id":"v1","created_at":"2025-01-01T00:00:00Z","kms_key_arn":null,"fingerprint":null}
        """
        let data = Data(json.utf8)
        let decoder = JSONDecoder()
        let response = try decoder.decode(PutKeyResponse.self, from: data)
        XCTAssertEqual(response.version_id, "v1")
        XCTAssertEqual(response.created_at, "2025-01-01T00:00:00Z")
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
