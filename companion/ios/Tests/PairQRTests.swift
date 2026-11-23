import XCTest
@testable import CompanionProtocol
final class PairQRTests: XCTestCase {
    let base = "notchlite://pair?v=1&host=192.168.1.2&port=43123&pin=" + String(repeating: "0", count: 64) + "&secret=" + String(repeating: "1", count: 64)
    func testPrivateRanges() throws {
        for host in ["10.0.0.1", "172.16.0.1", "172.31.255.254", "192.168.1.2"] {
            XCTAssertEqual(try PairQR(base.replacingOccurrences(of: "192.168.1.2", with: host)).host, host)
        }
    }
    func testRejectUntrustedEndpointAndAmbiguity() {
        for host in ["example.org", "127.0.0.1", "8.8.8.8", "172.32.0.1", "224.0.0.1", "192.168.001.2", "192.168.1.999", "192%2E168.1.2"] {
            XCTAssertThrowsError(try PairQR(base.replacingOccurrences(of: "192.168.1.2", with: host)))
        }
        for qr in [base + "&host=10.0.0.1", base + "#fragment", base + "&extra=x", base.replacingOccurrences(of: "//pair?", with: "//user@pair?"), base.replacingOccurrences(of: "//pair?", with: "//pair/path?")] {
            XCTAssertThrowsError(try PairQR(qr))
        }
    }
    func testRejectMalformedCredentials() {
        for qr in [base.replacingOccurrences(of: "pin=" + String(repeating: "0", count: 64), with: "pin=bad"), base.replacingOccurrences(of: "43123", with: "65536"), base.replacingOccurrences(of: "v=1", with: "v=2"), base + String(repeating: "x", count: 2048)] {
            XCTAssertThrowsError(try PairQR(qr))
        }
    }
}
