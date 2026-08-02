import Foundation

enum JSONValue: Codable, Equatable, CustomStringConvertible {
    case bool(Bool), number(Double), string(String), array([JSONValue]), object([String: JSONValue]), null

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() { self = .null }
        else if let value = try? container.decode(Bool.self) { self = .bool(value) }
        else if let value = try? container.decode(Double.self) { self = .number(value) }
        else if let value = try? container.decode(String.self) { self = .string(value) }
        else if let value = try? container.decode([JSONValue].self) { self = .array(value) }
        else { self = .object(try container.decode([String: JSONValue].self)) }
    }
    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .bool(let value): try container.encode(value)
        case .number(let value): try container.encode(value)
        case .string(let value): try container.encode(value)
        case .array(let value): try container.encode(value)
        case .object(let value): try container.encode(value)
        case .null: try container.encodeNil()
        }
    }
    var bool: Bool? { if case .bool(let value) = self { value } else { nil } }
    var number: Double? { if case .number(let value) = self { value } else { nil } }
    var description: String {
        switch self { case .bool(let v): return v.description; case .number(let v): return v.formatted(); case .string(let v): return v; case .array(let v): return v.map(\.description).joined(separator: ", "); case .object: return "…"; case .null: return "–" }
    }
}

struct Catalog: Codable {
    let groups: [CatalogGroup]
    let presets: [CatalogPreset]
    let articles: [CatalogArticle]
}
struct CatalogGroup: Codable, Identifiable { let id: String; let title: String; let settings: [CatalogSetting] }
struct CatalogSetting: Codable, Identifiable { let id: String; let label: String; let help: String; let `default`: JSONValue; let control: CatalogControl; let unit: String? }
struct CatalogControl: Codable {
    let kind: String
    let integer: Bool?
    let min: Double?
    let max: Double?
    let step: Double?
    let choices: [CatalogChoice]?
}
struct CatalogChoice: Codable, Identifiable { let value: Int; let label: String; var id: Int { value } }
struct CatalogPreset: Codable, Identifiable { let id: String; let label: String; let values: [String: JSONValue] }
struct CatalogArticle: Codable, Identifiable {
    let id: String
    let locale: String
    let title: String
    let summary: String?
    let image: String?
    let contentPath: String
    let demonstrations: [CatalogDemonstration]
    enum CodingKeys: String, CodingKey { case id, locale, title, summary, image, contentPath = "content_path", demonstrations }
}
struct CatalogDemonstration: Codable, Identifiable { let id: String; let label: String; let presets: [String] }

enum UIStrings {
    static func text(_ key: String, locale: Locale) -> String {
        let de = locale.language.languageCode?.identifier == "de"
        let values: [String: (String, String)] = [
            "camera": ("Camera", "Kamera"), "media": ("Photo / video", "Bild / Video"),
            "fullscreen": ("Full screen", "Vollbild"), "exitFullscreen": ("Tap to exit full screen", "Tippen beendet den Vollbildmodus"),
            "settings": ("Settings", "Einstellungen"), "articles": ("Articles", "Artikel"),
            "resetAll": ("Reset manual changes", "Manuelle Änderungen zurücksetzen"), "reset": ("Reset", "Zurücksetzen"),
            "openArticle": ("Open article", "Artikel öffnen"),
            "explainingArticle": ("Open explaining article", "Erklärenden Artikel öffnen"),
            "activate": ("Activate", "Aktivieren"), "deactivate": ("Deactivate", "Deaktivieren"),
            "articlePosition": ("Article position; active simulations are shown with thick segments", "Artikelposition; aktive Simulationen werden als dicke Segmente dargestellt"),
            "of": ("of", "von"), "active": ("active", "aktiv"),
            "permissionTitle": ("Camera access needed", "Kamerazugriff erforderlich"),
            "permissionMessage": ("Allow camera access to use the live simulation, or choose a photo or video.", "Erlaube den Kamerazugriff für die Live-Simulation oder wähle ein Bild oder Video."),
            "tryAgain": ("Try again", "Erneut versuchen"), "openSettings": ("Open Settings", "Einstellungen öffnen"),
            "error": ("Error", "Fehler"), "dismiss": ("Dismiss", "Schließen"), "cancel": ("Cancel", "Abbrechen"),
            "removePreset": ("Remove preset?", "Preset entfernen?"),
            "presetConflict": ("This preset affects manual changes. Keep or discard those changes?", "Dieses Preset betrifft manuelle Änderungen. Sollen sie beibehalten oder verworfen werden?"),
            "keepOverrides": ("Keep changes", "Änderungen behalten"), "discardOverrides": ("Discard affected", "Betroffene verwerfen"),
            "applyDemo": ("Apply demonstration", "Demonstration anwenden"),
            "demoMessage": ("Replace the current simulation or add these presets?", "Aktuelle Simulation ersetzen oder diese Presets hinzufügen?"),
            "replace": ("Replace", "Ersetzen"), "add": ("Add presets", "Presets hinzufügen"),
            "noCamera": ("No camera image", "Kein Kamerabild"), "loading": ("Loading…", "Lädt …")
        ]
        let value = values[key] ?? (key, key)
        return de ? value.1 : value.0
    }
}
