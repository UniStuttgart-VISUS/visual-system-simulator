import SwiftUI
import WebKit

struct ArticleWebView: UIViewRepresentable {
    let path: String
    func makeUIView(context: Context) -> WKWebView { WKWebView() }
    func updateUIView(_ webView: WKWebView, context: Context) {
        guard let root = Bundle.main.resourceURL?.appendingPathComponent("articles", isDirectory: true) else { return }
        let file = root.appendingPathComponent(path)
        if webView.url != file { webView.loadFileURL(file, allowingReadAccessTo: root) }
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
                    ScrollView(.horizontal) {
                        HStack { ForEach(article.demonstrations) { demo in
                            Button(demo.label) { model.pendingDemo = demo; dismiss() }.buttonStyle(.borderedProminent)
                        } }.padding()
                    }
                }
            }
            .navigationTitle(article.title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button(UIStrings.text("dismiss", locale: .current)) { dismiss() } } }
        }
    }
}
