// Original NotchLite companion code. SPDX-License-Identifier: Apache-2.0
import Foundation
import Security
import CryptoKit

let maxFileSize: Int64 = 256 * 1024 * 1024
enum CompanionError: LocalizedError {
    case invalid(String)
    var errorDescription: String? { if case .invalid(let text) = self { return text }; return nil }
}

private final class CancellationHandle: @unchecked Sendable {
    private let lock = NSLock()
    private var operation: Transport?
    private var cancelled = false
    func installAndStart(_ value: Transport, request: URLRequest, upload: URL?) -> Bool {
        lock.lock(); defer { lock.unlock() }
        guard !cancelled else { return false }
        operation = value
        // Task creation/resume must finish before cancellation may invalidate this session.
        value.start(request, upload: upload)
        return true
    }
    func cancel() { lock.lock(); cancelled = true; let value = operation; lock.unlock(); value?.cancel() }
}

private final class Transport: NSObject, URLSessionDataDelegate, URLSessionDownloadDelegate, @unchecked Sendable {
    let qr: PairQR
    let expected: Int64?
    let progress: @Sendable (Int64) -> Void
    let completion: (Result<(Data, URL?), Error>) -> Void
    private var session: URLSession!
    private var task: URLSessionTask?
    private var data = Data()
    private var saved: URL?
    private var failure: Error?
    private var lastProgress = Date.distantPast
    init(qr: PairQR, expected: Int64?, progress: @escaping @Sendable (Int64) -> Void, completion: @escaping (Result<(Data, URL?), Error>) -> Void) {
        self.qr = qr; self.expected = expected; self.progress = progress; self.completion = completion
        super.init()
        let config = URLSessionConfiguration.ephemeral
        config.httpShouldSetCookies = false; config.httpCookieStorage = nil; config.urlCache = nil
        config.requestCachePolicy = .reloadIgnoringLocalCacheData; config.timeoutIntervalForRequest = 15; config.timeoutIntervalForResource = 600
        session = URLSession(configuration: config, delegate: self, delegateQueue: nil)
    }
    func start(_ request: URLRequest, upload: URL?) {
        if expected != nil { task = session.downloadTask(with: request) }
        else if let upload { task = session.uploadTask(with: request, fromFile: upload) }
        else { task = session.dataTask(with: request) }
        task?.resume()
    }
    func cancel() { session.invalidateAndCancel() }
    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) { completionHandler(nil) }
    func urlSession(_ session: URLSession, didReceive challenge: URLAuthenticationChallenge, completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void) {
        let space = challenge.protectionSpace
        guard space.authenticationMethod == NSURLAuthenticationMethodServerTrust, space.host == qr.host, space.port == qr.port,
              let trust = space.serverTrust, let leaf = SecTrustGetCertificateAtIndex(trust, 0) else { completionHandler(.cancelAuthenticationChallenge, nil); return }
        let der = SecCertificateCopyData(leaf) as Data
        let pin = SHA256.hash(data: der).map { String(format: "%02x", $0) }.joined()
        guard pin == qr.pin,
              SecTrustSetAnchorCertificates(trust, [leaf] as CFArray) == errSecSuccess,
              SecTrustSetAnchorCertificatesOnly(trust, true) == errSecSuccess,
              SecTrustSetPolicies(trust, SecPolicyCreateSSL(true, qr.host as CFString)) == errSecSuccess,
              SecTrustSetNetworkFetchAllowed(trust, false) == errSecSuccess,
              SecTrustEvaluateWithError(trust, nil) else { completionHandler(.cancelAuthenticationChallenge, nil); return }
        completionHandler(.useCredential, URLCredential(trust: trust))
    }
    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive response: URLResponse, completionHandler: @escaping (URLSession.ResponseDisposition) -> Void) {
        guard let http = response as? HTTPURLResponse, http.statusCode == 200, response.expectedContentLength <= 65536 else {
            failure = CompanionError.invalid("Risposta PC non valida (HTTP \((response as? HTTPURLResponse)?.statusCode ?? 0))")
            completionHandler(.cancel); return
        }
        completionHandler(.allow)
    }
    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive bytes: Data) {
        guard data.count + bytes.count <= 65536 else { failure = CompanionError.invalid("Risposta troppo grande"); dataTask.cancel(); return }
        data.append(bytes)
    }
    func urlSession(_ session: URLSession, task: URLSessionTask, didSendBodyData bytesSent: Int64, totalBytesSent: Int64, totalBytesExpectedToSend: Int64) { report(totalBytesSent) }
    private func report(_ count: Int64) { let now = Date(); if now.timeIntervalSince(lastProgress) >= 0.1 { lastProgress = now; progress(count) } }
    func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didWriteData bytesWritten: Int64, totalBytesWritten: Int64, totalBytesExpectedToWrite: Int64) {
        guard let expected, totalBytesWritten <= expected, totalBytesExpectedToWrite == expected else {
            failure = CompanionError.invalid("Dimensione file diversa dall'offerta"); downloadTask.cancel(); return
        }
        report(totalBytesWritten)
    }
    func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didFinishDownloadingTo location: URL) {
        do {
            guard let expected, let response = downloadTask.response as? HTTPURLResponse, response.statusCode == 200,
                  response.expectedContentLength == expected,
                  (try location.resourceValues(forKeys: [.fileSizeKey]).fileSize).map(Int64.init) == expected else { throw CompanionError.invalid("Download incompleto") }
            let target = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
            try FileManager.default.moveItem(at: location, to: target); saved = target
        } catch { failure = error }
    }
    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        session.finishTasksAndInvalidate()
        if let error = failure ?? error { if let saved { try? FileManager.default.removeItem(at: saved) }; completion(.failure(error)); return }
        if expected != nil && saved == nil { completion(.failure(CompanionError.invalid("Download mancante"))); return }
        completion(.success((data, saved)))
    }
}

