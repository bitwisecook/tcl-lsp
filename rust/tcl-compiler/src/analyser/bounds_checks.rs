//! Loop-termination index-bounds checks (W230-W242) — GAP-A4.
//!
//! Port of `core/analysis/checks/_bounds.py`.  Lands the
//! **loop-termination** family (W240 / W241 / W242 over `while` / `for`,
//! including the `for`-step provably-infinite heuristic) and the
//! **index-bounds** family (W230 over `lindex` / `lrange` / `lreplace`,
//! W231 over `lset`, W232 over `string index` / `range` / `replace`).
//! W242 (unprovable termination) is emitted here like Python's
//! `analyse`; its default-off opt-in is a consuming-layer concern.
//!
//! The analysis is intentionally shallow — it inspects the literal text
//! of the condition and body.  A dynamic condition (anything not a
//! constant true/false literal) yields no diagnostic, avoiding false
//! positives.

use tcl_lexer::Token;

use super::types::{Diagnostic, Severity};

/// W240 (constant-false condition → dead body) / W241 (constant-true
/// condition with no `break`/`return`/`error`/`exit` → provably
/// infinite) for `while` / `for`.  `args` / `arg_tokens` exclude the
/// command name.  Mirrors the constant-condition arm of
/// `check_loop_termination`.
pub(crate) fn loop_termination_diagnostics(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[Token],
) -> Vec<Diagnostic> {
    // (init, cond, step, body, cond_tok) — init/step empty for `while`.
    let (init_text, cond_text, step_text, body_text, cond_tok) = match cmd_name {
        "while" if args.len() >= 2 && arg_tokens.len() >= 2 => {
            ("", args[0].as_str(), "", args[1].as_str(), &arg_tokens[0])
        }
        "for" if args.len() >= 4 && arg_tokens.len() >= 4 => (
            args[0].as_str(),
            args[1].as_str(),
            args[2].as_str(),
            args[3].as_str(),
            &arg_tokens[1],
        ),
        _ => return Vec::new(),
    };

    match condition_constant(cond_text) {
        Some(false) => {
            return vec![Diagnostic {
                code: "W240".to_string(),
                span: cond_tok.span,
                message: format!("{cmd_name} condition is constant false; body never executes."),
                severity: Severity::Warning,
                fixes: Vec::new(),
            }]
        }
        Some(true) if !body_may_exit(body_text) => {
            return vec![Diagnostic {
                code: "W241".to_string(),
                span: cond_tok.span,
                message: format!(
                    "{cmd_name} is provably infinite: condition is constant true and body has no \
                     break/return/error/exit."
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            }]
        }
        Some(true) => return Vec::new(),
        None => {}
    }

    // `for {init} {cond} {step} body` provably-infinite counter shape.
    if cmd_name == "for" {
        if let Some(reason) = for_is_provably_infinite(init_text, cond_text, step_text, body_text) {
            return vec![Diagnostic {
                code: "W241".to_string(),
                span: cond_tok.span,
                message: format!("for loop is provably infinite: {reason}"),
                severity: Severity::Warning,
                fixes: Vec::new(),
            }];
        }
    }

    // W242 (default-off): a counter variable appears in the condition but
    // neither the step nor the body provably modifies it.  Reported on
    // the condition token, mirroring W240/W241.  Like Python's
    // `core.analysis.analyse`, the analyser always emits W242; the
    // default-off opt-in is applied by the consuming LSP/config layer.
    if let Some(var) = extract_counter_name(cond_text) {
        if !loop_modifies_var(&var, step_text, body_text) {
            return vec![Diagnostic {
                code: "W242".to_string(),
                span: cond_tok.span,
                message: format!(
                    "{cmd_name} termination cannot be proven: variable '{var}' in the \
                     condition is never modified by the step or body."
                ),
                severity: Severity::Hint,
                fixes: Vec::new(),
            }];
        }
    }
    Vec::new()
}

/// Return the first `$var` / `${var}` referenced by a condition
/// expression, or `None`.  Mirrors `_extract_counter_name`.
fn extract_counter_name(cond: &str) -> Option<String> {
    let re = regex::Regex::new(r"\$\{?(\w+)\}?").expect("valid counter regex");
    re.captures(strip_braces(cond))
        .map(|cap| cap[1].to_string())
}

/// True when the step expression or the body provably updates `var`.
/// Mirrors `_loop_modifies_var`.
fn loop_modifies_var(var: &str, step: &str, body: &str) -> bool {
    if !step.is_empty() {
        if let Some((step_var, _)) = parse_step_incr(step) {
            if step_var == var {
                return true;
            }
        }
        if body_writes_var(strip_braces(step), var) {
            return true;
        }
    }
    body_writes_var(body, var)
}

/// Prove that a `for {set v INT} {$v OP INT} {incr v INT} body` loop
/// never terminates (no write to `v` elsewhere); returns the reason.
/// Mirrors `_for_is_provably_infinite`.
fn for_is_provably_infinite(init: &str, cond: &str, step: &str, body: &str) -> Option<String> {
    let (var_c, op, bound) = parse_simple_for_cond(cond)?;
    let (var_i, start) = parse_init_var_value(init)?;
    let (var_s, delta) = parse_step_incr(step)?;
    if var_c != var_i || var_c != var_s {
        return None;
    }
    if body_writes_var(body, &var_c) || body_may_exit(body) {
        return None;
    }
    let counter = format!("${var_c}");
    // Step of zero with the condition initially true → infinite.
    if delta == 0 && cond_true_at(&op, start, bound) {
        return Some("step is zero ('incr' with 0) and condition holds on entry".to_string());
    }
    // Wrong-direction step: moving away from the bound.
    if matches!(op.as_str(), "<" | "<=") && cond_true_at(&op, start, bound) && delta < 0 {
        return Some(format!(
            "counter {counter} starts at {start}, moves by {delta} per step, and compares {op} \
             {bound} (never reached)"
        ));
    }
    if matches!(op.as_str(), ">" | ">=") && cond_true_at(&op, start, bound) && delta > 0 {
        return Some(format!(
            "counter {counter} starts at {start}, moves by {delta} per step, and compares {op} \
             {bound} (never reached)"
        ));
    }
    if matches!(op.as_str(), "!=" | "ne") {
        if delta == 0 && start != bound {
            return Some(format!("counter {counter} never changes and !={bound}"));
        }
        if delta != 0 && start != bound {
            let diff = bound - start;
            if diff * delta < 0 {
                return Some(format!(
                    "counter {counter} starts at {start}, moves by {delta} per step, never \
                     reaches {bound}"
                ));
            }
            if diff % delta != 0 {
                return Some(format!(
                    "counter {counter} starts at {start}, moves by {delta} per step, never \
                     exactly equals {bound}"
                ));
            }
        }
    }
    None
}

/// Evaluate a simple comparison at a concrete value.  Mirrors
/// `_cond_true_at`.
fn cond_true_at(op: &str, value: i64, bound: i64) -> bool {
    match op {
        "<" => value < bound,
        "<=" => value <= bound,
        ">" => value > bound,
        ">=" => value >= bound,
        "==" | "eq" => value == bound,
        "!=" | "ne" => value != bound,
        _ => false,
    }
}

/// `(var, op, bound)` when `cond` is exactly `$v OP literal` or
/// `literal OP $v` (no compound `&&` / `||` / `?` / `!`).  Mirrors
/// `_parse_simple_for_cond`.
fn parse_simple_for_cond(cond: &str) -> Option<(String, String, i64)> {
    let c = strip_braces(cond);
    // Compound markers disqualify (Rust regex has no look-behind, so the
    // lone-`!` case is checked manually: a `!` not part of `!=`).
    if c.contains("&&") || c.contains("||") || c.contains('?') || has_logical_not(c) {
        return None;
    }
    let fwd = regex::Regex::new(
        r"^\s*\$\{?(?P<v>\w+)\}?\s*(?P<op><=|>=|<|>|==|!=|eq|ne)\s*(?P<bound>-?\d+)\s*$",
    )
    .expect("valid counter regex");
    if let Some(m) = fwd.captures(c) {
        return Some((
            m["v"].to_string(),
            m["op"].to_string(),
            m["bound"].parse().ok()?,
        ));
    }
    let rev = regex::Regex::new(
        r"^\s*(?P<bound>-?\d+)\s*(?P<op><=|>=|<|>|==|!=|eq|ne)\s*\$\{?(?P<v>\w+)\}?\s*$",
    )
    .expect("valid reversed counter regex");
    if let Some(m) = rev.captures(c) {
        // Flip the operator so the variable is on the left.
        let op = match &m["op"] {
            "<" => ">",
            ">" => "<",
            "<=" => ">=",
            ">=" => "<=",
            other => other,
        };
        return Some((m["v"].to_string(), op.to_string(), m["bound"].parse().ok()?));
    }
    None
}

/// True when `c` contains a logical-not `!` that is not part of `!=`
/// and not preceded by `< > = !`.  Mirrors the `(?<![<>=!])!(?!=)`
/// alternative of `_COMPOUND_MARKERS_RE`.
fn has_logical_not(c: &str) -> bool {
    let b = c.as_bytes();
    for (i, &ch) in b.iter().enumerate() {
        if ch != b'!' {
            continue;
        }
        let prev_ok = i == 0 || !matches!(b[i - 1], b'<' | b'>' | b'=' | b'!');
        let next_ok = b.get(i + 1) != Some(&b'=');
        if prev_ok && next_ok {
            return true;
        }
    }
    false
}

/// `(var, value)` from an init clause `set v INT`.  Mirrors
/// `_parse_init_var_value`.
fn parse_init_var_value(init: &str) -> Option<(String, i64)> {
    let re = regex::Regex::new(r"^\s*set\s+(\w+)\s+(-?\d+)\s*$").expect("valid init regex");
    let m = re.captures(strip_braces(init).trim())?;
    Some((m[1].to_string(), m[2].parse().ok()?))
}

/// `(var, delta)` from a step clause `incr v ?INT?`.  Mirrors
/// `_parse_step_incr`.
fn parse_step_incr(step: &str) -> Option<(String, i64)> {
    let re = regex::Regex::new(r"^\s*incr\s+(\w+)(?:\s+(-?\d+))?\s*$").expect("valid step regex");
    let m = re.captures(strip_braces(step).trim())?;
    let delta = m.get(2).map_or(Some(1), |g| g.as_str().parse().ok())?;
    Some((m[1].to_string(), delta))
}

/// Shallow scan: does `body` write `var` via `set` / `incr` / `lset` /
/// `append` / `lappend`?  Mirrors `_body_writes_var`.
fn body_writes_var(body: &str, var: &str) -> bool {
    let escaped = regex::escape(var);
    for kw in ["set", "incr", "lset", "append", "lappend"] {
        let re =
            regex::Regex::new(&format!(r"\b{kw}\s+{escaped}\b")).expect("valid body-write regex");
        if re.is_match(body) {
            return true;
        }
    }
    false
}

/// W230: a constant list literal with a constant out-of-range index
/// (`lindex`) or a provably-empty slice (`lrange` / `lreplace`)
/// silently returns empty / clamps.  `args` / `arg_tokens` exclude the
/// command name.  Mirrors `check_list_index_out_of_range` (W231 `lset`
/// needs const-var tracking and is a follow-up).
#[allow(clippy::similar_names)] // first_text/first_tok/first_val read clearly
pub(crate) fn list_index_diagnostics(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[Token],
) -> Vec<Diagnostic> {
    if !matches!(cmd_name, "lindex" | "lrange" | "lreplace")
        || args.len() < 2
        || arg_tokens.len() < 2
    {
        return Vec::new();
    }
    let list_tok = &arg_tokens[0];
    if !is_braced_or_esc(list_tok) || has_subst(&args[0], list_tok) {
        return Vec::new();
    }
    // `args[0]` already has its outer `{…}` delimiter stripped by the
    // segmenter, so split the list content directly — a second
    // `strip_braces` here would wrongly peel a single-element list like
    // `{{a b c}}` (segmented to `{a b c}`) down to its three inner words.
    let length =
        i64::try_from(crate::tcl_expr_eval::split_tcl_list(&args[0]).len()).unwrap_or(i64::MAX);

    if cmd_name == "lindex" {
        return lindex_diagnostics(args, arg_tokens, length);
    }

    // lrange / lreplace: a (first, last) pair that resolves to an empty
    // slice.
    if args.len() < 3 || arg_tokens.len() < 3 || (cmd_name == "lrange" && args.len() != 3) {
        return Vec::new();
    }
    let (first_text, last_text) = (&args[1], &args[2]);
    let (first_tok, last_tok) = (&arg_tokens[1], &arg_tokens[2]);
    if has_subst(first_text, first_tok)
        || !is_literal_index(first_text)
        || has_subst(last_text, last_tok)
        || !is_literal_index(last_text)
    {
        return Vec::new();
    }
    let (Some(first_val), Some(last_val)) = (
        resolve_index(first_text, length),
        resolve_index(last_text, length),
    ) else {
        return Vec::new();
    };
    if !pair_slice_empty(first_val, last_val, length) {
        return Vec::new();
    }
    let verb = if cmd_name == "lrange" {
        "lrange slice is empty".to_string()
    } else if first_val < 0 && last_val < 0 {
        "lreplace prepends instead of replacing (both indices resolve before the list)".to_string()
    } else if first_val >= length && last_val >= length {
        "lreplace appends instead of replacing (both indices resolve past the list)".to_string()
    } else {
        "lreplace touches no element (first > last after clamping)".to_string()
    };
    vec![Diagnostic {
        code: "W230".to_string(),
        span: tcl_lexer::Span::new(first_tok.span.start(), last_tok.span.end()),
        message: format!(
            "{verb}: first='{first_text}' resolves to {first_val}, last='{last_text}' resolves \
             to {last_val} (list has {length} element{}).",
            if length == 1 { "" } else { "s" }
        ),
        severity: Severity::Warning,
        fixes: Vec::new(),
    }]
}

/// The per-index `lindex` arm of W230.
fn lindex_diagnostics(args: &[String], arg_tokens: &[Token], length: i64) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (pos, idx_text) in args.iter().enumerate().skip(1) {
        let Some(idx_tok) = arg_tokens.get(pos) else {
            continue;
        };
        if has_subst(idx_text, idx_tok) || !is_literal_index(idx_text) {
            continue;
        }
        let Some(resolved) = resolve_index(idx_text, length) else {
            continue;
        };
        if (0..length).contains(&resolved) {
            continue;
        }
        out.push(Diagnostic {
            code: "W230".to_string(),
            span: idx_tok.span,
            message: format!(
                "Index '{}' {}; lindex silently returns empty string.",
                idx_text.trim(),
                describe_index(resolved, length)
            ),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }
    out
}

/// W231: an `lset` with a constant index known to be out of range.
/// Unlike `lindex` (which silently returns empty), `lset` raises a
/// runtime error; a plain negative literal always errors, and — when the
/// list length is recoverable from a recent literal `set` — an index past
/// the append slot (`> length`) or below zero errors too.  Mirrors
/// `check_lset_index_out_of_range`.
pub(crate) fn lset_index_diagnostics(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[Token],
    source: &str,
) -> Vec<Diagnostic> {
    if cmd_name != "lset" || args.len() < 3 || arg_tokens.len() < 3 {
        return Vec::new();
    }
    // `lset varName ?index ...? value` — index positions are 1..len-1.
    let index_positions: Vec<usize> = (1..args.len() - 1).collect();
    let nested = index_positions.len() > 1;
    // Multi-index (nested-sublist) forms only fire on plain-negative
    // literals; length-based checks need the single-index top-level form.
    let list_len = if nested {
        None
    } else {
        infer_list_length_from_recent_set(source, &args[0], arg_tokens[0].span.start())
    };

    let mut out = Vec::new();
    for pos in index_positions {
        let idx_text = &args[pos];
        let Some(idx_tok) = arg_tokens.get(pos) else {
            continue;
        };
        if has_subst(idx_text, idx_tok) || !is_literal_index(idx_text) {
            continue;
        }
        let stripped = idx_text.trim();
        // Plain negative integer: always errors.
        if let Some(n) = parse_strict_int(stripped) {
            if n < 0 {
                out.push(Diagnostic {
                    code: "W231".to_string(),
                    span: idx_tok.span,
                    message: format!(
                        "lset index '{stripped}' is negative; \
                         raises 'index out of range' at runtime."
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
                continue;
            }
        }
        // `lset` accepts the append slot (`index == length`); only
        // indices past it or below zero are runtime errors.
        if let Some(length) = list_len {
            let Some(resolved) = resolve_index(stripped, length) else {
                continue;
            };
            if resolved < 0 || resolved > length {
                out.push(Diagnostic {
                    code: "W231".to_string(),
                    span: idx_tok.span,
                    message: format!(
                        "lset index '{stripped}' {}; \
                         raises 'index out of range' at runtime.",
                        describe_index(resolved, length)
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }
    out
}

/// Recover the literal length of `var_name` from the most recent
/// `set var {literal}` before `before_offset`, when that assignment
/// shares the `lset`'s (flat) scope.  Mirrors
/// `_infer_list_length_from_recent_set` (the regex excludes brace-bearing
/// values, so the captured literal can't hide nested braces).
fn infer_list_length_from_recent_set(
    source: &str,
    var_name: &str,
    before_offset: u32,
) -> Option<i64> {
    let before = before_offset as usize;
    if before == 0 || before > source.len() || var_name.is_empty() {
        return None;
    }
    // `set <var> {literal}` at a line start (no lookahead — the trailing
    // `\s*(?:;|$)` matches Python's `\s*(?=\n|$|;)` under `(?m)`).
    let re = regex::Regex::new(r"(?m)^\s*set\s+(\w+)\s+(\{[^{}]*\})\s*(?:;|$)")
        .expect("valid list-set regex");
    let mut best: Option<i64> = None;
    for cap in re.captures_iter(&source[..before]) {
        if &cap[1] != var_name {
            continue;
        }
        let whole = cap.get(0).expect("match 0");
        let between = &source[whole.end()..before];
        if !scope_is_flat(between) {
            continue;
        }
        let literal = &cap[2];
        let inner = &literal[1..literal.len() - 1];
        let len =
            i64::try_from(crate::tcl_expr_eval::split_tcl_list(inner).len()).unwrap_or(i64::MAX);
        best = Some(len);
    }
    best
}

/// True when `between` stays at brace depth 0 and introduces no
/// `proc` / `namespace eval` / `apply` / `try` scope — i.e. the trailing
/// `lset` shares the originating `set`'s scope.  Mirrors `_scope_is_flat`.
fn scope_is_flat(between: &str) -> bool {
    let markers = regex::Regex::new(r"\b(?:proc|namespace\s+eval|apply|try)\b")
        .expect("valid scope-marker regex");
    if markers.is_match(between) {
        return false;
    }
    let bytes = between.as_bytes();
    let mut depth: i64 = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => {
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
        i += 1;
    }
    depth == 0
}

/// True when a `(first, last)` index pair resolves to a provably-empty
/// slice over `length`: both below, both above, clamped first > clamped
/// last, or an empty container.  Mirrors the shared clamp logic in
/// `check_list_index_out_of_range` / `check_string_index_out_of_range`.
fn pair_slice_empty(first: i64, last: i64, length: i64) -> bool {
    if length == 0 {
        return true;
    }
    let both_below = first < 0 && last < 0;
    let both_above = first >= length && last >= length;
    let clamped_first = first.clamp(0, length - 1);
    let clamped_last = last.clamp(0, length - 1);
    both_below || both_above || clamped_first > clamped_last
}

/// W232: a constant `string index` / `range` / `replace` / `insert`
/// into a literal string with a constant out-of-range (or negative)
/// index returns empty / is a no-op.  Mirrors
/// `check_string_index_out_of_range`.
pub(crate) fn string_index_diagnostics(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[Token],
) -> Vec<Diagnostic> {
    if cmd_name != "string" || args.len() < 2 {
        return Vec::new();
    }
    let sub = args[0].as_str();
    let min_args = match sub {
        "index" => 3,
        "range" | "replace" | "insert" => 4,
        _ => return Vec::new(),
    };
    if args.len() < min_args || arg_tokens.len() < min_args {
        return Vec::new();
    }
    let str_tok = &arg_tokens[1];
    let str_text = &args[1];
    // Safe runtime length: braced strings do no backslash processing;
    // a backslash-free ESC word is byte-identical to its runtime form.
    // Back off (None) otherwise.
    let str_len: Option<i64> = if has_subst(str_text, str_tok) || !is_braced_or_esc(str_tok) {
        None
    } else if str_tok.kind == tcl_lexer::TokenType::Str {
        // `str_text` already has its outer `{…}` delimiter stripped by
        // the segmenter; a braced string does no backslash processing,
        // so its char count is its runtime length.  A second
        // `strip_braces` here would wrongly shorten a literal braced
        // string such as `{{hello}}` (segmented to `{hello}`).
        i64::try_from(str_text.chars().count()).ok()
    } else if str_text.contains('\\') {
        None
    } else {
        i64::try_from(str_text.chars().count()).ok()
    };

    if sub == "index" || sub == "insert" {
        return string_single_index(sub, args, arg_tokens, str_len);
    }
    string_pair_index(sub, args, arg_tokens, str_len)
}

/// The single-index `string index` / `string insert` arm of W232.
fn string_single_index(
    sub: &str,
    args: &[String],
    arg_tokens: &[Token],
    str_len: Option<i64>,
) -> Vec<Diagnostic> {
    let (idx_text, idx_tok) = (&args[2], &arg_tokens[2]);
    if has_subst(idx_text, idx_tok) || !is_literal_index(idx_text) {
        return Vec::new();
    }
    let stripped = idx_text.trim();
    // A plain negative literal is always invalid.
    if let Some(n) = parse_strict_int(stripped) {
        if n < 0 {
            return vec![Diagnostic {
                code: "W232".to_string(),
                span: idx_tok.span,
                message: format!(
                    "string {sub}: index '{stripped}' is negative; result is empty or a no-op."
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            }];
        }
    }
    // `string insert` clamps other overshoots; only `string index`
    // flags an in-bounds miss.
    if sub == "index" {
        if let Some(len) = str_len {
            if let Some(resolved) = resolve_index(stripped, len) {
                if !(0..len).contains(&resolved) {
                    return vec![Diagnostic {
                        code: "W232".to_string(),
                        span: idx_tok.span,
                        message: format!(
                            "string index: '{stripped}' {}; returns empty string.",
                            describe_index_string(resolved, len)
                        ),
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    }];
                }
            }
        }
    }
    Vec::new()
}

/// The `string range` / `string replace` `(first, last)` arm of W232.
fn string_pair_index(
    sub: &str,
    args: &[String],
    arg_tokens: &[Token],
    str_len: Option<i64>,
) -> Vec<Diagnostic> {
    let (first_text, last_text) = (&args[2], &args[3]);
    let (first_tok, last_tok) = (&arg_tokens[2], &arg_tokens[3]);
    if has_subst(first_text, first_tok)
        || !is_literal_index(first_text)
        || has_subst(last_text, last_tok)
        || !is_literal_index(last_text)
    {
        return Vec::new();
    }
    let verb = if sub == "range" {
        "slice is empty"
    } else {
        "replace is a no-op"
    };
    let span = tcl_lexer::Span::new(first_tok.span.start(), last_tok.span.end());

    // Both plain negative literals → always empty, even for a dynamic
    // string.
    if let (Some(f), Some(l)) = (
        parse_strict_int(first_text.trim()),
        parse_strict_int(last_text.trim()),
    ) {
        if f < 0 && l < 0 {
            return vec![Diagnostic {
                code: "W232".to_string(),
                span,
                message: format!(
                    "string {sub}: both indices are negative ('{first_text}', '{last_text}'); \
                     {verb}."
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            }];
        }
    }
    let Some(len) = str_len else {
        return Vec::new();
    };
    let (Some(first_val), Some(last_val)) = (
        resolve_index(first_text, len),
        resolve_index(last_text, len),
    ) else {
        return Vec::new();
    };
    if !pair_slice_empty(first_val, last_val, len) {
        return Vec::new();
    }
    vec![Diagnostic {
        code: "W232".to_string(),
        span,
        message: format!(
            "string {sub}: {verb}: first='{first_text}' resolves to {first_val}, \
             last='{last_text}' resolves to {last_val} (string has {len} character{}).",
            if len == 1 { "" } else { "s" }
        ),
        severity: Severity::Warning,
        fixes: Vec::new(),
    }]
}

/// Human-readable description of a resolved out-of-range string index.
/// Mirrors `_describe_index_string`.
fn describe_index_string(resolved: i64, length: i64) -> String {
    if resolved < 0 {
        format!("resolves to {resolved} (before start of string)")
    } else {
        format!(
            "resolves to {resolved} (string has {length} character{})",
            if length == 1 { "" } else { "s" }
        )
    }
}

/// Token is a literal word (braced string or plain word).  Mirrors
/// `_is_braced_or_esc`.
fn is_braced_or_esc(tok: &Token) -> bool {
    matches!(
        tok.kind,
        tcl_lexer::TokenType::Str | tcl_lexer::TokenType::Esc
    )
}

/// Word contains a variable / command substitution.  Mirrors
/// `_has_subst`.
fn has_subst(text: &str, tok: &Token) -> bool {
    matches!(
        tok.kind,
        tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
    ) || text.contains('$')
        || text.contains('[')
}

/// `s` is a constant index expression we can evaluate (`end`, an
/// integer, or `end±N`).  Mirrors `_is_literal_index`.
fn is_literal_index(s: &str) -> bool {
    let s = s.trim();
    s == "end" || parse_strict_int(s).is_some() || end_offset(s).is_some()
}

/// Resolve a constant index to an absolute offset given `length`, or
/// `None` when `s` is not a literal index.  Mirrors `_resolve_index`.
fn resolve_index(s: &str, length: i64) -> Option<i64> {
    let s = s.trim();
    if s == "end" {
        return Some(length - 1);
    }
    if let Some((sign, n)) = end_offset(s) {
        return Some(length - 1 + sign * n);
    }
    parse_strict_int(s)
}

/// Parse a strict `-?\d+` integer (no `+` sign, matching `_INT_RE`).
fn parse_strict_int(s: &str) -> Option<i64> {
    let body = s.strip_prefix('-').unwrap_or(s);
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse::<i64>().ok()
}

/// Parse an `end-N` / `end+N` offset, returning `(sign, n)` where sign
/// is -1 / +1.  Mirrors `_END_MINUS_RE` / `_END_PLUS_RE` (whitespace
/// around the operator allowed).
fn end_offset(s: &str) -> Option<(i64, i64)> {
    let rest = s.strip_prefix("end")?.trim_start();
    let (sign, digits) = if let Some(d) = rest.strip_prefix('-') {
        (-1, d)
    } else if let Some(d) = rest.strip_prefix('+') {
        (1, d)
    } else {
        return None;
    };
    let digits = digits.trim_start();
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<i64>().ok().map(|n| (sign, n))
}

/// Human-readable description of a resolved out-of-range index.
/// Mirrors `_describe_index`.
fn describe_index(resolved: i64, length: i64) -> String {
    if resolved < 0 {
        format!("resolves to {resolved} (before start of list)")
    } else {
        format!(
            "resolves to {resolved} (list has {length} element{})",
            if length == 1 { "" } else { "s" }
        )
    }
}

const TRUE_LITERALS: &[&str] = &["1", "true", "yes", "on", "!0"];
const FALSE_LITERALS: &[&str] = &["0", "false", "no", "off", "!1"];

/// Strip a single enclosing `{ … }` and surrounding whitespace.
/// Mirrors `_strip_braces`.
fn strip_braces(s: &str) -> &str {
    let t = s.trim();
    if t.starts_with('{') && t.ends_with('}') && t.len() >= 2 {
        t[1..t.len() - 1].trim()
    } else {
        t
    }
}

/// `Some(true)` / `Some(false)` when `cond` is a constant-true /
/// constant-false expression; `None` when dynamic.  Mirrors
/// `_condition_constant`.
fn condition_constant(cond: &str) -> Option<bool> {
    let c = strip_braces(cond).to_ascii_lowercase();
    if TRUE_LITERALS.contains(&c.as_str()) {
        return Some(true);
    }
    if FALSE_LITERALS.contains(&c.as_str()) {
        return Some(false);
    }
    c.trim().parse::<f64>().ok().map(|v| v != 0.0)
}

/// True when `body` contains a `break` / `return` / `error` / `exit`
/// keyword (shallow scan).  Mirrors `_body_may_exit` /
/// `_BREAK_RE = (?<![\w:-])(break|return|error|exit)\b` — Rust regex
/// has no look-behind, so the `:` / `-` prefix exclusion is applied as
/// a post-filter on each `\b`-anchored match.
fn body_may_exit(body: &str) -> bool {
    let re = regex::Regex::new(r"\b(break|return|error|exit)\b").expect("valid keyword regex");
    let bytes = body.as_bytes();
    for m in re.find_iter(body) {
        let start = m.start();
        if start == 0 || !matches!(bytes[start - 1], b':' | b'-') {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;

    fn codes(src: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W240" | "W241"))
            .map(|d| d.code.clone())
            .collect()
    }

    #[test]
    fn w240_constant_false_condition() {
        assert_eq!(codes("while 0 {puts hi}\n"), vec!["W240"]);
        assert_eq!(codes("for {set i 0} 0 {incr i} {}\n"), vec!["W240"]);
    }

    #[test]
    fn w241_constant_true_no_exit() {
        assert_eq!(codes("while 1 {puts hi}\n"), vec!["W241"]);
        // A `break` in the body suppresses W241.
        assert!(codes("while 1 {break}\n").is_empty());
        assert!(codes("while 1 {return}\n").is_empty());
    }

    #[test]
    fn dynamic_condition_is_silent() {
        assert!(codes("while {$x < 10} {incr x}\n").is_empty());
    }

    #[test]
    fn w241_for_step_provably_infinite() {
        // Step 0, wrong-direction step, increment-away, never-equal-skip.
        assert_eq!(
            codes("for {set i 0} {$i < 10} {incr i 0} {}\n"),
            vec!["W241"]
        );
        assert_eq!(
            codes("for {set i 0} {$i < 10} {incr i -1} {}\n"),
            vec!["W241"]
        );
        assert_eq!(codes("for {set i 5} {$i > 0} {incr i} {}\n"), vec!["W241"]);
        assert_eq!(
            codes("for {set i 0} {$i != 10} {incr i 3} {}\n"),
            vec!["W241"]
        );
        // A correct counting loop is silent.
        assert!(codes("for {set i 0} {$i < 10} {incr i} {}\n").is_empty());
    }

    #[test]
    fn for_cond_parsers() {
        assert_eq!(
            super::parse_simple_for_cond("$i < 10"),
            Some(("i".into(), "<".into(), 10)),
        );
        assert_eq!(
            super::parse_simple_for_cond("10 > $i"),
            Some(("i".into(), "<".into(), 10)), // flipped
        );
        assert_eq!(super::parse_simple_for_cond("$i < 10 && 0"), None);
        assert_eq!(
            super::parse_init_var_value("set i 5"),
            Some(("i".into(), 5))
        );
        assert_eq!(super::parse_step_incr("incr i"), Some(("i".into(), 1)));
        assert_eq!(super::parse_step_incr("incr i -2"), Some(("i".into(), -2)));
    }

    fn idx_codes(src: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W230" | "W232"))
            .map(|d| d.code.clone())
            .collect()
    }

    #[test]
    fn w230_list_index_out_of_range() {
        assert_eq!(idx_codes("lindex {a b c} 5\n"), vec!["W230"]);
        assert_eq!(idx_codes("lindex {a b c} -1\n"), vec!["W230"]);
        assert_eq!(idx_codes("lindex {a b c} end-5\n"), vec!["W230"]);
        // In range and dynamic list → none.
        assert!(idx_codes("lindex {a b c} 1\n").is_empty());
        assert!(idx_codes("lindex {a b c} end\n").is_empty());
        assert!(idx_codes("lindex $x 5\n").is_empty());
    }

    #[test]
    fn w230_single_element_nested_list_length() {
        // `{{a b c}}` is a one-element list (the inner `{a b c}` is a
        // single braced element).  The segmenter already strips the outer
        // braces, so its length is 1 — index 2 is out of range.  A
        // double brace-strip would wrongly count three words and miss it.
        assert_eq!(idx_codes("lindex {{a b c}} 2\n"), vec!["W230"]);
        // Index 0 is the lone element → in range.
        assert!(idx_codes("lindex {{a b c}} 0\n").is_empty());
    }

    #[test]
    fn w232_string_index_out_of_range() {
        assert_eq!(idx_codes("string index abc 10\n"), vec!["W232"]);
        assert_eq!(idx_codes("string index abc -1\n"), vec!["W232"]);
        assert!(idx_codes("string index abc 1\n").is_empty());
        assert!(idx_codes("string index abc end\n").is_empty());
    }

    #[test]
    fn w232_braced_literal_string_length() {
        // `{{hello}}` is a braced word whose runtime value is the literal
        // 7-char string `{hello}` (the segmenter strips only the outer
        // braces).  Index 6 is the last char — in range.  A double
        // brace-strip would count five chars and flag it spuriously.
        assert!(idx_codes("string index {{hello}} 6\n").is_empty());
        // One past the end is still out of range.
        assert_eq!(idx_codes("string index {{hello}} 7\n"), vec!["W232"]);
    }

    #[test]
    fn w230_lrange_lreplace_empty_slice() {
        assert_eq!(idx_codes("lrange {a b c} 5 7\n"), vec!["W230"]);
        assert_eq!(idx_codes("lrange {a b c} -3 -1\n"), vec!["W230"]);
        assert_eq!(idx_codes("lrange {a b c} 2 0\n"), vec!["W230"]); // clamped first>last
        assert_eq!(idx_codes("lreplace {a b c} 5 7 X\n"), vec!["W230"]);
        assert!(idx_codes("lrange {a b c} 0 1\n").is_empty());
    }

    #[test]
    fn w232_string_range_replace_empty_slice() {
        assert_eq!(idx_codes("string range abc 5 7\n"), vec!["W232"]);
        assert_eq!(idx_codes("string range abc -3 -1\n"), vec!["W232"]);
        assert_eq!(idx_codes("string replace abc 5 7 X\n"), vec!["W232"]);
        assert!(idx_codes("string range abc 0 1\n").is_empty());
    }

    #[test]
    fn pair_slice_empty_logic() {
        assert!(super::pair_slice_empty(5, 7, 3)); // both above
        assert!(super::pair_slice_empty(-3, -1, 3)); // both below
        assert!(super::pair_slice_empty(2, 0, 3)); // clamped first>last
        assert!(super::pair_slice_empty(0, 0, 0)); // empty container
        assert!(!super::pair_slice_empty(0, 1, 3)); // valid
    }

    #[test]
    fn index_helpers() {
        assert_eq!(super::resolve_index("end", 3), Some(2));
        assert_eq!(super::resolve_index("end-5", 3), Some(-3));
        assert_eq!(super::resolve_index("end+1", 3), Some(3));
        assert_eq!(super::resolve_index("2", 3), Some(2));
        assert_eq!(super::resolve_index("$x", 3), None);
        assert!(super::is_literal_index("end-2"));
        assert!(!super::is_literal_index("+5")); // strict: no leading +
    }

    #[test]
    fn condition_constant_classifies() {
        assert_eq!(super::condition_constant("0"), Some(false));
        assert_eq!(super::condition_constant("1"), Some(true));
        assert_eq!(super::condition_constant("{true}"), Some(true));
        assert_eq!(super::condition_constant("$x < 10"), None);
    }

    // -- W231 (lset out of range) & W242 (unprovable termination) -----
    //
    // Cross-checked against the live Python analyser.

    fn code_msgs(src: &str, code: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == code)
            .map(|d| d.message.clone())
            .collect()
    }

    #[test]
    fn w231_lset_negative_index_always_errors() {
        let m = code_msgs("lset L -1 x\n", "W231");
        assert_eq!(m.len(), 1);
        assert!(m[0].contains("lset index '-1' is negative"), "{m:?}");
        // Negative literal fires even for nested (multi-index) forms.
        assert_eq!(code_msgs("lset L i j -1 v\n", "W231").len(), 1);
    }

    #[test]
    fn w231_lset_index_past_append_slot() {
        // List length recovered from the recent literal `set`.
        let m = code_msgs("set L {a b c}\nlset L 5 x\n", "W231");
        assert_eq!(m.len(), 1);
        assert!(
            m[0].contains("resolves to 5 (list has 3 elements)"),
            "{m:?}"
        );
        // The append slot (index == length) and valid indices are fine.
        assert!(code_msgs("set L {a b c}\nlset L 3 x\n", "W231").is_empty());
        assert!(code_msgs("set L {a b c}\nlset L end+1 x\n", "W231").is_empty());
        assert!(code_msgs("set L {a b c}\nlset L 2 x\n", "W231").is_empty());
    }

    #[test]
    fn w231_silent_without_recoverable_length() {
        // No prior literal `set` → only negative literals would fire.
        assert!(code_msgs("lset L 5 x\n", "W231").is_empty());
        // A `set` in a deeper scope must not be trusted.
        assert!(
            code_msgs("proc p {} { set L {a b c} }\nlset L 5 x\n", "W231").is_empty(),
            "deeper-scope set should not leak its length"
        );
    }

    #[test]
    fn w242_counter_not_modified() {
        let m = code_msgs("while {$x < 10} {puts hi}\n", "W242");
        assert_eq!(m.len(), 1);
        assert!(
            m[0].contains("variable 'x' in the condition is never modified"),
            "{m:?}"
        );
        // `for` with an empty step and a body that ignores the counter.
        assert_eq!(
            code_msgs("for {set i 0} {$i < 10} {} {puts hi}\n", "W242").len(),
            1
        );
    }

    #[test]
    fn w242_silent_when_counter_modified() {
        // Body writes the counter via incr / set / lappend → no W242.
        assert!(code_msgs("while {$x < 10} {incr x}\n", "W242").is_empty());
        assert!(code_msgs("while {$x < 10} {set x 5}\n", "W242").is_empty());
        // A normal `for` whose step advances the counter is silent.
        assert!(code_msgs("for {set i 0} {$i < 10} {incr i} {puts hi}\n", "W242").is_empty());
    }

    #[test]
    fn w242_severity_is_hint() {
        let mut a = Analyser::new();
        let r = a.analyse("while {$x < 10} {puts hi}\n", "tcl8.6");
        let w242 = r.diagnostics.iter().find(|d| d.code == "W242").unwrap();
        assert_eq!(w242.severity, super::Severity::Hint);
    }
}
