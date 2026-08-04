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
                    .ignoresSafeArea()
                    .overlay(alignment: .topLeading) {
                        Button { model.fullscreen = false } label: { Image(systemName: "xmark").frame(width: 44, height: 44) }
                            .buttonStyle(.borderedProminent).padding()
                    }
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
        .onChange(of: model.fullscreen && model.session.eyeMode == .both, initial: true) { _, locked in requestLandscape(locked) }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.image, .movie]) { result in
            do { let url = try result.get(); model.source = .file(url); controller.startMedia(url) }
            catch { model.errorMessage = error.localizedDescription }
        }
        .sheet(item: $model.selectedArticle) { ArticleSheet(article: $0, model: model) }
        .alert(t("error"), isPresented: Binding(get: { model.errorMessage != nil || controller.errorMessage != nil }, set: { if !$0 { model.errorMessage = nil; controller.errorMessage = nil } })) {
            Button(t("dismiss")) { model.errorMessage = nil; controller.errorMessage = nil }
        } message: { Text(model.errorMessage ?? controller.errorMessage ?? "") }
    }
    private func activateSource() { switch model.source { case .camera: controller.startCamera(); case .file(let url): controller.startMedia(url) } }
    private func requestLandscape(_ locked: Bool) {
        guard let scene = UIApplication.shared.connectedScenes.compactMap({ $0 as? UIWindowScene }).first else { return }
        scene.requestGeometryUpdate(.iOS(interfaceOrientations: locked ? .landscape : .all))
    }
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
                    Spacer(minLength: 8)
                    EyeModeSelector(model: model)
                    Spacer(minLength: 8)
                    Button { model.fullscreen = true } label: { Image(systemName: "arrow.up.left.and.arrow.down.right").frame(width: 44, height: 44) }
                }.buttonStyle(.borderedProminent).padding().frame(maxHeight: .infinity, alignment: .bottom)
            }
        }.clipped()
    }
}

private struct EyeModeSelector: View {
    @Bindable var model: SimulatorModel
    var body: some View {
        HStack(spacing: 4) {
            ForEach(EyeMode.allCases, id: \.self) { mode in
                let selected = model.session.eyeMode == mode
                Button { model.setEyeMode(mode) } label: {
                    EyeModeArtwork(mode: mode).frame(width: 32, height: 22).frame(minWidth: 48, minHeight: 44)
                }
                .buttonStyle(.bordered)
                .tint(selected ? .accentColor : .secondary)
                .accessibilityLabel(UIStrings.text(mode.rawValue, locale: .current))
                .accessibilityAddTraits(selected ? .isSelected : [])
            }
        }
    }
}

private struct EyeModeArtwork: View {
    let mode: EyeMode
    var body: some View {
        Canvas { context, size in
            let color = Color.primary, sx = size.width / 32, sy = size.height / 22
            var viewer = Path()
            viewer.move(to: CGPoint(x: 3 * sx, y: 8.5 * sy)); viewer.addLine(to: CGPoint(x: 6 * sx, y: 5 * sy)); viewer.addLine(to: CGPoint(x: 26 * sx, y: 5 * sy)); viewer.addLine(to: CGPoint(x: 29 * sx, y: 8.5 * sy)); viewer.addLine(to: CGPoint(x: 29 * sx, y: 16.5 * sy)); viewer.addLine(to: CGPoint(x: 26 * sx, y: 19 * sy)); viewer.addLine(to: CGPoint(x: 6 * sx, y: 19 * sy)); viewer.addLine(to: CGPoint(x: 3 * sx, y: 16.5 * sy)); viewer.closeSubpath()
            context.stroke(viewer, with: .color(color.opacity(0.72)), lineWidth: 1.8 * min(sx, sy))
            for (index, x) in [11.0, 21.0].enumerated() {
                let active = mode == .both || (mode == .left && index == 0) || (mode == .right && index == 1)
                let radius = 4 * min(sx, sy), center = CGPoint(x: x * sx, y: 12 * sy)
                let lens = Path(ellipseIn: CGRect(x: center.x - radius, y: center.y - radius, width: radius * 2, height: radius * 2))
                if active { context.fill(lens, with: .color(color)) } else { context.stroke(lens, with: .color(color.opacity(0.35)), lineWidth: 1.8 * min(sx, sy)) }
            }
        }
    }
}

