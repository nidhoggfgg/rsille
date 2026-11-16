//! Declarative CLI configuration. Display sanitization never changes stored values.
use std::collections::{BTreeSet, HashSet};

use serde::Deserialize;
use serde_json::Value;

const MAX_DEPTH: usize = 32;
const MAX_ITEMS: usize = 10_000;
const MAX_CONFIG_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    pub layout: Node,
}

fn default_mode() -> String {
    "fullscreen".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Node {
    Row(Vec<Node>),
    Column(Vec<Node>),
    Text(String),
    Input(InputSpec),
    Select(SelectSpec),
    Checkbox(CheckboxSpec),
    Table(TableSpec),
    Button(ButtonSpec),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputSpec {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectSpec {
    pub id: String,
    pub label: String,
    pub options: Vec<String>,
    #[serde(default)]
    pub selected: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckboxSpec {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub checked: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableSpec {
    pub id: Option<String>,
    pub source: Option<String>,
    #[serde(default)]
    pub rows: Vec<Value>,
    #[serde(default)]
    pub selectable: bool,
    #[serde(default)]
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ButtonSpec {
    pub label: String,
    pub action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Submit,
    Cancel,
}

/// Parse one YAML document, rejecting unknown fields before validating it.
pub fn parse(text: &str, data: &Value) -> Result<Config, String> {
    if text.len() > MAX_CONFIG_BYTES {
        return Err("configuration exceeds 4 MiB".into());
    }
    // The adapter accepts the documented `column: [...]` representation rather
    // than requiring serde_yaml's native `!column` enum tags.
    let mut config: Config = serde_yaml::with::singleton_map_recursive::deserialize(
        serde_yaml::Deserializer::from_str(text),
    )
    .map_err(|error| format!("invalid configuration: {error}"))?;
    validate(&mut config, data)?;
    Ok(config)
}

/// Validate programmatically constructed configurations as well as parsed ones.
/// Sources are resolved on each call, replacing rows without modifying `data`.
pub fn validate(config: &mut Config, data: &Value) -> Result<(), String> {
    if config.version != 1 {
        return Err(format!(
            "unsupported version {}; expected 1",
            config.version
        ));
    }
    if !matches!(config.mode.as_str(), "fullscreen" | "inline") {
        return Err("mode must be fullscreen or inline".into());
    }
    validate_data(data, "data")?;
    let mut ids = HashSet::new();
    let mut count = 0;
    validate_node(&mut config.layout, data, "layout", 1, &mut count, &mut ids)
}

fn register_id(id: &str, path: &str, ids: &mut HashSet<String>) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err(format!("{path}.id must not be empty"));
    }
    if !ids.insert(id.to_owned()) {
        return Err(format!("{path}.id duplicates an existing component id"));
    }
    Ok(())
}

fn limit(len: usize, path: &str) -> Result<(), String> {
    if len > MAX_ITEMS {
        Err(format!("{path} exceeds {MAX_ITEMS} items"))
    } else {
        Ok(())
    }
}

fn validate_node(
    node: &mut Node,
    data: &Value,
    path: &str,
    depth: usize,
    count: &mut usize,
    ids: &mut HashSet<String>,
) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err(format!("{path} exceeds layout depth {MAX_DEPTH}"));
    }
    *count += 1;
    limit(*count, "layout nodes")?;
    match node {
        Node::Row(children) | Node::Column(children) => {
            limit(children.len(), path)?;
            for (index, child) in children.iter_mut().enumerate() {
                validate_node(
                    child,
                    data,
                    &format!("{path}[{index}]"),
                    depth + 1,
                    count,
                    ids,
                )?;
            }
        }
        Node::Input(spec) => register_id(&spec.id, path, ids)?,
        Node::Checkbox(spec) => register_id(&spec.id, path, ids)?,
        Node::Select(spec) => {
            register_id(&spec.id, path, ids)?;
            limit(spec.options.len(), &format!("{path}.options"))?;
            if spec.options.is_empty() || spec.selected >= spec.options.len() {
                return Err(format!(
                    "{path}.selected must index a nonempty options array"
                ));
            }
        }
        Node::Table(spec) => {
            if let Some(id) = &spec.id {
                register_id(id, path, ids)?;
            } else if spec.selectable {
                return Err(format!("{path}.id is required for a selectable table"));
            }
            if let Some(source) = &spec.source {
                let rows = resolve_source(source, data)
                    .map_err(|error| format!("{path}.source: {error}"))?
                    .as_array()
                    .ok_or_else(|| format!("{path}.source must reference an array of objects"))?;
                limit(rows.len(), &format!("{path}.rows"))?;
                spec.rows = rows.clone();
            }
            limit(spec.rows.len(), &format!("{path}.rows"))?;
            let mut keys = BTreeSet::new();
            for (index, row) in spec.rows.iter().enumerate() {
                let row_path = format!("{path}.rows[{index}]");
                let object = row
                    .as_object()
                    .ok_or_else(|| format!("{row_path} must be an object"))?;
                validate_data(row, &row_path)?;
                if spec.columns.is_empty() {
                    keys.extend(object.keys().cloned());
                    limit(keys.len(), &format!("{path}.columns"))?;
                }
            }
            if spec.columns.is_empty() {
                spec.columns = keys.into_iter().collect();
            }
            limit(spec.columns.len(), &format!("{path}.columns"))?;
        }
        Node::Text(_) | Node::Button(_) => {}
    }
    Ok(())
}

fn resolve_source<'a>(source: &str, data: &'a Value) -> Result<&'a Value, String> {
    let mut parts = source.split('.');
    if parts.next() != Some("data") {
        return Err("expected data or data.field path".into());
    }
    let mut value = data;
    for (index, field) in parts.enumerate() {
        if index >= MAX_DEPTH {
            return Err(format!("source path exceeds depth {MAX_DEPTH}"));
        }
        let mut chars = field.chars();
        if !chars.next().is_some_and(|c| c == '_' || c.is_alphabetic())
            || !chars.all(|c| c == '_' || c.is_alphanumeric())
        {
            return Err("source fields must be identifiers; expressions and array indices are not supported".into());
        }
        value = value
            .as_object()
            .ok_or_else(|| format!("source segment {} requires an object", index + 1))?
            .get(field)
            .ok_or_else(|| format!("source segment {} does not exist", index + 1))?;
    }
    Ok(value)
}

