//! Pure utility functions for compile-time folding.
//!
//! These are standalone helpers with no emitter state — used by every
//! other codegen submodule.  Ported from `core/compiler/codegen/_helpers.py`.

/// Tcl 9.0 default trim characters — pushed when `string trim` is
/// called without an explicit chars argument.  Includes ASCII
/// whitespace, NUL, and all Unicode category Zs space separators.
pub const DEFAULT_TRIM_CHARS: &str = "\t\n\x0b\x0c\r \
    \x00\u{0085}\u{00a0}\
    \u{1680}\u{180e}\
    \u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\
    \u{2008}\u{2009}\u{200a}\u{200b}\u{2028}\u{2029}\u{202f}\u{205f}\
    \u{3000}\u{feff}";

/// Split a constant Tcl list string into elements (compile-time only).
///
/// Handles braces, quotes, and backslash escaping.  Does not validate
/// syntax strictly — used only for known-good constant arguments.
#[must_use]
pub fn split_list_simple(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut result = Vec::new();
    let mut i = 0;

    while i < n {
        // Skip whitespace
        while i < n && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
            i += 1;
        }
        if i >= n {
            break;
        }

        if bytes[i] == b'{' {
            // Braced element
            let mut level: u32 = 1;
            i += 1;
            let start = i;
            while i < n && level > 0 {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'{' {
                    level += 1;
                } else if bytes[i] == b'}' {
                    level -= 1;
                }
                i += 1;
            }
            // i now points past the closing }
            result.push(text[start..i.saturating_sub(1)].to_owned());
        } else if bytes[i] == b'"' {
            // Quoted element
            i += 1;
            let start = i;
            while i < n && bytes[i] != b'"' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            result.push(text[start..i].to_owned());
            if i < n {
                i += 1; // skip closing "
            }
        } else {
            // Bare word
            let start = i;
            while i < n && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            result.push(text[start..i].to_owned());
        }
    }
    result
}

/// Compute the Tcl string hash (matches `Tcl_HashString`).
#[must_use]
pub fn tcl_string_hash(s: &str) -> u32 {
    let mut h: u32 = 0;
    for ch in s.bytes() {
        h = h
            .wrapping_add(h.wrapping_shl(3))
            .wrapping_add(u32::from(ch));
    }
    h
}

/// Return jump-table entries in Tcl hash-table iteration order.
///
/// Tcl hash tables iterate buckets 0..N-1 and within each bucket
/// entries are in LIFO order (most recently inserted first).
/// The initial bucket count is 4.
#[must_use]
pub fn tcl_hash_table_order(entries: &[(String, String)]) -> Vec<(String, String)> {
    let mut num_buckets: usize = 4;
    // Grow buckets if needed (Tcl doubles when entries >= 3 * buckets).
    while entries.len() >= 3 * num_buckets {
        num_buckets *= 2;
    }

    // Assign entries to buckets in insertion order.
    let mut buckets: HashMap<usize, Vec<(String, String)>> = HashMap::new();
    for item in entries {
        let bucket = (tcl_string_hash(&item.0) as usize) & (num_buckets - 1);
        buckets.entry(bucket).or_default().push(item.clone());
    }

    // Iterate buckets 0..N-1; within each bucket reverse (LIFO).
    let mut result = Vec::with_capacity(entries.len());
    for b in 0..num_buckets {
        if let Some(bucket_entries) = buckets.get(&b) {
            for item in bucket_entries.iter().rev() {
                result.push(item.clone());
            }
        }
    }
    result
}

use std::collections::HashMap;

