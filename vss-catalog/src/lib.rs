use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use vss::registry;

rust_i18n::i18n!("locales", fallback = "en");

mod config;
pub use config::*;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    En,
    De,
}

impl Locale {
    pub fn from_tag(tag: &str) -> Self {
        if tag.to_ascii_lowercase().starts_with("de") {
            Self::De
        } else {
            Self::En
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Catalog {
    pub groups: Vec<Group>,
    pub presets: Vec<Preset>,
    pub articles: Vec<Article>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Group {
    pub id: &'static str,
    pub title: String,
    pub settings: Vec<Setting>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Setting {
    pub id: &'static str,
    pub label: String,
    pub help: String,
    pub default: Value,
    pub control: Control,
    pub unit: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Control {
    Boolean,
    Number {
        integer: bool,
        min: Option<f64>,
        max: Option<f64>,
        step: f64,
    },
    Choice {
        choices: Vec<Choice>,
    },
    Text,
    Point,
}

#[derive(Clone, Debug, Serialize)]
pub struct Choice {
    pub value: i32,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Preset {
    pub id: String,
    pub label: String,
    pub values: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct CompiledPreset {
    id: String,
    values: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Article {
    pub id: String,
    pub locale: String,
    pub title: String,
    pub summary: Option<String>,
    pub image: Option<String>,
    pub content_path: String,
    pub demonstrations: Vec<Demonstration>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Demonstration {
    pub id: String,
    pub label: String,
    pub presets: Vec<String>,
}

pub fn catalog(locale: Locale) -> Catalog {
    let de = matches!(locale, Locale::De);
    let locale = if de { "de" } else { "en" };
    let cataract = vec![
        Setting {
            id: "cataract.enabled",
            label: rust_i18n::t!("settings.enabled", locale = locale).into_owned(),
            help: rust_i18n::t!("settings.cataract.enabled_help", locale = locale).into_owned(),
            default: json!(false),
            control: Control::Boolean,
            unit: None,
        },
        Setting {
            id: "cataract.blur",
            label: rust_i18n::t!("settings.blur", locale = locale).into_owned(),
            help: rust_i18n::t!("settings.cataract.blur_help", locale = locale).into_owned(),
            default: json!(0.0),
            control: Control::Number {
                integer: false,
                min: Some(0.0),
                max: Some(100.0),
                step: 1.0,
            },
            unit: Some("%"),
        },
        Setting {
            id: "cataract.contrast",
            label: rust_i18n::t!("settings.contrast_loss", locale = locale).into_owned(),
            help: rust_i18n::t!("settings.cataract.contrast_help", locale = locale).into_owned(),
            default: json!(0.0),
            control: Control::Number {
                integer: false,
                min: Some(0.0),
                max: Some(100.0),
                step: 1.0,
            },
            unit: Some("%"),
        },
    ];
    let color = vec![
        Setting {
            id: "color.enabled",
            label: rust_i18n::t!("settings.enabled", locale = locale).into_owned(),
            help: rust_i18n::t!("settings.color.enabled_help", locale = locale).into_owned(),
            default: json!(false),
            control: Control::Boolean,
            unit: None,
        },
        Setting {
            id: "color.strength",
            label: rust_i18n::t!("settings.strength", locale = locale).into_owned(),
            help: rust_i18n::t!("settings.color.strength_help", locale = locale).into_owned(),
            default: json!(0.0),
            control: Control::Number {
                integer: false,
                min: Some(0.0),
                max: Some(1.0),
                step: 0.05,
            },
            unit: Some("%"),
        },
        Setting {
            id: "color.type",
            label: rust_i18n::t!("settings.type", locale = locale).into_owned(),
            help: rust_i18n::t!("settings.color.type_help", locale = locale).into_owned(),
            default: json!(0),
            control: Control::Choice {
                choices: vec![
                    Choice {
                        value: 0,
                        label: rust_i18n::t!("choices.red", locale = locale).into_owned(),
                    },
                    Choice {
                        value: 1,
                        label: rust_i18n::t!("choices.green", locale = locale).into_owned(),
                    },
                    Choice {
                        value: 2,
                        label: rust_i18n::t!("choices.blue", locale = locale).into_owned(),
                    },
                    Choice {
                        value: 3,
                        label: rust_i18n::t!("choices.monochrome", locale = locale).into_owned(),
                    },
                ],
            },
            unit: None,
        },
    ];
    let glaucoma = vec![
        boolean("glaucoma.enabled", tr("settings.enabled", locale)),
        number(
            "glaucoma.field",
            tr("settings.field_loss", locale),
            true,
            Some("%"),
        ),
    ];
    let achromatopsia = vec![
        boolean("achromatopsia.enabled", tr("settings.enabled", locale)),
        number(
            "achromatopsia.intensity",
            tr("settings.intensity", locale),
            true,
            Some("%"),
        ),
        number(
            "achromatopsia.blur",
            tr("settings.blur", locale),
            false,
            Some("%"),
        ),
    ];
    let nyctalopia = vec![
        boolean("nyctalopia.enabled", tr("settings.enabled", locale)),
        number(
            "nyctalopia.intensity",
            tr("settings.intensity", locale),
            true,
            Some("%"),
        ),
    ];
    let macular = vec![
        boolean("macular.enabled", tr("settings.enabled", locale)),
        boolean("macular.simple", tr("settings.simple_model", locale)),
        number(
            "macular.simple-intensity",
            tr("settings.severity", locale),
            true,
            Some("%"),
        ),
        boolean("macular.advanced", tr("settings.advanced_model", locale)),
        number(
            "macular.radius",
            tr("settings.radius", locale),
            false,
            Some("°"),
        ),
        number(
            "macular.intensity",
            tr("settings.intensity", locale),
            false,
            Some("%"),
        ),
    ];
    let eye = vec![
        number(
            "eye.axis-x",
            tr("settings.axis_x", locale),
            false,
            Some("rad"),
        ),
        number(
            "eye.axis-y",
            tr("settings.axis_y", locale),
            false,
            Some("rad"),
        ),
        number(
            "eye.center-distance",
            tr("settings.center_distance", locale),
            false,
            None,
        ),
        boolean("eye.refraction-enabled", tr("settings.refraction", locale)),
        number(
            "eye.refraction-diopters",
            tr("settings.refraction_power", locale),
            false,
            Some("dpt"),
        ),
        boolean("eye.presbyopia-enabled", tr("settings.presbyopia", locale)),
        number(
            "eye.near-point",
            tr("settings.near_point", locale),
            false,
            None,
        ),
        number(
            "eye.astigmatism-diopters",
            tr("settings.astigmatism", locale),
            false,
            Some("dpt"),
        ),
        number(
            "eye.astigmatism-angle",
            tr("settings.astigmatism_angle", locale),
            false,
            Some("°"),
        ),
    ];
    let retina = vec![
        boolean(
            "retina.color-deficiency-enabled",
            tr("settings.color_deficiency", locale),
        ),
        number(
            "retina.color-deficiency-type",
            tr("settings.color_deficiency_type", locale),
            true,
            None,
        ),
        number(
            "retina.color-deficiency-intensity",
            tr("settings.color_deficiency_intensity", locale),
            true,
            Some("%"),
        ),
        boolean(
            "retina.receptor-density-enabled",
            tr("settings.receptor_density", locale),
        ),
        asset("retina.map-pos-x", tr("settings.retina_map_pos_x", locale)),
        asset("retina.map-neg-x", tr("settings.retina_map_neg_x", locale)),
        asset("retina.map-pos-y", tr("settings.retina_map_pos_y", locale)),
        asset("retina.map-neg-y", tr("settings.retina_map_neg_y", locale)),
        asset("retina.map-pos-z", tr("settings.retina_map_pos_z", locale)),
        asset("retina.map-neg-z", tr("settings.retina_map_neg_z", locale)),
    ];
    let pose = vec![
        point("gaze", tr("settings.gaze", locale)),
        point("view", tr("settings.view", locale)),
    ];
    let groups = vec![
        Group {
            id: "cataract",
            title: tr("groups.cataract", locale),
            settings: cataract,
        },
        Group {
            id: "color",
            title: tr("groups.color", locale),
            settings: color,
        },
        Group {
            id: "glaucoma",
            title: tr("groups.glaucoma", locale),
            settings: glaucoma,
        },
        Group {
            id: "achromatopsia",
            title: tr("groups.achromatopsia", locale),
            settings: achromatopsia,
        },
        Group {
            id: "nyctalopia",
            title: tr("groups.nyctalopia", locale),
            settings: nyctalopia,
        },
        Group {
            id: "macular",
            title: tr("groups.macular", locale),
            settings: macular,
        },
        Group {
            id: "eye",
            title: tr("groups.eye", locale),
            settings: eye,
        },
        Group {
            id: "retina",
            title: tr("groups.retina", locale),
            settings: retina,
        },
        Group {
            id: "pose",
            title: tr("groups.pose", locale),
            settings: pose,
        },
    ];
    let compiled_presets: Vec<CompiledPreset> =
        serde_json::from_slice(include_bytes!(concat!(env!("OUT_DIR"), "/presets.json")))
            .expect("compiled presets are valid");
    let requested_locale = if de { "de" } else { "en" };
    let all_articles: Vec<Article> =
        serde_json::from_slice(include_bytes!(concat!(env!("OUT_DIR"), "/articles.json")))
            .expect("compiled articles are valid");
    let article_ids: Vec<String> = all_articles.iter().fold(Vec::new(), |mut ids, article| {
        if !ids.contains(&article.id) {
            ids.push(article.id.clone());
        }
        ids
    });
    let articles: Vec<Article> = article_ids
        .into_iter()
        .filter_map(|id| {
            all_articles
                .iter()
                .find(|article| article.id == id && article.locale == requested_locale)
                .or_else(|| {
                    all_articles
                        .iter()
                        .find(|article| article.id == id && article.locale == "en")
                })
                .cloned()
        })
        .collect();
    let presets: Vec<Preset> = compiled_presets
        .into_iter()
        .map(|preset| {
            let label = articles
                .iter()
                .flat_map(|article| &article.demonstrations)
                .find(|demo| demo.presets.contains(&preset.id))
                .unwrap_or_else(|| panic!("Preset '{}' has no localized demonstration", preset.id))
                .label
                .clone();
            Preset {
                id: preset.id,
                label,
                values: preset.values,
            }
        })
        .collect();
    let preset_ids: BTreeSet<&str> = presets.iter().map(|preset| preset.id.as_str()).collect();
    for article in &articles {
        for demonstration in &article.demonstrations {
            for preset in &demonstration.presets {
                assert!(
                    preset_ids.contains(preset.as_str()),
                    "Article '{}' references unknown preset '{}'",
                    article.id,
                    preset
                );
            }
        }
    }
    validate_preset_ownership(&articles, &preset_ids).unwrap_or_else(|message| panic!("{message}"));
    Catalog {
        groups,
        presets,
        articles,
    }
}

fn validate_preset_ownership(
    articles: &[Article],
    preset_ids: &BTreeSet<&str>,
) -> Result<(), String> {
    let mut owners = BTreeMap::<&str, &str>::new();
    for article in articles {
        for preset in article.demonstrations.iter().flat_map(|demo| &demo.presets) {
            if let Some(owner) = owners.insert(preset.as_str(), article.id.as_str()) {
                if owner != article.id {
                    return Err(format!(
                        "Preset '{preset}' belongs to both articles '{owner}' and '{}'",
                        article.id
                    ));
                }
            }
        }
    }
    let unowned: Vec<_> = preset_ids
        .iter()
        .copied()
        .filter(|id| !owners.contains_key(id))
        .collect();
    if unowned.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Presets without an article demonstration: {}",
            unowned.join(", ")
        ))
    }
}

fn boolean(id: &'static str, label: String) -> Setting {
    Setting {
        id,
        label,
        help: String::new(),
        default: json!(false),
        control: Control::Boolean,
        unit: None,
    }
}

fn number(id: &'static str, label: String, integer: bool, unit: Option<&'static str>) -> Setting {
    Setting {
        id,
        label,
        help: String::new(),
        default: if integer { json!(0) } else { json!(0.0) },
        control: Control::Number {
            integer,
            min: None,
            max: None,
            step: 1.0,
        },
        unit,
    }
}

pub fn compose(
    locale: Locale,
    active: &[String],
    manual: &Map<String, Value>,
) -> Map<String, Value> {
    let catalog = catalog(locale);
    let active: BTreeSet<&str> = active.iter().map(String::as_str).collect();
    let mut result = Map::new();
    for group in &catalog.groups {
        for p in &group.settings {
            result.insert(p.id.into(), p.default.clone());
        }
    }
    for layer in &catalog.presets {
        if active.contains(layer.id.as_str()) {
            for (k, v) in &layer.values {
                result.insert(k.clone(), v.clone());
            }
        }
    }
    for (k, v) in manual {
        result.insert(k.clone(), v.clone());
    }
    normalize(locale, &mut result);
    result
}

pub fn normalize(locale: Locale, values: &mut Map<String, Value>) {
    for p in catalog(locale).groups.into_iter().flat_map(|g| g.settings) {
        let Some(value) = values.get_mut(p.id) else {
            continue;
        };
        match p.control {
            Control::Boolean => {
                if !value.is_boolean() {
                    *value = p.default
                }
            }
            Control::Number {
                integer, min, max, ..
            } => {
                let Some(mut n) = value.as_f64() else {
                    *value = p.default;
                    continue;
                };
                if let Some(min) = min {
                    n = n.max(min);
                }
                if let Some(max) = max {
                    n = n.min(max);
                }
                *value = if integer {
                    json!(n.round() as i64)
                } else {
                    json!(n)
                };
            }
            Control::Choice { choices } => {
                let allowed: BTreeSet<i64> = choices.into_iter().map(|c| c.value as i64).collect();
                if value.as_i64().is_none_or(|v| !allowed.contains(&v)) {
                    *value = p.default;
                }
            }
            Control::Text => {
                if !(value.is_string() || value.is_null()) {
                    *value = p.default;
                }
            }
            Control::Point => {
                if value.as_array().is_none_or(|values| {
                    values.len() != 2 || values.iter().any(|value| !value.is_number())
                }) {
                    *value = p.default;
                }
            }
        }
    }
}

pub fn contract_json(locale: Locale) -> String {
    serde_json::to_string(&catalog(locale)).expect("static catalog serializes")
}

fn asset(id: &'static str, label: String) -> Setting {
    Setting {
        id,
        label,
        help: String::new(),
        default: Value::Null,
        control: Control::Text,
        unit: None,
    }
}

fn point(id: &'static str, label: String) -> Setting {
    Setting {
        id,
        label,
        help: String::new(),
        default: json!([0.0, 0.0]),
        control: Control::Point,
        unit: None,
    }
}

fn tr(key: &str, locale: &str) -> String {
    rust_i18n::t!(key, locale = locale).into_owned()
}

pub fn validate_settings(values: &BTreeMap<String, ConfigValue>) -> Result<(), Vec<Diagnostic>> {
    let settings: BTreeMap<_, _> = catalog(Locale::En)
        .groups
        .into_iter()
        .flat_map(|group| group.settings)
        .map(|setting| (setting.id, setting))
        .collect();
    let parameters: BTreeSet<_> = registry().iter().map(|parameter| parameter.id()).collect();
    let mut diagnostics = Vec::new();
    for (id, value) in values {
        let Some(setting) = settings.get(id.as_str()) else {
            diagnostics.push(Diagnostic {
                source: value.source.clone(),
                path: value.path.clone(),
                message: rust_i18n::t!("diagnostics.unknown_setting", id = id).into_owned(),
            });
            continue;
        };
        if !setting.control.accepts(&value.value) {
            diagnostics.push(Diagnostic {
                source: value.source.clone(),
                path: value.path.clone(),
                message: rust_i18n::t!("diagnostics.invalid_setting_value", id = id).into_owned(),
            });
            continue;
        }
        if !parameters.contains(id.as_str()) {
            diagnostics.push(Diagnostic {
                source: value.source.clone(),
                path: value.path.clone(),
                message: rust_i18n::t!("diagnostics.unknown_setting", id = id).into_owned(),
            });
        }
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

impl Control {
    fn accepts(&self, value: &Value) -> bool {
        match self {
            Self::Boolean => value.is_boolean(),
            Self::Number {
                integer, min, max, ..
            } => value.as_f64().is_some_and(|number| {
                (!integer || value.as_i64().is_some())
                    && min.is_none_or(|min| number >= min)
                    && max.is_none_or(|max| number <= max)
            }),
            Self::Choice { choices } => value
                .as_i64()
                .is_some_and(|value| choices.iter().any(|choice| choice.value as i64 == value)),
            Self::Text => value.is_string() || value.is_null(),
            Self::Point => value
                .as_array()
                .is_some_and(|values| values.len() == 2 && values.iter().all(Value::is_number)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contract_is_consistent() {
        let catalog = catalog(Locale::En);
        let settings: Vec<_> = catalog
            .groups
            .iter()
            .flat_map(|group| &group.settings)
            .collect();
        assert_eq!(
            settings
                .iter()
                .map(|setting| setting.id)
                .collect::<BTreeSet<_>>()
                .len(),
            settings.len()
        );
        assert_eq!(
            catalog
                .presets
                .iter()
                .map(|preset| preset.id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            catalog.presets.len()
        );
        let ids: BTreeSet<_> = settings.iter().map(|setting| setting.id).collect();
        let parameter_ids: BTreeSet<_> =
            registry().iter().map(|parameter| parameter.id()).collect();
        assert!(
            ids.iter().all(|id| parameter_ids.contains(id)),
            "every setting targets a VSS parameter"
        );
        for setting in settings {
            assert!(
                setting.control.accepts(&setting.default),
                "invalid default for {}",
                setting.id
            );
        }
        for preset in &catalog.presets {
            for (id, value) in &preset.values {
                let setting = catalog
                    .groups
                    .iter()
                    .flat_map(|group| &group.settings)
                    .find(|setting| setting.id == id.as_str())
                    .expect("preset setting exists");
                assert!(
                    setting.control.accepts(value),
                    "invalid preset value for {id}"
                );
                assert!(ids.contains(id.as_str()));
            }
        }
    }

    #[test]
    fn exported_contract_uses_the_rust_setting_and_preset_ids() {
        let catalog = catalog(Locale::En);
        let contract: Value = serde_json::from_str(&contract_json(Locale::En)).unwrap();
        let setting_ids: BTreeSet<_> = contract["groups"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|group| group["settings"].as_array().unwrap())
            .map(|setting| setting["id"].as_str().unwrap())
            .collect();
        let preset_ids: BTreeSet<_> = contract["presets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|preset| preset["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            setting_ids,
            catalog
                .groups
                .iter()
                .flat_map(|group| &group.settings)
                .map(|setting| setting.id)
                .collect()
        );
        assert_eq!(
            preset_ids,
            catalog
                .presets
                .iter()
                .map(|preset| preset.id.as_str())
                .collect()
        );
    }

    #[test]
    fn all_presets_use_valid_v2_settings() {
        fn visit(path: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    visit(&path, files);
                } else if path
                    .extension()
                    .is_some_and(|extension| extension == "json")
                {
                    files.push(path);
                }
            }
        }
        let mut files = Vec::new();
        visit(std::path::Path::new("presets"), &mut files);
        assert!(!files.is_empty());
        for path in files {
            let text = std::fs::read_to_string(&path).unwrap();
            let (document, diagnostics) = parse_config_str(&path.to_string_lossy(), &text).unwrap();
            assert!(
                diagnostics.is_empty(),
                "{}: {}",
                path.display(),
                diagnostics_to_string(&diagnostics)
            );
            for section in [document.effective_left(), document.effective_right()] {
                validate_settings(&section.value_map()).unwrap_or_else(|diagnostics| {
                    panic!(
                        "{}: {}",
                        path.display(),
                        diagnostics_to_string(&diagnostics)
                    )
                });
            }
        }
    }

    #[test]
    fn later_presets_and_manual_values_win() {
        let active = vec!["cataract-mild".into(), "cataract-severe".into()];
        let mut manual = Map::new();
        manual.insert("cataract.blur".into(), json!(10.0));
        let values = compose(Locale::En, &active, &manual);
        assert_eq!(values["cataract.blur"], json!(10.0));
        assert_eq!(values["cataract.contrast"], json!(70.0));
    }

    #[test]
    fn preset_composition_is_independent_of_click_order() {
        let forward = vec!["cataract-mild".into(), "cataract-severe".into()];
        let reverse = vec!["cataract-severe".into(), "cataract-mild".into()];
        assert_eq!(
            compose(Locale::En, &forward, &Map::new()),
            compose(Locale::En, &reverse, &Map::new())
        );
    }

    #[test]
    fn invalid_values_reject_the_entire_compilation() {
        let values = BTreeMap::from([
            (
                "cataract.enabled".into(),
                ConfigValue {
                    value: json!(true),
                    source: "test".into(),
                    path: "cataract.enabled".into(),
                },
            ),
            (
                "cataract.blur".into(),
                ConfigValue {
                    value: json!("invalid"),
                    source: "test".into(),
                    path: "cataract.blur".into(),
                },
            ),
        ]);
        let diagnostics = validate_settings(&values).unwrap_err();
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn normalization_clamps_and_rejects_invalid_choices() {
        let mut values = Map::from_iter([
            ("cataract.blur".into(), json!(500)),
            ("color.type".into(), json!(99)),
        ]);
        normalize(Locale::En, &mut values);
        assert_eq!(values["cataract.blur"], json!(100.0));
        assert_eq!(values["color.type"], json!(0));
    }

    #[test]
    fn german_and_fallback_locales_are_stable() {
        assert_eq!(
            catalog(Locale::from_tag("de-DE")).groups[0].title,
            "Katarakt"
        );
        assert_eq!(
            catalog(Locale::from_tag("fr-FR")).groups[0].title,
            "Cataract"
        );
    }

    #[test]
    fn markdown_articles_are_compiled_and_localized() {
        let english = catalog(Locale::En);
        let german = catalog(Locale::De);
        assert_eq!(english.articles.len(), 10);
        assert_eq!(german.articles.len(), 10);
        let cataract = german
            .articles
            .iter()
            .find(|article| article.id == "cataract")
            .unwrap();
        assert_eq!(cataract.content_path, "cataract/cataract_de.html");
        let html = std::fs::read_to_string("articles/cataract/cataract_de.html").unwrap();
        assert!(html.contains("<h2>Ursachen und Entstehung</h2>"));
        assert!(html.contains("cataract/images/katarakt-schwach.png"));
        assert_eq!(cataract.demonstrations[0].presets, vec!["cataract-mild"]);
    }

    #[test]
    fn articles_follow_the_editorial_index_and_serialize_card_metadata() {
        let contract: Value = serde_json::from_str(&contract_json(Locale::En)).unwrap();
        let articles = contract["articles"].as_array().unwrap();
        assert_eq!(
            articles
                .iter()
                .map(|article| article["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "ametropia",
                "strabismus",
                "presbyopia",
                "cataract",
                "color-deficiency",
                "achromatopsia",
                "nyctalopia",
                "hemeralopia",
                "glaucoma",
                "macular-degeneration",
            ]
        );
        assert!(articles
            .iter()
            .all(|article| article.get("summary").is_some()));
        assert!(articles
            .iter()
            .all(|article| article.get("image").is_some()));
    }

    #[test]
    fn preset_ownership_is_unique_across_articles_but_reuse_within_one_is_valid() {
        let mut articles = catalog(Locale::En).articles;
        let preset_ids = BTreeSet::from(["cataract-mild"]);
        let cataract = articles
            .iter_mut()
            .find(|article| article.id == "cataract")
            .unwrap();
        cataract
            .demonstrations
            .push(cataract.demonstrations[0].clone());
        assert!(validate_preset_ownership(&articles, &preset_ids).is_ok());
        articles
            .iter_mut()
            .find(|article| article.id == "glaucoma")
            .unwrap()
            .demonstrations
            .push(Demonstration {
                id: "duplicate-owner".into(),
                label: "Duplicate".into(),
                presets: vec!["cataract-mild".into()],
            });
        assert!(validate_preset_ownership(&articles, &preset_ids)
            .unwrap_err()
            .contains("belongs to both articles"));
    }
}
