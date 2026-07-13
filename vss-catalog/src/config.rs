use serde_json::Value;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::error::Error;
use std::path::{Path, PathBuf};
use vss::{registry, AssetId, Flow, ParameterKind, ParameterPatch, ParameterValue};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Diagnostic {
    pub source: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigValue {
    pub value: Value,
    pub source: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfigSection {
    pub values: BTreeMap<String, ConfigValue>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfigDocument {
    pub both: ConfigSection,
    pub left: ConfigSection,
    pub right: ConfigSection,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectiveSection {
    pub values: BTreeMap<String, ConfigValue>,
}

fn diagnostic(source: &str, path: impl Into<String>, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        source: source.to_string(),
        path: path.into(),
        message: message.into(),
    }
}

fn join_path(base: &str, child: &str) -> String {
    if base.is_empty() {
        child.to_string()
    } else {
        format!("{base}.{child}")
    }
}

impl ConfigValue {
    fn from_json(source: &str, path: &str, value: &Value) -> Self {
        Self {
            value: value.clone(),
            source: source.to_string(),
            path: path.to_string(),
        }
    }
}

fn merge_config_value(dst: &mut ConfigValue, src: &ConfigValue) {
    *dst = src.clone();
}

impl ConfigSection {
    pub fn value_map(&self) -> BTreeMap<String, ConfigValue> {
        self.values.clone()
    }
}

impl EffectiveSection {
    pub fn value_map(&self) -> BTreeMap<String, ConfigValue> {
        self.values.clone()
    }
}

fn merge_section(dst: &mut ConfigSection, src: &ConfigSection) {
    for (key, value) in &src.values {
        match dst.values.get_mut(key) {
            Some(existing) => merge_config_value(existing, value),
            None => {
                dst.values.insert(key.clone(), value.clone());
            }
        }
    }
}

fn parse_setting_value(source: &str, path: &str, value: &Value) -> ConfigValue {
    ConfigValue::from_json(source, path, value)
}

fn parse_section(
    source: &str,
    path: &str,
    value: &Value,
    diagnostics: &mut Vec<Diagnostic>,
) -> ConfigSection {
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            source,
            path,
            rust_i18n::t!("diagnostics.expected_object"),
        ));
        return ConfigSection::default();
    };

    let mut section = ConfigSection::default();
    for (key, child) in object {
        let child_path = join_path(path, key);
        section
            .values
            .insert(key.clone(), parse_setting_value(source, &child_path, child));
    }

    section
}

fn parse_structured_document(
    source: &str,
    root: &Value,
    diagnostics: &mut Vec<Diagnostic>,
) -> ConfigDocument {
    let Some(object) = root.as_object() else {
        diagnostics.push(diagnostic(
            source,
            "",
            rust_i18n::t!("diagnostics.expected_object"),
        ));
        return ConfigDocument::default();
    };

    let mut document = ConfigDocument::default();
    for (key, value) in object {
        match key.as_str() {
            "both" => document.both = parse_section(source, "both", value, diagnostics),
            "left" => document.left = parse_section(source, "left", value, diagnostics),
            "right" => document.right = parse_section(source, "right", value, diagnostics),
            _ => diagnostics.push(diagnostic(
                source,
                key,
                rust_i18n::t!("diagnostics.unknown_configuration_field"),
            )),
        }
    }
    document
}

pub fn parse_config_value(source: &str, root: &Value) -> (ConfigDocument, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let document = match root.as_object() {
        Some(object)
            if !object.is_empty()
                && !object
                    .keys()
                    .any(|key| matches!(key.as_str(), "both" | "left" | "right")) =>
        {
            diagnostics.push(diagnostic(
                source,
                "",
                rust_i18n::t!("diagnostics.unsupported_configuration_format"),
            ));
            ConfigDocument::default()
        }
        _ => parse_structured_document(source, root, &mut diagnostics),
    };
    (document, diagnostics)
}

pub fn parse_config_str(
    source: &str,
    json: &str,
) -> Result<(ConfigDocument, Vec<Diagnostic>), Diagnostic> {
    let root: Value = serde_json::from_str(json).map_err(|err| {
        diagnostic(
            source,
            "",
            rust_i18n::t!("diagnostics.invalid_json", error = err),
        )
    })?;
    Ok(parse_config_value(source, &root))
}

