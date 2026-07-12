use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

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
    pub profiles: Vec<ConfigLayer>,
    pub articles: Vec<Article>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Group {
    pub id: &'static str,
    pub title: &'static str,
    pub parameters: Vec<Parameter>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Parameter {
    pub id: &'static str,
    pub key: &'static str,
    pub label: &'static str,
    pub help: &'static str,
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
}

#[derive(Clone, Debug, Serialize)]
pub struct Choice {
    pub value: i32,
    pub label: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConfigLayer {
    pub id: &'static str,
    pub label: &'static str,
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
    pub profiles: Vec<String>,
}

pub fn catalog(locale: Locale) -> Catalog {
    let de = matches!(locale, Locale::De);
    let cataract = vec![
        Parameter {
            id: "cataract.enabled",
            key: "ct_onoff",
            label: if de { "Aktiv" } else { "Enabled" },
            help: if de {
                "Kataraktsimulation ein- oder ausschalten."
            } else {
                "Enable or disable cataract simulation."
            },
            default: json!(false),
            control: Control::Boolean,
            unit: None,
        },
        Parameter {
            id: "cataract.blur",
            key: "ct_blur_factor",
            label: if de { "Unschärfe" } else { "Blur" },
            help: if de {
                "Stärke der Lichtstreuung."
            } else {
                "Strength of light scattering."
            },
            default: json!(0.0),
            control: Control::Number {
                integer: false,
                min: Some(0.0),
                max: Some(100.0),
                step: 1.0,
            },
            unit: Some("%"),
        },
        Parameter {
            id: "cataract.contrast",
            key: "ct_contrast_factor",
            label: if de {
                "Kontrastverlust"
            } else {
                "Contrast loss"
            },
            help: if de {
                "Verringerung des Bildkontrasts."
            } else {
                "Reduction of image contrast."
            },
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
        Parameter {
            id: "color.enabled",
            key: "peacock_cb_onoff",
            label: if de { "Aktiv" } else { "Enabled" },
            help: if de {
                "Farbsehschwäche simulieren."
            } else {
                "Simulate color vision deficiency."
            },
            default: json!(false),
            control: Control::Boolean,
            unit: None,
        },
        Parameter {
            id: "color.strength",
            key: "peacock_cb_strength",
            label: if de { "Stärke" } else { "Strength" },
            help: if de {
                "Mischstärke des Effekts."
            } else {
                "Effect blend strength."
            },
            default: json!(0.0),
            control: Control::Number {
                integer: false,
                min: Some(0.0),
                max: Some(1.0),
                step: 0.05,
            },
            unit: Some("%"),
        },
        Parameter {
            id: "color.type",
            key: "peacock_cb_type",
            label: if de { "Typ" } else { "Type" },
            help: if de {
                "Betroffener Rezeptortyp."
            } else {
                "Affected receptor type."
            },
            default: json!(0),
            control: Control::Choice {
                choices: vec![
                    Choice {
                        value: 0,
                        label: if de { "Rot" } else { "Red" },
                    },
                    Choice {
                        value: 1,
                        label: if de { "Grün" } else { "Green" },
                    },
                    Choice {
                        value: 2,
                        label: if de { "Blau" } else { "Blue" },
                    },
                    Choice {
                        value: 3,
                        label: if de { "Monochrom" } else { "Monochrome" },
                    },
                ],
            },
            unit: None,
        },
    ];
    let glaucoma = vec![
        boolean(
            "glaucoma.enabled",
            "glaucoma_onoff",
            if de { "Aktiv" } else { "Enabled" },
        ),
        number(
            "glaucoma.field",
            "glaucoma_fov",
            if de {
                "Gesichtsfeldverlust"
            } else {
                "Field loss"
            },
            true,
            Some("%"),
        ),
    ];
    let achromatopsia = vec![
        boolean(
            "achromatopsia.enabled",
            "achromatopsia_onoff",
            if de { "Aktiv" } else { "Enabled" },
        ),
        number(
            "achromatopsia.intensity",
            "achromatopsia_int",
            if de { "Intensität" } else { "Intensity" },
            true,
            Some("%"),
        ),
        number(
            "achromatopsia.blur",
            "achromatopsia_blur_factor",
            if de { "Unschärfe" } else { "Blur" },
            false,
            Some("%"),
        ),
    ];
    let nyctalopia = vec![
        boolean(
            "nyctalopia.enabled",
            "nyctalopia_onoff",
            if de { "Aktiv" } else { "Enabled" },
        ),
        number(
            "nyctalopia.intensity",
            "nyctalopia_int",
            if de { "Intensität" } else { "Intensity" },
            true,
            Some("%"),
        ),
    ];
    let macular = vec![
        boolean(
            "macular.enabled",
            "maculardegeneration_onoff",
            if de { "Aktiv" } else { "Enabled" },
        ),
        boolean(
            "macular.simple",
            "maculardegeneration_veasy",
            if de {
                "Einfaches Modell"
            } else {
                "Simple model"
            },
        ),
        number(
            "macular.simple_intensity",
            "maculardegeneration_inteasy",
            if de { "Schweregrad" } else { "Severity" },
            true,
            Some("%"),
        ),
        boolean(
            "macular.advanced",
            "maculardegeneration_vadvanced",
            if de {
                "Erweitertes Modell"
            } else {
                "Advanced model"
            },
        ),
        number(
            "macular.radius",
            "maculardegeneration_radius",
            "Radius",
            false,
            Some("°"),
        ),
        number(
            "macular.intensity",
            "maculardegeneration_intadvanced",
            if de { "Intensität" } else { "Intensity" },
            false,
            Some("%"),
        ),
    ];
    let groups = vec![
        Group {
            id: "cataract",
            title: if de { "Katarakt" } else { "Cataract" },
            parameters: cataract,
        },
        Group {
            id: "color",
            title: if de { "Farbsehen" } else { "Color vision" },
            parameters: color,
        },
        Group {
            id: "glaucoma",
            title: if de { "Glaukom" } else { "Glaucoma" },
            parameters: glaucoma,
        },
        Group {
            id: "achromatopsia",
            title: if de { "Achromatopsie" } else { "Achromatopsia" },
            parameters: achromatopsia,
        },
        Group {
            id: "nyctalopia",
            title: if de {
                "Nachtblindheit"
            } else {
                "Night blindness"
            },
            parameters: nyctalopia,
        },
        Group {
            id: "macular",
            title: if de {
                "Makuladegeneration"
            } else {
                "Macular degeneration"
            },
            parameters: macular,
        },
    ];
    let profiles = vec![
        layer(
            "cataract-light",
            if de {
                "Leichte Katarakt"
            } else {
                "Mild cataract"
            },
            [
                ("ct_onoff", json!(true)),
                ("ct_blur_factor", json!(25.0)),
                ("ct_contrast_factor", json!(20.0)),
            ],
        ),
        layer(
            "cataract-strong",
            if de {
                "Starke Katarakt"
            } else {
                "Severe cataract"
            },
            [
                ("ct_onoff", json!(true)),
                ("ct_blur_factor", json!(75.0)),
                ("ct_contrast_factor", json!(65.0)),
            ],
        ),
        layer(
            "protanopia",
            if de { "Protanopie" } else { "Protanopia" },
            [
                ("peacock_cb_onoff", json!(true)),
                ("peacock_cb_strength", json!(1.0)),
                ("peacock_cb_type", json!(0)),
            ],
        ),
        layer(
            "glaucoma-moderate",
            if de {
                "Mittleres Glaukom"
            } else {
                "Moderate glaucoma"
            },
            [("glaucoma_onoff", json!(true)), ("glaucoma_fov", json!(55))],
        ),
        layer(
            "achromatopsia",
            if de { "Achromatopsie" } else { "Achromatopsia" },
            [
                ("achromatopsia_onoff", json!(true)),
                ("achromatopsia_int", json!(100)),
                ("achromatopsia_blur_factor", json!(25.0)),
            ],
        ),
        layer(
            "night-blindness",
            if de {
                "Nachtblindheit"
            } else {
                "Night blindness"
            },
            [
                ("nyctalopia_onoff", json!(true)),
                ("nyctalopia_int", json!(70)),
            ],
        ),
        layer(
            "macular-moderate",
            if de {
                "Mittlere Makuladegeneration"
            } else {
                "Moderate macular degeneration"
            },
            [
                ("maculardegeneration_onoff", json!(true)),
                ("maculardegeneration_veasy", json!(true)),
                ("maculardegeneration_inteasy", json!(55)),
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
    let profile_ids: BTreeSet<&str> = profiles.iter().map(|profile| profile.id).collect();
    for article in &articles {
        for demonstration in &article.demonstrations {
            for profile in &demonstration.profiles {
                assert!(
                    profile_ids.contains(profile.as_str()),
                    "Article '{}' references unknown profile '{}'",
                    article.id,
                    profile
                );
            }
        }
    }
    Catalog {
        groups,
        profiles,
        articles,
    }
}

fn layer<const N: usize>(
    id: &'static str,
    label: &'static str,
    values: [(&'static str, Value); N],
) -> ConfigLayer {
    ConfigLayer {
        id,
        label,
        values: values.into_iter().collect(),
    }
}

fn boolean(id: &'static str, key: &'static str, label: &'static str) -> Parameter {
    Parameter {
        id,
        key,
        label,
        help: "",
        default: json!(false),
        control: Control::Boolean,
        unit: None,
    }
}

fn number(
    id: &'static str,
    key: &'static str,
    label: &'static str,
    integer: bool,
    unit: Option<&'static str>,
) -> Parameter {
    Parameter {
        id,
        key,
        label,
        help: "",
        default: if integer { json!(0) } else { json!(0.0) },
        control: Control::Number {
            integer,
            min: Some(0.0),
            max: Some(100.0),
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
        for p in &group.parameters {
            result.insert(p.key.into(), p.default.clone());
        }
    }
    for id in active {
        if let Some(layer) = catalog.profiles.iter().find(|p| p.id == id) {
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
    for p in catalog(locale)
        .groups
        .into_iter()
        .flat_map(|g| g.parameters)
    {
        let Some(value) = values.get_mut(p.key) else {
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
        }
    }
}

pub fn contract_json(locale: Locale) -> String {
    serde_json::to_string(&catalog(locale)).expect("static catalog serializes")
}

pub fn engine_settings(values: &Map<String, Value>) -> Value {
    let mut cataract = Map::new();
    let mut retina = Map::new();
    let mut peacock = Map::new();
    for (key, value) in values {
        let node = if key.starts_with("ct_") {
            &mut cataract
        } else if key.starts_with("peacock_") {
            &mut peacock
        } else {
            &mut retina
        };
        node.insert(key.clone(), value.clone());
    }
    json!([{ "Cataract": cataract, "Retina": retina, "PeacockCB": peacock }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_profiles_and_manual_values_win() {
        let active = vec!["cataract-light".into(), "cataract-strong".into()];
        let mut manual = Map::new();
        manual.insert("ct_blur_factor".into(), json!(10.0));
        let values = compose(Locale::En, &active, &manual);
        assert_eq!(values["ct_blur_factor"], json!(10.0));
        assert_eq!(values["ct_contrast_factor"], json!(65.0));
    }

    #[test]
    fn normalization_clamps_and_rejects_invalid_choices() {
        let mut values = Map::from_iter([
            ("ct_blur_factor".into(), json!(500)),
            ("peacock_cb_type".into(), json!(99)),
        ]);
        normalize(Locale::En, &mut values);
        assert_eq!(values["ct_blur_factor"], json!(100.0));
        assert_eq!(values["peacock_cb_type"], json!(0));
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
    fn engine_settings_are_grouped_by_real_node_names() {
        let values = Map::from_iter([
            ("ct_onoff".into(), json!(true)),
            ("glaucoma_onoff".into(), json!(true)),
            ("peacock_cb_type".into(), json!(2)),
        ]);
        let engine = engine_settings(&values);
        assert_eq!(engine[0]["Cataract"]["ct_onoff"], json!(true));
        assert_eq!(engine[0]["Retina"]["glaucoma_onoff"], json!(true));
        assert_eq!(engine[0]["PeacockCB"]["peacock_cb_type"], json!(2));
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
        assert_eq!(cataract.demonstrations[0].profiles, vec!["cataract-light"]);
    }
}
