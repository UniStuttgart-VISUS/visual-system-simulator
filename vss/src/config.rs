use crate::{AssetId, Flow, Inspector, Node};
use cgmath::Matrix4;
use serde_json::{Map, Value};
use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::error::Error;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub source: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigValue {
    pub value: Value,
    pub source: String,
    pub path: String,
    pub children: BTreeMap<String, ConfigValue>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PoseConfig {
    pub gaze: Option<[f64; 2]>,
    pub view: Option<[f64; 2]>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfigSection {
    pub simulator: BTreeMap<String, ConfigValue>,
    pub pose: PoseConfig,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfigDocument {
    pub both: ConfigSection,
    pub left: ConfigSection,
    pub right: ConfigSection,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectiveSection {
    pub simulator: BTreeMap<String, ConfigValue>,
    pub pose: PoseConfig,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FieldKind {
    Bool,
    F64,
    F32,
    I32,
    U32,
    String,
    Asset,
    Matrix,
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FlowSchema {
    pub fields: BTreeMap<String, FieldKind>,
}

fn diagnostic(
    severity: DiagnosticSeverity,
    source: &str,
    path: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        severity,
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

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Null => "null",
    }
}

fn expected_type_name(expected: FieldKind) -> &'static str {
    match expected {
        FieldKind::Bool => "bool",
        FieldKind::F64 => "f64",
        FieldKind::F32 => "f32",
        FieldKind::I32 => "i32",
        FieldKind::U32 => "u32",
        FieldKind::String => "string",
        FieldKind::Asset => "string",
        FieldKind::Matrix => "matrix",
        FieldKind::Unknown => "value",
    }
}

fn rebuild_object_value(children: &BTreeMap<String, ConfigValue>) -> Value {
    let mut object = Map::new();
    for (key, child) in children {
        object.insert(key.clone(), child.value.clone());
    }
    Value::Object(object)
}

impl ConfigValue {
    fn from_json(source: &str, path: &str, value: &Value) -> Self {
        let children = value
            .as_object()
            .map(|object| {
                object
                    .iter()
                    .map(|(key, child)| {
                        let child_path = join_path(path, key);
                        (
                            key.clone(),
                            ConfigValue::from_json(source, &child_path, child),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self {
            value: value.clone(),
            source: source.to_string(),
            path: path.to_string(),
            children,
        }
    }
}

fn merge_config_value(dst: &mut ConfigValue, src: &ConfigValue) {
    if dst.value.is_object() && src.value.is_object() {
        for (key, src_child) in &src.children {
            match dst.children.get_mut(key) {
                Some(dst_child) => merge_config_value(dst_child, src_child),
                None => {
                    dst.children.insert(key.clone(), src_child.clone());
                }
            }
        }
        dst.value = rebuild_object_value(&dst.children);
        dst.source = src.source.clone();
        dst.path = src.path.clone();
    } else {
        *dst = src.clone();
    }
}

impl ConfigSection {
    pub fn simulator_value_map(&self) -> BTreeMap<String, ConfigValue> {
        self.simulator.clone()
    }
}

impl EffectiveSection {
    pub fn simulator_value_map(&self) -> BTreeMap<String, ConfigValue> {
        self.simulator.clone()
    }
}

fn merge_section(dst: &mut ConfigSection, src: &ConfigSection) {
    for (key, value) in &src.simulator {
        match dst.simulator.get_mut(key) {
            Some(existing) => merge_config_value(existing, value),
            None => {
                dst.simulator.insert(key.clone(), value.clone());
            }
        }
    }

    if src.pose.gaze.is_some() {
        dst.pose.gaze = src.pose.gaze;
    }
    if src.pose.view.is_some() {
        dst.pose.view = src.pose.view;
    }
}

fn parse_pose(
    source: &str,
    path: &str,
    value: &Value,
    diagnostics: &mut Vec<Diagnostic>,
) -> PoseConfig {
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            DiagnosticSeverity::Error,
            source,
            path,
            "expected an object for pose",
        ));
        return PoseConfig::default();
    };

    let mut pose = PoseConfig::default();
    for (key, child) in object {
        let child_path = join_path(path, key);
        match key.as_str() {
            "gaze" | "view" => {
                if child.is_null() {
                    continue;
                }
                let Some(array) = child.as_array() else {
                    diagnostics.push(diagnostic(
                        DiagnosticSeverity::Error,
                        source,
                        child_path,
                        "expected an array of two numbers",
                    ));
                    continue;
                };
                if array.len() != 2 {
                    diagnostics.push(diagnostic(
                        DiagnosticSeverity::Error,
                        source,
                        child_path,
                        "expected exactly two numbers",
                    ));
                    continue;
                }
                let Some(x) = array[0].as_f64() else {
                    diagnostics.push(diagnostic(
                        DiagnosticSeverity::Error,
                        source,
                        join_path(&child_path, "0"),
                        "expected a number",
                    ));
                    continue;
                };
                let Some(y) = array[1].as_f64() else {
                    diagnostics.push(diagnostic(
                        DiagnosticSeverity::Error,
                        source,
                        join_path(&child_path, "1"),
                        "expected a number",
                    ));
                    continue;
                };
                match key.as_str() {
                    "gaze" => pose.gaze = Some([x, y]),
                    "view" => pose.view = Some([x, y]),
                    _ => unreachable!(),
                }
            }
            _ => diagnostics.push(diagnostic(
                DiagnosticSeverity::Warning,
                source,
                child_path,
                "unknown configuration field",
            )),
        }
    }

    pose
}

fn parse_simulator_value(source: &str, path: &str, value: &Value) -> ConfigValue {
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
            DiagnosticSeverity::Error,
            source,
            path,
            "expected an object",
        ));
        return ConfigSection::default();
    };

    let mut section = ConfigSection::default();
    for (key, child) in object {
        let child_path = join_path(path, key);
        match key.as_str() {
            "simulator" => {
                let Some(simulator) = child.as_object() else {
                    diagnostics.push(diagnostic(
                        DiagnosticSeverity::Error,
                        source,
                        child_path,
                        "expected an object for simulator",
                    ));
                    continue;
                };
                for (simulator_key, simulator_value) in simulator {
                    let simulator_path = join_path(&child_path, simulator_key);
                    section.simulator.insert(
                        simulator_key.clone(),
                        parse_simulator_value(source, &simulator_path, simulator_value),
                    );
                }
            }
            "pose" => {
                section.pose = parse_pose(source, &child_path, child, diagnostics);
            }
            _ => diagnostics.push(diagnostic(
                DiagnosticSeverity::Warning,
                source,
                child_path,
                "unknown configuration field",
            )),
        }
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
            DiagnosticSeverity::Error,
            source,
            "",
            "expected an object",
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
                DiagnosticSeverity::Warning,
                source,
                key,
                "unknown configuration field",
            )),
        }
    }
    document
}

