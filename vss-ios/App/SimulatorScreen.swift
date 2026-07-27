import AVFoundation
import SwiftUI
import UIKit
import UniformTypeIdentifiers

struct SimulatorScreen: View {
    @State private var controller = SimulatorController()
    @State private var model = SimulatorModel()
    @State private var importing = false
    @Environment(\.scenePhase) private var scenePhase
    @Environment(\.openURL) private var openURL
    private var t: (String) -> String { { UIStrings.text($0, locale: .current) } }

    var body: some View {
        GeometryReader { geometry in
            if model.fullscreen {
                Preview(model: model, controller: controller, importing: $importing, controls: false)
                    .ignoresSafeArea().contentShape(Rectangle()).onTapGesture { model.fullscreen = false }
                    .overlay(alignment: .bottom) { Text(t("exitFullscreen")).padding(10).background(.ultraThinMaterial, in: Capsule()).padding() }
            } else if geometry.size.width >= 760 {
                HStack(spacing: 12) {
                    Preview(model: model, controller: controller, importing: $importing, controls: true).frame(maxWidth: .infinity, maxHeight: .infinity)
                    CatalogPanel(model: model).frame(minWidth: 360, idealWidth: 420, maxWidth: 520)
                }.padding(12)
            } else {
                VStack(spacing: 0) {
                    Preview(model: model, controller: controller, importing: $importing, controls: true).aspectRatio(16 / 9, contentMode: .fit)
                    CatalogPanel(model: model)
                }
            }
        }
        .task { model.controller = controller; model.load(); activateSource() }
        .onDisappear { controller.stopSources() }
        .onOpenURL { url in model.source = .file(url); controller.startMedia(url) }
        .onChange(of: scenePhase) { _, phase in if phase == .active { activateSource() } else { controller.stopSources() } }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.image, .movie]) { result in
            do { let url = try result.get(); model.source = .file(url); controller.startMedia(url) }
            catch { model.errorMessage = error.localizedDescription }
        }
        .sheet(item: $model.selectedArticle) { ArticleSheet(article: $0, model: model) }
        .alert(t("error"), isPresented: Binding(get: { model.errorMessage != nil || controller.errorMessage != nil }, set: { if !$0 { model.errorMessage = nil; controller.errorMessage = nil } })) {
            Button(t("dismiss")) { model.errorMessage = nil; controller.errorMessage = nil }
        } message: { Text(model.errorMessage ?? controller.errorMessage ?? "") }
        .alert(t("removePreset"), isPresented: Binding(get: { model.pendingPresetRemoval != nil }, set: { if !$0 { model.pendingPresetRemoval = nil } })) {
            if let preset = model.pendingPresetRemoval {
                Button(t("keepOverrides")) { model.remove(preset, discardAffected: false) }
                Button(t("discardOverrides"), role: .destructive) { model.remove(preset, discardAffected: true) }
            }
            Button(t("cancel"), role: .cancel) { model.pendingPresetRemoval = nil }
        } message: { Text(t("presetConflict")) }
        .alert(t("applyDemo"), isPresented: Binding(get: { model.pendingDemo != nil }, set: { if !$0 { model.pendingDemo = nil } })) {
            if let demo = model.pendingDemo {
                Button(t("replace")) { model.applyDemo(demo, replacing: true) }
                Button(t("add")) { model.applyDemo(demo, replacing: false) }
            }
            Button(t("cancel"), role: .cancel) { model.pendingDemo = nil }
        } message: { Text(t("demoMessage")) }
    }
    private func activateSource() { switch model.source { case .camera: controller.startCamera(); case .file(let url): controller.startMedia(url) } }
}

private struct Preview: View {
    @Bindable var model: SimulatorModel
    @Bindable var controller: SimulatorController
    @Binding var importing: Bool
    let controls: Bool
    @Environment(\.openURL) private var openURL
    var body: some View {
        ZStack {
            Color.black
            SimulatorView(controller: controller)
            if model.source == .camera && [.denied, .restricted].contains(controller.cameraAuthorization) {
                VStack(spacing: 12) {
                    Text(UIStrings.text("permissionTitle", locale: .current)).font(.headline)
                    Text(UIStrings.text("permissionMessage", locale: .current)).multilineTextAlignment(.center)
                    HStack {
                        Button(UIStrings.text("tryAgain", locale: .current)) { controller.startCamera() }
                        Button(UIStrings.text("openSettings", locale: .current)) { openURL(URL(string: UIApplication.openSettingsURLString)!) }
                    }.buttonStyle(.borderedProminent)
                }.foregroundStyle(.white).padding().background(.black.opacity(0.7), in: RoundedRectangle(cornerRadius: 16)).padding()
            }
            if controls {
                HStack {
                    Menu {
                        Button(UIStrings.text("camera", locale: .current), systemImage: "camera") { model.source = .camera; controller.startCamera() }
                        Button(UIStrings.text("media", locale: .current), systemImage: "photo.on.rectangle") { importing = true }
                    } label: { Image(systemName: "photo.on.rectangle").frame(width: 44, height: 44) }
                    Spacer()
                    Button { model.fullscreen = true } label: { Image(systemName: "arrow.up.left.and.arrow.down.right").frame(width: 44, height: 44) }
                }.buttonStyle(.borderedProminent).padding().frame(maxHeight: .infinity, alignment: .bottom)
            }
        }.clipped()
    }
}

