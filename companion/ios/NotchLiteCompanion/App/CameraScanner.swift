// Original NotchLite companion code. SPDX-License-Identifier: Apache-2.0
import SwiftUI
import AVFoundation

struct CameraScanner: UIViewControllerRepresentable {
    let completion: (String?) -> Void
    func makeUIViewController(context: Context) -> ScannerController { ScannerController(completion) }
    func updateUIViewController(_ controller: ScannerController, context: Context) {}
    static func dismantleUIViewController(_ controller: ScannerController, coordinator: ()) { controller.stop() }
}

final class ScannerController: UIViewController, AVCaptureMetadataOutputObjectsDelegate {
    private let session = AVCaptureSession()
    private let queue = DispatchQueue(label: "it.notchlite.qr.camera")
    private var preview: AVCaptureVideoPreviewLayer?
    private let completion: (String?) -> Void
    private var finished = false
    init(_ completion: @escaping (String?) -> Void) { self.completion = completion; super.init(nibName: nil, bundle: nil) }
    required init?(coder: NSCoder) { fatalError("init(coder:) unsupported") }
    override func viewDidLoad() {
        super.viewDidLoad(); view.backgroundColor = .black
        let cancel = UIButton(type: .system); cancel.setTitle("Annulla", for: .normal); cancel.addTarget(self, action: #selector(cancelScan), for: .touchUpInside)
        cancel.translatesAutoresizingMaskIntoConstraints = false; view.addSubview(cancel)
        NSLayoutConstraint.activate([cancel.topAnchor.constraint(equalTo: view.safeAreaLayoutGuide.topAnchor, constant: 16), cancel.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -24)])
        AVCaptureDevice.requestAccess(for: .video) { [weak self] granted in
            DispatchQueue.main.async { guard let self, !self.finished else { return }; if granted { self.configure() } else { self.showError("Fotocamera non disponibile. Puoi incollare il QR manualmente.") } }
        }
    }
    private func configure() {
        guard let camera = AVCaptureDevice.default(for: .video), let input = try? AVCaptureDeviceInput(device: camera), session.canAddInput(input) else { showError("Fotocamera non disponibile. Usa il campo QR."); return }
        session.addInput(input); let output = AVCaptureMetadataOutput()
        guard session.canAddOutput(output) else { showError("Scanner non disponibile"); return }
        session.addOutput(output); output.setMetadataObjectsDelegate(self, queue: .main); output.metadataObjectTypes = [.qr]
        let layer = AVCaptureVideoPreviewLayer(session: session); layer.videoGravity = .resizeAspectFill; layer.frame = view.bounds; view.layer.insertSublayer(layer, at: 0); preview = layer
        queue.async { [session] in session.startRunning() }
    }
    override func viewDidLayoutSubviews() { super.viewDidLayoutSubviews(); preview?.frame = view.bounds }
    override func viewWillDisappear(_ animated: Bool) { super.viewWillDisappear(animated); stop() }
    func stop() { queue.async { [session] in if session.isRunning { session.stopRunning() } } }
    @objc private func cancelScan() { finish(nil) }
    private func finish(_ value: String?) { guard !finished else { return }; finished = true; stop(); completion(value) }
    private func showError(_ text: String) {
        let alert = UIAlertController(title: "Scansione QR", message: text, preferredStyle: .alert)
        alert.addAction(UIAlertAction(title: "Chiudi", style: .default) { [weak self] _ in self?.finish(nil) }); present(alert, animated: true)
    }
    func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput objects: [AVMetadataObject], from connection: AVCaptureConnection) {
        guard let text = (objects.first as? AVMetadataMachineReadableCodeObject)?.stringValue, text.utf8.count <= 2048 else { return }; finish(text)
    }
}