final class PinnedClient: @unchecked Sendable {
    private let qr: PairQR
    private let lock = NSLock()
    private var active: CancellationHandle?
    init(_ qr: PairQR) { self.qr = qr }
    func cancel() { lock.lock(); let current = active; lock.unlock(); current?.cancel() }
    static func id(_ value: String) throws -> String {
        guard value.range(of: "^[A-Za-z0-9_-]{1,128}$", options: .regularExpression) != nil else { throw CompanionError.invalid("ID non valido") }; return value
    }
    private func perform(_ method: String, _ path: String, token: String?, body: Data? = nil, upload: URL? = nil, size: Int64? = nil, download: Int64? = nil, progress: @escaping @Sendable (Int64) -> Void = { _ in }) async throws -> (Data, URL?) {
        try Task.checkCancellation()
        var parts = URLComponents(); parts.scheme = "https"; parts.host = qr.host; parts.port = qr.port; parts.path = path
        var request = URLRequest(url: parts.url!); request.httpMethod = method
        if let token {
            guard token.range(of: "^[A-Za-z0-9_-]{1,256}$", options: .regularExpression) != nil else { throw CompanionError.invalid("Token non valido") }
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        request.httpBody = body
        if method == "POST" && body == nil { request.httpBody = Data(); request.setValue("0", forHTTPHeaderField: "Content-Length") }
        if body != nil { request.setValue("application/json", forHTTPHeaderField: "Content-Type") }
        if let size { request.setValue(String(size), forHTTPHeaderField: "Content-Length"); request.setValue("application/octet-stream", forHTTPHeaderField: "Content-Type") }
        let handle = CancellationHandle()
        lock.withLock { active = handle }
        defer { lock.withLock { if active === handle { active = nil } } }
        let result: (Data, URL?) = try await withTaskCancellationHandler(operation: {
            try await withCheckedThrowingContinuation { continuation in
                let transport = Transport(qr: qr, expected: download, progress: progress) { continuation.resume(with: $0) }
                if !handle.installAndStart(transport, request: request, upload: upload) {
                    // No task was created, so no task delegate can complete this continuation.
                    transport.cancel(); continuation.resume(throwing: CancellationError())
                }
            }
        }, onCancel: { handle.cancel() })
        do { try Task.checkCancellation() } catch { if let url = result.1 { try? FileManager.default.removeItem(at: url) }; throw error }
        return result
    }
    func json(_ method: String, _ path: String, token: String? = nil, body: [String: Any]? = nil) async throws -> [String: Any] {
        let bytes = try body.map { try JSONSerialization.data(withJSONObject: $0) }
        let (data, _) = try await perform(method, path, token: token, body: bytes)
        if data.isEmpty { return [:] }
        guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any] else { throw CompanionError.invalid("JSON non valido") }; return object
    }
    func upload(id: String, token: String, file: URL, size: Int64, progress: @escaping @Sendable (Int64) -> Void) async throws {
        guard (0...maxFileSize).contains(size) else { throw CompanionError.invalid("Limite 256 MiB") }
        _ = try await perform("PUT", "/v1/incoming/\(try Self.id(id))", token: token, upload: file, size: size, progress: progress)
    }
    func download(id: String, token: String, size: Int64, progress: @escaping @Sendable (Int64) -> Void) async throws -> URL {
        guard (0...maxFileSize).contains(size) else { throw CompanionError.invalid("Limite 256 MiB") }
        let (_, url) = try await perform("GET", "/v1/outgoing/\(try Self.id(id))", token: token, download: size, progress: progress)
        guard let url else { throw CompanionError.invalid("Download mancante") }; return url
    }
}
