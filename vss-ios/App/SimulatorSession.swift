import Foundation

enum EyeMode: String, CaseIterable, Hashable { case left, both, right }

struct SessionLayer: Equatable {
    var selectedDemonstrations: [String: String] = [:]
    var manual: [String: JSONValue] = [:]
    var maskedFallback: [String: JSONValue] = [:]
    var maskedByBoth = Set<String>()
}

struct SimulatorSession: Equatable {
    var eyeMode: EyeMode = .left
    var layers: [EyeMode: SessionLayer] = Dictionary(uniqueKeysWithValues: EyeMode.allCases.map { ($0, SessionLayer()) })

    func withEyeMode(_ mode: EyeMode) -> SimulatorSession {
        var next = self; next.eyeMode = mode; return next
    }
    func currentLayer() -> SessionLayer { layer(eyeMode) }
    func selectedDemonstration(articleID: String) -> String? {
        if eyeMode != .both { return currentLayer().selectedDemonstrations[articleID] }
        return layer(.both).selectedDemonstrations[articleID]
            ?? layer(.left).selectedDemonstrations[articleID]
            ?? layer(.right).selectedDemonstrations[articleID]
    }

    func activePresets(in catalog: Catalog) -> Set<String> {
        if eyeMode != .both { return presets(in: currentLayer(), catalog: catalog) }
        let shared = presets(in: layer(.both), catalog: catalog)
        let intrinsic = presets(in: layer(.left), catalog: catalog).union(presets(in: layer(.right), catalog: catalog))
            .filter { catalog.preset($0)?.isIntrinsic == true }
        return shared.union(intrinsic)
    }

    func selectDemonstration(in catalog: Catalog, articleID: String, demonstrationID: String) -> SimulatorSession {
        guard let article = catalog.articles.first(where: { $0.id == articleID }),
              let selected = article.demonstrations.first(where: { $0.id == demonstrationID }) else { return self }
        let selectedPresets = selected.presets.compactMap(catalog.preset)
        let intrinsic = selectedPresets.contains { $0.isIntrinsic }
        let targets = intrinsic ? [EyeMode.left, .right].filter { target in selectedPresets.contains { !$0.values(for: target).isEmpty } } : [eyeMode]
        let enabled = !targets.allSatisfy { layer($0).selectedDemonstrations[articleID] == demonstrationID }
        var next = self
        for target in targets {
            next = next.withLayer(target, transition(layer: next.layer(target), catalog: catalog, target: target, articleID: articleID, selected: selected, enabled: enabled))
            if target != .both && enabled { next = next.mask(targets: [target], settings: demonstrationValues(selected, catalog: catalog, target: target).keys, masked: false) }
        }
        if targets.contains(.both) {
            let changed = Set(demonstrationValues(selected, catalog: catalog, target: .both).keys)
            next = enabled
                ? next.mask(targets: [.left, .right], settings: changed, masked: true)
                : next.mask(targets: [.left, .right], settings: changed.subtracting(next.sharedSettings(in: catalog)), masked: false)
        }
        if intrinsic && enabled { next.eyeMode = .both }
        return next
    }

    func edit(_ settingID: String, value: JSONValue) -> SimulatorSession {
        var targetLayer = currentLayer(); targetLayer.manual[settingID] = value
        var next = withLayer(eyeMode, targetLayer)
        return next.mask(targets: eyeMode == .both ? [.left, .right] : [eyeMode], settings: [settingID], masked: eyeMode == .both)
    }

    func reset(_ settingID: String, in catalog: Catalog? = nil) -> SimulatorSession {
        var targetLayer = currentLayer(); targetLayer.manual.removeValue(forKey: settingID)
        var next = withLayer(eyeMode, targetLayer)
        if eyeMode == .both && (catalog == nil || !next.sharedSettings(in: catalog!).contains(settingID)) {
            next = next.mask(targets: [.left, .right], settings: [settingID], masked: false)
        }
        return next
    }

    func effectiveValues(in catalog: Catalog, for eye: EyeMode) -> [String: JSONValue] {
        precondition(eye != .both)
        var result = Dictionary(uniqueKeysWithValues: catalog.groups.flatMap(\.settings).map { ($0.id, $0.default) })
        apply(layer(.both), to: &result, catalog: catalog, target: .both)
        apply(layer(eye), to: &result, catalog: catalog, target: eye)
        return result
    }

    func editableValues(in catalog: Catalog) -> [String: JSONValue] {
        if eyeMode != .both { return effectiveValues(in: catalog, for: eyeMode) }
        var result = Dictionary(uniqueKeysWithValues: catalog.groups.flatMap(\.settings).map { ($0.id, $0.default) })
        apply(layer(.both), to: &result, catalog: catalog, target: .both)
        return result
    }

