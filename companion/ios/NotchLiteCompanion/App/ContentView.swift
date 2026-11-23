// Original NotchLite companion code. SPDX-License-Identifier: Apache-2.0
import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    @StateObject private var model = CompanionModel()
    @Environment(\.scenePhase) private var phase
    @State private var scanner = false
    @State private var importer = false
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                Text("NotchLite Companion").font(.largeTitle.bold())
                Text("Graphite · rete locale").foregroundStyle(.cyan)
                Text("Apri Telefono sul PC, scansiona il QR e confronta il codice. Limite file: 256 MiB.")
                if !model.paired {
                    TextField("Incolla notchlite://pair?...", text: $model.qrText, axis: .vertical)
                        .textInputAutocapitalization(.never).autocorrectionDisabled().textFieldStyle(.roundedBorder)
                    Button("Scansiona QR", systemImage: "qrcode.viewfinder") { scanner = true }.disabled(model.busy)
                    Button("Associa al PC") { model.pair() }.disabled(model.busy)
                }
                Text(model.message).textSelection(.enabled).accessibilityAddTraits(.updatesFrequently)
                if model.busy { ProgressView() }
                if model.paired {
                    Button("Invia un file al PC", systemImage: "square.and.arrow.up") { importer = true }.disabled(model.busy)
                    if let offer = model.outgoing {
                        Text("Dal PC: \(offer.name)\n\(offer.size) byte")
                        Button("Ricevi e salva", systemImage: "square.and.arrow.down") { model.receive() }.disabled(model.busy || model.acknowledgementPending)
                    }
                    if model.acknowledgementPending { Button("Conferma file già salvato") { model.acknowledge() }.disabled(model.busy) }
                    Button("Annulla trasferimento", role: .destructive) { model.cancelTransfer() }
                }
                Button("Disconnetti") { model.disconnect() }
                Text("Nessun cloud o account. La sessione rimane solo in memoria. Scanner AVFoundation e interfaccia SwiftUI native.").font(.footnote).foregroundStyle(.secondary)
            }.padding(24).frame(maxWidth: 600, alignment: .leading)
        }.background(Color(red: 24/255, green: 29/255, blue: 38/255)).tint(.cyan)
            .onChange(of: phase) { _, value in model.scene(active: value == .active) }
            .sheet(isPresented: $scanner) { CameraScanner { value in scanner = false; if let value { model.qrText = value } } }
            .fileImporter(isPresented: $importer, allowedContentTypes: [.item]) { result in
                switch result { case .success(let url): model.send(url); case .failure(let error): model.message = "File non selezionato: \(error.localizedDescription)" }
            }
            .sheet(isPresented: Binding(get: { model.exportFile != nil }, set: { if !$0 { model.exported(saved: false) } })) {
                if let url = model.exportFile { ExportPicker(file: url) { model.exported(saved: $0) } }
            }
    }
}

private struct ExportPicker: UIViewControllerRepresentable {
    let file: URL
    let completion: (Bool) -> Void
    func makeCoordinator() -> Coordinator { Coordinator(completion) }
    func makeUIViewController(context: Context) -> UIDocumentPickerViewController {
        let view = UIDocumentPickerViewController(forExporting: [file], asCopy: true); view.delegate = context.coordinator; return view
    }
    func updateUIViewController(_ controller: UIDocumentPickerViewController, context: Context) {}
    final class Coordinator: NSObject, UIDocumentPickerDelegate {
        let completion: (Bool) -> Void
        var finished = false
        init(_ completion: @escaping (Bool) -> Void) { self.completion = completion }
        func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) { finish(!urls.isEmpty) }
        func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) { finish(false) }
        private func finish(_ saved: Bool) { guard !finished else { return }; finished = true; completion(saved) }
    }
}
