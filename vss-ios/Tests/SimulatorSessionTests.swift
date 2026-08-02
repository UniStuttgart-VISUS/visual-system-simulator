import XCTest
@testable import VSS

final class SimulatorSessionTests: XCTestCase {
    private let catalog = Catalog(
        groups: [],
        presets: [
            CatalogPreset(id: "weak", label: "Weak", values: ["blur": .number(25), "enabled": .bool(true)]),
            CatalogPreset(id: "strong", label: "Strong", values: ["blur": .number(75), "enabled": .bool(true)]),
            CatalogPreset(id: "same", label: "Same", values: ["enabled": .bool(true)]),
            CatalogPreset(id: "conflict", label: "Conflict", values: ["blur": .number(40)]),
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
        let manual = SimulatorSession().edit("blur", value: .number(10))
        let active = manual.selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
        XCTAssertNil(active.manual["blur"])
        XCTAssertEqual(active.maskedFallback["blur"], .number(10))

        let disabled = active.selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
        XCTAssertEqual(disabled.manual["blur"], .number(10))
    }

    func testNewestManualValueWinsWhenPresetIsDisabled() {
        let state = SimulatorSession().edit("blur", value: .number(10))
            .selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
            .edit("blur", value: .number(15))
            .selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
        XCTAssertEqual(state.manual["blur"], .number(15))
        XCTAssertNil(state.maskedFallback["blur"])
    }

    func testSwitchingVariantsMasksLatestManualValue() {
        let state = SimulatorSession()
            .selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
            .edit("blur", value: .number(15))
            .selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "strong")
        XCTAssertEqual(state.activePresets(in: catalog), ["strong"])
        XCTAssertEqual(state.maskedFallback["blur"], .number(15))
        XCTAssertNil(state.manual["blur"])
    }

    func testCompatibleDemonstrationsCombineAndConflictsReplace() {
        let combined = SimulatorSession()
            .selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
            .selectDemonstration(in: catalog, articleID: "other", demonstrationID: "same")
        XCTAssertEqual(combined.activePresets(in: catalog), ["weak", "same"])

        let replaced = combined.selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "strong")
        XCTAssertEqual(replaced.activePresets(in: catalog), ["strong", "same"])

        let conflict = replaced.selectDemonstration(in: catalog, articleID: "other", demonstrationID: "multi")
        XCTAssertEqual(conflict.activePresets(in: catalog), ["same", "conflict"])
        XCTAssertNil(conflict.selectedDemonstrations["cataract"])
    }

    func testResetRevealsPresetProvenanceThenDefault() {
        let active = SimulatorSession()
            .selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
            .edit("blur", value: .number(12))
        XCTAssertNil(active.sourceArticleID(in: catalog, settingID: "blur"))

        let reset = active.reset("blur")
        XCTAssertEqual(reset.sourceArticleID(in: catalog, settingID: "blur"), "cataract")

        let disabled = reset.selectDemonstration(in: catalog, articleID: "cataract", demonstrationID: "weak")
        XCTAssertNil(disabled.sourceArticleID(in: catalog, settingID: "blur"))
    }
}