    func sourceArticleID(in catalog: Catalog, settingID: String) -> String? {
        let targetLayer = currentLayer()
        guard targetLayer.manual[settingID] == nil,
              let owner = catalog.presets.first(where: { presets(in: targetLayer, catalog: catalog).contains($0.id) && $0.values(for: eyeMode)[settingID] != nil }) else { return nil }
        return catalog.articles.first { $0.demonstrations.contains { $0.presets.contains(owner.id) } }?.id
    }

    private func layer(_ mode: EyeMode) -> SessionLayer { layers[mode] ?? SessionLayer() }
    private func withLayer(_ mode: EyeMode, _ layer: SessionLayer) -> SimulatorSession { var next = self; next.layers[mode] = layer; return next }
    private func mask<S: Sequence>(targets: [EyeMode], settings: S, masked: Bool) -> SimulatorSession where S.Element == String {
        let settings = Set(settings); var next = self
        for target in targets { var value = next.layer(target); if masked { value.maskedByBoth.formUnion(settings) } else { value.maskedByBoth.subtract(settings) }; next.layers[target] = value }
        return next
    }
    private func sharedSettings(in catalog: Catalog) -> Set<String> {
        Set(layer(.both).manual.keys).union(settings(for: presets(in: layer(.both), catalog: catalog), catalog: catalog, target: .both))
    }
}

private extension Catalog {
    func preset(_ id: String) -> CatalogPreset? { presets.first { $0.id == id } }
}
private extension CatalogPreset {
    var isIntrinsic: Bool { !left.isEmpty || !right.isEmpty }
    func values(for target: EyeMode) -> [String: JSONValue] {
        if !both.isEmpty { return both }
        switch target { case .left: return left; case .right: return right; case .both: return [:] }
    }
}
private func presets(in layer: SessionLayer, catalog: Catalog) -> Set<String> {
    Set(catalog.articles.flatMap { article in article.demonstrations.first { $0.id == layer.selectedDemonstrations[article.id] }?.presets ?? [] })
}
private func demonstrationValues(_ demo: CatalogDemonstration, catalog: Catalog, target: EyeMode) -> [String: JSONValue] {
    demo.presets.compactMap(catalog.preset).reduce(into: [:]) { result, preset in result.merge(preset.values(for: target)) { _, new in new } }
}
private func settings(for presets: Set<String>, catalog: Catalog, target: EyeMode) -> Set<String> {
    Set(presets.compactMap(catalog.preset).flatMap { $0.values(for: target).keys })
}
private func transition(layer: SessionLayer, catalog: Catalog, target: EyeMode, articleID: String, selected: CatalogDemonstration, enabled: Bool) -> SessionLayer {
    let before = presets(in: layer, catalog: catalog); var selections = layer.selectedDemonstrations
    if !enabled { selections.removeValue(forKey: articleID) } else {
        let values = demonstrationValues(selected, catalog: catalog, target: target)
        for article in catalog.articles where article.id != articleID {
            guard let id = selections[article.id], let other = article.demonstrations.first(where: { $0.id == id }) else { continue }
            if demonstrationValues(other, catalog: catalog, target: target).contains(where: { setting, value in values[setting].map { $0 != value } ?? false }) { selections.removeValue(forKey: article.id) }
        }
        selections[articleID] = selected.id
    }
    var changed = layer; changed.selectedDemonstrations = selections
    return cascade(catalog: catalog, target: target, before: before, after: presets(in: changed, catalog: catalog), layer: changed)
}
private func cascade(catalog: Catalog, target: EyeMode, before: Set<String>, after: Set<String>, layer: SessionLayer) -> SessionLayer {
    let beforeSettings = settings(for: before, catalog: catalog, target: target), afterSettings = settings(for: after, catalog: catalog, target: target)
    var next = layer
    for setting in beforeSettings.union(afterSettings) {
        if afterSettings.contains(setting) { if let value = next.manual.removeValue(forKey: setting) { next.maskedFallback[setting] = value } }
        else if beforeSettings.contains(setting) { if next.manual[setting] != nil { next.maskedFallback.removeValue(forKey: setting) } else if let value = next.maskedFallback.removeValue(forKey: setting) { next.manual[setting] = value } }
    }
    return next
}
private func apply(_ layer: SessionLayer, to result: inout [String: JSONValue], catalog: Catalog, target: EyeMode) {
    for preset in catalog.presets where presets(in: layer, catalog: catalog).contains(preset.id) { result.merge(preset.values(for: target).filter { !layer.maskedByBoth.contains($0.key) }) { _, new in new } }
    result.merge(layer.manual.filter { !layer.maskedByBoth.contains($0.key) }) { _, new in new }
}
