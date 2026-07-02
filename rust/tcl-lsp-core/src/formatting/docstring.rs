//! Docstring parsing + stub generation — the shared, consumer-agnostic home
//! for the proc-docstring model. Ports `tooling.formatter.docstring`.
//!
//! - [`parse_docstring`] turns raw `# @param …` comment text into a structured
//!   [`DocstringInfo`];
//! - [`render_comment_block`] / [`generate_stub_for_proc`] render a stub back to
//!   Tcl comment lines (Doxygen or plain style, with optional decoration).
//!
//! The `tcl-lsp-core` code-actions layer and the `tcl_lsp_py` facade share this
//! one implementation.

use tcl_compiler::analyser::ProcDef;

use super::config::DocstringTagStyle;

/// Characters that make up a decoration-only rule line (`# ......`), skipped
/// when parsing description text. Mirrors Python `shared.docstrings`.
const DECORATION_CHARS: &str = ".-=*~#";

/// Documentation for a single parameter.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParamDoc {
    /// Parameter name (no leading `$`).
    pub name: String,
    /// Free-text description (may be empty).
    pub description: String,
}

/// Structured representation of a proc docstring.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocstringInfo {
    /// One-line `@brief` summary.
    pub brief: String,
    /// Free-form description (non-tag lines, newline-joined).
    pub description: String,
    /// `@param` entries in source order.
    pub params: Vec<ParamDoc>,
    /// `@return` / `@returns` text (space-joined when repeated).
    pub returns: String,
}

impl DocstringInfo {
    /// Serialise to the JSON-friendly shape the AI tools consume — keys are
    /// omitted when empty, matching Python `DocstringInfo.to_dict`.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        if !self.brief.is_empty() {
            map.insert("brief".to_owned(), serde_json::json!(self.brief));
        }
        if !self.description.is_empty() {
            map.insert("description".to_owned(), serde_json::json!(self.description));
        }
        if !self.params.is_empty() {
            let params: Vec<serde_json::Value> = self
                .params
                .iter()
                .map(|p| serde_json::json!({ "name": p.name, "description": p.description }))
                .collect();
            map.insert("params".to_owned(), serde_json::json!(params));
        }
        if !self.returns.is_empty() {
            map.insert("returns".to_owned(), serde_json::json!(self.returns));
        }
        serde_json::Value::Object(map)
    }
}

/// Resolve a tag-style string to the enum (case-insensitive); anything other
/// than `"plain"` maps to Doxygen. Mirrors Python `resolve_tag_style`.
#[must_use]
pub fn resolve_tag_style(style: &str) -> DocstringTagStyle {
    if style.eq_ignore_ascii_case("plain") {
        DocstringTagStyle::Plain
    } else {
        DocstringTagStyle::Doxygen
    }
}

/// Parse raw docstring text into structured form, recognising `@param`,
/// `@return`/`@returns`, and `@brief`; other lines accumulate as description.
#[must_use]
pub fn parse_docstring(text: &str) -> DocstringInfo {
    let mut brief = String::new();
    let mut description_lines: Vec<&str> = Vec::new();
    let mut params: Vec<ParamDoc> = Vec::new();
    let mut returns_parts: Vec<String> = Vec::new();

    for line in text.lines() {
        let stripped = line.trim();
        let low = stripped.to_ascii_lowercase();

        if low.starts_with("@param ") || low.starts_with("@param\t") {
            let rest = stripped[7..].trim();
            // Split on the first run of whitespace into (name, desc).
            match rest.split_once(char::is_whitespace) {
                Some((name, desc)) => {
                    let name = name.trim_end_matches([' ', '-']);
                    let desc = desc.trim_start_matches(['-', ' ']);
                    params.push(ParamDoc {
                        name: name.to_owned(),
                        description: desc.to_owned(),
                    });
                }
                None if !rest.is_empty() => params.push(ParamDoc {
                    name: rest.to_owned(),
                    description: String::new(),
                }),
                None => {}
            }
        } else if low.starts_with("@return ")
            || low.starts_with("@return\t")
            || low.starts_with("@returns ")
            || low.starts_with("@returns\t")
        {
            let rest = stripped
                .split_once(char::is_whitespace)
                .map_or("", |(_, r)| r);
            returns_parts.push(rest.trim_start_matches(['-', ' ']).to_owned());
        } else if low.starts_with("@brief ") || low.starts_with("@brief\t") {
            stripped[7..].trim().clone_into(&mut brief);
        } else {
            // Skip decoration-only lines (dots, dashes, hashes, etc.).
            if !stripped.is_empty() && stripped.chars().all(|c| DECORATION_CHARS.contains(c)) {
                continue;
            }
            description_lines.push(stripped);
        }
    }

    DocstringInfo {
        brief,
        description: description_lines.join("\n").trim().to_owned(),
        params,
        returns: returns_parts.join(" "),
    }
}

