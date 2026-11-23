// Original NotchLite companion code. SPDX-License-Identifier: Apache-2.0
import Foundation
import CoreFoundation
import Combine

struct FileOffer {
    let id: String
    let name: String
    let size: Int64
    init(_ object: [String: Any]) throws {
        guard let id = object["id"] as? String, let name = object["name"] as? String, !name.isEmpty, name.count <= 255,
              let number = object["size"] as? NSNumber, CFGetTypeID(number) != CFBooleanGetTypeID(),
              number.doubleValue == Double(number.int64Value), (0...maxFileSize).contains(number.int64Value) else { throw CompanionError.invalid("Offerta non valida") }
        self.id = try PinnedClient.id(id); self.name = name; self.size = number.int64Value
    }
}

@MainActor final class CompanionModel: ObservableObject {
    @Published var qrText = ""
    @Published var message = "Nessun PC associato"
    @Published var paired = false
    @Published var busy = false
    @Published var outgoing: FileOffer?
    @Published var exportFile: URL?
    @Published var acknowledgementPending = false
    private var client: PinnedClient?
    private var token: String?
    private var pairID: String?
    private var pollToken: String?
    private var transferID: String?
    private var acknowledgementID: String?
    private var task: Task<Void, Never>?
    private var foreground = true
    private var generation = 0
    func scene(active: Bool) {
        foreground = active
        if active { poll() } else { generation += 1; task?.cancel(); client?.cancel(); task = nil; busy = false }
    }
    func disconnect() {
        generation += 1; task?.cancel(); client?.cancel(); task = nil; client = nil; token = nil; pairID = nil; pollToken = nil
        paired = false; busy = false; outgoing = nil; transferID = nil; acknowledgementID = nil; acknowledgementPending = false
        if let exportFile { try? FileManager.default.removeItem(at: exportFile) }; exportFile = nil
        qrText = ""; message = "Disconnesso. Genera un nuovo QR sul PC per riprovare."
    }
    private func work(_ action: @escaping @MainActor () async throws -> Void) {
        generation += 1; let run = generation; task?.cancel(); client?.cancel(); busy = true
        task = Task {
            defer { if generation == run { busy = false; task = nil; poll() } }
            do { try await action() } catch is CancellationError {} catch { if generation == run { message = "Operazione non completata: \(error.localizedDescription)" } }
        }
    }
    func pair() {
        let qr: PairQR
        do { qr = try PairQR(qrText) } catch { message = "QR non valido. Usa il QR del PC sulla stessa rete locale."; return }
        disconnect(); client = PinnedClient(qr)
        work { [self] in
            let result = try await request("POST", "/v1/pair", body: ["secret": qr.secret, "name": "iPhone"], auth: nil)
            guard let id = result["id"] as? String, let poll = result["pollToken"] as? String, let code = result["code"] as? String, code.count <= 32 else { throw CompanionError.invalid("Associazione non valida") }
            pairID = try PinnedClient.id(id); pollToken = poll
            message = "Codice: \(code)\nConfrontalo sul PC e approva solo se coincide."
            busy = false; try await pollLoop()
        }
    }
    private func request(_ method: String, _ path: String, body: [String: Any]? = nil, auth: String? = nil) async throws -> [String: Any] {
        guard let client else { throw CompanionError.invalid("PC non associato") }
        let run = generation
        let result = try await client.json(method, path, token: auth ?? token, body: body)
        try Task.checkCancellation(); guard generation == run else { throw CancellationError() }; return result
    }
    private func poll() {
        guard foreground, !busy, task == nil, client != nil, token != nil || pairID != nil else { return }
        let run = generation
        task = Task { [self] in
            do { try await pollLoop() } catch is CancellationError {} catch { if generation == run { message = "Connessione interrotta: \(error.localizedDescription)" } }
            if generation == run { task = nil }
        }
    }
    private func pollLoop() async throws {
        while foreground {
            try Task.checkCancellation()
            if token == nil {
                guard let pairID else { return }
                let result = try await request("GET", "/v1/pair/\(pairID)", auth: pollToken)
                switch result["status"] as? String {
                case "approved":
                    guard let value = result["token"] as? String else { throw CompanionError.invalid("Token mancante") }
                    token = value; paired = true; self.pairID = nil; pollToken = nil; message = "PC associato"
                case "pending": break
                case "denied", "expired": disconnect(); message = "Associazione rifiutata o scaduta. Genera un nuovo QR."; return
                default: throw CompanionError.invalid("Stato associazione non valido")
                }
            } else {
                let state = try await request("GET", "/v1/state")
                guard state["paired"] as? Bool == true else { throw CompanionError.invalid("Sessione non valida") }
                outgoing = try (state["outgoing"] as? [String: Any]).map(FileOffer.init)
            }
            try await Task.sleep(nanoseconds: 1_500_000_000)
        }
    }
    func send(_ source: URL) {
        work { [self] in
            let run = generation
            let copyTask = Task.detached { try Self.copyUpload(source) }
            let (file, name, size) = try await withTaskCancellationHandler(operation: { try await copyTask.value }, onCancel: { copyTask.cancel() })
            defer { try? FileManager.default.removeItem(at: file) }
            try Task.checkCancellation()
            let offer = try await request("POST", "/v1/offers", body: ["name": name, "size": size])
            guard let id = offer["id"] as? String else { throw CompanionError.invalid("Offerta non valida") }; transferID = try PinnedClient.id(id)
            message = "Attendi l'approvazione del file sul PC"
            while true {
                let state = try await request("GET", "/v1/state")
                guard let incoming = state["incoming"] as? [String: Any], incoming["id"] as? String == id else { throw CompanionError.invalid("Offerta annullata") }
                if incoming["status"] as? String == "accepted" { break }
                guard incoming["status"] as? String == "pending" else { throw CompanionError.invalid("File rifiutato o annullato") }
                try await Task.sleep(nanoseconds: 1_500_000_000)
            }
            guard let client, let token else { throw CompanionError.invalid("Sessione terminata") }
            try await client.upload(id: id, token: token, file: file, size: size) { [weak self] count in Task { @MainActor in if self?.generation == run { self?.message = "Invio: \(count) / \(size) byte" } } }
            try Task.checkCancellation(); guard generation == run else { throw CancellationError() }
            transferID = nil; message = "File ricevuto dal PC"
        }
    }
    nonisolated private static func copyUpload(_ source: URL) throws -> (URL, String, Int64) {
        let scoped = source.startAccessingSecurityScopedResource(); defer { if scoped { source.stopAccessingSecurityScopedResource() } }
        let info = try source.resourceValues(forKeys: [.fileSizeKey, .isRegularFileKey])
        guard info.isRegularFile == true, let raw = info.fileSize, (0...maxFileSize).contains(Int64(raw)) else { throw CompanionError.invalid("File non disponibile o oltre 256 MiB") }
        let size = Int64(raw), target = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        guard FileManager.default.createFile(atPath: target.path, contents: nil) else { throw CompanionError.invalid("Spazio temporaneo non disponibile") }
        do {
            let input = try FileHandle(forReadingFrom: source); defer { try? input.close() }
            let output = try FileHandle(forWritingTo: target); defer { try? output.close() }
            var copied: Int64 = 0
            while copied < size {
                try Task.checkCancellation()
                let bytes = try input.read(upToCount: Int(min(65536, size - copied))) ?? Data()
                guard !bytes.isEmpty else { throw CompanionError.invalid("File incompleto") }; try output.write(contentsOf: bytes); copied += Int64(bytes.count)
            }
            guard (try input.read(upToCount: 1))?.isEmpty != false else { throw CompanionError.invalid("File modificato durante la lettura") }
            try output.synchronize()
            let name = source.lastPathComponent.filter { !$0.isNewline && !$0.isASCIIControl }.prefix(180)
            return (target, name.isEmpty ? "file" : String(name), size)
        } catch { try? FileManager.default.removeItem(at: target); throw error }
    }
    func receive() {
        guard let offer = outgoing else { return }
        work { [self] in
            let run = generation
            guard let client, let token else { throw CompanionError.invalid("Sessione terminata") }; transferID = offer.id
            let temp = try await client.download(id: offer.id, token: token, size: offer.size) { [weak self] count in Task { @MainActor in if self?.generation == run { self?.message = "Ricezione: \(count) / \(offer.size) byte" } } }
            do { try Task.checkCancellation(); guard generation == run else { throw CancellationError() } } catch { try? FileManager.default.removeItem(at: temp); throw error }
            let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
            do {
                try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
                let safe = offer.name.components(separatedBy: CharacterSet(charactersIn: "/\\").union(.controlCharacters)).joined().prefix(180)
                let target = directory.appendingPathComponent(safe.isEmpty ? "file" : String(safe))
                try FileManager.default.moveItem(at: temp, to: target); exportFile = target; message = "Scegli dove salvare il file"
            } catch { try? FileManager.default.removeItem(at: temp); throw error }
        }
    }
    func exported(saved: Bool) {
        guard let file = exportFile else { return }; exportFile = nil
        try? FileManager.default.removeItem(at: file.deletingLastPathComponent())
        if saved { acknowledgementID = transferID; transferID = nil; acknowledgementPending = true; acknowledge() }
        else { message = "Salvataggio annullato. Il file rimane disponibile sul PC." }
    }
    func acknowledge() {
        guard let id = acknowledgementID else { return }
        work { [self] in
            _ = try await request("POST", "/v1/outgoing/\(id)/done")
            acknowledgementID = nil; acknowledgementPending = false; outgoing = nil; message = "File salvato e confermato al PC"
        }
    }
    func cancelTransfer() {
        guard let id = transferID ?? outgoing?.id else { return }
        work { [self] in _ = try await request("DELETE", "/v1/transfers/\(try PinnedClient.id(id))"); transferID = nil; outgoing = nil; message = "Trasferimento annullato" }
    }
}

private extension Character {
    var isASCIIControl: Bool { unicodeScalars.contains { CharacterSet.controlCharacters.contains($0) } }
}
