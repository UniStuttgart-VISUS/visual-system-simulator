import Foundation

struct SimulatorSession: Equatable {
    var selectedDemonstrations: [String: String] = [:]
    var manual: [String: JSONValue] = [:]
    var maskedFallback: [String: JSONValue] = [:]

    func activePresets(in catalog: Catalog) -> Set<String> {
        Set(catalog.articles.flatMap { article in
            article.demonstrations.first { $0.id == selectedDemonstrations[article.id] }?.presets ?? []
        })
    }

    func selectDemonstration(in catalog: Catalog, articleID: String, demonstrationID: String) -> SimulatorSession {
        guard let article = catalog.articles.first(where: { $0.id == articleID }),
              let selected = article.demonstrations.first(where: { $0.id == demonstrationID }) else { return self }
        let beforePresets = activePresets(in: catalog)
        var nextSelections = selectedDemonstrations
        if nextSelections[articleID] == demonstrationID {
            nextSelections.removeValue(forKey: articleID)
        } else {
            let selectedValues = values(for: selected, in: catalog)
            for otherArticle in catalog.articles where otherArticle.id != articleID {
                guard let selectedID = nextSelections[otherArticle.id],
                      let other = otherArticle.demonstrations.first(where: { $0.id == selectedID }) else { continue }
                if values(for: other, in: catalog).contains(where: { setting, value in
                    selectedValues[setting].map { $0 != value } ?? false
                }) {
                    nextSelections.removeValue(forKey: otherArticle.id)
                }
            }
            nextSelections[articleID] = demonstrationID
        }
        var next = self
        next.selectedDemonstrations = nextSelections
        return next.cascadingPresetTransition(in: catalog, before: beforePresets, after: next.activePresets(in: catalog))
    }

    func edit(_ settingID: String, value: JSONValue) -> SimulatorSession {
        var next = self
        next.manual[settingID] = value
        return next
    }

    func reset(_ settingID: String) -> SimulatorSession {
        var next = self
        next.manual.removeValue(forKey: settingID)
        return next
    }

    func sourceArticleID(in catalog: Catalog, settingID: String) -> String? {
        guard manual[settingID] == nil else { return nil }
        let active = activePresets(in: catalog)
        guard let preset = catalog.presets.first(where: { active.contains($0.id) && $0.values[settingID] != nil }) else { return nil }
        return catalog.articles.first { article in
            article.demonstrations.contains { $0.presets.contains(preset.id) }
        }?.id
    }

    private func values(for demonstration: CatalogDemonstration, in catalog: Catalog) -> [String: JSONValue] {
        var result: [String: JSONValue] = [:]
        for presetID in demonstration.presets {
            guard let preset = catalog.presets.first(where: { $0.id == presetID }) else { continue }
            result.merge(preset.values) { _, new in new }
        }
        return result
    }

    private func cascadingPresetTransition(in catalog: Catalog, before: Set<String>, after: Set<String>) -> SimulatorSession {
        let beforeSettings = Set(catalog.presets.filter { before.contains($0.id) }.flatMap { $0.values.keys })
        let afterSettings = Set(catalog.presets.filter { after.contains($0.id) }.flatMap { $0.values.keys })
        var next = self
        for setting in beforeSettings.union(afterSettings) {
            switch (beforeSettings.contains(setting), afterSettings.contains(setting)) {
            case (false, true), (true, true):
                if let value = next.manual.removeValue(forKey: setting) { next.maskedFallback[setting] = value }
            case (true, false):
                if next.manual[setting] != nil { next.maskedFallback.removeValue(forKey: setting) }
                else if let value = next.maskedFallback.removeValue(forKey: setting) { next.manual[setting] = value }
            case (false, false): break
            }
        }
        return next
    }
}
