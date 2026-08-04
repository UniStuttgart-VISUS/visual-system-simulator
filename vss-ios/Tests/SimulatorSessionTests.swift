import XCTest
@testable import VSS

final class SimulatorSessionTests: XCTestCase {
    private func session() -> SimulatorSession { SimulatorSession().withEyeMode(.both) }

    func testIntrinsicPresetSelectsBothThenSymmetricPresetTargetsVisibleLeftEye() {
        let eyeCatalog = Catalog(groups: [], presets: [
            CatalogPreset(id: "strabismus", label: "Strabismus", both: [:], left: ["axis": .number(0.05)], right: ["axis": .number(-0.05)]),
            CatalogPreset(id: "cataract", label: "Cataract", both: ["blur": .number(25)], left: [:], right: [:]),
        ], articles: [
            CatalogArticle(id: "strabismus", locale: "en", title: "Strabismus", summary: nil, image: nil, contentPath: "", demonstrations: [CatalogDemonstration(id: "strabismus", label: "Strabismus", presets: ["strabismus"])]),
            CatalogArticle(id: "cataract", locale: "en", title: "Cataract", summary: nil, image: nil, contentPath: "", demonstrations: [CatalogDemonstration(id: "cataract", label: "Cataract", presets: ["cataract"])]),
        ])
        var state = SimulatorSession()
        XCTAssertEqual(state.eyeMode, .left)
        state = state.selectDemonstration(in: eyeCatalog, articleID: "strabismus", demonstrationID: "strabismus")
        XCTAssertEqual(state.eyeMode, .both)
        XCTAssertEqual(state.effectiveValues(in: eyeCatalog, for: .left)["axis"], .number(0.05))
        XCTAssertEqual(state.effectiveValues(in: eyeCatalog, for: .right)["axis"], .number(-0.05))
        state = state.withEyeMode(.left).selectDemonstration(in: eyeCatalog, articleID: "cataract", demonstrationID: "cataract")
        XCTAssertEqual(state.effectiveValues(in: eyeCatalog, for: .left)["blur"], .number(25))
        XCTAssertNil(state.effectiveValues(in: eyeCatalog, for: .right)["blur"])
        XCTAssertEqual(state.effectiveValues(in: eyeCatalog, for: .right)["axis"], .number(-0.05))
    }

    private let catalog = Catalog(
        groups: [],
        presets: [
            CatalogPreset(id: "weak", label: "Weak", both: ["blur": .number(25), "enabled": .bool(true)], left: [:], right: [:]),
            CatalogPreset(id: "strong", label: "Strong", both: ["blur": .number(75), "enabled": .bool(true)], left: [:], right: [:]),
            CatalogPreset(id: "same", label: "Same", both: ["enabled": .bool(true)], left: [:], right: [:]),
            CatalogPreset(id: "conflict", label: "Conflict", both: ["blur": .number(40)], left: [:], right: [:]),
        ],
        articles: [
            CatalogArticle(id: "cataract", locale: "en", title: "Cataract", summary: nil, image: nil, contentPath: "cataract.html", demonstrations: [
                CatalogDemonstration(id: "weak", label: "Weak", presets: ["weak"]),
                CatalogDemonstration(id: "strong", label: "Strong", presets: ["strong"]),
            ]),
            CatalogArticle(id: "other", locale: "en", title: "Other", summary: nil, image: nil, contentPath: "other.html", demonstrations: [
                CatalogDemonstration(id: "same", label: "Same", presets: ["same"]),
                CatalogDemonstration(id: "multi", label: "Multi", presets: ["same", "conflict"]),
            ]),
        ]
    )

    func testManualValueIsRestoredAfterLastRelevantPresetIsDisabled() {
        let manual = session().edit("blur", value: .number(10))
        let active = manual.selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
        XCTAssertNil(active.currentLayer().manual["blur"])
        XCTAssertEqual(active.currentLayer().maskedFallback["blur"], .number(10))

        let disabled = active.selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
        XCTAssertEqual(disabled.currentLayer().manual["blur"], .number(10))
    }

    func testNewestManualValueWinsWhenPresetIsDisabled() {
        let state = session().edit("blur", value: .number(10))
            .selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
            .edit("blur", value: .number(15))
            .selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
        XCTAssertEqual(state.currentLayer().manual["blur"], .number(15))
        XCTAssertNil(state.currentLayer().maskedFallback["blur"])
    }

    func testSwitchingVariantsMasksLatestManualValue() {
        let state = session()
            .selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
            .edit("blur", value: .number(15))
            .selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "strong")
        XCTAssertEqual(state.activePresets(in: catalog), ["strong"])
        XCTAssertEqual(state.currentLayer().maskedFallback["blur"], .number(15))
        XCTAssertNil(state.currentLayer().manual["blur"])
    }

    func testCompatibleDemonstrationsCombineAndConflictsReplace() {
        let combined = session()
            .selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
            .selectDemonstration(in: catalog, articleID: "other", demonstrationID: "same")
        XCTAssertEqual(combined.activePresets(in: catalog), ["weak", "same"])

        let replaced = combined.selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "strong")
        XCTAssertEqual(replaced.activePresets(in: catalog), ["strong", "same"])

        let conflict = replaced.selectDemonstration(in: catalog, articleID: "other", demonstrationID: "multi")
        XCTAssertEqual(conflict.activePresets(in: catalog), ["same", "conflict"])
        XCTAssertNil(conflict.currentLayer().selectedDemonstrations["cataract"])
    }

    func testResetRevealsPresetProvenanceThenDefault() {
        let active = session()
            .selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
            .edit("blur", value: .number(12))
        XCTAssertNil(active.sourceArticleID(in: catalog, settingID: "blur"))

        let reset = active.reset("blur", in: catalog)
        XCTAssertEqual(reset.sourceArticleID(in: catalog, settingID: "blur"), "cataract")

        let disabled = reset.selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
        XCTAssertNil(disabled.sourceArticleID(in: catalog, settingID: "blur"))
    }
}