// Iterative traversal also protects callers that construct deeply nested JSON
// directly, bypassing a JSON parser's recursion limit.
fn validate_data(value: &Value, path: &str) -> Result<(), String> {
    let mut stack = vec![(value, 1)];
    while let Some((value, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            return Err(format!("{path} exceeds data depth {MAX_DEPTH}"));
        }
        match value {
            Value::Array(items) => {
                limit(items.len(), path)?;
                stack.extend(items.iter().map(|item| (item, depth + 1)));
            }
            Value::Object(fields) => {
                limit(fields.len(), path)?;
                stack.extend(fields.values().map(|item| (item, depth + 1)));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Strip C0/C1 (including ESC, newline, tab and DEL) and Unicode bidi controls
/// from terminal-facing text. Use the original values for protocol output.
pub fn display_text(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control()
            && !matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn document(layout: &str) -> String {
        format!("version: 1\nlayout:\n{layout}\n")
    }

    #[test]
    fn parses_all_nodes_and_defaults() {
        let config = parse(
            &document(
                r#"  column:
    - text: 欢迎
    - input: {id: name, label: Name}
    - select: {id: env, label: Environment, options: [dev, prod]}
    - checkbox: {id: ready, label: Ready}
    - table: {rows: [{name: api}]}
    - row:
        - button: {label: OK, action: submit}
        - button: {label: Cancel, action: cancel}"#,
            ),
            &Value::Null,
        )
        .unwrap();
        assert_eq!(config.title, "");
        assert_eq!(config.mode, "fullscreen");
        let Node::Column(nodes) = config.layout else {
            panic!()
        };
        assert!(matches!(&nodes[1], Node::Input(s) if s.value.is_empty()));
        assert!(matches!(&nodes[2], Node::Select(s) if s.selected == 0));
        assert!(matches!(&nodes[3], Node::Checkbox(s) if !s.checked));
        assert!(matches!(&nodes[4], Node::Table(s) if s.columns == ["name"]));
    }

    #[test]
    fn rejects_unknown_fields_components_actions_and_versions() {
        for text in [
            "version: 2\nlayout: {text: hello}",
            "version: 1\nextra: true\nlayout: {text: hello}",
            "version: 1\nmode: other\nlayout: {text: hello}",
            "version: 1\nlayout: {unknown: hello}",
            "version: 1\nlayout: {button: {label: OK, action: shell}}",
            "version: 1\nlayout: {input: {id: x, label: X, typo: 1}}",
            "version: 1\nlayout: {table: {typo: 1}}",
            "version: 1\nlayout: {text: hello, row: []}",
        ] {
            assert!(parse(text, &Value::Null).is_err(), "{text}");
        }
    }

    #[test]
    fn validates_ids_and_selection() {
        for layout in [
            "  input: {id: '  ', label: X}",
            "  column: [{input: {id: x, label: X}}, {checkbox: {id: x, label: X}}]",
            "  table: {selectable: true}",
            "  select: {id: x, label: X, options: []}",
            "  select: {id: x, label: X, options: [a], selected: 1}",
        ] {
            assert!(parse(&document(layout), &Value::Null).is_err());
        }
    }

    #[test]
    fn resolves_sources_and_preserves_raw_values() {
        let data = json!({"服务": {"rows": [{"name": "a\u{001b}[31m", "z": 1}, {"extra": true}]}});
        let original = data.clone();
        let mut config = parse(
            &document("  table: {id: t, source: data.服务.rows, selectable: true}"),
            &data,
        )
        .unwrap();
        validate(&mut config, &data).unwrap();
        let Node::Table(table) = config.layout else {
            panic!()
        };
        assert_eq!(table.rows, data["服务"]["rows"].as_array().unwrap().clone());
        assert_eq!(table.columns, ["extra", "name", "z"]);
        assert_eq!(data, original);
        assert_eq!(
            display_text("中文\n\t\r\x1b[31m\u{009b}\u{202e}"),
            "中文[31m"
        );
    }

    #[test]
    fn rejects_invalid_sources_and_rows() {
        let data = json!({"rows": [1], "scalar": 3});
        for source in [
            "other.rows",
            "data.missing",
            "data.rows[0]",
            "data..rows",
            "data.rows",
            "data.scalar",
            "data.scalar.x",
        ] {
            assert!(
                parse(
                    &document(&format!("  table: {{source: '{source}'}}")),
                    &data
                )
                .is_err(),
                "{source}"
            );
        }
        assert!(parse(&document("  table: {rows: [1]}"), &Value::Null).is_err());
        let config = parse(&document("  table: {source: data}"), &json!([])).unwrap();
        assert!(
            matches!(config.layout, Node::Table(s) if s.rows.is_empty() && s.columns.is_empty())
        );
    }

    #[test]
    fn enforces_depth_and_count_for_direct_callers() {
        let mut config = Config {
            version: 1,
            title: String::new(),
            mode: default_mode(),
            layout: Node::Text("x".into()),
        };
        for _ in 1..MAX_DEPTH {
            config.layout = Node::Column(vec![config.layout]);
        }
        validate(&mut config, &Value::Null).unwrap();
        config.layout = Node::Row(vec![config.layout]);
        assert!(validate(&mut config, &Value::Null).is_err());
        config.layout = Node::Row(vec![Node::Text(String::new()); MAX_ITEMS]);
        assert!(validate(&mut config, &Value::Null).is_err());
        config.layout = Node::Text(String::new());
        assert!(validate(&mut config, &json!(vec![0; MAX_ITEMS + 1])).is_err());
        let mut data = Value::Null;
        for _ in 0..MAX_DEPTH {
            data = json!([data]);
        }
        assert!(validate(&mut config, &data).is_err());
    }
}