fn parse_legacy_document(
    source: &str,
    root: &Value,
    diagnostics: &mut Vec<Diagnostic>,
) -> ConfigDocument {
    let Some(object) = root.as_object() else {
        diagnostics.push(diagnostic(
            DiagnosticSeverity::Error,
            source,
            "",
            "expected an object",
        ));
        return ConfigDocument::default();
    };

    let mut document = ConfigDocument::default();
    for (key, value) in object {
        document
            .both
            .simulator
            .insert(key.clone(), parse_simulator_value(source, key, value));
    }
    diagnostics.push(diagnostic(
        DiagnosticSeverity::Warning,
        source,
        "",
        "legacy flat configuration is deprecated; wrap simulator fields inside `both.simulator`",
    ));
    document
}

pub fn parse_config_value(source: &str, root: &Value) -> (ConfigDocument, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let document = if root
        .as_object()
        .map(|object| {
            object
                .keys()
                .any(|key| matches!(key.as_str(), "both" | "left" | "right"))
        })
        .unwrap_or(false)
    {
        parse_structured_document(source, root, &mut diagnostics)
    } else {
        parse_legacy_document(source, root, &mut diagnostics)
    };
    (document, diagnostics)
}

pub fn parse_config_str(
    source: &str,
    json: &str,
) -> Result<(ConfigDocument, Vec<Diagnostic>), Diagnostic> {
    let root: Value = serde_json::from_str(json).map_err(|err| {
        diagnostic(
            DiagnosticSeverity::Error,
            source,
            "",
            format!("invalid JSON: {err}"),
        )
    })?;
    Ok(parse_config_value(source, &root))
}

