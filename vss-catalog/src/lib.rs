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
    pub id: &'static str,
    pub label: String,
    pub values: BTreeMap<&'static str, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Article {
    pub id: String,
    pub locale: String,
    pub title: String,
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
    let presets = vec![
        layer(
            "cataract-light",
            tr("presets.cataract_light", locale),
            [
                ("cataract.enabled", json!(true)),
                ("cataract.blur", json!(25.0)),
                ("cataract.contrast", json!(20.0)),
            ],
        ),
        layer(
            "cataract-strong",
            tr("presets.cataract_strong", locale),
            [
                ("cataract.enabled", json!(true)),
                ("cataract.blur", json!(75.0)),
                ("cataract.contrast", json!(65.0)),
            ],
        ),
        layer(
            "protanopia",
            tr("presets.protanopia", locale),
            [
                ("color.enabled", json!(true)),
                ("color.strength", json!(1.0)),
                ("color.type", json!(0)),
            ],
        ),
        layer(
            "glaucoma-moderate",
            tr("presets.glaucoma_moderate", locale),
            [
                ("glaucoma.enabled", json!(true)),
                ("glaucoma.field", json!(55)),
            ],
        ),
        layer(
            "achromatopsia",
            tr("presets.achromatopsia", locale),
            [
                ("achromatopsia.enabled", json!(true)),
                ("achromatopsia.intensity", json!(100)),
                ("achromatopsia.blur", json!(25.0)),
            ],
        ),
        layer(
            "night-blindness",
            tr("presets.night_blindness", locale),
            [
                ("nyctalopia.enabled", json!(true)),
                ("nyctalopia.intensity", json!(70)),
            ],
        ),
        layer(
            "macular-moderate",
            tr("presets.macular_moderate", locale),
            [
                ("macular.enabled", json!(true)),
                ("macular.simple", json!(true)),
                ("macular.simple-intensity", json!(55)),
            ],
        ),
    ];
    let requested_locale = if de { "de" } else { "en" };
    let all_articles: Vec<Article> =
        serde_json::from_slice(include_bytes!(concat!(env!("OUT_DIR"), "/articles.json")))
            .expect("compiled articles are valid");
    let article_ids: BTreeSet<String> = all_articles
        .iter()
        .map(|article| article.id.clone())
        .collect();
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
    let preset_ids: BTreeSet<&str> = presets.iter().map(|preset| preset.id).collect();
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
    Catalog {
        groups,
        presets,
        articles,
    }
}

fn layer<const N: usize>(
    id: &'static str,
    label: String,
    values: [(&'static str, Value); N],
) -> Preset {
    Preset {
        id,
        label,
        values: values.into_iter().collect(),
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
    let mut result = Map::new();
    for group in &catalog.groups {
        for p in &group.settings {
            result.insert(p.id.into(), p.default.clone());
        }
    }
    for id in active {
        if let Some(layer) = catalog.presets.iter().find(|p| p.id == id) {
            for (k, v) in &layer.values {
                result.insert((*k).into(), v.clone());
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
                .map(|preset| preset.id)
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
                    .find(|setting| setting.id == *id)
                    .expect("preset setting exists");
                assert!(
                    setting.control.accepts(value),
                    "invalid preset value for {id}"
                );
                assert!(ids.contains(id));
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
            catalog.presets.iter().map(|preset| preset.id).collect()
        );
    }

    #[test]
    fn all_asset_configs_use_valid_v2_settings() {
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
        visit(std::path::Path::new("../assets/configs"), &mut files);
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
        let active = vec!["cataract-light".into(), "cataract-strong".into()];
        let mut manual = Map::new();
        manual.insert("cataract.blur".into(), json!(10.0));
        let values = compose(Locale::En, &active, &manual);
        assert_eq!(values["cataract.blur"], json!(10.0));
        assert_eq!(values["cataract.contrast"], json!(65.0));
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
        assert_eq!(english.articles.len(), 8);
        assert_eq!(german.articles.len(), 8);
        let cataract = german
            .articles
            .iter()
            .find(|article| article.id == "cataract")
            .unwrap();
        assert_eq!(cataract.content_path, "cataract/cataract_de.html");
        let html = std::fs::read_to_string("articles/cataract/cataract_de.html").unwrap();
        assert!(html.contains("<h2>Ursachen und Entstehung</h2>"));
        assert!(html.contains("cataract/images/katarakt-schwach.png"));
        assert_eq!(cataract.demonstrations[0].presets, vec!["cataract-light"]);
    }
}
