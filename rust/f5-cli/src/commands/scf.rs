//! tmsh → SCF normalisation — port of `dialects/f5/bigip/tmsh_parse.py`
//! (`_to_scf` / `looks_like_tmsh_output` / `normalise_tmsh_to_scf`).
//!
//! The config verbs accept `tmsh create` / `tmsh modify` script output as well
//! as SCF. Without this, the SCF parser would read `tmsh` as the module name and
//! record generic objects instead of the real pools / virtuals / etc. `to_scf`
//! strips the top-level `tmsh create`/`modify` prefixes (only at brace depth
//! zero, so an embedded `tmsh create …` inside an iRule body survives).

const TMSH_PREFIXES: &[&str] = &["tmsh create ", "tmsh modify "];

/// Normalise tmsh output to SCF when it looks like tmsh, else return it
/// unchanged (mirrors `_to_scf`).
#[must_use]
pub fn to_scf(text: &str) -> String {
    if looks_like_tmsh_output(text) {
        normalise_tmsh_to_scf(text)
    } else {
        text.to_owned()
    }
}

/// True when the first non-blank, non-comment line starts with a tmsh prefix
/// (mirrors `looks_like_tmsh_output`).
fn looks_like_tmsh_output(text: &str) -> bool {
    for line in text.lines() {
        let stripped = line.trim_start();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        return TMSH_PREFIXES.iter().any(|p| stripped.starts_with(p));
    }
    false
}

/// Strip `tmsh create`/`modify` prefixes from stanza headers at brace depth
/// zero (mirrors `normalise_tmsh_to_scf`).
fn normalise_tmsh_to_scf(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth: i32 = 0;
    for line in text.split_inclusive('\n') {
        let mut rendered = line.to_owned();
        if depth == 0 {
            let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
            let (indent, rest) = line.split_at(indent_len);
            for prefix in TMSH_PREFIXES {
                if let Some(stripped) = rest.strip_prefix(prefix) {
                    rendered = format!("{indent}{stripped}");
                    break;
                }
            }
        }
        depth += net_brace_delta(&rendered);
        out.push_str(&rendered);
    }
    out
}

/// `{` minus `}` count outside quoted strings and comments (mirrors
/// `_net_brace_delta`): a `#` outside a string starts a Tcl-style comment.
fn net_brace_delta(line: &str) -> i32 {
    let bytes = line.as_bytes();
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if c == b'"' {
            in_string = !in_string;
        } else if !in_string {
            match c {
                b'#' => break,
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
        }
        i += 1;
    }
    depth
}
