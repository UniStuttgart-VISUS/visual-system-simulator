import SwiftUI
import WebKit

struct ArticleWebView: UIViewRepresentable {
    let path: String

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeUIView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.defaultWebpagePreferences.allowsContentJavaScript = false
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.isOpaque = false
        webView.backgroundColor = .clear
        webView.scrollView.backgroundColor = .clear
        webView.scrollView.contentInsetAdjustmentBehavior = .never
        return webView
    }

    func updateUIView(_ webView: WKWebView, context: Context) {
        guard let root = Bundle.main.resourceURL?.appendingPathComponent("articles", isDirectory: true) else { return }
        let file = root.appendingPathComponent(path)
        guard context.coordinator.loadedPath != path,
              let fragment = try? String(contentsOf: file, encoding: .utf8) else { return }
        context.coordinator.loadedPath = path
        webView.loadHTMLString(Self.document(containing: fragment), baseURL: root)
    }

    final class Coordinator {
        var loadedPath: String?
    }

    private static func document(containing article: String) -> String {
        """
        <!doctype html>
        <html>
        <head>
          <meta charset="utf-8">
          <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
          <style>
            :root { color-scheme: light dark; font: -apple-system-body; }
            html { -webkit-text-size-adjust: 100%; }
            body {
              box-sizing: border-box;
              margin: 0;
              padding: 20px 20px 32px;
              background: transparent;
              color: #1c1c1e;
              font-family: -apple-system, BlinkMacSystemFont, sans-serif;
              font-size: 17px;
              line-height: 1.5;
              overflow-wrap: anywhere;
            }
            h1, h2, h3 { line-height: 1.2; text-wrap: balance; }
            h2 { margin: 1.6em 0 .55em; font-size: 1.35em; }
            h3 { margin: 1.4em 0 .5em; font-size: 1.15em; }
            p { margin: 0 0 1em; }
            ul, ol { margin: 0 0 1em; padding-left: 1.4em; }
            li { margin: .35em 0; }
            img {
              display: block;
              width: auto;
              max-width: 100%;
              height: auto;
              margin: .25em auto 1.4em;
              border-radius: 12px;
            }
            a { color: #0a84ff; }
            table { display: block; max-width: 100%; overflow-x: auto; border-collapse: collapse; }
            th, td { padding: .45em .6em; border-bottom: 1px solid #d1d1d6; }
            blockquote {
              margin: 1em 0;
              padding-left: 1em;
              border-left: 3px solid #8e8e93;
              color: #636366;
            }
            @media (prefers-color-scheme: dark) {
              body { color: #f2f2f7; }
              th, td { border-bottom-color: #48484a; }
              blockquote { color: #aeaeb2; }
            }
          </style>
        </head>
        <body>\(article)</body>
        </html>
        """
    }
}

struct ArticleSheet: View {
    let article: CatalogArticle
    @Bindable var model: SimulatorModel
    @Environment(\.dismiss) private var dismiss
    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                ArticleWebView(path: article.contentPath)
                if !article.demonstrations.isEmpty {
                    Divider()
                    HStack(spacing: 4) {
                        ForEach(article.demonstrations) { demo in
                            let selected = model.session.selectedDemonstrations[article.id] == demo.id
                            Button {
                                model.selectDemonstration(articleID: article.id, demonstrationID: demo.id)
                                dismiss()
                            } label: {
                                Text(label(for: demo, selected: selected))
                                    .font(.subheadline.weight(.medium)).lineLimit(2)
                                    .frame(maxWidth: .infinity, minHeight: 44)
                                    .background(selected ? Color.accentColor : Color(uiColor: .tertiarySystemFill))
                                    .foregroundStyle(selected ? Color.white : Color.primary)
                                    .clipShape(RoundedRectangle(cornerRadius: 8))
                            }
                            .buttonStyle(.plain)
                            .accessibilityAddTraits(selected ? .isSelected : [])
                        }
                    }
                    .padding(12)
                }
            }
            .navigationTitle(article.title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button(UIStrings.text("dismiss", locale: .current)) { dismiss() } } }
        }
        .presentationDetents([.large])
        .presentationDragIndicator(.visible)
    }

    private func label(for demo: CatalogDemonstration, selected: Bool) -> String {
        demo.label.caseInsensitiveCompare(article.title) == .orderedSame
            ? UIStrings.text(selected ? "deactivate" : "activate", locale: .current)
            : demo.label
    }
}