fn sidecar_path_for_input(input: &Path) -> PathBuf {
    PathBuf::from(format!("{}.vss.json", input.to_string_lossy()))
}

pub fn diagnostics_to_string(diagnostics: &[Diagnostic]) -> String {
    let warnings = diagnostics
        .iter()
        .filter(|diag| matches!(diag.severity, DiagnosticSeverity::Warning))
        .count();
    let errors = diagnostics
        .iter()
        .filter(|diag| matches!(diag.severity, DiagnosticSeverity::Error))
        .count();

    let mut lines = vec![format!(
        "Configuration contains {warnings} warnings and {errors} errors."
    )];
    for diagnostic in diagnostics {
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Error => "error",
        };
        lines.push(format!("{severity}: {}", diagnostic.message));
        if !diagnostic.source.is_empty() {
            lines.push(format!("  source: {}", diagnostic.source));
        }
        if !diagnostic.path.is_empty() {
            lines.push(format!("  path:   {}", diagnostic.path));
        }
    }
    lines.join("\n")
}

pub fn diagnostics_have_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diag| matches!(diag.severity, DiagnosticSeverity::Error))
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
            simulator: section.simulator,
            pose: section.pose,
        }
    }

    pub fn effective_right(&self) -> EffectiveSection {
        let mut section = self.both.clone();
        merge_section(&mut section, &self.right);
        EffectiveSection {
            simulator: section.simulator,
            pose: section.pose,
        }
    }
}

pub fn load_config_layers_from_text(
    base_config: Option<&Path>,
    inputs: &[String],
    read_text: impl Fn(&Path) -> Result<Option<String>, Box<dyn Error>>,
) -> Result<(ConfigDocument, Vec<Diagnostic>), Box<dyn Error>> {
    let mut merged = ConfigDocument::default();
    let mut diagnostics = Vec::new();

    let mut paths = Vec::new();
    if let Some(config_path) = base_config {
        paths.push((config_path.to_path_buf(), true));
    }
    for input in inputs {
        paths.push((sidecar_path_for_input(Path::new(input)), false));
    }

    for (path, required) in paths {
        let Some(text) = read_text(&path)? else {
            if required {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("configuration file not found: {}", path.display()),
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
                    format!(
                        "failed to parse configuration {}: {}",
                        path.display(),
                        diag.message
                    ),
                )
            })?;
        merged.merge(&document);
        diagnostics.append(&mut layer_diagnostics);
    }

    Ok((merged, diagnostics))
}

#[cfg(test)]
mod tests {
    use super::load_config_layers_from_text;
    use std::io::ErrorKind;
    use std::path::Path;

    #[test]
    fn missing_explicit_base_config_is_an_error() {
        let err = load_config_layers_from_text(Some(Path::new("missing.toml")), &[], |_| Ok(None))
            .unwrap_err();

        assert_eq!(
            err.downcast_ref::<std::io::Error>().map(|err| err.kind()),
            Some(ErrorKind::NotFound)
        );
        assert!(err.to_string().contains("missing.toml"));
    }

    #[test]
    fn missing_sidecar_config_is_optional() {
        let result = load_config_layers_from_text(None, &["input.png".to_string()], |_| Ok(None));

        assert!(result.is_ok());
    }
}