private struct CatalogPanel: View {
    @Bindable var model: SimulatorModel
    var body: some View {
        if let catalog = model.catalog {
            ScrollView {
                LazyVStack(spacing: 10) {
                    ArticleIndex(catalog: catalog, model: model)
                    ArticleGallery(catalog: catalog, model: model)
                    ForEach(catalog.groups) { SettingsGroup(group: $0, model: model) }
                }.padding(.vertical, 8)
            }
        } else { ProgressView(UIStrings.text("loading", locale: .current)) }
    }
}

private struct ArticleGallery: View {
    let catalog: Catalog
    @Bindable var model: SimulatorModel

    var body: some View {
        GeometryReader { geometry in
            ScrollView(.horizontal, showsIndicators: false) {
                LazyHStack(spacing: 10) {
                    ForEach(Array(catalog.articles.enumerated()), id: \.element.id) { index, article in
                        ArticleCard(article: article, index: index, count: catalog.articles.count, catalog: catalog, model: model)
                            .frame(width: max(260, geometry.size.width * 0.88))
                            .id(article.id)
                    }
                }
                .scrollTargetLayout()
                .padding(.horizontal, 12)
            }
            .scrollTargetBehavior(.viewAligned)
            .scrollPosition(id: $model.viewedArticleID, anchor: .leading)
        }
        .frame(height: 260)
    }
}

private struct ArticleCard: View {
    let article: CatalogArticle
    let index: Int
    let count: Int
    let catalog: Catalog
    @Bindable var model: SimulatorModel

    private var selected: CatalogDemonstration? {
        article.demonstrations.first { $0.id == model.session.selectedDemonstration(articleID: article.id) }
    }
    private var positionLabel: String {
        "\(article.title), \(index + 1) \(UIStrings.text("of", locale: .current)) \(count)" + (selected.map { ", \($0.label) \(UIStrings.text("active", locale: .current))" } ?? "")
    }

    var body: some View {
        VStack(spacing: 0) {
            Button { model.selectedArticle = article } label: {
                ZStack(alignment: .bottom) {
                    GalleryImage(path: article.image)
                    HStack(spacing: 8) {
                        Text(article.title).font(.title2.weight(.semibold)).multilineTextAlignment(.leading).lineLimit(2)
                        Spacer(minLength: 4)
                        Image(systemName: "info.circle.fill").font(.title2)
                    }
                    .foregroundStyle(.white)
                    .padding(.horizontal, 16).padding(.vertical, 9)
                    .background(.black.opacity(0.62))
                }
            }
            .buttonStyle(.plain)
            .accessibilityLabel(positionLabel)
            .accessibilityHint(UIStrings.text("openArticle", locale: .current))

            if article.demonstrations.isEmpty {
                Color.clear.frame(height: 64)
            } else {
                DemonstrationSegments(article: article, model: model)
            }
        }
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 14))
        .clipShape(RoundedRectangle(cornerRadius: 14))
    }
}