/// Format a string as a canonical Tcl list element (brace quoting).
#[must_use]
pub fn tcl_list_element(s: &str) -> String {
    if s.is_empty() {
        return "{}".to_owned();
    }

    let needs_quoting = s.bytes().any(|ch| {
        matches!(
            ch,
            b' ' | b'\t' | b'\n' | b'\r' | b'{' | b'}' | b'"' | b'\\' | b';' | b'$' | b'[' | b']'
        )
    });

    if !needs_quoting {
        return s.to_owned();
    }

    // Prefer brace quoting when the string has balanced braces
    // and does not end with a backslash.
    if !s.ends_with('\\')
        && s.bytes().filter(|&b| b == b'{').count() == s.bytes().filter(|&b| b == b'}').count()
    {
        let mut out = String::with_capacity(s.len() + 2);
        out.push('{');
        out.push_str(s);
        out.push('}');
        return out;
    }

    // Fall back to backslash quoting.
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        if matches!(
            ch,
            ' ' | '\t' | '\n' | '\r' | '{' | '}' | '"' | '\\' | ';' | '$' | '[' | ']'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// A part of a parsed substitution template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubstPart {
    /// Literal text.
    Lit(String),
    /// Variable reference (`$varname`).
    Var(String),
    /// Braced scalar reference (`$={name}`).
    Scalar(String),
    /// Command substitution (`[cmd ...]`).
    Cmd(String),
}

/// Parse a substitution template into parts.
///
/// Returns `None` if the template contains constructs we cannot
/// inline (unbalanced brackets, bare `$` without a name, etc.).
#[must_use]
pub fn parse_subst_template(template: &str) -> Option<Vec<SubstPart>> {
    let bytes = template.as_bytes();
    let n = bytes.len();
    let mut parts = Vec::new();
    let mut i = 0;
    let mut buf = String::new();

    while i < n {
        let ch = bytes[i];

        if ch == b'[' {
            // Command substitution
            if !buf.is_empty() {
                parts.push(SubstPart::Lit(std::mem::take(&mut buf)));
            }
            let mut depth: u32 = 0;
            let start = i;
            while i < n {
                if bytes[i] == b'[' {
                    depth += 1;
                } else if bytes[i] == b']' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                i += 1;
            }
            if depth != 0 {
                return None;
            }
            parts.push(SubstPart::Cmd(template[start..i].to_owned()));
            continue;
        }

        if ch == b'\\' {
            if i + 1 < n {
                let next_ch = bytes[i + 1];
                let replacement = match next_ch {
                    b'a' => '\x07',
                    b'b' => '\x08',
                    b'f' => '\x0c',
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'v' => '\x0b',
                    b'\\' => '\\',
                    _ => char::from(next_ch),
                };
                buf.push(replacement);
                i += 2;
            } else {
                buf.push('\\');
                i += 1;
            }
            continue;
        }

        if ch == b'$' {
            if !buf.is_empty() {
                parts.push(SubstPart::Lit(std::mem::take(&mut buf)));
            }
            i += 1;
            if i >= n {
                return None;
            }
            if bytes[i] == b'=' && i + 1 < n && bytes[i + 1] == b'{' {
                // Braced scalar: $={name}
                let end = template[i + 2..].find('}').map(|p| i + 2 + p)?;
                parts.push(SubstPart::Scalar(template[i + 2..end].to_owned()));
                i = end + 1;
            } else if bytes[i] == b'{' {
                // Braced variable: ${name}
                let end = template[i + 1..].find('}').map(|p| i + 1 + p)?;
                parts.push(SubstPart::Var(template[i + 1..end].to_owned()));
                i = end + 1;
            } else {
                // Bare variable: $varname
                let start = i;
                while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                if i == start {
                    return None;
                }
                parts.push(SubstPart::Var(template[start..i].to_owned()));
            }
            continue;
        }

        buf.push(char::from(ch));
        i += 1;
    }

    if !buf.is_empty() {
        parts.push(SubstPart::Lit(buf));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}

/// Convert a simple anchored regex to an equivalent glob pattern.
///
/// Handles:
/// - `{hello}` → `*hello*` (unanchored)
/// - `{^abc}` → `abc*` (left-anchored)
/// - `{abc$}` → `*abc` (right-anchored)
/// - `{^abc$}` → `abc` (fully anchored)
///
/// Returns `None` if the pattern contains regex metacharacters that
/// cannot be expressed as a glob.
#[must_use]
pub fn regexp_to_glob(pattern: &str) -> Option<String> {
    const REGEX_SPECIAL: &[u8] = b".+*?[](){}|\\";

    let pattern = pattern
        .strip_prefix('{')
        .and_then(|p| p.strip_suffix('}'))
        .unwrap_or(pattern);

    if pattern.is_empty() {
        return None;
    }

    let left_anchor = pattern.starts_with('^');
    let right_anchor = pattern.ends_with('$');

    let core = &pattern[usize::from(left_anchor)..pattern.len() - usize::from(right_anchor)];
    if core.is_empty() {
        return None;
    }

    if core.bytes().any(|b| REGEX_SPECIAL.contains(&b)) {
        return None;
    }

    Some(match (left_anchor, right_anchor) {
        (true, true) => core.to_owned(),
        (true, false) => format!("{core}*"),
        (false, true) => format!("*{core}"),
        (false, false) => format!("*{core}*"),
    })
}

/// Constant-fold a command with all-literal args to a single string.
///
/// `prefix` is the command text including trailing space, e.g.
/// `"list "` or `"dict create "`.  Returns `None` when the value
/// doesn't match or contains substitutions.
#[must_use]
pub fn fold_cmd_args(value: &str, prefix: &str) -> Option<String> {
    let full_prefix = format!("[{prefix}");
    let inner = value
        .strip_prefix(&full_prefix)
        .and_then(|s| s.strip_suffix(']'))?;

    // Resolve backslash-newline continuations to a single space.
    let inner = resolve_backslash_newline(inner);

    // Cannot fold if UNBRACED arguments contain substitutions.
    let mut depth: i32 = 0;
    for ch in inner.bytes() {
        match ch {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'$' | b'[' if depth == 0 => return None,
            _ => {}
        }
    }

    // Parse the literal arguments and re-format as a canonical Tcl list.
    let args = split_list_simple(&inner);
    Some(
        args.iter()
            .map(|a| tcl_list_element(a))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Resolve backslash-newline continuations (`\<newline><whitespace>`)
/// to a single space, matching `re.sub(r"\\\n\s*", " ", s)`.
fn resolve_backslash_newline(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            result.push(' ');
            i += 2;
            // Skip trailing whitespace
            while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                i += 1;
            }
        } else {
            result.push(char::from(bytes[i]));
            i += 1;
        }
    }
    result
}

/// Constant-fold `[list arg1 arg2 ...]` to the result string.
#[must_use]
pub fn fold_list_cmd(value: &str) -> Option<String> {
    fold_cmd_args(value, "list ")
}

/// Constant-fold `[dict create k v ...]` to the result string.
#[must_use]
pub fn fold_dict_create_cmd(value: &str) -> Option<String> {
    fold_cmd_args(value, "dict create ")
}

/// Constant-fold `[format "..." arg ...]` → result string.
///
/// Only handles simple `%s` and `%d` conversions with literal args.
/// Returns `None` if the format cannot be folded.
#[must_use]
pub fn try_format_fold(value: &str) -> Option<String> {
    let inner = value.strip_prefix("[format ")?.strip_suffix(']')?;

    // Parse the command parts (format string + arguments)
    let parts = parse_format_parts(inner);
    if parts.is_empty() {
        return None;
    }

    let fmt = &parts[0];
    let args = &parts[1..];

    // Process format specifiers
    let fmt_bytes = fmt.as_bytes();
    let mut result = String::new();
    let mut ai = 0;
    let mut fi = 0;

    while fi < fmt_bytes.len() {
        if fmt_bytes[fi] == b'%' && fi + 1 < fmt_bytes.len() {
            match fmt_bytes[fi + 1] {
                b's' if ai < args.len() => {
                    result.push_str(&args[ai]);
                    ai += 1;
                    fi += 2;
                }
                b'd' if ai < args.len() => {
                    let n: i64 = args[ai].parse().ok()?;
                    result.push_str(&n.to_string());
                    ai += 1;
                    fi += 2;
                }
                b'%' => {
                    result.push('%');
                    fi += 2;
                }
                _ => return None,
            }
        } else {
            result.push(char::from(fmt_bytes[fi]));
            fi += 1;
        }
    }

    Some(result)
}

/// Parse the argument parts of a `format` command.
fn parse_format_parts(inner: &str) -> Vec<String> {
    let bytes = inner.as_bytes();
    let mut parts = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' => i += 1,
            b'"' => {
                // Quoted string
                i += 1;
                let mut buf = String::new();
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        if i + 1 < bytes.len() {
                            buf.push(char::from(bytes[i]));
                            buf.push(char::from(bytes[i + 1]));
                            i += 2;
                        } else {
                            buf.push(char::from(bytes[i]));
                            i += 1;
                        }
                    } else {
                        buf.push(char::from(bytes[i]));
                        i += 1;
                    }
                }
                if i < bytes.len() {
                    i += 1; // skip closing "
                }
                parts.push(buf);
            }
            b'{' => {
                // Braced string
                let mut depth: u32 = 0;
                let start = i;
                while i < bytes.len() {
                    if bytes[i] == b'{' {
                        depth += 1;
                    } else if bytes[i] == b'}' {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    i += 1;
                }
                parts.push(inner[start + 1..i.saturating_sub(1)].to_owned());
            }
            _ => {
                // Bare word
                let start = i;
                while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t') {
                    i += 1;
                }
                parts.push(inner[start..i].to_owned());
            }
        }
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- split_list_simple --

    #[test]
    fn split_list_simple_basic() {
        assert_eq!(split_list_simple("a b c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn split_list_simple_braces() {
        assert_eq!(
            split_list_simple("{hello world} foo"),
            vec!["hello world", "foo"]
        );
    }

    #[test]
    fn split_list_simple_quotes() {
        assert_eq!(
            split_list_simple(r#""hello world" foo"#),
            vec!["hello world", "foo"]
        );
    }

    #[test]
    fn split_list_simple_nested_braces() {
        assert_eq!(split_list_simple("{a {b c}} d"), vec!["a {b c}", "d"]);
    }

    #[test]
    fn split_list_simple_empty() {
        assert!(split_list_simple("").is_empty());
        assert!(split_list_simple("   ").is_empty());
    }

    #[test]
    fn split_list_simple_backslash() {
        assert_eq!(split_list_simple(r"a\ b c"), vec![r"a\ b", "c"]);
    }

    // -- tcl_string_hash --

    #[test]
    fn tcl_string_hash_empty() {
        assert_eq!(tcl_string_hash(""), 0);
    }

    #[test]
    fn tcl_string_hash_deterministic() {
        let h1 = tcl_string_hash("hello");
        let h2 = tcl_string_hash("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn tcl_string_hash_different() {
        assert_ne!(tcl_string_hash("a"), tcl_string_hash("b"));
    }

    // -- tcl_hash_table_order --

    #[test]
    fn hash_table_order_preserves_all() {
        let entries = vec![
            ("a".into(), "L1".into()),
            ("b".into(), "L2".into()),
            ("c".into(), "L3".into()),
        ];
        let ordered = tcl_hash_table_order(&entries);
        assert_eq!(ordered.len(), 3);
        // All original entries present (order may differ due to hashing)
        for e in &entries {
            assert!(ordered.contains(e));
        }
    }

    #[test]
    fn hash_table_order_bucket_growth() {
        // 12+ entries should trigger bucket growth
        let entries: Vec<(String, String)> = (0..15)
            .map(|i| (format!("key{i}"), format!("L{i}")))
            .collect();
        let ordered = tcl_hash_table_order(&entries);
        assert_eq!(ordered.len(), 15);
    }

    // -- tcl_list_element --

    #[test]
    fn tcl_list_element_empty() {
        assert_eq!(tcl_list_element(""), "{}");
    }

    #[test]
    fn tcl_list_element_simple() {
        assert_eq!(tcl_list_element("hello"), "hello");
    }

    #[test]
    fn tcl_list_element_space() {
        assert_eq!(tcl_list_element("hello world"), "{hello world}");
    }

    #[test]
    fn tcl_list_element_backslash_end() {
        // Ends with backslash → can't brace-quote → backslash quoting
        let result = tcl_list_element("foo\\");
        assert!(result.contains('\\'));
        assert!(!result.starts_with('{'));
    }

    // -- parse_subst_template --

    #[test]
    fn subst_template_lit_only() {
        let parts = parse_subst_template("hello").unwrap();
        assert_eq!(parts, vec![SubstPart::Lit("hello".into())]);
    }

    #[test]
    fn subst_template_var() {
        let parts = parse_subst_template("Hello, $name!").unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], SubstPart::Lit("Hello, ".into()));
        assert_eq!(parts[1], SubstPart::Var("name".into()));
        assert_eq!(parts[2], SubstPart::Lit("!".into()));
    }

    #[test]
    fn subst_template_braced_var() {
        let parts = parse_subst_template("${foo}bar").unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], SubstPart::Var("foo".into()));
        assert_eq!(parts[1], SubstPart::Lit("bar".into()));
    }

    #[test]
    fn subst_template_cmd() {
        let parts = parse_subst_template("x[cmd arg]y").unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], SubstPart::Lit("x".into()));
        assert_eq!(parts[1], SubstPart::Cmd("[cmd arg]".into()));
        assert_eq!(parts[2], SubstPart::Lit("y".into()));
    }

    #[test]
    fn subst_template_backslash() {
        let parts = parse_subst_template(r"a\nb").unwrap();
        assert_eq!(parts, vec![SubstPart::Lit("a\nb".into())]);
    }

    #[test]
    fn subst_template_bare_dollar() {
        assert!(parse_subst_template("$").is_none());
    }

    #[test]
    fn subst_template_empty() {
        assert!(parse_subst_template("").is_none());
    }

    #[test]
    fn subst_template_scalar() {
        let parts = parse_subst_template("$={a(1)}rest").unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], SubstPart::Scalar("a(1)".into()));
        assert_eq!(parts[1], SubstPart::Lit("rest".into()));
    }

    // -- regexp_to_glob --

    #[test]
    fn regexp_to_glob_unanchored() {
        assert_eq!(regexp_to_glob("{hello}"), Some("*hello*".into()));
    }

    #[test]
    fn regexp_to_glob_left_anchored() {
        assert_eq!(regexp_to_glob("{^abc}"), Some("abc*".into()));
    }

    #[test]
    fn regexp_to_glob_right_anchored() {
        assert_eq!(regexp_to_glob("{abc$}"), Some("*abc".into()));
    }

    #[test]
    fn regexp_to_glob_fully_anchored() {
        assert_eq!(regexp_to_glob("{^abc$}"), Some("abc".into()));
    }

    #[test]
    fn regexp_to_glob_metachar() {
        assert_eq!(regexp_to_glob("{a.*b}"), None);
    }

    #[test]
    fn regexp_to_glob_empty() {
        assert_eq!(regexp_to_glob("{}"), None);
    }

    #[test]
    fn regexp_to_glob_without_braces() {
        assert_eq!(regexp_to_glob("^abc$"), Some("abc".into()));
    }

    // -- fold_cmd_args / fold_list_cmd / fold_dict_create_cmd --

    #[test]
    fn fold_list_cmd_basic() {
        assert_eq!(fold_list_cmd("[list a b c]"), Some("a b c".into()));
    }

    #[test]
    fn fold_list_cmd_braced() {
        assert_eq!(
            fold_list_cmd("[list {hello world} b]"),
            Some("{hello world} b".into())
        );
    }

    #[test]
    fn fold_list_cmd_substitution() {
        // Contains $ → cannot fold
        assert_eq!(fold_list_cmd("[list $x b]"), None);
    }

    #[test]
    fn fold_list_cmd_no_match() {
        assert_eq!(fold_list_cmd("not a list"), None);
    }

    #[test]
    fn fold_dict_create_basic() {
        assert_eq!(
            fold_dict_create_cmd("[dict create a 1 b 2]"),
            Some("a 1 b 2".into())
        );
    }

    // -- try_format_fold --

    #[test]
    fn format_fold_simple_s() {
        assert_eq!(
            try_format_fold("[format \"%s world\" hello]"),
            Some("hello world".into())
        );
    }

    #[test]
    fn format_fold_simple_d() {
        assert_eq!(try_format_fold("[format \"%d\" 42]"), Some("42".into()));
    }

    #[test]
    fn format_fold_percent_escape() {
        assert_eq!(try_format_fold("[format \"100%%\"]"), Some("100%".into()));
    }

    #[test]
    fn format_fold_no_match() {
        assert_eq!(try_format_fold("not format"), None);
    }
}
