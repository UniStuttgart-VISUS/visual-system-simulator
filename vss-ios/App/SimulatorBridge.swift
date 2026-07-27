import Foundation
import Metal

@_silgen_name("vss_create") private func nativeCreate(_ view: UnsafeMutableRawPointer, _ width: UInt32, _ height: UInt32, _ assets: UnsafePointer<CChar>) -> Bool
@_silgen_name("vss_destroy") private func nativeDestroy()
@_silgen_name("vss_resize") private func nativeResize(_ width: UInt32, _ height: UInt32)
@_silgen_name("vss_draw") private func nativeDraw()
@_silgen_name("vss_post_camera_frame") private func nativePostFrame(_ y: UnsafeMutableRawPointer, _ uv: UnsafeMutableRawPointer, _ depth: UnsafeMutableRawPointer?, _ width: UInt32, _ height: UInt32, _ depthWidth: UInt32, _ depthHeight: UInt32, _ rotation: Int32, _ fullRange: Bool)
@_silgen_name("vss_post_settings") private func nativePostSettings(_ json: UnsafePointer<CChar>) -> Bool
@_silgen_name("vss_catalog_json") private func nativeCatalog(_ locale: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?
@_silgen_name("vss_compose_settings") private func nativeCompose(_ locale: UnsafePointer<CChar>, _ presets: UnsafePointer<CChar>, _ overrides: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?
@_silgen_name("vss_string_free") private func nativeStringFree(_ value: UnsafeMutablePointer<CChar>)

enum SimulatorBridgeError: LocalizedError {
    case unavailable(String), invalidJSON(String), rejectedSettings
    var errorDescription: String? {
        switch self {
        case .unavailable(let operation): return "Native operation failed: \(operation)"
        case .invalidJSON(let operation): return "Invalid JSON returned by: \(operation)"
        case .rejectedSettings: return "The renderer rejected the settings."
        }
    }
}

enum SimulatorBridge {
    static func create(view: AnyObject, size: CGSize) -> Bool {
        let pointer = Unmanaged.passUnretained(view).toOpaque()
        return Bundle.main.resourcePath!.withCString { nativeCreate(pointer, UInt32(size.width), UInt32(size.height), $0) }
    }
    static func destroy() { nativeDestroy() }
    static func resize(_ size: CGSize) { nativeResize(UInt32(max(1, size.width)), UInt32(max(1, size.height))) }
    static func draw() { nativeDraw() }
    static func post(y: MTLTexture, uv: MTLTexture, depth: MTLTexture?, width: Int, height: Int, depthWidth: Int, depthHeight: Int, rotation: Int, fullRange: Bool) {
        nativePostFrame(Unmanaged.passUnretained(y).toOpaque(), Unmanaged.passUnretained(uv).toOpaque(), depth.map { Unmanaged.passUnretained($0).toOpaque() }, UInt32(width), UInt32(height), UInt32(depthWidth), UInt32(depthHeight), Int32(rotation), fullRange)
    }
    static func catalog(locale: String) throws -> Catalog {
        let text = try ownedString(operation: "catalog") { locale.withCString(nativeCatalog) }
        guard let data = text.data(using: .utf8) else { throw SimulatorBridgeError.invalidJSON("catalog") }
        return try JSONDecoder().decode(Catalog.self, from: data)
    }
    static func compose(locale: String, presets: Set<String>, overrides: [String: JSONValue]) throws -> [String: JSONValue] {
        let presetData = try JSONEncoder().encode(Array(presets).sorted())
        let overrideData = try JSONEncoder().encode(overrides)
        guard let presetJSON = String(data: presetData, encoding: .utf8), let overrideJSON = String(data: overrideData, encoding: .utf8) else { throw SimulatorBridgeError.invalidJSON("compose input") }
        let text = try ownedString(operation: "compose") {
            locale.withCString { localePtr in presetJSON.withCString { presetPtr in overrideJSON.withCString { nativeCompose(localePtr, presetPtr, $0) } } }
        }
        guard let data = text.data(using: .utf8) else { throw SimulatorBridgeError.invalidJSON("compose") }
        return try JSONDecoder().decode([String: JSONValue].self, from: data)
    }
    static func post(settings: [String: JSONValue]) throws {
        let data = try JSONEncoder().encode(settings)
        guard let json = String(data: data, encoding: .utf8) else { throw SimulatorBridgeError.invalidJSON("settings") }
        guard json.withCString(nativePostSettings) else { throw SimulatorBridgeError.rejectedSettings }
    }
    private static func ownedString(operation: String, _ body: () -> UnsafeMutablePointer<CChar>?) throws -> String {
        guard let pointer = body() else { throw SimulatorBridgeError.unavailable(operation) }
        defer { nativeStringFree(pointer) }
        return String(cString: pointer)
    }
}