/// Render a [`DocstringInfo`] as a Tcl comment block (no trailing newline).
/// Mirrors Python `render_comment_block`.
#[must_use]
pub fn render_comment_block(
    info: &DocstringInfo,
    tag_style: DocstringTagStyle,
    decoration: bool,
    decoration_char: char,
    decoration_width: usize,
    indent: &str,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    let rule = || format!("{indent}# {}", decoration_char.to_string().repeat(decoration_width));

    if decoration {
        lines.push(rule());
    }

    if matches!(tag_style, DocstringTagStyle::Doxygen) {
        if !info.brief.is_empty() {
            lines.push(format!("{indent}# @brief {}", info.brief));
        }
        for desc_line in info.description.lines() {
            lines.push(if desc_line.is_empty() {
                format!("{indent}#")
            } else {
                format!("{indent}# {desc_line}")
            });
        }
        for p in &info.params {
            if p.description.is_empty() {
                lines.push(format!("{indent}# @param {}", p.name));
            } else {
                lines.push(format!("{indent}# @param {} - {}", p.name, p.description));
            }
        }
        if !info.returns.is_empty() {
            lines.push(format!("{indent}# @return {}", info.returns));
        }
    } else {
        // Plain style (also used for `None` — tags left as-is falls through here).
        if !info.brief.is_empty() {
            lines.push(format!("{indent}# {}", info.brief));
        }
        for desc_line in info.description.lines() {
            lines.push(if desc_line.is_empty() {
                format!("{indent}#")
            } else {
                format!("{indent}# {desc_line}")
            });
        }
        if !info.params.is_empty() {
            lines.push(format!("{indent}#"));
            lines.push(format!("{indent}# Arguments:"));
            for p in &info.params {
                if p.description.is_empty() {
                    lines.push(format!("{indent}#   {}", p.name));
                } else {
                    lines.push(format!("{indent}#   {} - {}", p.name, p.description));
                }
            }
        }
        if !info.returns.is_empty() {
            lines.push(format!("{indent}#"));
            lines.push(format!("{indent}# Returns: {}", info.returns));
        }
    }

    if decoration {
        lines.push(rule());
    }

    lines.join("\n")
}

/// The stub [`ParamDoc`] for one parameter — `args` gets a prose note,
/// defaulted params get a `(default: …)` note, others are bare.
fn stub_param_doc(name: &str, has_default: bool, default_value: Option<&str>) -> ParamDoc {
    let description = if name == "args" {
        "Additional arguments".to_owned()
    } else if has_default {
        format!("(default: {})", default_value.unwrap_or(""))
    } else {
        String::new()
    };
    ParamDoc {
        name: name.to_owned(),
        description,
    }
}

/// Generate a docstring stub from a [`ProcDef`], extracting parameter names and
/// defaults from the signature. Mirrors Python `generate_stub_for_proc`.
#[must_use]
pub fn generate_stub_for_proc(
    proc_def: &ProcDef,
    tag_style: DocstringTagStyle,
    decoration: bool,
    decoration_char: char,
    decoration_width: usize,
    indent: &str,
) -> String {
    let info = DocstringInfo {
        brief: format!("TODO: describe {}", proc_def.name),
        description: String::new(),
        params: proc_def
            .params
            .iter()
            .map(|p| stub_param_doc(&p.name, p.has_default, p.default_value.as_deref()))
            .collect(),
        returns: String::new(),
    };
    render_comment_block(
        &info,
        tag_style,
        decoration,
        decoration_char,
        decoration_width,
        indent,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_tags() {
        let info = parse_docstring("# @brief does a thing\n# @param x - the x\n# @param y\n# @return the sum");
        // The `#` prefixes are stripped by the caller; here we feed tag lines.
        let info2 = parse_docstring("@brief does a thing\n@param x - the x\n@param y\n@return the sum");
        assert_eq!(info2.brief, "does a thing");
        assert_eq!(info2.params.len(), 2);
        assert_eq!(info2.params[0].name, "x");
        assert_eq!(info2.params[0].description, "the x");
        assert_eq!(info2.params[1].name, "y");
        assert_eq!(info2.returns, "the sum");
        // The `#`-prefixed form treats the whole line as description text.
        assert!(info.brief.is_empty());
    }

    #[test]
    fn resolve_tag_style_case_insensitive() {
        assert_eq!(resolve_tag_style("plain"), DocstringTagStyle::Plain);
        assert_eq!(resolve_tag_style("PLAIN"), DocstringTagStyle::Plain);
        assert_eq!(resolve_tag_style("doxygen"), DocstringTagStyle::Doxygen);
        assert_eq!(resolve_tag_style("whatever"), DocstringTagStyle::Doxygen);
    }
}