impl PoseConfig {
    pub fn resolve(&self, width: u32, height: u32) -> ([f64; 2], [f64; 2]) {
        let center = [width as f64 / 2.0, height as f64 / 2.0];
        (self.gaze.unwrap_or(center), self.view.unwrap_or(center))
    }
}

struct SchemaCollector {
    schema: RefCell<FlowSchema>,
}

impl SchemaCollector {
    fn new() -> Self {
        Self {
            schema: RefCell::new(FlowSchema::default()),
        }
    }
}

impl Inspector for SchemaCollector {
    fn flow(&self, _index: usize, flow: &Flow) {
        flow.inspect(self);
    }

    fn mut_node(&self, node: &mut dyn Node) {
        node.inspect(self);
    }

    fn mut_bool(&self, name: &'static str, _value: &mut bool) -> bool {
        self.schema
            .borrow_mut()
            .fields
            .insert(name.to_string(), FieldKind::Bool);
        false
    }

    fn mut_f64(&self, name: &'static str, _value: &mut f64) -> bool {
        self.schema
            .borrow_mut()
            .fields
            .insert(name.to_string(), FieldKind::F64);
        false
    }

    fn mut_f32(&self, name: &'static str, _value: &mut f32) -> bool {
        self.schema
            .borrow_mut()
            .fields
            .insert(name.to_string(), FieldKind::F32);
        false
    }

    fn mut_i32(&self, name: &'static str, _value: &mut i32) -> bool {
        self.schema
            .borrow_mut()
            .fields
            .insert(name.to_string(), FieldKind::I32);
        false
    }

    fn mut_u32(&self, name: &'static str, _value: &mut u32) -> bool {
        self.schema
            .borrow_mut()
            .fields
            .insert(name.to_string(), FieldKind::U32);
        false
    }

    fn mut_img(&self, name: &'static str, _value: &mut String) -> bool {
        self.schema
            .borrow_mut()
            .fields
            .insert(name.to_string(), FieldKind::String);
        false
    }

    fn mut_asset(&self, name: &'static str, _value: &mut AssetId) -> bool {
        self.schema
            .borrow_mut()
            .fields
            .insert(name.to_string(), FieldKind::Asset);
        false
    }

    fn mut_matrix(&self, name: &'static str, _value: &mut Matrix4<f32>) -> bool {
        self.schema
            .borrow_mut()
            .fields
            .insert(name.to_string(), FieldKind::Matrix);
        false
    }
}

pub fn schema_from_flow(flow: &Flow) -> FlowSchema {
    let collector = SchemaCollector::new();
    flow.inspect(&collector);
    collector.schema.into_inner()
}

fn validate_number(value: &Value, expected: FieldKind) -> Result<(), &'static str> {
    match expected {
        FieldKind::F64 => {
            if value.as_f64().is_some() {
                Ok(())
            } else {
                Err("expected a number")
            }
        }
        FieldKind::F32 => {
            let Some(v) = value.as_f64() else {
                return Err("expected a number");
            };
            if v.is_finite() && v >= f32::MIN as f64 && v <= f32::MAX as f64 {
                Ok(())
            } else {
                Err("expected a number within the f32 range")
            }
        }
        FieldKind::I32 => {
            let Some(v) = value.as_i64() else {
                return Err("expected an integer");
            };
            if i32::try_from(v).is_ok() {
                Ok(())
            } else {
                Err("expected an integer within the i32 range")
            }
        }
        FieldKind::U32 => {
            let Some(v) = value.as_u64() else {
                return Err("expected a non-negative integer");
            };
            if u32::try_from(v).is_ok() {
                Ok(())
            } else {
                Err("expected a non-negative integer within the u32 range")
            }
        }
        _ => Err("expected a number"),
    }
}

