@preconcurrency import AVFoundation
import CoreImage
import ImageIO
import Metal
import UIKit

/// Decodes still images and timed video frames into bi-planar CVPixelBuffers, then reuses the
/// renderer's Metal texture import path. Images are thumbnail-decoded to cap memory use.
@MainActor final class MediaFrameSource {
    var onFrame: ((MTLTexture, MTLTexture, MTLTexture?, Int, Int, Int, Int, Int, Bool) -> Void)?
    var onError: ((Error) -> Void)?
    private var player: AVPlayer?
    private var output: AVPlayerItemVideoOutput?
    private var displayLink: CADisplayLink?
    private var scopedURL: URL?
    private var endObserver: NSObjectProtocol?
    private var videoRotation = 0
    private let context = CIContext(options: [.cacheIntermediates: false])
    private let textureCache: CVMetalTextureCache

    init() {
        var cache: CVMetalTextureCache?
        CVMetalTextureCacheCreate(nil, nil, MTLCreateSystemDefaultDevice()!, nil, &cache)
        textureCache = cache!
    }
    func start(url: URL) {
        stop()
        if url.startAccessingSecurityScopedResource() { scopedURL = url }
        decode(url: url)
    }
    private func decode(url: URL) {
        let asset = AVURLAsset(url: url)
        Task {
            do {
                let tracks = try await asset.loadTracks(withMediaType: .video)
                if let track = tracks.first {
                    videoRotation = Self.rotation(for: try await track.load(.preferredTransform))
                    play(asset: asset)
                } else { try await showImage(url: url) }
            } catch { onError?(error); stopScope() }
        }
    }
    private func showImage(url: URL) async throws {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else { throw MediaError.decode }
        let options = [kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceCreateThumbnailWithTransform: true, kCGImageSourceThumbnailMaxPixelSize: 2560] as CFDictionary
        guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options) else { throw MediaError.decode }
        guard let buffer = makeBuffer(width: image.width, height: image.height) else { throw MediaError.buffer }
        context.render(CIImage(cgImage: image), to: buffer, bounds: CGRect(x: 0, y: 0, width: image.width, height: image.height), colorSpace: CGColorSpaceCreateDeviceRGB())
        post(buffer)
        stopScope()
    }
    private func play(asset: AVAsset) {
        let item = AVPlayerItem(asset: asset)
        let attributes: [String: Any] = [kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
                                         kCVPixelBufferMetalCompatibilityKey as String: true]
        let output = AVPlayerItemVideoOutput(pixelBufferAttributes: attributes)
        item.add(output)
        endObserver = NotificationCenter.default.addObserver(forName: .AVPlayerItemDidPlayToEndTime, object: item, queue: .main) { [weak self] _ in
            Task { @MainActor in
                self?.player?.seek(to: .zero)
                self?.player?.play()
            }
        }
        self.output = output
        player = AVPlayer(playerItem: item)
        let link = CADisplayLink(target: self, selector: #selector(tick))
        link.add(to: .main, forMode: .common); displayLink = link
        player?.play()
    }
    @objc private func tick() {
        guard let output, let item = player?.currentItem else { return }
        let time = item.currentTime()
        guard output.hasNewPixelBuffer(forItemTime: time), let buffer = output.copyPixelBuffer(forItemTime: time, itemTimeForDisplay: nil) else { return }
        post(buffer)
    }
    private func makeBuffer(width: Int, height: Int) -> CVPixelBuffer? {
        var buffer: CVPixelBuffer?
        CVPixelBufferCreate(nil, width, height, kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
                            [kCVPixelBufferIOSurfacePropertiesKey: [:], kCVPixelBufferMetalCompatibilityKey: true] as CFDictionary, &buffer)
        return buffer
    }
    private func post(_ buffer: CVPixelBuffer) {
        guard CVPixelBufferGetPlaneCount(buffer) == 2 else { return }
        let width = CVPixelBufferGetWidthOfPlane(buffer, 0), height = CVPixelBufferGetHeightOfPlane(buffer, 0)
        var yRef: CVMetalTexture?, uvRef: CVMetalTexture?
        guard CVMetalTextureCacheCreateTextureFromImage(nil, textureCache, buffer, nil, .r8Unorm, width, height, 0, &yRef) == kCVReturnSuccess,
              CVMetalTextureCacheCreateTextureFromImage(nil, textureCache, buffer, nil, .rg8Unorm, (width + 1) / 2, (height + 1) / 2, 1, &uvRef) == kCVReturnSuccess,
              let yRef, let uvRef, let y = CVMetalTextureGetTexture(yRef), let uv = CVMetalTextureGetTexture(uvRef) else { return }
        onFrame?(y, uv, nil, width, height, 0, 0, videoRotation, true)
    }
    func stop() {
        displayLink?.invalidate(); displayLink = nil
        player?.pause(); player = nil; output = nil
        videoRotation = 0
        if let endObserver { NotificationCenter.default.removeObserver(endObserver); self.endObserver = nil }
        stopScope()
    }
    private func stopScope() { scopedURL?.stopAccessingSecurityScopedResource(); scopedURL = nil }
    private static func rotation(for transform: CGAffineTransform) -> Int {
        let degrees = Int(round(atan2(transform.b, transform.a) * 180 / .pi))
        switch degrees.rem_euclid(360) { case 45...134: return 90; case 135...224: return 180; case 225...314: return 270; default: return 0 }
    }
    enum MediaError: LocalizedError { case decode, buffer; var errorDescription: String? { self == .decode ? "The selected image could not be decoded." : "The image buffer could not be created." } }
    deinit { displayLink?.invalidate(); player?.pause(); if let endObserver { NotificationCenter.default.removeObserver(endObserver) } }
}

private extension Int { func rem_euclid(_ divisor: Int) -> Int { let result = self % divisor; return result >= 0 ? result : result + divisor } }