pub fn sidecar_path_for_input(input: &Path) -> PathBuf {
    PathBuf::from(format!("{}.vss.json", input.to_string_lossy()))
}

pub fn resolve_asset_reference(source: &str, reference: &str) -> AssetId {
    let reference = Path::new(reference);
    if reference.is_absolute() {
        return AssetId::from_str(reference.to_string_lossy());
    }
    let base = Path::new(source)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    AssetId::from_str(
        base.map_or_else(|| reference.to_path_buf(), |base| base.join(reference))
            .to_string_lossy(),
    )
}

pub fn diagnostics_to_string(diagnostics: &[Diagnostic]) -> String {
    let mut lines = vec![format!(
        "{}",
        rust_i18n::t!("diagnostics.error_count", count = diagnostics.len())
    )];
    for diagnostic in diagnostics {
        lines.push(format!(
            "{}: {}",
            rust_i18n::t!("diagnostics.error"),
            diagnostic.message
        ));
        if !diagnostic.source.is_empty() {
            lines.push(format!(
                "  {}: {}",
                rust_i18n::t!("diagnostics.source"),
                diagnostic.source
            ));
        }
        if !diagnostic.path.is_empty() {
            lines.push(format!(
                "  {}:   {}",
                rust_i18n::t!("diagnostics.path"),
                diagnostic.path
            ));
        }
    }
    lines.join("\n")
}

impl ConfigDocument {
    pub fn merge(&mut self, other: &ConfigDocument) {
        merge_section(&mut self.both, &other.both);
        merge_section(&mut self.left, &other.left);
        merge_section(&mut self.right, &other.right);
    }

    pub fn effective_left(&self) -> EffectiveSection {
        let mut section = self.both.clone();
        merge_section(&mut section, &self.left);
        EffectiveSection {
            values: section.values,
        }
    }

    pub fn effective_right(&self) -> EffectiveSection {
        let mut section = self.both.clone();
        merge_section(&mut section, &self.right);
        EffectiveSection {
            values: section.values,
        }
    }
}

pub fn load_config_layers_from_text(
    config_paths: &[PathBuf],
    inputs: &[String],
    read_text: impl Fn(&Path) -> Result<Option<String>, Box<dyn Error>>,
) -> Result<(ConfigDocument, Vec<Diagnostic>), Box<dyn Error>> {
    let mut merged = ConfigDocument::default();
    let mut diagnostics = Vec::new();

    let mut paths = Vec::new();
    paths.extend(config_paths.iter().cloned().map(|path| (path, true)));
    for input in inputs {
        paths.push((sidecar_path_for_input(Path::new(input)), false));
    }

    for (path, required) in paths {
        let Some(text) = read_text(&path)? else {
            if required {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    rust_i18n::t!(
                        "diagnostics.configuration_file_not_found",
                        path = path.display()
                    ),
                )
                .into());
            }
            continue;
        };
        let source = path.to_string_lossy();
        let (document, mut layer_diagnostics) =
            parse_config_str(&source, &text).map_err(|diag| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    rust_i18n::t!(
                        "diagnostics.failed_to_parse_configuration",
                        path = path.display(),
                        error = diag.message
                    ),
                )
            })?;
        if layer_diagnostics.is_empty() {
            merged.merge(&document);
        } else {
            diagnostics.append(&mut layer_diagnostics);
        }
    }

    Ok((merged, diagnostics))
}

#[cfg(test)]
mod tests {
    use super::{load_config_layers_from_text, parse_config_str, resolve_asset_reference};
    use std::io::ErrorKind;
    use std::path::{Path, PathBuf};

    #[test]
    fn missing_explicit_base_config_is_an_error() {
        let err = load_config_layers_from_text(&[PathBuf::from("missing.toml")], &[], |_| Ok(None))
            .unwrap_err();

        assert_eq!(
            err.downcast_ref::<std::io::Error>().map(|err| err.kind()),
            Some(ErrorKind::NotFound)
        );
        assert!(err.to_string().contains("missing.toml"));
    }

    #[test]
    fn missing_sidecar_config_is_optional() {
        let result = load_config_layers_from_text(&[], &["input.png".to_string()], |_| Ok(None));

        assert!(result.is_ok());
    }

