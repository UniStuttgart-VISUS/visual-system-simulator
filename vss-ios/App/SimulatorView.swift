import AVFoundation
import MetalKit
import Observation
import SwiftUI
import UIKit

struct SimulatorView: UIViewRepresentable {
    let controller: SimulatorController
    func makeCoordinator() -> Coordinator { Coordinator(controller: controller) }
    func makeUIView(context: Context) -> MTKView {
        let view = MTKView(frame: .zero, device: MTLCreateSystemDefaultDevice())
        view.framebufferOnly = true; view.colorPixelFormat = .bgra8Unorm; view.enableSetNeedsDisplay = false
        view.preferredFramesPerSecond = 60; view.delegate = controller
        controller.attach(view)
        context.coordinator.install(on: view)
        return view
    }
    func updateUIView(_ view: MTKView, context: Context) { controller.resize(view.drawableSize, for: view) }
    static func dismantleUIView(_ view: MTKView, coordinator: Coordinator) {
        (view.delegate as? SimulatorController)?.detach(view)
        view.delegate = nil
    }

    final class Coordinator: NSObject {
        let controller: SimulatorController
        init(controller: SimulatorController) { self.controller = controller }
        func install(on view: UIView) {
            let gaze = UIPanGestureRecognizer(target: self, action: #selector(gaze(_:))); gaze.minimumNumberOfTouches = 1; gaze.maximumNumberOfTouches = 1
            let camera = UIPanGestureRecognizer(target: self, action: #selector(camera(_:))); camera.minimumNumberOfTouches = 2; camera.maximumNumberOfTouches = 2
            let reset = UITapGestureRecognizer(target: self, action: #selector(resetPose)); reset.numberOfTouchesRequired = 1; reset.numberOfTapsRequired = 2
            view.addGestureRecognizer(gaze); view.addGestureRecognizer(camera); view.addGestureRecognizer(reset)
        }
        @objc private func gaze(_ gesture: UIPanGestureRecognizer) { post(gesture, kind: "gaze_delta") }
        @objc private func camera(_ gesture: UIPanGestureRecognizer) { post(gesture, kind: "view_delta") }
        @objc private func resetPose() { SimulatorBridge.semanticInput("reset_pose") }
        private func post(_ gesture: UIPanGestureRecognizer, kind: String) {
            guard let view = gesture.view else { return }
            let delta = gesture.translation(in: view); gesture.setTranslation(.zero, in: view)
            SimulatorBridge.semanticInput(kind, x: Float(delta.x / max(1, view.bounds.width)), y: Float(delta.y / max(1, view.bounds.height)))
        }
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
    func post(left: [String: JSONValue], right: [String: JSONValue]) throws { try SimulatorBridge.post(left: left, right: right) }
    func setEyeMode(_ mode: EyeMode) throws { try SimulatorBridge.setEyeMode(mode) }
    func detach(_ view: MTKView) {
        guard attachedView === view else { return }
        if attached { SimulatorBridge.destroy() }
        attached = false
        attachedView = nil
    }
    deinit { camera.stop(); SimulatorBridge.destroy() }
}
