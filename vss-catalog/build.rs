use comrak::{markdown_to_html, Options};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
struct FrontMatter {
    id: String,
    locale: String,
    title: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    demonstrations: Vec<Demonstration>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Demonstration {
    id: String,
    label: String,
    presets: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CompiledArticle {
    id: String,
    locale: String,
    title: String,
    summary: Option<String>,
    image: Option<String>,
    content_path: String,
    demonstrations: Vec<Demonstration>,
}

fn main() {
    println!("cargo:rerun-if-changed=articles");
    let root = PathBuf::from("articles");
    let index_path = root.join("index.toml");
    let article_order: ArticleIndex = toml::from_str(
        &fs::read_to_string(&index_path)
            .unwrap_or_else(|err| panic!("Cannot read {}: {err}", index_path.display())),
    )
    .unwrap_or_else(|err| panic!("Invalid article index {}: {err}", index_path.display()));
    validate_index_shape(&article_order.articles);
    let mut markdown = Vec::new();
    collect_markdown(&root, &mut markdown);
    markdown.sort();

    let mut articles = Vec::new();
    let mut variants = BTreeSet::new();
    let mut demos_by_article: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    for path in markdown {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("Cannot read {}: {err}", path.display()));
        let (front, body) = split_front_matter(&source, &path);
        assert!(
            !body.contains(".html)"),
            "Obsolete HTML link in {}",
            path.display()
        );
        let metadata: FrontMatter = serde_yml::from_str(front)
            .unwrap_or_else(|err| panic!("Invalid front matter in {}: {err}", path.display()));
        validate_metadata(&metadata, &path);
        let parent = path.parent().expect("article has parent");
        validate_images(body, parent, &path);
        let variant = (metadata.id.clone(), metadata.locale.clone());
        assert!(
            variants.insert(variant),
            "Duplicate article locale in {}",
            path.display()
        );

        let demo_shapes = demos_by_article.entry(metadata.id.clone()).or_default();
        for demo in &metadata.demonstrations {
            let shape = demo_shapes
                .entry(demo.id.clone())
                .or_insert_with(|| demo.presets.clone());
            assert_eq!(
                shape, &demo.presets,
                "Demonstration '{}' differs between locales",
                demo.id
            );
        }

        let asset_base = parent
            .file_name()
            .and_then(|value| value.to_str())
            .expect("article directory is UTF-8");
        let normalized = normalize_legacy_markdown(body, asset_base);
        let mut options = Options::default();
        options.extension.strikethrough = true;
        options.extension.table = true;
        options.extension.autolink = true;
        options.extension.tasklist = true;
        options.extension.footnotes = true;
        options.parse.smart = true;
        options.render.r#unsafe = false;
        let html = markdown_to_html(&normalized, &options);
        let html_path = path.with_extension("html");
        write_if_changed(&html_path, html.as_bytes());
        let content_path = html_path
            .strip_prefix(&root)
            .expect("article lives below articles")
            .to_string_lossy()
            .replace('\\', "/");
        articles.push(CompiledArticle {
            id: metadata.id,
            locale: metadata.locale,
            title: metadata.title,
            summary: metadata.summary,
            image: metadata.image,
            content_path,
            demonstrations: metadata.demonstrations,
        });
    }

    let known_ids: BTreeSet<_> = articles.iter().map(|article| article.id.as_str()).collect();
    let indexed_ids: BTreeSet<_> = article_order.articles.iter().map(String::as_str).collect();
    let missing: Vec<_> = known_ids.difference(&indexed_ids).copied().collect();
    let unknown: Vec<_> = indexed_ids.difference(&known_ids).copied().collect();
    assert!(
        missing.is_empty(),
        "Article index is missing: {}",
        missing.join(", ")
    );
    assert!(
        unknown.is_empty(),
        "Article index contains unknown ids: {}",
        unknown.join(", ")
    );
    let positions: BTreeMap<_, _> = article_order
        .articles
        .iter()
        .enumerate()
        .map(|(position, id)| (id.as_str(), position))
        .collect();
    articles.sort_by_key(|article| (positions[article.id.as_str()], article.locale.clone()));

    for id in articles
        .iter()
        .map(|article| &article.id)
        .collect::<BTreeSet<_>>()
    {
        assert!(
            articles
                .iter()
                .any(|article| &article.id == id && article.locale == "en"),
            "Article '{id}' has no English fallback"
        );
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set")).join("articles.json");
    fs::write(
        &out,
        serde_json::to_vec(&articles).expect("articles serialize"),
    )
    .expect("articles.json is written");
}

#[derive(Debug, Deserialize)]
struct ArticleIndex {
    articles: Vec<String>,
}

fn validate_index_shape(ids: &[String]) {
    let unique: BTreeSet<_> = ids.iter().collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "Article index contains duplicate ids"
    );
}

fn write_if_changed(path: &Path, content: &[u8]) {
    if fs::read(path).ok().as_deref() != Some(content) {
        fs::write(path, content)
            .unwrap_or_else(|err| panic!("Cannot write {}: {err}", path.display()));
    }
}

fn collect_markdown(dir: &Path, output: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(dir).unwrap_or_else(|err| panic!("Cannot read {}: {err}", dir.display()))
    {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            collect_markdown(&path, output);
        } else if path.extension().and_then(|value| value.to_str()) == Some("md") {
            output.push(path);
        }
    }
}

fn split_front_matter<'a>(source: &'a str, path: &Path) -> (&'a str, &'a str) {
    let source = source
        .strip_prefix("---")
        .unwrap_or_else(|| panic!("{} has no front matter", path.display()));
    let end = source
        .find("\n---")
        .unwrap_or_else(|| panic!("{} has unterminated front matter", path.display()));
    (
        &source[..end],
        source[end + 4..].trim_start_matches(['\r', '\n']),
    )
}

fn validate_metadata(metadata: &FrontMatter, path: &Path) {
    assert!(
        !metadata.id.is_empty()
            && metadata
                .id
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-'),
        "Invalid article id in {}",
        path.display()
    );
    assert!(
        matches!(metadata.locale.as_str(), "en" | "de"),
        "Unsupported locale in {}",
        path.display()
    );
    assert!(
        !metadata.title.trim().is_empty(),
        "Missing title in {}",
        path.display()
    );
    let mut ids = BTreeSet::new();
    for demo in &metadata.demonstrations {
        assert!(
            ids.insert(&demo.id),
            "Duplicate demonstration '{}' in {}",
            demo.id,
            path.display()
        );
        assert!(
            !demo.presets.is_empty(),
            "Demonstration '{}' has no presets",
            demo.id
        );
    }
}

fn validate_images(markdown: &str, parent: &Path, path: &Path) {
    let mut rest = markdown;
    while let Some(start) = rest.find("](") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find(')') else { break };
        let target = &rest[..end];
        if target.starts_with("images/") {
            let image = parent.join(target);
            assert!(
                image.is_file(),
                "Missing image '{}' referenced by {}",
                image.display(),
                path.display()
            );
        }
        rest = &rest[end + 1..];
    }
}

fn normalize_legacy_markdown(source: &str, asset_base: &str) -> String {
    source
        .replace("<br><br>", "\n\n")
        .replace("<br>", "\n")
        .replace("](images/", &format!("]({asset_base}/images/"))
}
