import Foundation
import Observation

enum MediaSource: Equatable { case camera, file(URL) }

@MainActor @Observable final class SimulatorModel {
    var catalog: Catalog?
    var activePresets = Set<String>() { didSet { refreshSettings() } }
    var overrides: [String: JSONValue] = [:] { didSet { refreshSettings() } }
    var effective: [String: JSONValue] = [:]
    var expanded = Set<String>()
    var selectedArticle: CatalogArticle?
    var pendingPresetRemoval: CatalogPreset?
    var pendingDemo: CatalogDemonstration?
    var source: MediaSource = .camera
    var fullscreen = false
    var errorMessage: String?
    private(set) var localeTag = Locale.current.identifier
    private var postTask: Task<Void, Never>?
    @ObservationIgnored weak var controller: SimulatorController?

    func load(locale: Locale = .current) {
        localeTag = locale.identifier
        do {
            catalog = try SimulatorBridge.catalog(locale: localeTag)
            expanded = Set(catalog?.groups.map(\.id) ?? [])
            refreshSettings()
        } catch { errorMessage = error.localizedDescription }
    }
    func toggle(_ preset: CatalogPreset) {
        if activePresets.contains(preset.id) {
            if !Set(preset.values.keys).isDisjoint(with: overrides.keys) { pendingPresetRemoval = preset }
            else { activePresets.remove(preset.id) }
        } else { activePresets.insert(preset.id) }
    }
    func remove(_ preset: CatalogPreset, discardAffected: Bool) {
        activePresets.remove(preset.id)
        if discardAffected { overrides = overrides.filter { !preset.values.keys.contains($0.key) } }
        pendingPresetRemoval = nil
    }
    func applyDemo(_ demo: CatalogDemonstration, replacing: Bool) {
        if replacing { activePresets = Set(demo.presets); overrides = [:] }
        else { activePresets.formUnion(demo.presets) }
        pendingDemo = nil
    }
    func refreshSettings() {
        guard catalog != nil else { return }
        do { effective = try SimulatorBridge.compose(locale: localeTag, presets: activePresets, overrides: overrides) }
        catch { errorMessage = error.localizedDescription; return }
        postTask?.cancel()
        let settings = effective
        postTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(100))
            guard !Task.isCancelled else { return }
            do { try self?.controller?.post(settings: settings) } catch { self?.errorMessage = error.localizedDescription }
        }
    }
}
