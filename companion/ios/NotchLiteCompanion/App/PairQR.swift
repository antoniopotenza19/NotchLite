// Original NotchLite companion code. SPDX-License-Identifier: Apache-2.0
import Foundation

struct PairQR {
    let host: String
    let port: Int
    let pin: String
    let secret: String
    enum Invalid: Error { case qr }
    init(_ raw: String) throws {
        guard raw.utf8.count <= 2048, !raw.contains(where: { $0.isWhitespace }),
              let url = URLComponents(string: raw), url.scheme == "notchlite", url.host == "pair",
              url.user == nil, url.password == nil, url.port == nil, url.path.isEmpty, url.fragment == nil,
              let query = url.percentEncodedQuery else { throw Invalid.qr }
        var values: [String: String] = [:]
        for part in query.split(separator: "&", omittingEmptySubsequences: false) {
            let pair = part.split(separator: "=", omittingEmptySubsequences: false)
            guard pair.count == 2, values[String(pair[0])] == nil else { throw Invalid.qr }
            values[String(pair[0])] = String(pair[1])
        }
        guard Set(values.keys) == Set(["v", "host", "port", "pin", "secret"]), values["v"] == "1",
              let host = values["host"], let portText = values["port"],
              portText.range(of: "^[1-9][0-9]{0,4}$", options: .regularExpression) != nil,
              let port = Int(portText), (1...65535).contains(port),
              let pin = values["pin"], let secret = values["secret"],
              [pin, secret].allSatisfy({ $0.range(of: "^[0-9a-fA-F]{64}$", options: .regularExpression) != nil }) else { throw Invalid.qr }
        let parts = host.split(separator: ".", omittingEmptySubsequences: false)
        guard parts.count == 4, parts.allSatisfy({ $0.range(of: "^(0|[1-9][0-9]{0,2})$", options: .regularExpression) != nil }),
              parts.allSatisfy({ Int($0).map { $0 <= 255 } == true }) else { throw Invalid.qr }
        let n = parts.map { Int($0)! }
        guard n[0] == 10 || n[0] == 172 && (16...31).contains(n[1]) || n[0] == 192 && n[1] == 168 else { throw Invalid.qr }
        self.host = host; self.port = port; self.pin = pin.lowercased(); self.secret = secret
    }
}
