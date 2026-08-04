import Foundation
import Observation

enum MediaSource: Equatable { case camera, file(URL) }

@MainActor @Observable final class SimulatorModel {
    var catalog: Catalog?
    var session = SimulatorSession()
    var effective: [String: JSONValue] = [:]
    var expanded = Set<String>()
    var selectedArticle: CatalogArticle?
    var viewedArticleID: String?
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
            if viewedArticleID == nil, let catalog {
                let active = session.activePresets(in: catalog)
                viewedArticleID = catalog.articles.first { article in
                    article.demonstrations.contains { !$0.presets.allSatisfy { !active.contains($0) } }
                }?.id ?? catalog.articles.first?.id
            }
            refreshSettings()
        } catch { errorMessage = error.localizedDescription }
    }
    func selectDemonstration(articleID: String, demonstrationID: String) {
        guard let catalog else { return }
        let previousMode = session.eyeMode
        session = session.selectDemonstration(in: catalog, articleID: articleID, demonstrationID: demonstrationID)
        if session.eyeMode != previousMode {
            do { try controller?.setEyeMode(session.eyeMode) } catch { errorMessage = error.localizedDescription }
        }
        refreshSettings()
    }
    func edit(_ settingID: String, value: JSONValue) {
        session = session.edit(settingID, value: value)
        refreshSettings()
    }
    func setEyeMode(_ mode: EyeMode) {
        session = session.withEyeMode(mode)
        do { try controller?.setEyeMode(mode) } catch { errorMessage = error.localizedDescription }
        refreshSettings()
    }
    func reset(_ settingID: String) {
        session = session.reset(settingID, in: catalog)
        refreshSettings()
    }
    func sourceArticle(for settingID: String) -> CatalogArticle? {
        guard let catalog, let id = session.sourceArticleID(in: catalog, settingID: settingID) else { return nil }
        return catalog.articles.first { $0.id == id }
    }
    func refreshSettings() {
        guard let catalog else { return }
        effective = session.editableValues(in: catalog)
        postTask?.cancel()
        let left = session.effectiveValues(in: catalog, for: .left)
        let right = session.effectiveValues(in: catalog, for: .right)
        postTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(100))
            guard !Task.isCancelled else { return }
            do { try self?.controller?.post(left: left, right: right) } catch { self?.errorMessage = error.localizedDescription }
        }
    }
}