fn validate_value_kind(expected: FieldKind, value: &ConfigValue) -> Result<(), String> {
    match expected {
        FieldKind::Bool => {
            if value.value.is_boolean() {
                Ok(())
            } else {
                Err(format!(
                    "expected {}, found {}",
                    expected_type_name(expected),
                    value_type_name(&value.value)
                ))
            }
        }
        FieldKind::F64 | FieldKind::F32 | FieldKind::I32 | FieldKind::U32 => {
            validate_number(&value.value, expected).map_err(ToString::to_string)
        }
        FieldKind::String | FieldKind::Asset => {
            if value.value.is_string() {
                Ok(())
            } else {
                Err(format!(
                    "expected {}, found {}",
                    expected_type_name(expected),
                    value_type_name(&value.value)
                ))
            }
        }
        FieldKind::Matrix => Err("matrix configuration is not supported yet".to_string()),
        FieldKind::Unknown => Ok(()),
    }
}

pub fn validate_simulator_values(
    simulator: &BTreeMap<String, ConfigValue>,
    schema: &FlowSchema,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (field_name, field_value) in simulator {
        let Some(expected) = schema.fields.get(field_name) else {
            diagnostics.push(diagnostic(
                DiagnosticSeverity::Warning,
                &field_value.source,
                field_value.path.clone(),
                "unknown configuration field",
            ));
            continue;
        };
        if let Err(message) = validate_value_kind(*expected, field_value) {
            diagnostics.push(diagnostic(
                DiagnosticSeverity::Error,
                &field_value.source,
                field_value.path.clone(),
                message,
            ));
        }
    }

    diagnostics
}

fn apply_field(
    field: &ConfigValue,
    target: &mut dyn Any,
    asset_resolver: &dyn Fn(&str, &str) -> AssetId,
) -> Result<(), Diagnostic> {
    if let Some(target) = target.downcast_mut::<bool>() {
        if let Some(v) = field.value.as_bool() {
            *target = v;
            return Ok(());
        }
        return Err(diagnostic(
            DiagnosticSeverity::Error,
            &field.source,
            field.path.clone(),
            format!(
                "expected {}, found {}",
                expected_type_name(FieldKind::Bool),
                value_type_name(&field.value)
            ),
        ));
    }
    if let Some(target) = target.downcast_mut::<f64>() {
        if let Some(v) = field.value.as_f64() {
            *target = v;
            return Ok(());
        }
        return Err(diagnostic(
            DiagnosticSeverity::Error,
            &field.source,
            field.path.clone(),
            "expected a number",
        ));
    }
    if let Some(target) = target.downcast_mut::<f32>() {
        let Some(v) = field.value.as_f64() else {
            return Err(diagnostic(
                DiagnosticSeverity::Error,
                &field.source,
                field.path.clone(),
                "expected a number",
            ));
        };
        if !(v.is_finite() && v >= f32::MIN as f64 && v <= f32::MAX as f64) {
            return Err(diagnostic(
                DiagnosticSeverity::Error,
                &field.source,
                field.path.clone(),
                "expected a number within the f32 range",
            ));
        }
        *target = v as f32;
        return Ok(());
    }
    if let Some(target) = target.downcast_mut::<i32>() {
        let Some(v) = field.value.as_i64() else {
            return Err(diagnostic(
                DiagnosticSeverity::Error,
                &field.source,
                field.path.clone(),
                "expected an integer",
            ));
        };
        let Ok(v) = i32::try_from(v) else {
            return Err(diagnostic(
                DiagnosticSeverity::Error,
                &field.source,
                field.path.clone(),
                "expected an integer within the i32 range",
            ));
        };
        *target = v;
        return Ok(());
    }
    if let Some(target) = target.downcast_mut::<u32>() {
        let Some(v) = field.value.as_u64() else {
            return Err(diagnostic(
                DiagnosticSeverity::Error,
                &field.source,
                field.path.clone(),
                "expected a non-negative integer",
            ));
        };
        let Ok(v) = u32::try_from(v) else {
            return Err(diagnostic(
                DiagnosticSeverity::Error,
                &field.source,
                field.path.clone(),
                "expected a non-negative integer within the u32 range",
            ));
        };
        *target = v;
        return Ok(());
    }
    if let Some(target) = target.downcast_mut::<String>() {
        if let Some(v) = field.value.as_str() {
            *target = v.to_string();
            return Ok(());
        }
        return Err(diagnostic(
            DiagnosticSeverity::Error,
            &field.source,
            field.path.clone(),
            "expected a string",
        ));
    }
    if let Some(target) = target.downcast_mut::<AssetId>() {
        if let Some(v) = field.value.as_str() {
            *target = asset_resolver(&field.source, v);
            return Ok(());
        }
        return Err(diagnostic(
            DiagnosticSeverity::Error,
            &field.source,
            field.path.clone(),
            "expected a string",
        ));
    }
    Err(diagnostic(
        DiagnosticSeverity::Error,
        &field.source,
        field.path.clone(),
        "unsupported field type",
    ))
}

