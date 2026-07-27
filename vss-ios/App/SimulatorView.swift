import AVFoundation
import MetalKit
import Observation
import SwiftUI

struct SimulatorView: UIViewRepresentable {
    let controller: SimulatorController
    func makeUIView(context: Context) -> MTKView {
        let view = MTKView(frame: .zero, device: MTLCreateSystemDefaultDevice())
        view.framebufferOnly = true; view.colorPixelFormat = .bgra8Unorm; view.enableSetNeedsDisplay = false
        view.preferredFramesPerSecond = 60; view.delegate = controller
        controller.attach(view)
        return view
    }
    func updateUIView(_ view: MTKView, context: Context) { controller.resize(view.drawableSize, for: view) }
    static func dismantleUIView(_ view: MTKView, coordinator: ()) {
        (view.delegate as? SimulatorController)?.detach(view)
        view.delegate = nil
    }
}

@MainActor @Observable final class SimulatorController: NSObject, MTKViewDelegate {
    private var attached = false
    @ObservationIgnored private weak var attachedView: MTKView?
    private let camera = CameraFrameSource()
    private let media = MediaFrameSource()
    var cameraAuthorization: AVAuthorizationStatus = AVCaptureDevice.authorizationStatus(for: .video)
    var errorMessage: String?
    func attach(_ view: MTKView) {
        if attached, attachedView !== view { SimulatorBridge.destroy() }
        attachedView = view
        attached = SimulatorBridge.create(view: view, size: view.drawableSize)
        if !attached {
            attachedView = nil
            errorMessage = SimulatorBridgeError.unavailable("renderer").localizedDescription
        }
        camera.onFrame = SimulatorBridge.post; media.onFrame = SimulatorBridge.post
        camera.onAuthorization = { [weak self] status in
            Task { @MainActor in self?.cameraAuthorization = status }
        }
        media.onError = { [weak self] error in self?.errorMessage = error.localizedDescription }
    }
    func resize(_ size: CGSize, for view: MTKView) { if attached, attachedView === view { SimulatorBridge.resize(size) } }
    func draw(in view: MTKView) { if attached, attachedView === view { SimulatorBridge.draw() } }
    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) { resize(size, for: view) }
    func startCamera() { media.stop(); camera.start() }
    func startMedia(_ url: URL) { camera.stop(); media.start(url: url) }
    func stopSources() { camera.stop(); media.stop() }
    func post(settings: [String: JSONValue]) throws { try SimulatorBridge.post(settings: settings) }
    func detach(_ view: MTKView) {
        guard attachedView === view else { return }
        if attached { SimulatorBridge.destroy() }
        attached = false
        attachedView = nil
    }
    deinit { camera.stop(); SimulatorBridge.destroy() }
}
