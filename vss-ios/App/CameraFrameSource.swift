@preconcurrency import AVFoundation
import Metal
import UIKit

final class CameraFrameSource: NSObject, AVCaptureVideoDataOutputSampleBufferDelegate, AVCaptureDataOutputSynchronizerDelegate, @unchecked Sendable {
    var onFrame: ((MTLTexture, MTLTexture, MTLTexture?, Int, Int, Int, Int, Int, Bool) -> Void)?
    var onAuthorization: (@Sendable (AVAuthorizationStatus) -> Void)?
    private let session = AVCaptureSession(); private let queue = DispatchQueue(label: "com.vss.camera", qos: .userInteractive)
    private let textureCache: CVMetalTextureCache
    private var videoOutput: AVCaptureVideoDataOutput?
    private var depthOutput: AVCaptureDepthDataOutput?
    private var outputSynchronizer: AVCaptureDataOutputSynchronizer?
    override init() {
        var cache: CVMetalTextureCache?; CVMetalTextureCacheCreate(nil, nil, MTLCreateSystemDefaultDevice()!, nil, &cache); textureCache = cache!; super.init()
    }
    func start() {
        Task {
            let granted = await AVCaptureDevice.requestAccess(for: .video)
            let status = AVCaptureDevice.authorizationStatus(for: .video)
            await MainActor.run { self.onAuthorization?(status) }
            guard granted else { return }
            queue.async { self.configure() }
        }
    }
    private func configure() {
        if session.isRunning { session.stopRunning() }
        session.beginConfiguration()
        var shouldStart = false
        defer {
            session.commitConfiguration()
            if shouldStart { session.startRunning() }
        }
        session.inputs.forEach(session.removeInput)
        session.outputs.forEach(session.removeOutput)
        videoOutput = nil; depthOutput = nil; outputSynchronizer = nil

        let depthDevice = AVCaptureDevice.default(.builtInLiDARDepthCamera, for: .video, position: .back)
        var useDepth = depthDevice != nil
        var device = useDepth ? depthDevice : AVCaptureDevice.default(.builtInWideAngleCamera, for: .video, position: .back)
        if useDepth, let depthDevice, !configureDepthFormat(depthDevice) {
            useDepth = false
            device = AVCaptureDevice.default(.builtInWideAngleCamera, for: .video, position: .back)
        }
        guard let device,
              let input = try? AVCaptureDeviceInput(device: device), session.canAddInput(input) else { return }
        session.addInput(input)

        if useDepth {
            session.sessionPreset = .inputPriority
        } else {
            session.sessionPreset = .high
        }

        let video = AVCaptureVideoDataOutput()
        video.alwaysDiscardsLateVideoFrames = true
        video.videoSettings = [kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_420YpCbCr8BiPlanarFullRange]
        guard session.canAddOutput(video) else { return }
        session.addOutput(video); videoOutput = video

        if useDepth {
            let depth = AVCaptureDepthDataOutput()
            depth.alwaysDiscardsLateDepthData = true
            depth.isFilteringEnabled = true
            guard session.canAddOutput(depth) else { return }
            session.addOutput(depth); depthOutput = depth
            let synchronizer = AVCaptureDataOutputSynchronizer(dataOutputs: [video, depth])
            synchronizer.setDelegate(self, queue: queue)
            outputSynchronizer = synchronizer
        } else {
            video.setSampleBufferDelegate(self, queue: queue)
        }
        shouldStart = true
    }
    private func configureDepthFormat(_ device: AVCaptureDevice) -> Bool {
        let candidates = device.formats.compactMap { videoFormat -> (AVCaptureDevice.Format, AVCaptureDevice.Format)? in
            guard videoFormat.formatDescription.mediaSubType.rawValue == kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
                  let depthFormat = videoFormat.supportedDepthDataFormats.last(where: {
                      $0.formatDescription.mediaSubType.rawValue == kCVPixelFormatType_DepthFloat16
                  }) else { return nil }
            return (videoFormat, depthFormat)
        }
        let selected = candidates
            .filter { $0.0.formatDescription.dimensions.width <= 1920 }
            .max { lhs, rhs in
                let l = lhs.0.formatDescription.dimensions, r = rhs.0.formatDescription.dimensions
                return l.width * l.height < r.width * r.height
            } ?? candidates.first
        guard let selected else { return false }
        do {
            try device.lockForConfiguration()
            device.activeFormat = selected.0
            device.activeDepthDataFormat = selected.1
            device.unlockForConfiguration()
            return true
        } catch {
            return false
        }
    }
    func captureOutput(_ output: AVCaptureOutput, didOutput sampleBuffer: CMSampleBuffer, from connection: AVCaptureConnection) {
        guard let buffer = CMSampleBufferGetImageBuffer(sampleBuffer) else { return }
        post(video: buffer, depth: nil)
    }
    func dataOutputSynchronizer(_ synchronizer: AVCaptureDataOutputSynchronizer, didOutput collection: AVCaptureSynchronizedDataCollection) {
        guard let videoOutput, let depthOutput,
              let videoData = collection.synchronizedData(for: videoOutput) as? AVCaptureSynchronizedSampleBufferData,
              let depthData = collection.synchronizedData(for: depthOutput) as? AVCaptureSynchronizedDepthData,
              !videoData.sampleBufferWasDropped, !depthData.depthDataWasDropped,
              let video = CMSampleBufferGetImageBuffer(videoData.sampleBuffer) else { return }
        let converted = depthData.depthData.depthDataType == kCVPixelFormatType_DepthFloat16
            ? depthData.depthData
            : depthData.depthData.converting(toDepthDataType: kCVPixelFormatType_DepthFloat16)
        post(video: video, depth: converted.depthDataMap)
    }
    private func post(video: CVPixelBuffer, depth: CVPixelBuffer?) {
        guard CVPixelBufferGetPlaneCount(video) == 2 else { return }
        let w = CVPixelBufferGetWidthOfPlane(video, 0), h = CVPixelBufferGetHeightOfPlane(video, 0)
        var yRef: CVMetalTexture?, uvRef: CVMetalTexture?
        guard CVMetalTextureCacheCreateTextureFromImage(nil, textureCache, video, nil, .r8Unorm, w, h, 0, &yRef) == kCVReturnSuccess,
              CVMetalTextureCacheCreateTextureFromImage(nil, textureCache, video, nil, .rg8Unorm, (w+1)/2, (h+1)/2, 1, &uvRef) == kCVReturnSuccess,
              let yRef, let uvRef, let y = CVMetalTextureGetTexture(yRef), let uv = CVMetalTextureGetTexture(uvRef) else { return }
        var depthTexture: MTLTexture?
        var dw = 0, dh = 0
        var depthRef: CVMetalTexture?
        if let depth {
            dw = CVPixelBufferGetWidth(depth); dh = CVPixelBufferGetHeight(depth)
            if CVMetalTextureCacheCreateTextureFromImage(nil, textureCache, depth, nil, .r16Float, dw, dh, 0, &depthRef) == kCVReturnSuccess,
               let depthRef {
                depthTexture = CVMetalTextureGetTexture(depthRef)
            }
        }
        onFrame?(y, uv, depthTexture, w, h, dw, dh, Self.rotation(), CVPixelBufferGetPixelFormatType(video) == kCVPixelFormatType_420YpCbCr8BiPlanarFullRange)
    }
    private static func rotation() -> Int {
        switch UIDevice.current.orientation {
        case .landscapeLeft: return 0
        case .portrait: return 90
        case .landscapeRight: return 180
        case .portraitUpsideDown: return 270
        default: return 90
        }
    }
    func stop() { queue.async { if self.session.isRunning { self.session.stopRunning() } } }
}