struct SectionApplier<'a> {
    simulator: &'a BTreeMap<String, ConfigValue>,
    diagnostics: RefCell<Vec<Diagnostic>>,
    asset_resolver: &'a dyn Fn(&str, &str) -> AssetId,
}

impl<'a> SectionApplier<'a> {
    fn new(
        simulator: &'a BTreeMap<String, ConfigValue>,
        asset_resolver: &'a dyn Fn(&str, &str) -> AssetId,
    ) -> Self {
        Self {
            simulator,
            diagnostics: RefCell::new(Vec::new()),
            asset_resolver,
        }
    }

    fn apply_to_target(&self, name: &'static str, target: &mut dyn Any) -> bool {
        let Some(field) = self.simulator.get(name) else {
            return false;
        };
        match apply_field(field, target, self.asset_resolver) {
            Ok(()) => true,
            Err(diagnostic) => {
                self.diagnostics.borrow_mut().push(diagnostic);
                false
            }
        }
    }
}

impl Inspector for SectionApplier<'_> {
    fn flow(&self, _index: usize, flow: &Flow) {
        flow.inspect(self);
    }

    fn mut_node(&self, node: &mut dyn Node) {
        node.inspect(self);
    }

    fn mut_bool(&self, name: &'static str, value: &mut bool) -> bool {
        self.apply_to_target(name, value)
    }

    fn mut_f64(&self, name: &'static str, value: &mut f64) -> bool {
        self.apply_to_target(name, value)
    }

    fn mut_f32(&self, name: &'static str, value: &mut f32) -> bool {
        self.apply_to_target(name, value)
    }

    fn mut_i32(&self, name: &'static str, value: &mut i32) -> bool {
        self.apply_to_target(name, value)
    }

    fn mut_u32(&self, name: &'static str, value: &mut u32) -> bool {
        self.apply_to_target(name, value)
    }

    fn mut_img(&self, name: &'static str, value: &mut String) -> bool {
        self.apply_to_target(name, value)
    }

    fn mut_asset(&self, name: &'static str, value: &mut AssetId) -> bool {
        self.apply_to_target(name, value)
    }

    fn mut_matrix(&self, name: &'static str, _value: &mut Matrix4<f32>) -> bool {
        if let Some(field) = self.simulator.get(name) {
            self.diagnostics.borrow_mut().push(diagnostic(
                DiagnosticSeverity::Error,
                &field.source,
                field.path.clone(),
                "matrix configuration is not supported yet",
            ));
            true
        } else {
            false
        }
    }
}

pub type AssetResolver<'a> = dyn Fn(&str, &str) -> AssetId + 'a;

pub fn apply_simulator_values(
    flow: &Flow,
    simulator: &BTreeMap<String, ConfigValue>,
    asset_resolver: &AssetResolver<'_>,
) -> Vec<Diagnostic> {
    let applier = SectionApplier::new(simulator, asset_resolver);
    flow.inspect(&applier);
    applier.diagnostics.into_inner()
}