private struct GalleryImage: View {
    let path: String?
    var body: some View {
        Group {
            if let path,
               let root = Bundle.main.resourceURL?.appendingPathComponent("articles", isDirectory: true),
               let image = UIImage(contentsOfFile: root.appendingPathComponent(path).path) {
                Image(uiImage: image).resizable().scaledToFill()
            } else {
                ZStack {
                    Color(uiColor: .secondarySystemBackground)
                    Image(systemName: "doc.richtext").font(.system(size: 48)).foregroundStyle(.secondary)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .clipped()
        .accessibilityHidden(true)
    }
}

struct DemonstrationSegments: View {
    let article: CatalogArticle
    @Bindable var model: SimulatorModel

    var body: some View {
        HStack(spacing: 4) {
            ForEach(article.demonstrations) { demo in
                let isSelected = model.session.selectedDemonstration(articleID: article.id) == demo.id
                Button {
                    model.selectDemonstration(articleID: article.id, demonstrationID: demo.id)
                } label: {
                    Text(label(for: demo, selected: isSelected))
                        .font(.subheadline.weight(.medium)).lineLimit(2)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .background(isSelected ? Color.accentColor : Color(uiColor: .tertiarySystemFill))
                        .foregroundStyle(isSelected ? Color.white : Color.primary)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                }
                .buttonStyle(.plain)
                .accessibilityAddTraits(isSelected ? .isSelected : [])
            }
        }
        .padding(.horizontal, 8).padding(.vertical, 6)
        .frame(height: 64)
    }

    private func label(for demo: CatalogDemonstration, selected: Bool) -> String {
        demo.label.caseInsensitiveCompare(article.title) == .orderedSame
            ? UIStrings.text(selected ? "deactivate" : "activate", locale: .current)
            : demo.label
    }
}

private struct ArticleIndex: View {
    let catalog: Catalog
    @Bindable var model: SimulatorModel
    private var activePresets: Set<String> { model.session.activePresets(in: catalog) }

    var body: some View {
        HStack(spacing: 3) {
            ForEach(catalog.articles) { article in
                let current = article.id == model.viewedArticleID
                let active = article.demonstrations.contains { demo in demo.presets.contains { activePresets.contains($0) } }
                Capsule()
                    .fill(active ? Color.accentColor : current ? Color.secondary : Color(uiColor: .separator))
                    .frame(maxWidth: .infinity)
                    .frame(height: current ? 8 : active ? 6 : 3)
            }
        }
        .frame(height: 8)
        .padding(.horizontal, 12)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(UIStrings.text("articlePosition", locale: .current))
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
                if model.session.currentLayer().manual[setting.id] != nil {
                    Button { model.reset(setting.id) } label: { Image(systemName: "arrow.counterclockwise") }
                        .accessibilityLabel(UIStrings.text("reset", locale: .current))
                } else if let article = model.sourceArticle(for: setting.id) {
                    Button { model.selectedArticle = article } label: { Image(systemName: "info.circle") }
                        .accessibilityLabel(UIStrings.text("explainingArticle", locale: .current))
                }
                control
            }
            if !setting.help.isEmpty { Text(setting.help).font(.caption).foregroundStyle(.secondary) }
        }
    }
    @ViewBuilder private var control: some View {
        switch setting.control.kind {
        case "boolean": Toggle("", isOn: Binding(get: { value.bool ?? false }, set: { model.edit(setting.id, value: .bool($0)) })).labelsHidden()
        case "number":
            let number = value.number ?? 0, step = setting.control.step ?? 1
            HStack(spacing: 4) {
                Button { setNumber(number - step) } label: { Image(systemName: "minus.circle") }
                Text(number.formatted(.number.precision(.fractionLength(step < 1 ? 2 : 0))) + (setting.unit.map { " \($0)" } ?? "")).monospacedDigit().frame(minWidth: 65)
                Button { setNumber(number + step) } label: { Image(systemName: "plus.circle") }
            }
        case "choice":
            Picker(setting.label, selection: Binding(get: { Int(value.number ?? 0) }, set: { model.edit(setting.id, value: .number(Double($0))) })) {
                ForEach(setting.control.choices ?? []) { Text($0.label).tag($0.value) }
            }.labelsHidden().pickerStyle(.menu)
        default: Text(value.description).foregroundStyle(.secondary)
        }
    }
    private func setNumber(_ number: Double) {
        let clamped = min(setting.control.max ?? Double.greatestFiniteMagnitude, max(setting.control.min ?? -Double.greatestFiniteMagnitude, number))
        model.edit(setting.id, value: .number(setting.control.integer == true ? clamped.rounded() : clamped))
    }
}