    #[test]
    fn explicit_configs_are_merged_in_argument_order() {
        let paths = [PathBuf::from("first.json"), PathBuf::from("second.json")];
        let (document, diagnostics) = load_config_layers_from_text(&paths, &[], |path| {
            Ok(Some(
                if path == Path::new("first.json") {
                    r#"{"both":{"cataract.blur":1}}"#
                } else {
                    r#"{"both":{"cataract.blur":2}}"#
                }
                .into(),
            ))
        })
        .unwrap();
        assert!(diagnostics.is_empty());
        let value = &document.both.values["cataract.blur"];
        assert_eq!(value.value, serde_json::json!(2));
        assert_eq!(value.source, "second.json");
    }

    #[test]
    fn flat_config_format_is_rejected() {
        let (document, diagnostics) =
            parse_config_str("legacy.json", r#"{"glaucoma_onoff": true}"#).unwrap();

        assert!(document.both.values.is_empty());
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].message,
            rust_i18n::t!("diagnostics.unsupported_configuration_format")
        );
    }

    #[test]
    fn relative_assets_resolve_from_the_winning_source() {
        assert_eq!(
            resolve_asset_reference("configs/eye/layer.json", "maps/retina.png").raw(),
            Path::new("configs/eye")
                .join("maps/retina.png")
                .to_string_lossy()
        );
    }
}

pub type AssetResolver<'a> = dyn Fn(&str, &str) -> AssetId + 'a;

pub fn compile_parameter_patch(
    flow: &Flow,
    values: &BTreeMap<String, ConfigValue>,
    asset_resolver: &AssetResolver<'_>,
) -> Result<ParameterPatch, Vec<Diagnostic>> {
    let descriptors: BTreeMap<_, _> = registry().iter().map(|p| (p.id(), p.clone())).collect();
    let mut patch = ParameterPatch::new();
    let mut diagnostics = Vec::new();
    for (id, field) in values {
        let Some(descriptor) = descriptors.get(id.as_str()) else {
            diagnostics.push(diagnostic(
                &field.source,
                field.path.clone(),
                rust_i18n::t!("diagnostics.unknown_setting", id = id),
            ));
            continue;
        };
        let value = match descriptor.kind() {
            ParameterKind::Bool => field.value.as_bool().map(ParameterValue::Bool),
            ParameterKind::F64 => field.value.as_f64().map(ParameterValue::F64),
            ParameterKind::F32 => field
                .value
                .as_f64()
                .filter(|v| v.is_finite() && *v >= f32::MIN as f64 && *v <= f32::MAX as f64)
                .map(|v| ParameterValue::F32(v as f32)),
            ParameterKind::I32 => field
                .value
                .as_i64()
                .and_then(|v| i32::try_from(v).ok())
                .map(ParameterValue::I32),
            ParameterKind::U32 => field
                .value
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .map(ParameterValue::U32),
            ParameterKind::String => field
                .value
                .as_str()
                .map(|v| ParameterValue::String(v.to_string())),
            ParameterKind::Asset => {
                if field.value.is_null() {
                    Some(ParameterValue::Asset(AssetId::new()))
                } else {
                    field
                        .value
                        .as_str()
                        .map(|v| ParameterValue::Asset(asset_resolver(&field.source, v)))
                }
            }
            ParameterKind::Matrix => {
                diagnostics.push(diagnostic(
                    &field.source,
                    field.path.clone(),
                    rust_i18n::t!("diagnostics.matrix_not_supported"),
                ));
                None
            }
            ParameterKind::Point => field
                .value
                .as_array()
                .filter(|v| v.len() == 2)
                .and_then(|v| Some(ParameterValue::Point([v[0].as_f64()?, v[1].as_f64()?]))),
        };
        if let Some(value) = value {
            let _ = patch.set_erased(descriptor.clone(), value);
        } else if descriptor.kind() != ParameterKind::Matrix {
            diagnostics.push(diagnostic(
                &field.source,
                field.path.clone(),
                rust_i18n::t!("diagnostics.invalid_setting_value", id = id),
            ));
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    for id in patch.missing(flow) {
        if let Some(field) = values.get(id) {
            diagnostics.push(diagnostic(
                &field.source,
                field.path.clone(),
                rust_i18n::t!("diagnostics.setting_node_missing", id = id),
            ));
        }
    }
    if diagnostics.is_empty() {
        Ok(patch)
    } else {
        Err(diagnostics)
    }
}
