//! Forensic file builtins: `files`, `file`, `glob`, `grep`.
//!
//! These read the OS files inside the current source's UCS archive through the
//! [`crate::eval::FilesReader`] hook (injected by the f5 CLI; absent in the
//! in-browser console, where they error clearly). Each returns file objects
//! `{path, size, sha256, is_text, content?}` whose fields are ordinary `.field`
//! navigation — so `file("etc/passwd").content` and `glob("home/*/.ssh/*").path`
//! just work. The inventory is scoped to the forensically-relevant members (the
//! same set the report's Forensics tab uses).

use indexmap::IndexMap;

use crate::errors::QueryError;
use crate::eval::{EvalContext, FileOp, FilesReader};
use crate::value::Value;

use super::{BuiltinSpec, as_str, ctx};

/// The reader hook for the current context, or a clear error when unwired.
fn reader(ctx: &EvalContext, who: &str) -> Result<FilesReader, QueryError> {
    ctx.files_reader.clone().ok_or_else(|| {
        QueryError::builtin(format!(
            "{who}: no UCS file reader is wired in this context — run it through \
             the f5 CLI against a .ucs archive"
        ))
    })
}

/// The current source's forensic member inventory (metadata, no content).
fn inventory(ctx: &EvalContext, who: &str) -> Result<Vec<Value>, QueryError> {
    let uri = ctx.root.uri.clone();
    let read = reader(ctx, who)?;
    match read(&uri, &FileOp::Inventory)? {
        Value::List(v) | Value::Stream(v) => Ok(v),
        other => Err(QueryError::builtin(format!(
            "{who}: file reader returned {} (expected a list)",
            other.type_name()
        ))),
    }
}

/// Read one member's content, `None` when the reader has none (binary / absent).
fn read_content(ctx: &EvalContext, who: &str, path: &str) -> Result<Option<String>, QueryError> {
    let uri = ctx.root.uri.clone();
    let read = reader(ctx, who)?;
    match read(&uri, &FileOp::Content(path.to_owned()))? {
        Value::Str(s) => Ok(Some(s)),
        Value::Null => Ok(None),
        other => Err(QueryError::builtin(format!(
            "{who}: content reader returned {} (expected a string)",
            other.type_name()
        ))),
    }
}

fn path_of(v: &Value) -> Option<&str> {
    match v {
        Value::Object(m) => match m.get("path") {
            Some(Value::Str(s)) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn is_text(v: &Value) -> bool {
    matches!(v, Value::Object(m) if matches!(m.get("is_text"), Some(Value::Bool(true))))
}

/// Minimal shell-style glob → regex: `*` matches any run, `?` one character;
/// everything else is literal. Matched against the whole path.
fn glob_match(pattern: &str, text: &str) -> bool {
    let mut re = String::with_capacity(pattern.len() + 8);
    re.push('^');
    for ch in pattern.chars() {
        match ch {
            '*' => re.push_str(".*"),
            '?' => re.push('.'),
            c if r".+()|[]{}^$\".contains(c) => {
                re.push('\\');
                re.push(c);
            }
            c => re.push(c),
        }
    }
    re.push('$');
    regex::Regex::new(&re).is_ok_and(|r| r.is_match(text))
}

/// `files` — the forensic member inventory (`path` / `size` / `sha256` / `is_text`).
fn bi_files(_args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    Ok(Value::List(inventory(ctx, "files")?))
}

/// `glob(pattern)` — inventory entries whose path matches a shell glob.
fn bi_glob(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    let pat = as_str(&args[0], "glob", 1)?;
    let inv = inventory(ctx, "glob")?;
    Ok(Value::List(
        inv.into_iter()
            .filter(|f| path_of(f).is_some_and(|p| glob_match(&pat, p)))
            .collect(),
    ))
}

/// `file(path)` — one member by exact path, with `content` attached for text
/// files (`null` for binary / missing / sensitive). Returns `null` if absent.
fn bi_file(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    let want = as_str(&args[0], "file", 1)?;
    let inv = inventory(ctx, "file")?;
    let Some(entry) = inv.into_iter().find(|f| path_of(f) == Some(want.as_str())) else {
        return Ok(Value::Null);
    };
    let text = is_text(&entry);
    let mut m = match entry {
        Value::Object(m) => m,
        _ => IndexMap::new(),
    };
    let content = if text {
        read_content(ctx, "file", &want)?.map_or(Value::Null, Value::Str)
    } else {
        Value::Null
    };
    m.insert("content".into(), content);
    Ok(Value::Object(m))
}

/// `grep(pattern[, path_glob])` — regex-search text members (optionally
/// glob-scoped), returning `{path, line, text}` rows for each match.
fn bi_grep(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    let pattern = as_str(&args[0], "grep", 1)?;
    let re = regex::Regex::new(&pattern)
        .map_err(|e| QueryError::builtin(format!("grep: invalid regex: {e}")))?;
    let path_glob = match args.get(1) {
        Some(v) => Some(as_str(v, "grep", 2)?),
        None => None,
    };
    let inv = inventory(ctx, "grep")?;
    let mut hits: Vec<Value> = Vec::new();
    for f in &inv {
        let Some(p) = path_of(f) else { continue };
        if !is_text(f) {
            continue;
        }
        if let Some(g) = &path_glob
            && !glob_match(g, p)
        {
            continue;
        }
        let Some(content) = read_content(ctx, "grep", p)? else {
            continue;
        };
        for (i, line) in content.lines().enumerate() {
            if re.is_match(line) {
                let mut row = IndexMap::new();
                row.insert("path".into(), Value::Str(p.to_owned()));
                row.insert("line".into(), Value::Int(i64::try_from(i + 1).unwrap_or(i64::MAX)));
                row.insert("text".into(), Value::Str(line.to_owned()));
                hits.push(Value::Object(row));
            }
        }
    }
    Ok(Value::List(hits))
}

pub(crate) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        ctx("files", "forensic", 0, Some(0), bi_files),
        ctx("glob", "forensic", 1, Some(1), bi_glob),
        ctx("file", "forensic", 1, Some(1), bi_file),
        ctx("grep", "forensic", 1, Some(2), bi_grep),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matching() {
        assert!(glob_match("home/*/.ssh/*", "home/admin/.ssh/authorized_keys"));
        assert!(glob_match("etc/*.conf", "etc/syslog-ng.conf"));
        assert!(!glob_match("etc/*.conf", "etc/passwd"));
        assert!(glob_match("etc/passwd", "etc/passwd"));
        assert!(glob_match("root/.??shrc", "root/.bashrc"));
        // Regex metacharacters in the pattern are literal.
        assert!(glob_match("a.b", "a.b"));
        assert!(!glob_match("a.b", "axb"));
    }
}