private struct CatalogPanel: View {
    @Bindable var model: SimulatorModel
    var body: some View {
        if let catalog = model.catalog {
            VStack(spacing: 6) {
                ScrollView(.horizontal, showsIndicators: false) { HStack {
                    ForEach(catalog.articles) { article in Button(article.title, systemImage: "doc.text") { model.selectedArticle = article }.buttonStyle(.bordered) }
                }.padding(.horizontal) }
                ScrollView(.horizontal, showsIndicators: false) { HStack {
                    ForEach(catalog.presets) { preset in Toggle(preset.label, isOn: Binding(get: { model.activePresets.contains(preset.id) }, set: { _ in model.toggle(preset) })).toggleStyle(.button) }
                    if !model.overrides.isEmpty { Button(UIStrings.text("resetAll", locale: .current), systemImage: "arrow.counterclockwise") { model.overrides = [:] } }
                }.padding(.horizontal) }
                ScrollView { LazyVStack(spacing: 10) { ForEach(catalog.groups) { SettingsGroup(group: $0, model: model) } }.padding() }
            }
        } else { ProgressView(UIStrings.text("loading", locale: .current)) }
    }
}

private struct SettingsGroup: View {
    let group: CatalogGroup
    @Bindable var model: SimulatorModel
    var body: some View {
        DisclosureGroup(isExpanded: Binding(get: { model.expanded.contains(group.id) }, set: { if $0 { model.expanded.insert(group.id) } else { model.expanded.remove(group.id) } })) {
            VStack(spacing: 12) { ForEach(group.settings) { SettingControl(setting: $0, model: model) } }.padding(.top, 8)
        } label: { Text(group.title).font(.headline) }
        .padding().background(.regularMaterial, in: RoundedRectangle(cornerRadius: 14))
    }
}

private struct SettingControl: View {
    let setting: CatalogSetting
    @Bindable var model: SimulatorModel
    private var value: JSONValue { model.effective[setting.id] ?? setting.default }
    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack {
                Text(setting.label)
                Spacer()
                if model.overrides[setting.id] != nil { Button { model.overrides.removeValue(forKey: setting.id) } label: { Image(systemName: "arrow.counterclockwise") }.accessibilityLabel(UIStrings.text("reset", locale: .current)) }
                control
            }
            if !setting.help.isEmpty { Text(setting.help).font(.caption).foregroundStyle(.secondary) }
        }
    }
    @ViewBuilder private var control: some View {
        switch setting.control.kind {
        case "boolean": Toggle("", isOn: Binding(get: { value.bool ?? false }, set: { model.overrides[setting.id] = .bool($0) })).labelsHidden()
        case "number":
            let number = value.number ?? 0, step = setting.control.step ?? 1
            HStack(spacing: 4) {
                Button { setNumber(number - step) } label: { Image(systemName: "minus.circle") }
                Text(number.formatted(.number.precision(.fractionLength(step < 1 ? 2 : 0))) + (setting.unit.map { " \($0)" } ?? "")).monospacedDigit().frame(minWidth: 65)
                Button { setNumber(number + step) } label: { Image(systemName: "plus.circle") }
            }
        case "choice":
            Picker(setting.label, selection: Binding(get: { Int(value.number ?? 0) }, set: { model.overrides[setting.id] = .number(Double($0)) })) {
                ForEach(setting.control.choices ?? []) { Text($0.label).tag($0.value) }
            }.labelsHidden().pickerStyle(.menu)
        default: Text(value.description).foregroundStyle(.secondary)
        }
    }
    private func setNumber(_ number: Double) {
        let clamped = min(setting.control.max ?? Double.greatestFiniteMagnitude, max(setting.control.min ?? -Double.greatestFiniteMagnitude, number))
        model.overrides[setting.id] = .number(setting.control.integer == true ? clamped.rounded() : clamped)
    }
}
