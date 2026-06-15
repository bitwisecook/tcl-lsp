//! `tcl explore` — compiler-explorer views over aggregated input.
//!
//! Drives the `tcl-explorer` pipeline + serialiser (the same contract the
//! GUI / TUI / editor panels consume). `--json` emits the full
//! machine-readable JSON; the default is a compact per-view summary. The
//! rich ANSI / box-drawing renderer (`_render.py`) is a later increment;
//! the JSON path already exposes every ported view.

use serde_json::Value;

use tcl_cli_support::{OutputTarget, combine_sources, read_input_documents, write_text_output};

use crate::cli::{ColourArgs, InputArgs};

/// Handle `tcl explore`.
pub fn run_explore(
    input: &InputArgs,
    show: &[String],
    json: bool,
    _colour: &ColourArgs,
) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let source = combine_sources(&documents);

    let result = tcl_explorer::run_pipeline(&source, &input.dialect);
    let value = tcl_explorer::serialise_result(&result);

    let target = OutputTarget::from_arg(input.output.as_deref());
    let text = if json {
        serde_json::to_string_pretty(&value)?
    } else {
        render_summary(&value, show)
    };
    write_text_output(&target, &text)?;
    Ok(0)
}

/// A one-line description of a view's payload.
fn summarise(value: &Value) -> String {
    match value {
        Value::Array(a) => format!("{} item(s)", a.len()),
        Value::Object(o) => format!("{} field(s)", o.len()),
        Value::String(s) => format!("{} char(s)", s.chars().count()),
        Value::Null => "(none)".to_owned(),
        other => other.to_string(),
    }
}

/// A compact per-view summary of the serialised result, optionally filtered
/// to the views named in `show` (case-insensitive substring match).
fn render_summary(value: &Value, show: &[String]) -> String {
    let Some(obj) = value.as_object() else {
        return "compiler explorer: no result".to_owned();
    };
    let mut lines = vec!["Compiler explorer summary".to_owned()];
    let mut keys: Vec<&String> = obj.keys().collect();
    keys.sort();
    for key in keys {
        if !show.is_empty()
            && !show
                .iter()
                .any(|s| key.to_lowercase().contains(&s.to_lowercase()))
        {
            continue;
        }
        lines.push(format!("  {key}: {}", summarise(&obj[key])));
    }
    lines.join("\n")
}
