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
    content_path: String,
    demonstrations: Vec<Demonstration>,
}

fn main() {
    println!("cargo:rerun-if-changed=articles");
    let root = PathBuf::from("articles");
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
            content_path,
            demonstrations: metadata.demonstrations,
        });
    }

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
