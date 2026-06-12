//! Diagnostic-emission orchestrator — Rust port of
//! `core/analysis/_analyser/_diagnostics.py`.
//!
//! Three top-level methods, mirroring the Python file 1:1:
//!
//! - [`Analyser::emit_variable_usage_diagnostics`] — kept as a
//!   no-op hook for future scope-tree consumers (Python's W211
//!   moved to the SSA-based pass; same here).
//! - [`Analyser::emit_cfg_ssa_diagnostics`] — main entry; builds
//!   a [`crate::compilation_unit::CompilationUnit`] on demand, walks the top-level
//!   function and every procedure, dispatches per-function
//!   diagnostics, and runs the cross-function post-passes
//!   (var-as-command, interpolated-command resolution).
//! - [`Analyser::emit_cfg_ssa_diagnostics_for_function`] —
//!   per-function dispatcher; calls each landed emitter in
//!   declaration order.
//!
//! Two utility passes round out the Python file:
//!
//! - [`Analyser::dedupe_diagnostics`] — drop exact duplicates
//!   plus the line-based pairs (E002 swallowed by E101 on the
//!   same line; W122 swallowed by W124 on the same line).
//! - [`Analyser::apply_disabled_diagnostics`] — filter out
//!   codes the caller asked to silence.
//!
//! **Strip-by-strip status.**
//!
//! - **C41d1** — orchestrator scaffold + dedupe + disabled-
//!   codes filter.  ✅ landed.
//! - **C41d2** — `_diag_var_lifecycle.py`.  ✅ landed:
//!   W220 (dead store), W211 (unused variable), W214
//!   (unused parameter), W210 (read-before-set), W213
//!   (unset on possibly-undef), and H300 (paste error).
//!   W210 / W213 are gated on procs only — top-level RBS
//!   needs the ``globals_written_by_procs`` filter Python
//!   uses, deferred until interproc analysis is wired in.
//! - **C41d3** — `_diag_var_command.py`.  ✅ landed:
//!   ``var_command_sites`` / ``cmd_command_sites`` recorded
//!   during the walk dispatch; **W307** (non-literal command
//!   name) and **W308** (unknown method on object) both emit
//!   via the cross-function post-pass.  W308 uses the C41e0
//!   ``ClassHierarchy::method_target`` for MRO-aware method
//!   resolution, with all the Python suppression paths
//!   wired (inherited ``unknown`` handler, external
//!   superclass, ``oo::objdefine`` per-instance methods).
//!   The ``[cmd] method`` return-type suppression for W307
//!   on ``cmd_command_sites`` remains deferred — it needs
//!   IR-level type-lattice plumbing extended into the
//!   analyser, which is a separate strip.
//! - **C41d4** — `_diag_commands.py`.  ✅ partial: W123
//!   (unknown command) is wired via the cross-function post-
//!   pass.  ``command_invocations`` are now recorded for every
//!   command head during the walk dispatch.  Deferred:
//!   ``_resolve_interpolated_commands`` (CONSTSET-driven W123
//!   suppression for ``$``-bearing names),
//!   ``_globals_written_by_procs`` (used by the C41d2 W210
//!   top-level RBS filter), ``suggest_similar`` "did you
//!   mean…?" suggestions, and the
//!   ``unknown_proc_info`` / ``has_dynamic_providers``
//!   early-returns.
//! - **C41d5** — `_diag_branches.py` + `_diag_channel.py`.
//!   ✅ landed: I230 / I231 (constant branch / switch-arm) and
//!   W126 (channel argument validation) all wired through the
//!   per-function dispatcher.  Severity-Info Python diagnostics
//!   map to ``Severity::Hint`` here (no Info variant on the
//!   Rust side).
//! - **C41d6** — `_diag_ip.py`.  ✅ landed: W124 (invalid IP
//!   address literal) — IPv4 octet validation (over-255 →
//!   Error, leading-zero → Warning) and IPv6 parsing via
//!   ``std::net::Ipv6Addr``.  Anchors at the SSA def site;
//!   seen-offsets dedup avoids duplicates across SSA versions.
//! - **C41d7** — `_diag_racy.py`.  ⏸ deferred: IRULE4005
//!   (racy ``static::`` cross-event flow) needs the
//!   connection-scope / cross-event analysis that the Rust
//!   pipeline doesn't yet have (Python's
//!   ``cu.connection_scope.racy_static_defs``).  Once
//!   ``ConnectionScope`` lands on the Rust side, the emitter
//!   wires up in a single call to ``emit_racy_static_diagnostics``.

use std::collections::HashSet;

use tcl_lexer::SourceMap;

use super::state::Analyser;
use super::types::Severity;
use crate::expr_ast::{BinOp, ExprNode, UnaryOp};

/// Find a case-insensitive match for `variable` in `defined_vars`.
///
/// The source text covered by `span`, or `None` when the span is out
/// of bounds / not on char boundaries.
fn source_slice(source: &str, span: tcl_lexer::Span) -> Option<String> {
    let start = span.start() as usize;
    let end = span.end() as usize;
    if start <= end && end <= source.len() {
        source.get(start..end).map(str::to_owned)
    } else {
        None
    }
}

/// Parse a namespaced-ensemble dispatch head `${prefix}::tail` or
/// `$prefix::tail` from the source slice at `span`, returning
/// `(prefix_var_name, tail)`.  Returns `None` when the head isn't this shape
/// (e.g. a plain `$var`, an array element, or a `[cmd]::tail` substitution).
/// Mirrors the `is_namespaced_ensemble` detection + tail scan in
/// `_diag_var_command.py`.
fn parse_namespaced_ensemble(source: &str, span: tcl_lexer::Span) -> Option<(String, String)> {
    let start = span.start() as usize;
    let end = (span.end() as usize).min(source.len());
    if start >= end {
        return None;
    }
    let head = &source[start..end];
    let rest = head.strip_prefix('$')?;
    // `${prefix}::tail` (brace form) or `$prefix::tail` (bare form).
    let (prefix, after) = if let Some(braced) = rest.strip_prefix('{') {
        let close = braced.find('}')?;
        (&braced[..close], &braced[close + 1..])
    } else {
        let sep = rest.find("::")?;
        (&rest[..sep], &rest[sep..])
    };
    let tail = after.strip_prefix("::")?;
    // A bare-form prefix must be a plain variable name (no `(` array index,
    // no embedded `::` before the separator we split on); the brace form is
    // already delimited.  Both prefix and tail must be non-empty.
    if prefix.is_empty() || tail.is_empty() || prefix.contains('(') {
        return None;
    }
    Some((prefix.to_string(), tail.to_string()))
}

/// True when `tok` is a `${name}` (brace-form) VAR token.  Mirrors
/// `_is_brace_form` in `_diag_brace_then_paren.py`: bare `$name` spans
/// `name.len() + 1` (one `$`); brace `${name}` spans more (`${` + name).
/// The Rust [`tcl_lexer::Span`] end is exclusive, so the span length is
/// `end - start`, equal to Python's inclusive `end.offset - start.offset + 1`.
fn is_brace_form_var(sm: &SourceMap<'_>, tok: tcl_lexer::Token) -> bool {
    if tok.kind != tcl_lexer::TokenType::Var {
        return false;
    }
    let span_len = (tok.span.end() - tok.span.start()) as usize;
    span_len > sm.token_text(tok).len() + 1
}

/// Return `true` if `inner` contains a `$` or `[` the user likely expects to
/// substitute — the trigger for the `${arr($foo)}` Pattern-(2) variant of
/// W216.  Skips backslash escapes.  Mirrors `_index_has_substitution`.
fn index_has_substitution(inner: &str) -> bool {
    let bytes = inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => {
                i += 2;
                continue;
            }
            b'$' | b'[' => return true,
            _ => {}
        }
        i += 1;
    }
    false
}

/// Render the safe replacement for a W216 array-element reference.  Bare
/// `$name(idx)` is the only `$`-form that substitutes `$` inside the index,
/// so prefer it when `name` allows it; otherwise `[set "name(idx)"]` (the
/// command parser substitutes `$`-vars in `set`'s argument).  Mirrors
/// `_build_replacement`.
fn build_w216_replacement(name: &str, inner: &str) -> String {
    if tcl_syntax::naming::is_bare_var_name(name) {
        format!("${name}({inner})")
    } else {
        format!("[set \"{name}({inner})\"]")
    }
}

/// Indices into *args* (0-based, word index `i + 1`) that `cmd_name` reads as
/// a **variable name** — where the braced indirect-array idiom
/// `${name}(idx)` is correct rather than a typo.  Mirrors
/// `_varname_word_indices` in `_diag_brace_then_paren.py`.
fn w216_varname_word_indices(cmd_name: &str, args: &[String]) -> Vec<usize> {
    match cmd_name {
        "set" | "incr" | "append" | "lappend" | "vwait" => {
            if args.is_empty() {
                Vec::new()
            } else {
                vec![0]
            }
        }
        "unset" => {
            // unset ?-nocomplain? ?--? varName ?varName ...?
            let mut start = 0;
            for (i, a) in args.iter().enumerate() {
                if a == "--" {
                    start = i + 1;
                    break;
                }
                if a.starts_with('-') {
                    start = i + 1;
                    continue;
                }
                start = i;
                break;
            }
            (start..args.len()).collect()
        }
        "info" => {
            if args.len() >= 2 && args[0] == "exists" {
                vec![1]
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

/// Find the offset of the `)` matching the `(` at `paren_start`.  Skips
/// balanced `{...}`, double-quoted strings, and backslash escapes.  Returns
/// `None` on malformed input.  Mirrors `_find_matching_close_paren`.
fn find_matching_close_paren(source: &[u8], paren_start: usize) -> Option<usize> {
    let n = source.len();
    let mut depth = 1i32;
    let mut j = paren_start + 1;
    let mut in_quote = false;
    while j < n && depth > 0 {
        let c = source[j];
        if in_quote {
            if c == b'\\' && j + 1 < n {
                j += 2;
                continue;
            }
            if c == b'"' {
                in_quote = false;
            }
            j += 1;
            continue;
        }
        match c {
            b'"' => {
                in_quote = true;
                j += 1;
                continue;
            }
            b'{' => {
                let mut bd = 1i32;
                j += 1;
                while j < n && bd > 0 {
                    if source[j] == b'\\' && j + 1 < n {
                        j += 2;
                        continue;
                    }
                    match source[j] {
                        b'{' => bd += 1,
                        b'}' => bd -= 1,
                        _ => {}
                    }
                    j += 1;
                }
                continue;
            }
            b'\\' if j + 1 < n => {
                j += 2;
                continue;
            }
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        j += 1;
    }
    if depth != 0 {
        None
    } else {
        Some(j - 1)
    }
}

/// True when `ch` is a standard ASCII character W108 leaves alone: tab
/// / LF / CR, or printable ASCII `0x20`-`0x7e`.  Mirrors
/// `_NON_ASCII_RE = [^\x09\x0a\x0d\x20-\x7e]` (negated).
fn is_standard_ascii(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\r' | ' '..='~')
}

/// W108 "common" mode benign-Unicode test — mirrors
/// `core/analysis/checks/_style.py::_is_benign_unicode`. A character is
/// *intentional* (not flagged) when its Unicode general category is a
/// Letter, Number, Mark, Symbol, or Punctuation (any script). Control,
/// format, separator, surrogate, private-use, and unassigned characters
/// are *not* benign (they almost always indicate encoding/copy-paste
/// issues) and are flagged.
fn is_benign_unicode(ch: char) -> bool {
    use unicode_general_category::{get_general_category, GeneralCategory as G};
    matches!(
        get_general_category(ch),
        // Letters (L*)
        G::UppercaseLetter
            | G::LowercaseLetter
            | G::TitlecaseLetter
            | G::ModifierLetter
            | G::OtherLetter
            // Numbers (N*)
            | G::DecimalNumber
            | G::LetterNumber
            | G::OtherNumber
            // Marks (M*)
            | G::NonspacingMark
            | G::SpacingMark
            | G::EnclosingMark
            // Symbols (S*)
            | G::MathSymbol
            | G::CurrencySymbol
            | G::ModifierSymbol
            | G::OtherSymbol
            // Punctuation (P*)
            | G::ConnectorPunctuation
            | G::DashPunctuation
            | G::OpenPunctuation
            | G::ClosePunctuation
            | G::InitialPunctuation
            | G::FinalPunctuation
            | G::OtherPunctuation
    )
}

/// Integer format specifiers for `binary format` / `binary scan` that
/// accept the Tcl 8.5+ `u` / `s` modifier.  Mirrors
/// `_BINARY_INT_SPECIFIERS`.
const BINARY_INT_SPECIFIERS: &[u8] = b"csSiInTwWmrR";

/// Sentinel scope key for the W307 dispatcher-suppression maps covering
/// statements outside any proc body (mirrors Python's `_TOP_SCOPE`).
const W307_TOP_SCOPE: &str = "::top";

/// Mask-octet values that can appear in a contiguous subnet mask.
/// Mirrors `_VALID_MASK_OCTETS`.
const VALID_MASK_OCTETS: &[u32] = &[0, 128, 192, 224, 240, 248, 252, 254, 255];

/// True when the four octets form a valid contiguous subnet mask
/// (all-1s then all-0s).  Mirrors `_is_valid_subnet_mask`.
fn is_valid_subnet_mask(a: u32, b: u32, c: u32, d: u32) -> bool {
    let val = (a << 24) | (b << 16) | (c << 8) | d;
    if val == 0 {
        return true;
    }
    let inverted = val ^ 0xFFFF_FFFF;
    (inverted & inverted.wrapping_add(1)) == 0
}

/// Heuristic: the dotted-quad plausibly *intends* to be a mask.
/// Mirrors `_looks_like_subnet_mask`.
fn looks_like_subnet_mask(a: u32, b: u32, c: u32, d: u32) -> bool {
    if a == 255 && !(b == 255 && c == 255 && d == 255) {
        return true;
    }
    a >= 128 && [a, b, c, d].iter().all(|o| VALID_MASK_OCTETS.contains(o))
}

/// Suggest the nearest valid contiguous mask, or `None`.  Mirrors
/// `_nearest_valid_mask`.
fn nearest_valid_mask(a: u32, b: u32, c: u32, d: u32) -> Option<String> {
    let val = (a << 24) | (b << 16) | (c << 8) | d;
    let mut leading = 0u32;
    for bit in (0..32).rev() {
        if val & (1 << bit) != 0 {
            leading += 1;
        } else {
            break;
        }
    }
    if leading == 0 || leading == 32 {
        return None;
    }
    let candidate = 0xFFFF_FFFFu32 << (32 - leading);
    Some(format!(
        "{}.{}.{}.{}",
        (candidate >> 24) & 0xFF,
        (candidate >> 16) & 0xFF,
        (candidate >> 8) & 0xFF,
        candidate & 0xFF
    ))
}

// -- IP / ReDoS leaf scanners (regex-free) ----------------------------

/// A `\w` byte: ASCII alphanumeric or underscore (word boundary basis).
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// One dotted-quad match found in a value: the four octet substrings and
/// the byte offset where it begins (for context checks like a preceding
/// `/`).
struct DottedQuad<'a> {
    octets: [&'a str; 4],
    start: usize,
    /// Byte offset just past the final octet (the regex `m.end()`).
    end: usize,
}

/// Find every `\b\d{1,N}.\d{1,N}.\d{1,N}.\d{1,N}\b` dotted quad in
/// `text` (non-overlapping, left-to-right), replacing the regex scan.
/// `max_digits` caps each octet's digit count (`3` for the subnet-mask
/// check, `4` for the invalid-IP one).  Each octet starts at a word
/// boundary, so a longer digit run (a 4th/5th digit) simply fails to
/// align with the following `.` and is skipped — matching the regex.
fn find_dotted_quads(text: &str, max_digits: usize) -> Vec<DottedQuad<'_>> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let boundary_before = i == 0 || !is_word_byte(bytes[i - 1]);
        if boundary_before {
            if let Some((octets, end)) = match_dotted_quad(text, i, max_digits) {
                out.push(DottedQuad {
                    octets,
                    start: i,
                    end,
                });
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Match a dotted quad starting at byte `start` (a word boundary), each
/// octet `1..=max_digits` digits separated by `.`, requiring a trailing
/// word boundary.  Returns the octet substrings and the end offset.
fn match_dotted_quad(text: &str, start: usize, max_digits: usize) -> Option<([&str; 4], usize)> {
    let bytes = text.as_bytes();
    let mut pos = start;
    let mut octets: [&str; 4] = [""; 4];
    for (k, slot) in octets.iter_mut().enumerate() {
        let run_start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        let len = pos - run_start;
        if len == 0 || len > max_digits {
            return None;
        }
        *slot = &text[run_start..pos];
        if k < 3 {
            if bytes.get(pos) != Some(&b'.') {
                return None;
            }
            pos += 1; // consume the dot
        }
    }
    // Trailing `\b`: end of string or a non-word byte.
    if pos < bytes.len() && is_word_byte(bytes[pos]) {
        return None;
    }
    Some((octets, pos))
}

/// Find every IPv6 *candidate* substring — `\b[hex]{1,4}(:[hex]{0,4}){2,7}\b`
/// — in `text` (the caller validates each via `Ipv6Addr::from_str`).
/// Replaces the regex; each candidate begins at a word boundary, has a
/// 1-4 hex-digit first group, 2-7 following `:`-groups (each 0-4 hex),
/// and ends on a hex digit at a trailing word boundary.
fn find_ipv6_candidates(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let boundary_before = i == 0 || !is_word_byte(bytes[i - 1]);
        if boundary_before && bytes[i].is_ascii_hexdigit() {
            if let Some(end) = match_ipv6_candidate(bytes, i) {
                out.push(&text[i..end]);
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Read up to `max` contiguous hex-digit bytes from `start`, returning
/// the count.
fn hex_run_len(bytes: &[u8], start: usize, max: usize) -> usize {
    let mut k = 0;
    while k < max && start + k < bytes.len() && bytes[start + k].is_ascii_hexdigit() {
        k += 1;
    }
    k
}

/// Match an IPv6 candidate starting at `start`, returning the end offset
/// of the longest `hex(:hex?){2,7}` run that ends on a hex digit and is
/// followed by a word boundary, or `None`.
fn match_ipv6_candidate(bytes: &[u8], start: usize) -> Option<usize> {
    let first = hex_run_len(bytes, start, 4);
    if first == 0 {
        return None;
    }
    let mut pos = start + first;
    let mut groups = 0usize;
    let mut best: Option<usize> = None;
    while groups < 7 && bytes.get(pos) == Some(&b':') {
        let after_colon = pos + 1;
        let h = hex_run_len(bytes, after_colon, 4);
        pos = after_colon + h;
        groups += 1;
        // A valid `\b`-terminated end: ≥2 groups, ends on a hex digit,
        // and is followed by a non-word byte (or end of input).
        if groups >= 2 && h >= 1 && (pos >= bytes.len() || !is_word_byte(bytes[pos])) {
            best = Some(pos);
        }
    }
    best
}

/// True when `pattern` contains a catastrophic-backtracking shape: a
/// nested quantifier (`…+)+`, `…*)*`, `…+){`) or an overlapping
/// alternation (`(…|…)` immediately followed by `+` / `*` / `{`).
/// Hand-written replacement for the `_REDOS_PATTERN` regex.
fn has_redos_shape(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let quant = |b: Option<&u8>| matches!(b, Some(b'+' | b'*' | b'{'));
    // Nested quantifier: `<+|*> ) <+|*|{>`.
    for i in 0..bytes.len() {
        if matches!(bytes[i], b'+' | b'*')
            && bytes.get(i + 1) == Some(&b')')
            && quant(bytes.get(i + 2))
        {
            return true;
        }
    }
    // Overlapping alternation: `( [^)]* | [^)]* ) <+|*|{>`.
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            let mut j = i + 1;
            let mut has_pipe = false;
            while j < bytes.len() && bytes[j] != b')' {
                has_pipe |= bytes[j] == b'|';
                j += 1;
            }
            if j < bytes.len() && has_pipe && quant(bytes.get(j + 1)) {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Destructive builtins whose bare `catch {<cmd> ...}` form is the
/// documented "fire-and-forget" idiom — failure when the target is
/// already gone is expected and intentionally ignored.  Mirrors
/// `analyser/compiler_checks.py::_FIRE_AND_FORGET_BARE`.
fn fire_and_forget_bare(bare: &str) -> bool {
    matches!(bare, "close" | "unset" | "rename")
}

/// Ensemble commands where only certain destructive subcommands are
/// fire-and-forget (`chan close` is, `chan configure` is not).  Mirrors
/// `_FIRE_AND_FORGET_SUBCOMMANDS`.
fn fire_and_forget_subcommand(bare: &str, sub: &str) -> bool {
    match bare {
        "after" => sub == "cancel",
        "chan" => sub == "close",
        "array" | "dict" => sub == "unset",
        "interp" | "file" => sub == "delete",
        "namespace" => sub == "delete" || sub == "forget",
        _ => false,
    }
}

/// True when the body of a `catch` matches the documented
/// "fire-and-forget" idiom: a single command whose head is a
/// destructive builtin (`close $h`, `unset var`, `rename foo ""`) or a
/// documented destructive ensemble subcommand (`after cancel`, `chan
/// close`, `array unset`, …).  Conservative: only single-statement
/// bodies match, and ensemble heads are subcommand-checked.  Mirrors
/// `analyser/compiler_checks.py::_catch_body_is_fire_and_forget`.
fn catch_body_is_fire_and_forget(body: &str) -> bool {
    let segs: Vec<_> = crate::segmenter::segment_commands(body)
        .into_iter()
        .filter(|c| !c.texts.is_empty())
        .collect();
    if segs.len() != 1 {
        return false;
    }
    let Some(head) = segs[0].texts.first() else {
        return false;
    };
    if head.is_empty() {
        return false;
    }
    let bare = head
        .trim_start_matches(':')
        .rsplit("::")
        .next()
        .unwrap_or(head);
    if fire_and_forget_bare(bare) {
        return true;
    }
    match segs[0].texts.get(1) {
        Some(first_arg) => fire_and_forget_subcommand(bare, first_arg),
        None => false,
    }
}

/// True when `tok` is a brace-quoted word (`{…}`, a `Str` token).
/// Mirrors `_first_token_is_braced`.
fn is_braced_word(tok: &tcl_lexer::Token) -> bool {
    tok.kind == tcl_lexer::TokenType::Str
}

/// True when `text` carries a substitution (`$` / `[`) or `tok` is a
/// `Var` / `Cmd` token.  Mirrors `_has_substitution`.
fn has_substitution(text: &str, tok: &tcl_lexer::Token) -> bool {
    text.contains('$')
        || text.contains('[')
        || matches!(
            tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        )
}

/// First positional (pattern) argument index of `regexp` / `regsub`,
/// after skipping option switches (`-start` consumes a value, `--`
/// terminates).  Mirrors `regexp_pattern_index` (and the regexp arg-role
/// resolver's option skip).  `args` excludes the command name.
fn regexp_pattern_index(args: &[String]) -> Option<usize> {
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--" {
            i += 1;
            break;
        }
        if a.starts_with('-') {
            i += 1;
            if a == "-start" && i < args.len() {
                i += 1;
            }
            continue;
        }
        break;
    }
    (i < args.len()).then_some(i)
}

/// True when the **source** slice `raw` (backslashes intact) carries a
/// *live* substitution: an unescaped `[`, or a `$` that actually
/// introduces a variable name (`[A-Za-z0-9_]`, `{`, or `:`).  A `\[` /
/// `\$` is a literal regex character, and a `$` before a quote / end /
/// punctuation (the `(.*)$` end-anchor) is a literal dollar — neither
/// counts.  Mirrors `_raw_has_live_substitution`.
fn raw_has_live_substitution(raw: &str) -> bool {
    let b = raw.as_bytes();
    let n = b.len();
    let mut i = 0;
    while i < n {
        match b[i] {
            b'\\' => {
                i += 2; // the next char is escaped (literal) — skip both
                continue;
            }
            b'[' => return true,
            b'$' => {
                if let Some(&c) = b.get(i + 1) {
                    if c.is_ascii_alphanumeric() || matches!(c, b'_' | b'{' | b':') {
                        return true;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

/// True when `text` is a simple numeric or boolean literal that needs
/// no bracing.  Mirrors `_is_safe_literal`.
fn is_safe_literal(text: &str) -> bool {
    let t = text.trim();
    if t.parse::<f64>().is_ok() {
        return true;
    }
    matches!(
        t.to_ascii_lowercase().as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off"
    )
}

/// True when an expr string is substitution-free numeric / boolean /
/// operator text (safe to leave unbraced).  Mirrors
/// `_is_safe_literal_expr`.
fn is_safe_literal_expr(text: &str, dialect: &str) -> bool {
    use tcl_lexer::ExprTokenType as T;
    if is_safe_literal(text) {
        return true;
    }
    if text.contains('$') || text.contains('[') {
        return false;
    }
    let tokens = tcl_lexer::tokenise_expr(text, Some(dialect));
    if tokens.is_empty() {
        return false;
    }
    tokens.iter().all(|tok| {
        matches!(
            tok.kind,
            T::Number
                | T::Bool
                | T::Operator
                | T::ParenOpen
                | T::ParenClose
                | T::Whitespace
                | T::TernaryQ
                | T::TernaryC
                | T::Comma
        )
    })
}

/// Resolve which argument indices (into `args`, command-name excluded)
/// must be plain variable *names* for `cmd_name`.  Port of
/// `_NAME_ARG_INDICES` + its four resolvers (`_first_arg_name`,
/// `_unset_name_args`, `_info_exists_arg`, `_upvar_local_name_args`).
fn name_arg_indices(cmd_name: &str, args: &[String]) -> Vec<usize> {
    match cmd_name {
        // First argument is the variable name.
        "set" | "incr" | "append" | "lappend" => {
            if args.is_empty() {
                vec![]
            } else {
                vec![0]
            }
        }
        // `unset ?-nocomplain? ?--? var ?var …?` — names start after the
        // leading option flags.
        "unset" => {
            let mut start = 0;
            for (i, a) in args.iter().enumerate() {
                if a == "--" {
                    start = i + 1;
                    break;
                }
                if a.starts_with('-') {
                    start = i + 1;
                    continue;
                }
                start = i;
                break;
            }
            (start..args.len()).collect()
        }
        // Only the `exists` subcommand of `info` takes a name.
        "info" => {
            if args.len() >= 2 && args[0] == "exists" {
                vec![1]
            } else {
                vec![]
            }
        }
        // `upvar ?level? other local ?other local …?` — the *local*
        // names are every other arg after an optional level word.
        "upvar" => {
            if args.is_empty() {
                return vec![];
            }
            let head = &args[0];
            let is_level = head.starts_with('#')
                || (!head.is_empty()
                    && head
                        .trim_start_matches('-')
                        .bytes()
                        .all(|b| b.is_ascii_digit())
                    && head.trim_start_matches('-').bytes().next().is_some());
            let start = usize::from(is_level);
            (start + 1..args.len()).step_by(2).collect()
        }
        _ => vec![],
    }
}

/// Find the first `[`-`expr`-whitespace sequence in `slice` (the
/// `_NESTED_EXPR_RE = \[\s*expr\s` pattern) and return
/// `(open_bracket_index, matching_close_bracket_index)`.  The close is
/// located by a depth scan; an unmatched `[` falls back to the last
/// byte.  Returns `None` when no nested `[expr ` is present.
fn first_nested_expr(slice: &str) -> Option<(usize, usize)> {
    let bytes = slice.as_bytes();
    let len = bytes.len();
    let mut open = 0;
    while open < len {
        if bytes[open] == b'[' {
            // `\s*`
            let mut after_ws = open + 1;
            while after_ws < len && bytes[after_ws].is_ascii_whitespace() {
                after_ws += 1;
            }
            // `expr` followed by a whitespace byte.
            let kw_end = after_ws + 4;
            if kw_end < len
                && &bytes[after_ws..kw_end] == b"expr"
                && bytes[kw_end].is_ascii_whitespace()
            {
                // Depth-scan for the matching `]` (the open `[` is
                // already counted).
                let mut depth = 1;
                let mut scan = open + 1;
                while scan < len && depth > 0 {
                    match bytes[scan] {
                        b'[' => depth += 1,
                        b']' => depth -= 1,
                        _ => {}
                    }
                    scan += 1;
                }
                let close = if depth == 0 { scan - 1 } else { len - 1 };
                return Some((open, close));
            }
        }
        open += 1;
    }
    None
}

/// Mirrors `_find_case_mismatch` in
/// `core/analysis/_analyser/_diag_var_lifecycle.py:135-148`.
/// Returns the lexicographically smallest other-cased variant —
/// deterministic across runs.
fn find_case_mismatch<'a>(variable: &str, defined_vars: &'a HashSet<String>) -> Option<&'a str> {
    let lower = variable.to_lowercase();
    let mut matches: Vec<&str> = defined_vars
        .iter()
        .filter(|n| n.as_str() != variable && n.to_lowercase() == lower)
        .map(String::as_str)
        .collect();
    matches.sort_unstable();
    matches.into_iter().next()
}

/// SYNC-MAY31-3: variables this statement queries *only for
/// existence* (`info exists X` / `array exists X`, whether a bare call
/// or a `[...]` command substitution inside an assignment / argument).
/// Such a reference is not a value read, so it must not raise W210.
fn existence_query_vars(stmt: &crate::ir::Statement) -> Vec<String> {
    use crate::expr_ast::existence_query_in_text;
    use crate::ir::Statement;
    let mut out = Vec::new();
    // Bare-call form: `info exists X` / `array exists X`.
    if let Statement::Call { command, args, .. } = stmt {
        if matches!(command.as_str(), "info" | "array")
            && args.first().map(String::as_str) == Some("exists")
        {
            if let Some(v) = args.get(1) {
                out.push(v.clone());
            }
        }
    }
    // Command-substitution form: `set y [info exists X]`,
    // `puts [array exists X]`, etc.
    let texts: &[String] = match stmt {
        Statement::AssignValue { value, .. } => std::slice::from_ref(value),
        Statement::Call { args, .. } => args,
        _ => &[],
    };
    for t in texts {
        if let Some(v) = existence_query_in_text(t.trim()) {
            out.push(v);
        }
    }
    out
}

/// SYNC-MAY31-3: collect `(var, guard_block)` pairs for every
/// `[info exists X]` / `[array exists X]` branch condition in `fu`.
/// A read of `var` in any block dominated by `guard_block` is guarded
/// (X provably exists).  A positive query guards the true target; a
/// `![info exists X]` query guards the false target.
fn collect_existence_guards(fu: &crate::compilation_unit::FunctionUnit) -> Vec<(String, String)> {
    use crate::cfg::Terminator;
    let mut guards = Vec::new();
    for block in fu.cfg.blocks.values() {
        if let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            ..
        }) = &block.terminator
        {
            if let Some((var, negated)) = crate::expr_ast::existence_query_var(condition) {
                let target = if negated { false_target } else { true_target };
                guards.push((var, target.clone()));
            }
        }
    }
    guards
}

/// SYNC-MAY31-3: true when a read of `var` at `use_block` is exempt
/// from W210 because it is the existence-query word itself, or because
/// it sits in a region guarded by an enclosing `[info exists var]`.
fn existence_exempt(
    stmt_opt: Option<&crate::ir::Statement>,
    var: &str,
    exists_guards: &[(String, String)],
    ssa: &crate::ssa::SsaFunction,
    use_block: &str,
) -> bool {
    if let Some(stmt) = stmt_opt {
        if existence_query_vars(stmt).iter().any(|q| q == var) {
            return true;
        }
    }
    exists_guards
        .iter()
        .any(|(gv, gblk)| gv == var && block_dominated_by(ssa, use_block, gblk))
}

/// True when `block` is dominated by `dom` (walking the SSA immediate
/// dominator chain; a block dominates itself).
fn block_dominated_by(ssa: &crate::ssa::SsaFunction, block: &str, dom: &str) -> bool {
    let mut cur = block;
    loop {
        if cur == dom {
            return true;
        }
        match ssa.idom.get(cur) {
            Some(Some(parent)) => cur = parent,
            _ => return false,
        }
    }
}

/// True when a read of `var` at this use-site statement is in fact a safe
/// self-initialisation, not a read-before-set: a `safe_on_uninit` call (e.g.
/// `lappend`/`dict set`/`append`) that defines `var`, or an `incr` of its own
/// target (which initialises an unset var to 0 in Tcl 8.5+).
fn use_site_safe_initialises(stmt: Option<&crate::ir::Statement>, var: &str) -> bool {
    use crate::ir::Statement;
    match stmt {
        Some(Statement::Call {
            safe_on_uninit,
            defs,
            ..
        }) => *safe_on_uninit && defs.iter().any(|d| d == var),
        Some(Statement::Incr { name, .. }) => crate::naming::normalise_var_name(name) == var,
        _ => false,
    }
}

/// The namespace of a fully-qualified name: everything up to the last `::`,
/// or `::` for a top-level name.  Mirrors `qname.rsplit("::", 1)[0] or "::"`.
fn namespace_of(qualified_name: &str) -> String {
    match qualified_name.rsplit_once("::") {
        Some((ns, _)) if !ns.is_empty() => ns.to_string(),
        _ => "::".to_string(),
    }
}

/// Implicit / interpreter-provided variables that are always defined and
/// must never raise a read-before-set.  Mirrors `_IMPLICIT_VARS` in
/// `compiler/core_analyses.py`.
fn is_implicit_var(name: &str) -> bool {
    matches!(
        name,
        "argc"
            | "argv"
            | "argv0"
            | "auto_path"
            | "env"
            | "errorCode"
            | "errorInfo"
            | "errorResult"
            | "tcl_interactive"
            | "tcl_library"
            | "tcl_patchLevel"
            | "tcl_pkgPath"
            | "tcl_platform"
            | "tcl_precision"
            | "tcl_rcFileName"
            | "tcl_version"
            | "tcl_wordchars"
            | "tcl_nonwordchars"
            | "static"
    )
}

/// Names whose whole binding is removed by an `unset` call.  Conservative
/// vs. `compiler/core_analyses.py`: only a **literal** bare name kills
/// (a dynamic `unset $name` targets the variable *named by* `$name`, not
/// `name` itself — yet the IR records `name` in the call's defs, so a
/// `$`-stripping harvest would wrongly mark it killed).  Per-element
/// `unset x(k)` drops one array element, not the binding, so it is
/// skipped too.
fn whole_unset_names(args: &[String]) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut i = 0;
    while i < args.len() && args[i].starts_with('-') {
        let is_dashdash = args[i] == "--";
        i += 1;
        if is_dashdash {
            break;
        }
    }
    for raw in &args[i..] {
        // Literal bare names only — skip dynamic (`$`/`${…}`/`[…]`) and
        // element-subscripted (`x(k)`) targets.
        if raw.contains('$') || raw.contains('[') || raw.contains('(') {
            continue;
        }
        let base = crate::naming::normalise_var_name(raw);
        if !base.is_empty() {
            out.insert(base.to_string());
        }
    }
    out
}

/// Tcl ARE metacharacters: a pattern free of these reduces to a literal
/// substring search.  Mirrors `_TCL_REGEX_METACHARS`.
const TCL_REGEX_METACHARS: &str = r"\^$.|?*+()[]{}";

/// `regexp` switches that don't change match-vs-no-match for a pure-literal
/// pattern.  Mirrors `_REGEXP_LITERAL_SAFE_SWITCHES`.
fn is_regexp_literal_safe_switch(opt: &str) -> bool {
    matches!(
        opt,
        "-indices" | "-inline" | "-all" | "-line" | "-lineanchor" | "-linestop" | "-start" | "--"
    )
    // `-expanded` is handled separately (whitespace/comment-gated) by the
    // caller, so it is intentionally not listed here.
}

/// True iff `regexp PATTERN INPUT` provably returns 0.  Sound only when
/// `pat` is a pure-literal pattern (no ARE metacharacters), reducing the
/// match to substring search.  Unknown / unsafe switches bail (return
/// `false` = cannot prove no-match).  Mirrors `_regexp_literal_no_match`.
fn regexp_literal_no_match(pat: &str, inp: &str, options: &[String]) -> bool {
    if pat.chars().any(|c| TCL_REGEX_METACHARS.contains(c)) {
        return false;
    }
    let mut nocase = false;
    let mut expanded = false;
    for opt in options {
        if !opt.starts_with('-') {
            continue; // an option value (e.g. after `-start`)
        }
        if opt == "-nocase" {
            nocase = true;
            continue;
        }
        if opt == "-expanded" {
            expanded = true;
            continue;
        }
        if is_regexp_literal_safe_switch(opt) {
            continue;
        }
        return false; // unknown / unsafe switch
    }
    // `-expanded` makes Tcl ignore unescaped whitespace and `#`-comments in
    // the pattern, so a pattern containing either is NOT a plain substring
    // (`regexp -expanded {a b} {ab}` matches).  Bail in that case so the
    // no-match proof stays sound — a whitespace/comment-free literal is
    // still safe.
    if expanded && pat.chars().any(|c| c.is_whitespace() || c == '#') {
        return false;
    }
    if nocase {
        !inp.to_lowercase().contains(&pat.to_lowercase())
    } else {
        !inp.contains(pat)
    }
}

/// `Some(true)` when a `regexp` / `scan` call (`is_regexp` selects the arg
/// order) with literal pattern + input provably can't match; `Some(false)`
/// when it might match; `None` when the args can't be statically resolved
/// (dynamic substitution, too few args).  Mirrors the per-call arm of the
/// `provably_unset` setup in `_read_before_set`.
fn regexp_scan_no_match(is_regexp: bool, args: &[String]) -> Option<bool> {
    let value_opts: &[&str] = if is_regexp { &["-start"] } else { &[] };
    let pos = skip_options(args, value_opts);
    if pos + 1 >= args.len() {
        return None;
    }
    let a = &args[pos];
    let b = &args[pos + 1];
    // `regexp ?opts? PATTERN STRING …`; `scan STRING FORMAT …`.
    let (pat, inp) = if is_regexp { (a, b) } else { (b, a) };
    // Dynamic substitution markers — runtime value unknown.
    if pat.contains(['$', '[']) || inp.contains(['$', '[']) {
        return None;
    }
    if is_regexp {
        let opts: Vec<String> = args[..pos].to_vec();
        Some(regexp_literal_no_match(pat, inp, &opts))
    } else {
        Some(crate::scan_predicate::scan_provably_no_match(pat, inp))
    }
}

/// Index of the first non-option argument in `args`, skipping `-option`
/// flags and the values of options in `value_opts`.  Mirrors `skip_options`.
fn skip_options(args: &[String], value_opts: &[&str]) -> usize {
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--" {
            i += 1;
            break;
        }
        if a.starts_with('-') {
            i += 1;
            if value_opts.contains(&a.as_str()) && i < args.len() {
                i += 1;
            }
            continue;
        }
        break;
    }
    i
}

/// Phi-from-undef trace.  A use's SSA version > 0 normally proves a prior
/// definition reached it, but a phi result whose reachable incomings
/// include an undefined (version-0) or `unset`-killed origin only reaches
/// on a subset of paths — the others read an unset variable.  Returns
/// true when `(name, version)` can be undefined on some reachable path.
///
/// Mirrors `_phi_can_undef` in `compiler/core_analyses.py`.  Version 0 is
/// the undef origin; an `unset`-killed version is undef; a non-phi
/// (concrete) definition is never undef; a phi is undef if any of its
/// reachable, non-existence-guarded incomings is undef.  Cycles
/// (loop-header phis) conservatively resolve to *not* undef on the cycle.
#[allow(clippy::too_many_arguments)]
fn phi_can_undef(
    name: &str,
    version: crate::ssa::Version,
    phi_def: &std::collections::HashMap<(String, crate::ssa::Version), crate::ssa::Phi>,
    killed: &HashSet<(String, crate::ssa::Version)>,
    considered: &HashSet<String>,
    exists_guards: &[(String, String)],
    ssa: &crate::ssa::SsaFunction,
    seen: &mut HashSet<(String, crate::ssa::Version)>,
) -> bool {
    if version == 0 {
        return true;
    }
    let key = (name.to_string(), version);
    if killed.contains(&key) {
        return true;
    }
    if seen.contains(&key) {
        // Cycle (loop-header phi): the DFS seed already accounted for the
        // entry path's contribution; treat the back-edge as not-undef to
        // avoid every loop-header phi self-triggering.
        return false;
    }
    let Some(phi) = phi_def.get(&key) else {
        // Concrete (non-phi) definition reached this version — safe.
        return false;
    };
    seen.insert(key.clone());
    let mut result = false;
    for (pred, &incoming_ver) in &phi.incoming {
        if !considered.contains(pred) {
            continue;
        }
        // A dominating existence guard proves the variable is defined at
        // the predecessor; that incoming cannot be undef regardless of
        // its SSA version.
        if exists_guards
            .iter()
            .any(|(gv, gblk)| gv == name && block_dominated_by(ssa, pred, gblk))
        {
            continue;
        }
        if phi_can_undef(
            name,
            incoming_ver,
            phi_def,
            killed,
            considered,
            exists_guards,
            ssa,
            seen,
        ) {
            result = true;
            break;
        }
    }
    seen.remove(&key);
    result
}

/// `(name, version) → Phi` index used by [`phi_can_undef`].
type PhiDefMap = std::collections::HashMap<(String, crate::ssa::Version), crate::ssa::Phi>;

/// Build the `(name, version) → Phi` index and the set of `unset`-killed
/// versions for [`phi_can_undef`], restricted to `considered` (executable)
/// blocks.  Mirrors the `phi_def` / `killed_versions` setup in
/// `compiler/core_analyses.py::_read_before_set`.
fn build_phi_undef_index(
    ssa: &crate::ssa::SsaFunction,
    considered: &HashSet<String>,
) -> (PhiDefMap, HashSet<(String, crate::ssa::Version)>) {
    use crate::ir::Statement;
    let mut phi_def: std::collections::HashMap<(String, crate::ssa::Version), crate::ssa::Phi> =
        std::collections::HashMap::new();
    let mut killed: HashSet<(String, crate::ssa::Version)> = HashSet::new();
    for bn in considered {
        let Some(sblock) = ssa.blocks.get(bn) else {
            continue;
        };
        for phi in &sblock.phis {
            phi_def.insert((phi.name.clone(), phi.version), phi.clone());
        }
        for s in &sblock.statements {
            let Statement::Call {
                command,
                canonical_command,
                args,
                ..
            } = &s.statement
            else {
                continue;
            };
            let is_unset = canonical_command.as_deref() == Some("::unset") || command == "unset";
            if !is_unset {
                continue;
            }
            let whole = whole_unset_names(args);
            for (def_name, def_ver) in &s.defs {
                if whole.contains(def_name) {
                    killed.insert((def_name.clone(), *def_ver));
                }
            }
        }
    }
    (phi_def, killed)
}

/// True when an expression operand is provably the integer zero: a literal
/// `0` (int or float spelling) or a variable whose SCCP value at `versions`
/// is a constant zero.  Used by the W233 divide-by-zero check.
fn expr_operand_is_zero(
    node: &ExprNode,
    versions: &std::collections::HashMap<String, crate::ssa::Version>,
    sccp: &std::collections::HashMap<crate::ssa::ValueKey, crate::analyses::LatticeValue>,
) -> bool {
    use crate::analyses::{ConstValue, LatticeValue};
    let const_is_zero = |lv: Option<&LatticeValue>| match lv {
        Some(LatticeValue::Const(ConstValue::Int(0) | ConstValue::Bool(false))) => true,
        Some(LatticeValue::Const(ConstValue::Float(f))) => *f == 0.0,
        Some(LatticeValue::Const(ConstValue::String(s))) => {
            let t = s.trim();
            t.parse::<i64>() == Ok(0) || t.parse::<f64>().is_ok_and(|f| f == 0.0)
        }
        _ => false,
    };
    match node {
        ExprNode::Literal { text, .. } => {
            let t = text.trim();
            t.parse::<i64>() == Ok(0) || t.parse::<f64>().is_ok_and(|f| f == 0.0)
        }
        ExprNode::Var { name, .. } => versions
            .get(name)
            .is_some_and(|&ver| const_is_zero(sccp.get(&(name.clone(), ver)))),
        _ => false,
    }
}

/// Constant truthiness of `text` under Tcl boolean rules: a non-zero number
/// is true, `0`/`0.0` false, and the literal boolean words (case-insensitive)
/// map directly.  `None` when not a recognised constant.
fn const_truthiness(text: &str) -> Option<bool> {
    let t = text.trim();
    if let Ok(n) = t.parse::<i64>() {
        return Some(n != 0);
    }
    if let Ok(f) = t.parse::<f64>() {
        return Some(f != 0.0);
    }
    match t.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Some(true),
        "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Provable truthiness of an expression operand (`Some(true)`/`Some(false)`),
/// or `None` when not statically known.  Used to model short-circuit (`&&` /
/// `||`) and ternary reachability for the W233 divisor walk.
fn expr_truthiness(
    node: &ExprNode,
    versions: &std::collections::HashMap<String, crate::ssa::Version>,
    sccp: &std::collections::HashMap<crate::ssa::ValueKey, crate::analyses::LatticeValue>,
) -> Option<bool> {
    use crate::analyses::{ConstValue, LatticeValue};
    match node {
        ExprNode::Literal { text, .. } => const_truthiness(text),
        ExprNode::Var { name, .. } => {
            let ver = versions.get(name)?;
            match sccp.get(&(name.clone(), *ver))? {
                LatticeValue::Const(ConstValue::Int(n)) => Some(*n != 0),
                LatticeValue::Const(ConstValue::Float(f)) => Some(*f != 0.0),
                LatticeValue::Const(ConstValue::Bool(b)) => Some(*b),
                LatticeValue::Const(ConstValue::String(s)) => const_truthiness(s),
                _ => None,
            }
        }
        // `-1` / `+1` keep the operand's zero-ness; `!x` / `not x` invert it.
        ExprNode::Unary { op, operand } => match op {
            UnaryOp::Neg | UnaryOp::Pos | UnaryOp::BitNot => {
                expr_truthiness(operand, versions, sccp)
            }
            UnaryOp::Not | UnaryOp::WordNot => expr_truthiness(operand, versions, sccp).map(|b| !b),
        },
        _ => None,
    }
}

/// Find the first `/` or `%` operator in `node` whose divisor is provably
/// zero **and is actually reachable**, returning its [`BinOp`].  Models
/// short-circuit `&&` / `||` and ternary reachability so a guarded division
/// (`$d != 0 && 1/$d`, `0 ? 1/0 : 7`) does not fire.  Mirrors
/// `find_divide_by_zero` (`compiler/interval_bounds.py`).
fn find_divide_by_zero(
    node: &ExprNode,
    versions: &std::collections::HashMap<String, crate::ssa::Version>,
    sccp: &std::collections::HashMap<crate::ssa::ValueKey, crate::analyses::LatticeValue>,
) -> Option<BinOp> {
    let recurse = |n| find_divide_by_zero(n, versions, sccp);
    match node {
        ExprNode::Binary { op, left, right } => {
            if matches!(op, BinOp::Div | BinOp::Mod) && expr_operand_is_zero(right, versions, sccp)
            {
                return Some(*op);
            }
            // The left operand is always evaluated.  The right operand of a
            // short-circuit `&&` is reached only when the left is provably
            // truthy; of `||` only when the left is provably falsy.
            if let Some(hit) = recurse(left) {
                return Some(hit);
            }
            let right_reachable = match op {
                BinOp::And | BinOp::WordAnd => expr_truthiness(left, versions, sccp) == Some(true),
                BinOp::Or | BinOp::WordOr => expr_truthiness(left, versions, sccp) == Some(false),
                _ => true,
            };
            if right_reachable {
                recurse(right)
            } else {
                None
            }
        }
        ExprNode::Unary { operand, .. } => recurse(operand),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            // The condition is always evaluated; an arm only when the
            // condition provably selects it.
            if let Some(hit) = recurse(condition) {
                return Some(hit);
            }
            match expr_truthiness(condition, versions, sccp) {
                Some(true) => recurse(true_branch),
                Some(false) => recurse(false_branch),
                None => None,
            }
        }
        _ => None,
    }
}

/// Name-level suppression context for the `return`-value phi-from-undef W210
/// pass, harvested from `dict with` / `dict update` and qualified `variable`
/// declarations.  Mirrors the corresponding `skip` / key-set construction in
/// `compiler/core_analyses.py::_read_before_set`.
#[derive(Default)]
struct UndefSuppression {
    /// A `dict with` / `dict update` is present (enables the key-aware gate).
    has_dict_with: bool,
    /// At least one dict-with target's value shape is statically unknown.
    dict_with_any_unknown: bool,
    /// Keys provably unpacked by some known-literal dict-with target.
    dict_with_known_keys: HashSet<String>,
    /// The dict-with target variable names themselves.
    dict_vars: HashSet<String>,
    /// Names with a concrete (version > 0) statement/phi definition.
    explicitly_defined: HashSet<String>,
    /// Local-alias tails declared by a qualified `variable ns::tail`.
    alias_tails: HashSet<String>,
}

impl UndefSuppression {
    /// True when a read of `name` is suppressed by an alias declaration or a
    /// `dict with` / `dict update` unpack.  Blanket variant: an unknown-shape
    /// dict suppresses every non-concrete name (the conservative
    /// "might-have-the-key" stance, used where no truth source can confirm
    /// the dict is empty — e.g. a `return` after a `dict with` on a param).
    fn suppresses(&self, name: &str) -> bool {
        self.suppresses_strict(name)
            || (self.has_dict_with
                && self.dict_with_any_unknown
                && !self.explicitly_defined.contains(name))
    }

    /// Like [`Self::suppresses`] but **without** the unknown-shape blanket —
    /// only alias tails, dict vars, and *provably-unpacked* keys suppress.
    /// Used on statement reads inside a `dict with` body, where an
    /// unknown-shape dict (e.g. an interprocedurally-empty literal Rust's
    /// SCCP cannot yet resolve) must still fire so a genuine missing-key read
    /// is not hidden.
    fn suppresses_strict(&self, name: &str) -> bool {
        if self.alias_tails.contains(name) || self.dict_vars.contains(name) {
            return true;
        }
        self.has_dict_with
            && !self.explicitly_defined.contains(name)
            && self.dict_with_known_keys.contains(name)
    }
}

/// Build the [`UndefSuppression`] context over `considered` blocks.
fn build_undef_suppression(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<String>,
) -> UndefSuppression {
    use crate::ir::Statement;
    let mut s = UndefSuppression::default();

    // `dict with` / `dict update`: harvest the dict-var names and, when the
    // dict value is a same-block literal, its keys (key-aware suppression).
    for bn in considered {
        let Some(block) = fu.cfg.blocks.get(bn) else {
            continue;
        };
        for (idx, stmt) in block.statements.iter().enumerate() {
            let (Statement::Barrier { command, args, .. } | Statement::Call { command, args, .. }) =
                stmt
            else {
                continue;
            };
            let is_dict = command == "dict" || stmt.canonical_command_or_source() == "::dict";
            if !is_dict {
                continue;
            }
            if args.first().map(String::as_str) != Some("with")
                && args.first().map(String::as_str) != Some("update")
            {
                continue;
            }
            s.has_dict_with = true;
            let Some(dict_var) = args.get(1) else {
                s.dict_with_any_unknown = true;
                continue;
            };
            let dvar = crate::naming::normalise_var_name(dict_var).to_string();
            if dvar.is_empty() {
                s.dict_with_any_unknown = true;
                continue;
            }
            s.dict_vars.insert(dvar.clone());
            // Resolve the dict's value to harvest its keys.  Prefer the
            // SCCP CONST of the SPECIFIC version read by this dict-with (so
            // interprocedurally-propagated literals — a caller passing `{}`
            // — are honoured), falling back to a same-block literal `set`.
            // A known value (even empty) harvests its keys; only a value
            // that resolves to neither marks the dict shape unknown.
            let mut literal: Option<String> = None;
            if let Some(sb) = fu.ssa.blocks.get(bn) {
                if let Some(ver) = sb
                    .statements
                    .get(idx)
                    .and_then(|s| s.uses.get(&dvar).copied())
                {
                    if let Some(crate::analyses::LatticeValue::Const(
                        crate::analyses::ConstValue::String(v),
                    )) = fu.sccp.values.get(&(dvar.clone(), ver))
                    {
                        literal = Some(v.clone());
                    }
                }
            }
            if literal.is_none() {
                for prev in (0..idx).rev() {
                    match &block.statements[prev] {
                        Statement::AssignConst { name, value, .. }
                            if crate::naming::normalise_var_name(name) == dvar =>
                        {
                            literal = Some(value.clone());
                            break;
                        }
                        // A barrier between us and the literal invalidates it.
                        Statement::Barrier { .. } => break,
                        _ => {}
                    }
                }
            }
            match literal {
                Some(v) => {
                    for (i, key) in crate::tcl_expr_eval::split_tcl_list(&v)
                        .into_iter()
                        .enumerate()
                    {
                        if i % 2 == 0 {
                            s.dict_with_known_keys.insert(key);
                        }
                    }
                }
                None => s.dict_with_any_unknown = true,
            }
        }
    }

    // Names with a concrete (version > 0) statement or phi definition — a
    // dict-with scope never suppresses these (they are genuinely set).
    if s.has_dict_with {
        for bn in considered {
            let Some(sb) = fu.ssa.blocks.get(bn) else {
                continue;
            };
            for st in &sb.statements {
                for (n, v) in &st.defs {
                    if *v > 0 {
                        s.explicitly_defined.insert(n.clone());
                    }
                }
            }
            for phi in &sb.phis {
                if phi.version > 0 {
                    s.explicitly_defined.insert(phi.name.clone());
                }
            }
        }
    }

    s.alias_tails = collect_qualified_variable_alias_tails(fu, considered);
    s
}

/// Local-alias tail names declared by a *qualified* `variable`
/// (`variable ns::tail` / `variable ${name}::tail`): the bare tail read
/// resolves to the namespace var, not an unset local.  Mirrors
/// `compiler/core_analyses.py::_qualified_variable_alias_tails`.
fn collect_qualified_variable_alias_tails(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<String>,
) -> HashSet<String> {
    use crate::ir::Statement;
    let mut tails = HashSet::new();
    for bn in considered {
        let Some(block) = fu.cfg.blocks.get(bn) else {
            continue;
        };
        for stmt in &block.statements {
            let (Statement::Barrier { command, args, .. } | Statement::Call { command, args, .. }) =
                stmt
            else {
                continue;
            };
            if command != "variable" && stmt.canonical_command_or_source() != "::variable" {
                continue;
            }
            // `variable` alternates (name, value?) pairs — names at even args.
            let mut i = 0;
            while i < args.len() {
                let text = &args[i];
                if text.contains("::") {
                    let tail = text.rsplit("::").next().unwrap_or(text);
                    let (base, _) = crate::naming::split_array_name(tail);
                    if !base.is_empty()
                        && !base.contains('$')
                        && !base.contains('[')
                        && !base.contains('{')
                    {
                        tails.insert(crate::naming::normalise_var_name(base).to_string());
                    }
                }
                i += 2;
            }
        }
    }
    tails
}

/// Collect every variable name defined anywhere in `cfg`.
///
/// Mirrors `_collect_defined_vars` in
/// `_diag_var_lifecycle.py:123-133`.  Walks every block and pulls
/// the `defs` field off each [`crate::ir::Statement`] that has
/// one (assignments, ``incr``, ``Call`` statements with explicit
/// defs).  Used for the "did you mean…?" case-mismatch
/// suggestion in W210 / W211 / W220 messages.
fn collect_defined_vars(cfg: &crate::cfg::Function) -> HashSet<String> {
    use crate::ir::Statement;
    let mut names: HashSet<String> = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { name, .. }
                | Statement::AssignExpr { name, .. }
                | Statement::AssignValue { name, .. }
                | Statement::Incr { name, .. } => {
                    let normalised = crate::naming::normalise_var_name(name);
                    if !normalised.is_empty() {
                        names.insert(normalised.to_string());
                    }
                }
                Statement::Call { defs, .. } => {
                    for def in defs {
                        names.insert(def.clone());
                    }
                }
                _ => {}
            }
        }
    }
    names
}

/// Compute the set of global variable names that any procedure
/// in `cu` writes.
///
/// Mirrors `_globals_written_by_procs` in
/// `core/analysis/_analyser/_diag_commands.py:264-296`.
///
/// A global write happens when a proc either:
///
/// 1. assigns to a fully-qualified name (``::var``), or
/// 2. declares ``global var`` and then assigns to ``var`` in the
///    same proc body.
///
/// The result is the union of (1) and the intersection of
/// global aliases × locally-written names (case (2)).  Used at
/// top-level to suppress W210 for globals a helper proc may
/// populate before the top-level read.
///
/// **Simplification vs. Python.** The Rust port doesn't yet
/// have ``CommandRegistry::is_destroys_variable`` so commands
/// like ``unset`` aren't filtered out of the "writes" set.
/// That makes the suppression slightly more permissive (more
/// vars marked "written-by-procs" → more W210 suppressions).
/// Safe-on-correctness — the alternative is false positives
/// on real RBS sites.  When the registry gains
/// ``destroys_variable``, add the filter here for parity.
fn globals_written_by_procs(cu: &crate::compilation_unit::CompilationUnit) -> HashSet<String> {
    use crate::ir::Statement;
    let mut result: HashSet<String> = HashSet::new();
    for fu in cu.procedures.values() {
        let mut global_aliases: HashSet<String> = HashSet::new();
        let mut written: HashSet<String> = HashSet::new();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let names: Vec<&String> = match stmt {
                    Statement::Call { command, defs, .. } => {
                        if command == "global" {
                            for d in defs {
                                global_aliases.insert(d.clone());
                            }
                            continue;
                        }
                        if matches!(command.as_str(), "variable" | "upvar") {
                            continue;
                        }
                        defs.iter().collect()
                    }
                    Statement::AssignConst { name, .. }
                    | Statement::AssignExpr { name, .. }
                    | Statement::AssignValue { name, .. }
                    | Statement::Incr { name, .. } => vec![name],
                    _ => continue,
                };
                for name in names {
                    if let Some(bare) = name.strip_prefix("::") {
                        let bare = bare.trim_start_matches(':');
                        if !bare.is_empty() {
                            result.insert(bare.to_string());
                        }
                    } else {
                        written.insert(name.clone());
                    }
                }
            }
        }
        for n in global_aliases.intersection(&written) {
            result.insert(n.clone());
        }
    }
    result
}

/// External OO base classes that aren't in the per-document
/// ``ClassDef`` index but are recognised as legitimate
/// superclasses for W308 / W308-related gates.
const OO_BASE: [&str; 2] = ["oo::object", "oo::class"];

/// Extract the first single-quoted word from a diagnostic
/// message string, or `None` if the message has no quoted run.
///
/// Used by [`Analyser::resolve_interpolated_w123_diagnostics`]
/// to recover the command name from a "Unknown command 'NAME'"
/// W123 message.  Mirrors the Python equivalent in
/// `_diag_commands.py:233-237`.
fn extract_quoted_word(message: &str) -> Option<String> {
    let start = message.find('\'')?;
    let rest = &message[start + 1..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

/// Return ``true`` when ``body`` contains a ``$param`` /
/// ``${param}`` substitution.  Used as a fallback by the W214
/// (unused-parameter) emitter to suppress the warning when the
/// parameter is read inside a ``[expr {...}]`` / ``[cmd ...]``
/// substitution that the IR lowerer doesn't track as a use.
///
/// Conservative — false negatives are fine (W214 still fires
/// when the param genuinely isn't referenced), but false
/// positives would cause the over-emit this guard exists to
/// prevent.  The bare-name match enforces a non-identifier
/// boundary on each side so ``$abc`` doesn't match ``$ab``,
/// and skips the variable when it follows a ``\\`` escape.
fn body_references_param(body: &str, param: &str) -> bool {
    if param.is_empty() {
        return false;
    }
    let bytes = body.as_bytes();
    let plen = param.len();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c != b'$' {
            i += 1;
            continue;
        }
        // Skip backslash-escaped ``\$``.
        if i > 0 && bytes[i - 1] == b'\\' {
            i += 1;
            continue;
        }
        // ``${name}`` form.
        if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            let start = i + 2;
            if start + plen <= bytes.len()
                && &bytes[start..start + plen] == param.as_bytes()
                && start + plen < bytes.len()
                && bytes[start + plen] == b'}'
            {
                return true;
            }
        } else {
            // ``$name`` form — bare identifier match.
            let start = i + 1;
            if start + plen <= bytes.len() && &bytes[start..start + plen] == param.as_bytes() {
                let after = start + plen;
                let next_ok = after >= bytes.len() || !is_ident_continue(bytes[after]);
                if next_ok {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b':'
}

/// Credential-bearing option flags whose literal values trip W310
/// (the generic `_DEFAULT_PASSWORD_OPTIONS` from `_security.py`).
const DEFAULT_PASSWORD_OPTIONS: [&str; 5] = ["-password", "-pass", "-secret", "-token", "-apikey"];

/// True when `value` is a literal (not a `$var` / `[cmd]` substitution)
/// — the W310 `_is_literal_value` gate.
fn is_literal_credential_value(value: &str, tok: &tcl_lexer::Token) -> bool {
    !matches!(
        tok.kind,
        tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
    ) && !value.starts_with('$')
        && !value.contains('[')
}

/// Return `true` when an `uplevel` first argument is a level
/// specifier (`1`, `#0`, …) rather than the script itself.  Mirrors
/// Python's `args[0].lstrip("#").isdigit() or args[0] == "#0"`: strip
/// any leading `#` then require a non-empty all-digit remainder.
fn uplevel_has_level(arg0: &str) -> bool {
    let stripped = arg0.trim_start_matches('#');
    !stripped.is_empty() && stripped.bytes().all(|b| b.is_ascii_digit())
}

/// Parse `subst`'s flags, returning `(template_idx, nocommands,
/// novariables)` — the index of the first non-option argument (the
/// template) and which substitution-suppressing flags were seen.
/// Mirrors `core/analysis/checks/_helpers.py::_parse_subst_flags`
/// (`-nobackslashes` is accepted but irrelevant to the W102 message).
fn parse_subst_flags(args: &[String]) -> (Option<usize>, bool, bool) {
    let mut nocommands = false;
    let mut novariables = false;
    let mut template_idx = None;
    for (i, text) in args.iter().enumerate() {
        match text.as_str() {
            "-nocommands" => nocommands = true,
            "-novariables" => novariables = true,
            "-nobackslashes" => {}
            t if t.starts_with('-') => {}
            _ => {
                template_idx = Some(i);
                break;
            }
        }
    }
    (template_idx, nocommands, novariables)
}

/// Return `(pattern_text, token)` pairs for every regex pattern
/// argument in a `regexp` / `regsub` / `switch -regexp` command.
/// Mirrors `core/analysis/checks/_helpers.py::
/// _find_regex_patterns_in_command`: `regexp` / `regsub` contribute
/// their first positional (option-skipping) argument; `switch -regexp`
/// contributes every non-`default` pattern arm — inline pairs (form 1)
/// or a single braced case list (form 2, re-segmented via
/// [`super::handlers::parse_switch_body_elements`]).
fn find_regex_patterns_in_command(
    cmd_name: &str,
    args: &[String],
    arg_tokens: &[tcl_lexer::Token],
) -> Vec<(String, tcl_lexer::Token)> {
    if args.is_empty() || arg_tokens.is_empty() {
        return Vec::new();
    }
    match cmd_name {
        "regexp" | "regsub" => {
            // Skip leading flags to the pattern argument (`-start`
            // consumes its value; `--` ends the option section).
            let mut idx = 0;
            while idx < args.len() && args[idx].starts_with('-') && args[idx] != "--" {
                if args[idx] == "-start" && idx + 1 < args.len() {
                    idx += 2;
                } else {
                    idx += 1;
                }
            }
            if idx < args.len() && args[idx] == "--" {
                idx += 1;
            }
            match (args.get(idx), arg_tokens.get(idx)) {
                (Some(text), Some(tok)) => vec![(text.clone(), *tok)],
                _ => Vec::new(),
            }
        }
        "switch" => {
            let mut is_regexp = false;
            let mut i = 0;
            while i < args.len() && args[i].starts_with('-') {
                if args[i] == "-regexp" {
                    is_regexp = true;
                }
                if args[i] == "--" {
                    i += 1;
                    break;
                }
                i += 1;
            }
            if !is_regexp {
                return Vec::new();
            }
            // Skip the `string` argument.
            i += 1;
            let mut results = Vec::new();
            if i < args.len() && i == args.len() - 1 {
                // Form 2: single braced case list.
                if let Some(case_tok) = arg_tokens.get(i) {
                    let elements = super::handlers::parse_switch_body_elements(&args[i], *case_tok);
                    let mut j = 0;
                    while j + 1 < elements.len() {
                        let (text, tok) = &elements[j];
                        if text != "default" {
                            results.push((text.clone(), *tok));
                        }
                        j += 2;
                    }
                }
            } else {
                // Form 1: inline pattern/body pairs.
                while i + 1 < args.len() {
                    if let (Some(text), Some(tok)) = (args.get(i), arg_tokens.get(i)) {
                        if text != "default" {
                            results.push((text.clone(), *tok));
                        }
                    }
                    i += 2;
                }
            }
            results
        }
        _ => Vec::new(),
    }
}

/// Walk `node` and collect every `==`/`!=` operator whose at least
/// one operand is a string literal ([`ExprNode::String`]).
///
/// Mirrors `_find_string_eq_ne` in
/// `core/analysis/checks/_style.py:685-713`.  Comparisons between
/// two variables (`$x == $y`) are intentionally *not* collected —
/// the variables may hold integer values, making `==` correct.
fn find_string_eq_ne_ops(node: &ExprNode) -> Vec<BinOp> {
    let mut found = Vec::new();
    walk_string_eq_ne(node, &mut found);
    found
}

fn walk_string_eq_ne(node: &ExprNode, found: &mut Vec<BinOp>) {
    match node {
        ExprNode::Binary { op, left, right } => {
            walk_string_eq_ne(left, found);
            walk_string_eq_ne(right, found);
            if matches!(op, BinOp::Eq | BinOp::Ne)
                && (matches!(**left, ExprNode::String { .. })
                    || matches!(**right, ExprNode::String { .. }))
            {
                found.push(*op);
            }
        }
        ExprNode::Unary { operand, .. } => walk_string_eq_ne(operand, found),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_string_eq_ne(condition, found);
            walk_string_eq_ne(true_branch, found);
            walk_string_eq_ne(false_branch, found);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                walk_string_eq_ne(arg, found);
            }
        }
        _ => {}
    }
}

/// Count the total number of `==`/`!=` operators in the expression
/// tree.  Mirrors `_count_eq_ne_ops` in
/// `core/analysis/checks/_style.py:716-731`.
fn count_eq_ne_ops(node: &ExprNode) -> usize {
    match node {
        ExprNode::Binary { op, left, right } => {
            let mut n = count_eq_ne_ops(left) + count_eq_ne_ops(right);
            if matches!(op, BinOp::Eq | BinOp::Ne) {
                n += 1;
            }
            n
        }
        ExprNode::Unary { operand, .. } => count_eq_ne_ops(operand),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            count_eq_ne_ops(condition)
                + count_eq_ne_ops(true_branch)
                + count_eq_ne_ops(false_branch)
        }
        ExprNode::Call { args, .. } => args.iter().map(count_eq_ne_ops).sum(),
        _ => 0,
    }
}

/// Rewrite `==`/`!=` operators to ` eq `/` ne ` for use in a code
/// fix's replacement text.  Mirrors `_rewrite_string_compare_ops`
/// in `core/analysis/checks/_helpers.py:82-88`.
///
/// Implements the Python regex semantics manually:
/// * `(?<![=!])==(?!=)`  → ` eq `
/// * `!=`                → ` ne `
/// * `[ \t]{2,}`         → ` `  (collapse runs of 2+ ws)
fn rewrite_string_compare_ops(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut step1 = String::with_capacity(text.len() + 8);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // !=  →  " ne "
        if c == '!' && i + 1 < chars.len() && chars[i + 1] == '=' {
            step1.push_str(" ne ");
            i += 2;
            continue;
        }
        // ==  →  " eq "  (with negative look-around)
        if c == '=' && i + 1 < chars.len() && chars[i + 1] == '=' {
            let prev_ok = i == 0 || (chars[i - 1] != '=' && chars[i - 1] != '!');
            let next_ok = i + 2 >= chars.len() || chars[i + 2] != '=';
            if prev_ok && next_ok {
                step1.push_str(" eq ");
                i += 2;
                continue;
            }
        }
        step1.push(c);
        i += 1;
    }
    // Collapse runs of 2+ space/tab into a single space.  Single
    // whitespace characters are preserved (matches Python's
    // ``re.sub(r"[ \t]{2,}", " ", ...)``).
    let chars: Vec<char> = step1.chars().collect();
    let mut out = String::with_capacity(step1.len());
    let mut i = 0;
    while i < chars.len() {
        if (chars[i] == ' ' || chars[i] == '\t')
            && i + 1 < chars.len()
            && (chars[i + 1] == ' ' || chars[i + 1] == '\t')
        {
            out.push(' ');
            while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
                i += 1;
            }
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Scan `args` for the first positional argument that lacks a
/// preceding `--` terminator.  Mirrors
/// `core/analysis/checks/_helpers.py::_first_positional_without_terminator`.
///
/// Skips option words (text starts with `-`); skips an additional
/// argument when the option's [`OptionSpec`](tcl_registry::prelude::OptionSpec)
/// in [`ResolvedTerminator::options`](tcl_registry::ResolvedTerminator)
/// has `takes_value == true`.  Linear scan over the borrowed
/// option slice — per-command option counts are small (≤ a dozen
/// for the largest specs in practice), so this is cheaper than a
/// per-resolve `HashSet` allocation on the analyser hot path.
/// Returns `None` when a `--` is encountered (positional arguments
/// after `--` are explicitly terminated).
fn first_positional_without_terminator(
    args: &[String],
    profile: &tcl_registry::ResolvedTerminator,
) -> Option<usize> {
    let mut i = profile.scan_start;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--" {
            return None;
        }
        if arg.starts_with('-') {
            i += 1;
            let consumes_value = profile
                .options
                .iter()
                .any(|o| o.name == arg && o.takes_value);
            if consumes_value && i < args.len() {
                i += 1;
            }
            continue;
        }
        return Some(i);
    }
    None
}

/// Locate the most-recent literal `set var value` assignment whose
/// command-head precedes `before_offset`.  Mirrors
/// `core/analysis/checks/_helpers.py::_last_literal_set_value_for_var`.
///
/// Returns `Some((value_text, value_span, var_text))` when the
/// nearest preceding `set` is a fully-literal three-arg form.
/// Returns `None` when the latest assignment is dynamic / multi-
/// token (the runtime value cannot be proven statically).
fn last_literal_set_value_for_var(
    source: &str,
    var_name: &str,
    before_offset: u32,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, tcl_lexer::Span, String)> {
    if var_name.is_empty() || before_offset == 0 {
        return None;
    }
    let head = before_offset as usize;
    if head > source.len() {
        return None;
    }
    let prefix = &source[..head];
    let segments = crate::segmenter::segment_commands_with_offset_and_config(prefix, 0, config);

    for cmd in segments.iter().rev() {
        // Cross-scope guard: stop the backward scan at a `proc NAME
        // {PARAMS} BODY` whose body *contains* the use offset and whose
        // params include `var_name` — the parameter shadows any outer
        // scope, so an outer `set` must not be attributed to the inner
        // use.  The use is inside the proc body iff that proc is the one
        // left unclosed by the truncation at `before_offset`: its span
        // then reaches the last truncated byte (`end + 1 >= head`).  A
        // *complete* proc before the use ends well before that and does
        // not shadow.  Mirrors `_last_literal_set_value_for_var`.
        let use_inside_proc = cmd.span.end() as usize + 1 >= head;
        if use_inside_proc
            && cmd.texts.first().map(String::as_str) == Some("proc")
            && cmd.texts.len() >= 4
            && cmd.texts[2].contains(var_name)
        {
            let shadows = crate::tcl_expr_eval::split_tcl_list(&cmd.texts[2])
                .iter()
                .any(|el| el.split_whitespace().next() == Some(var_name));
            if shadows {
                return None;
            }
        }

        if cmd.texts.first().map(String::as_str) != Some("set") {
            continue;
        }
        if cmd.texts.len() < 3 {
            continue;
        }
        if cmd.texts[1] != var_name {
            continue;
        }
        // Most recent assignment wins.  If it's dynamic, the
        // runtime value can't be proven statically.
        if cmd.single_token_word.get(2).copied() != Some(true) {
            return None;
        }
        if cmd.argv.len() < 3 {
            return None;
        }
        let value_tok = cmd.argv[2];
        if !matches!(
            value_tok.kind,
            tcl_lexer::TokenType::Esc | tcl_lexer::TokenType::Str
        ) {
            return None;
        }
        return Some((cmd.texts[2].clone(), value_tok.span, var_name.to_string()));
    }
    None
}

impl Analyser {
    /// Scope-tree-driven variable diagnostic emitter.
    ///
    /// Mirrors `_emit_variable_usage_diagnostics` in
    /// `_diagnostics.py:111-116`.  Python keeps this method as
    /// an empty hook because W211 (unused-variable) moved to the
    /// SSA-based pass in `_emit_cfg_ssa_diagnostics_for_function`.
    /// The Rust port preserves the hook so future scope-tree-
    /// driven emitters (none currently planned) have a target.
    pub fn emit_variable_usage_diagnostics(&mut self) {
        // Intentionally empty — see module docstring.
    }

    /// **W105.** Emit "unbraced code block" warnings for body
    /// arguments that aren't braced.  Mirrors
    /// ``check_unbraced_body`` in
    /// ``core/analysis/checks/_style.py:238-302``.
    ///
    /// Severity is ERROR when the unbraced body contains
    /// substitutions (``$var`` / ``[cmd]``) — those risk double
    /// substitution.  Severity is WARNING otherwise.  Single
    /// barewords without substitution are silently allowed
    /// (some commands accept a proc name as a body alternative).
    pub(super) fn emit_w105_unbraced_body(
        &mut self,
        cmd_name: &str,
        body_text: &str,
        body_tok: tcl_lexer::Token,
        is_single_token: bool,
    ) {
        // Already braced — `Str` token kind means the source
        // started with ``{``.  Mirrors ``_first_token_is_braced``
        // in Python.
        if matches!(body_tok.kind, tcl_lexer::TokenType::Str) {
            return;
        }
        // A whole-word command-substitution body — `eval [list set y $x]`
        // (the recommended *safe* form), `uplevel [buildScript]` — is
        // produced dynamically and parsed once by the consumer: there is
        // no double-substitution risk and it cannot be braced
        // (`eval {[list …]}` changes the meaning).  Mirrors
        // `check_unbraced_body`'s `tok.type is TokenType.CMD` skip.  (A
        // `Var` body such as `while {$cond} $body` is *not* exempt — only
        // a `Cmd` word is.)
        if matches!(body_tok.kind, tcl_lexer::TokenType::Cmd) {
            return;
        }
        // A body that is a *single bare variable substitution* (`eval $cmd`,
        // `proc $n $a $body`, `after 0 $coroName`, `$state(-command)`) is a
        // script-valued reference, not an inline code block: the variable
        // already holds the script.  Bracing it (`{$cmd}`) would turn the
        // reference into the literal text `$cmd` — the W105 quick-fix is
        // actively wrong here — and the genuine double-substitution (eval) /
        // dynamic-dispatch (command name) risks are W101's and W307's to
        // flag.  A *single-token* word whose token is a `Var` is exactly this
        // case; a composite word (`${t}--Coro`) or quoted interpolated body
        // (`"do $script"`) has more than one fragment and is not exempt.
        // Mirrors `_word_is_single_var` in `analyser/checks/_style.py`.
        if is_single_token && matches!(body_tok.kind, tcl_lexer::TokenType::Var) {
            return;
        }
        let trimmed = body_text.trim();
        // Mirror Python's ``_has_substitution``: textual ``$`` /
        // ``[`` count as substitutions, and so do ``Var`` / ``Cmd``
        // tokens — even when the entire body is a direct
        // substitution (``while {$cond} $body``).  Those still
        // emit W105 at ERROR severity because an unbraced
        // substituted body double-evaluates at runtime.
        let has_substitution = trimmed.contains('$')
            || trimmed.contains('[')
            || matches!(
                body_tok.kind,
                tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
            );
        // Single bareword + no substitution is the alternative
        // form (e.g. body = a proc name).  Skip.
        if !trimmed.contains(char::is_whitespace) && !has_substitution {
            return;
        }
        let severity = if has_substitution {
            super::types::Severity::Error
        } else {
            super::types::Severity::Warning
        };
        let message = if has_substitution {
            format!(
                "Code block argument to '{cmd_name}' is not braced and \
contains substitutions \u{2014} risk of double substitution. \
Use braces: {{ \u{2026} }}"
            )
        } else {
            format!(
                "Code block argument to '{cmd_name}' should be braced \
for clarity and to prevent accidental substitution. \
Use braces: {{ \u{2026} }}"
            )
        };
        let new_text = format!("{{{body_text}}}");
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W105".to_string(),
            span: body_tok.span,
            message,
            severity,
            fixes: vec![super::types::CodeFix {
                span: body_tok.span,
                new_text,
                description: "Wrap code block in braces".to_string(),
            }],
        });
    }

    /// W100 (GAP-A8): an expression argument (`expr` / `if` / `while`
    /// / `for` conditions) that is not braced suffers double
    /// substitution and defeats byte-compilation.  Skips a braced
    /// (`{…}`) argument and a substitution-free numeric/boolean literal;
    /// otherwise emits W100 (ERROR when the text carries a `$`/`[`
    /// substitution, else WARNING) with a brace-wrapping fix.  Mirrors
    /// `check_unbraced_expr`.  `args` / `arg_tokens` exclude the command
    /// name.
    pub(super) fn emit_w100_unbraced_expr(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut indices = registry.arg_indices_for_role(
            cmd_name,
            &arg_strs,
            tcl_registry::arg_role::ArgRole::Expr,
        );
        if indices.is_empty() {
            return;
        }
        indices.sort_unstable();

        let is_expr = cmd_name == "expr";
        let dialect = self.dialect.clone();
        // The whole-`expr` argument span (used when the command is
        // `expr`, whose expression is the remaining words).
        let expr_full_span = (!arg_tokens.is_empty()).then(|| {
            tcl_lexer::Span::new(
                arg_tokens[0].span.start(),
                arg_tokens[arg_tokens.len() - 1].span.end(),
            )
        });
        let any_sub_token = arg_tokens.iter().any(|t| {
            matches!(
                t.kind,
                tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
            )
        });

        let mut pending: Vec<(tcl_lexer::Span, String, bool)> = Vec::new();
        for idx in indices {
            let (Some(tok), Some(arg_text)) = (arg_tokens.get(idx), args.get(idx)) else {
                continue;
            };
            // A braced word (`{…}`, i.e. a `Str` token) is already safe.
            if tok.kind == tcl_lexer::TokenType::Str {
                continue;
            }
            // Resolve the diagnostic span + text: for `expr` the whole
            // remaining-argument span; otherwise the single argument's
            // source slice (preserving `$var` substitutions).
            let (span, text) = if is_expr {
                let sp = expr_full_span.unwrap_or(tok.span);
                (
                    sp,
                    source_slice(&self.source, sp).unwrap_or_else(|| args.join(" ")),
                )
            } else {
                (
                    tok.span,
                    source_slice(&self.source, tok.span).unwrap_or_else(|| arg_text.clone()),
                )
            };
            let stripped = text.trim();
            let safe = if is_expr {
                is_safe_literal_expr(stripped, &dialect)
            } else {
                is_safe_literal(stripped)
            };
            if safe {
                continue;
            }
            let has_sub = text.contains('$')
                || text.contains('[')
                || if is_expr {
                    any_sub_token
                } else {
                    matches!(
                        tok.kind,
                        tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
                    )
                };
            pending.push((span, text, has_sub));
        }

        for (span, text, has_sub) in pending {
            self.push_w100(cmd_name, is_expr, span, &text, has_sub);
        }
    }

    /// Push one W100 diagnostic for an unbraced expression argument.
    fn push_w100(
        &mut self,
        cmd_name: &str,
        is_expr: bool,
        span: tcl_lexer::Span,
        text: &str,
        has_sub: bool,
    ) {
        let severity = if has_sub {
            super::types::Severity::Error
        } else {
            super::types::Severity::Warning
        };
        let message = if is_expr {
            "Expression is not braced: may cause double substitution and prevents \
             byte-compilation. Use expr {...} instead."
                .to_string()
        } else {
            format!(
                "Expression argument to '{cmd_name}' is not braced: may cause double \
                 substitution. Use braces: {{{text}}}"
            )
        };
        // Brace-wrapping fix; for a quoted `expr "…"` drop the quotes.
        let fix_inner = if is_expr {
            let s = text.trim();
            if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
                &s[1..s.len() - 1]
            } else {
                text
            }
        } else {
            text
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W100".to_string(),
            span,
            message,
            severity,
            fixes: vec![super::types::CodeFix {
                span,
                new_text: format!("{{{fix_inner}}}"),
                description: "Wrap expression in braces".to_string(),
            }],
        });
    }

    /// W311 (GAP-A8): a channel configured with `-encoding binary` *and*
    /// a non-binary `-translation` is contradictory (binary implies no
    /// translation) and can corrupt data / enable encoding-differential
    /// attacks.  Handles `fconfigure` and `chan configure`.  Mirrors
    /// `check_encoding_mismatch`.
    pub(super) fn emit_w311_encoding_mismatch(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        let opt_start = if cmd_name == "fconfigure" {
            1
        } else if cmd_name == "chan" && args.first().map(String::as_str) == Some("configure") {
            2
        } else {
            return;
        };
        if args.len() <= opt_start {
            return;
        }
        let mut binary_tok = None;
        let mut translation_tok = None;
        let mut i = opt_start;
        while i + 1 < args.len() {
            let (opt, val) = (&args[i], &args[i + 1]);
            if opt == "-encoding" && val == "binary" {
                binary_tok = arg_tokens.get(i + 1);
            } else if opt == "-translation" && val != "binary" {
                translation_tok = arg_tokens.get(i + 1);
            }
            i += 2;
        }
        if binary_tok.is_some() && translation_tok.is_some() {
            let target = translation_tok
                .or(binary_tok)
                .or_else(|| arg_tokens.first());
            if let Some(tok) = target {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W311".to_string(),
                    span: tok.span,
                    message: "Channel configured with -encoding binary and a non-binary \
                              -translation. Binary encoding implies no translation; the \
                              conflicting -translation may silently corrupt data or enable \
                              encoding-differential attacks."
                        .to_string(),
                    severity: super::types::Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// W200 (GAP-A8): a `u` / `s` modifier on a `binary format` / `binary
    /// scan` integer specifier requires Tcl 8.5+; under 8.4-based
    /// dialects (incl. F5 iRules / iApps) it is unavailable.  Mirrors
    /// `check_binary_format_modifiers`.
    pub(super) fn emit_w200_binary_format_modifiers(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if cmd_name != "binary" || args.is_empty() {
            return;
        }
        let fmt_idx = match args[0].as_str() {
            "format" if args.len() >= 2 => 1,
            "scan" if args.len() >= 3 => 2,
            _ => return,
        };
        if !matches!(self.dialect.as_str(), "tcl8.4" | "f5-irules" | "f5-iapps") {
            return;
        }
        let Some(fmt_tok) = arg_tokens.get(fmt_idx) else {
            return;
        };
        let fmt = args[fmt_idx].as_bytes();
        let mut i = 0;
        while i < fmt.len() {
            if fmt[i].is_ascii_whitespace() {
                i += 1;
                continue;
            }
            while i < fmt.len() && fmt[i].is_ascii_digit() {
                i += 1;
            }
            if i >= fmt.len() {
                break;
            }
            let spec = fmt[i];
            i += 1;
            if BINARY_INT_SPECIFIERS.contains(&spec)
                && i < fmt.len()
                && (fmt[i] == b'u' || fmt[i] == b's')
            {
                let modifier = fmt[i] as char;
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W200".to_string(),
                    span: fmt_tok.span,
                    message: format!(
                        "signed/unsigned modifier '{modifier}' on binary format specifier \
                         requires Tcl 8.5+"
                    ),
                    severity: super::types::Severity::Warning,
                    fixes: Vec::new(),
                });
                i += 1;
            }
            if i < fmt.len() && fmt[i] == b'*' {
                i += 1;
            }
        }
    }

    /// W121 (GAP-A8): a dotted-quad literal that looks like a subnet mask
    /// but has non-contiguous bits (`255.255.255.1`, `255.0.255.0`) is
    /// almost certainly a mistake.  Mirrors `check_invalid_subnet_mask`.
    pub(super) fn emit_w121_invalid_subnet_mask(
        &mut self,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for (tok, text) in arg_tokens.iter().zip(args.iter()) {
            if seen.contains(&tok.span.start()) {
                continue;
            }
            for quad in find_dotted_quads(text, 3) {
                let octets: Vec<u32> = quad
                    .octets
                    .iter()
                    .map(|o| o.parse::<u32>().unwrap_or(999))
                    .collect();
                if octets.iter().any(|&o| o > 255) {
                    continue;
                }
                let (a, b, c, d) = (octets[0], octets[1], octets[2], octets[3]);
                if !looks_like_subnet_mask(a, b, c, d) {
                    continue;
                }
                if is_valid_subnet_mask(a, b, c, d) {
                    continue;
                }
                seen.insert(tok.span.start());
                let quad = format!("{a}.{b}.{c}.{d}");
                let mut message = format!(
                    "'{quad}' looks like a subnet mask but has non-contiguous bits. A valid \
                     mask must be contiguous leading 1-bits followed by 0-bits."
                );
                if let Some(s) = nearest_valid_mask(a, b, c, d) {
                    if s != quad {
                        use std::fmt::Write as _;
                        let _ = write!(message, " Did you mean '{s}'?");
                    }
                }
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W121".to_string(),
                    span: tok.span,
                    message,
                    severity: super::types::Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// W108 (GAP-A3): a non-ASCII character in an argument token —
    /// either a Unicode confusable (visually resembling ASCII) or a
    /// known copy-paste artifact (smart quote, NBSP, em-dash, …).
    /// `args` / `arg_tokens` exclude the command name (matching Python,
    /// which does not scan the command word).  Mirrors `check_non_ascii`
    /// in the default **confusables** mode (→ **strict** for F5
    /// iRules / iApps); the `common` mode — which needs Unicode general-
    /// category data Rust std lacks — is a follow-up.  One diagnostic
    /// per offending character, with an ASCII-replacement fix when one
    /// is known.
    pub(super) fn emit_w108_non_ascii(&mut self, arg_tokens: &[tcl_lexer::Token]) {
        use super::confusables_table::{auto_fix_for, confusable_to_ascii};
        use super::state::NonAsciiMode;

        // Resolve the effective mode: an explicit `tclLsp.style.nonAscii`
        // setting, or the per-dialect default (strict for ASCII-only F5
        // dialects, confusables otherwise).  Mirrors Python's
        // `_non_ascii_mode` + the iRules override in `check_non_ascii`.
        let mode = match self.non_ascii_mode {
            NonAsciiMode::Default => {
                if matches!(self.dialect.as_str(), "f5-irules" | "f5-iapps") {
                    NonAsciiMode::Strict
                } else {
                    NonAsciiMode::Confusables
                }
            }
            explicit => explicit,
        };
        if mode == NonAsciiMode::Off {
            return;
        }

        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for tok in arg_tokens {
            if seen.contains(&tok.span.start()) {
                continue;
            }
            let Some(slice) = source_slice(&self.source, tok.span) else {
                continue;
            };
            // Skip a multi-line braced body — its inner commands are
            // checked when the body is recursed into.
            if tok.kind == tcl_lexer::TokenType::Esc && slice.contains('\n') {
                continue;
            }
            let mut flagged_here = false;
            for (rel, ch) in slice.char_indices() {
                if is_standard_ascii(ch) {
                    continue;
                }
                let fix = auto_fix_for(ch).or_else(|| confusable_to_ascii(ch));
                let is_confusable = fix.is_some();
                // Mode-dependent filtering (strict flags everything):
                //  * confusables — only confusables / auto-fix artifacts;
                //  * common — those plus any non-benign character
                //    (control / format / separator / unassigned / …).
                match mode {
                    NonAsciiMode::Confusables if !is_confusable => continue,
                    NonAsciiMode::Common if !is_confusable && is_benign_unicode(ch) => continue,
                    _ => {}
                }
                flagged_here = true;
                let start = tok.span.start() + u32::try_from(rel).unwrap_or(0);
                let end = start + u32::try_from(ch.len_utf8()).unwrap_or(1);
                let span = tcl_lexer::Span::new(start, end);
                let fixes = fix
                    .map(|repl| {
                        vec![super::types::CodeFix {
                            span,
                            new_text: repl.to_string(),
                            description: "Replace with ASCII equivalent".to_string(),
                        }]
                    })
                    .unwrap_or_default();
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W108".to_string(),
                    span,
                    message: format!(
                        "Non-ASCII character U+{:04X} '{ch}' \u{2014} outside the standard ASCII \
                         printable/whitespace set",
                        ch as u32
                    ),
                    severity: super::types::Severity::Warning,
                    fixes,
                });
            }
            if flagged_here {
                seen.insert(tok.span.start());
            }
        }
    }

    /// W104 (GAP-A8): `append` used with a space-padded value looks
    /// like list construction — fragile if the data contains special
    /// characters.  Fires once (HINT) on the first value argument that
    /// starts or ends with a space.  Mirrors `check_string_list_confusion`.
    pub(super) fn emit_w104_append_list(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if cmd_name != "append" || args.len() < 2 || arg_tokens.len() < 2 {
            return;
        }
        for (i, text) in args.iter().enumerate().skip(1) {
            if text.starts_with(' ') || text.ends_with(' ') {
                let tok = arg_tokens.get(i).unwrap_or(&arg_tokens[0]);
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W104".to_string(),
                    span: tok.span,
                    message: "append with space-separated values looks like list \
                              construction. Use [lappend] instead to safely handle values \
                              containing spaces, braces, or backslashes."
                        .to_string(),
                    severity: super::types::Severity::Hint,
                    fixes: Vec::new(),
                });
                return;
            }
        }
    }

    /// W106 (GAP-A8): an unbraced `switch` body undergoes an extra round
    /// of substitution (especially dangerous under `-regexp`).  Handles
    /// the single trailing-body form and the alternating pattern/body
    /// form; skips braced bodies and the `-` fall-through.  ERROR when a
    /// substitution is present or `-regexp` is set, else WARNING.
    /// Mirrors `check_unbraced_switch_body`.
    pub(super) fn emit_w106_unbraced_switch_body(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if cmd_name != "switch" || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        // Option flags, then the subject string.
        let mut i = 0;
        let mut has_regexp = false;
        while i < args.len() && args[i].starts_with('-') {
            if args[i] == "-regexp" {
                has_regexp = true;
            }
            if args[i] == "--" {
                i += 1;
                break;
            }
            i += 1;
        }
        if i >= args.len() {
            return;
        }
        i += 1; // skip subject
        if i >= args.len() {
            return;
        }

        // Single trailing arg: the braced-list form (W105 / bracing
        // handles a braced block); only flag an *unbraced* single block.
        if i == args.len() - 1 {
            if let Some(tok) = arg_tokens.get(i) {
                if !is_braced_word(tok) {
                    let dangerous = has_substitution(&args[i], tok);
                    self.push_w106(tok.span, dangerous, has_regexp, true);
                }
            }
            return;
        }

        // Alternating pattern/body pairs.
        while i + 1 < args.len() {
            let body_idx = i + 1;
            if let (Some(tok), Some(text)) = (arg_tokens.get(body_idx), args.get(body_idx)) {
                if !is_braced_word(tok) && text != "-" {
                    let dangerous = has_substitution(text, tok) || has_regexp;
                    self.push_w106(tok.span, dangerous, has_regexp, false);
                }
            }
            i += 2;
        }
    }

    /// Push one W106 diagnostic with the message variant selected by
    /// `has_regexp` / substitution danger.
    fn push_w106(
        &mut self,
        span: tcl_lexer::Span,
        dangerous: bool,
        has_regexp: bool,
        single_block: bool,
    ) {
        let message = if has_regexp {
            "switch -regexp body is not braced \u{2014} patterns and actions undergo extra \
             substitution, risking code injection. Use braces: { \u{2026} }"
                .to_string()
        } else if dangerous && single_block {
            "switch body is not braced \u{2014} contains substitutions that risk code \
             injection. Use braces: switch \u{2026} { pattern { body } \u{2026} }"
                .to_string()
        } else if single_block {
            "switch body is not braced \u{2014} Use braces: switch \u{2026} { pattern { body } \
             \u{2026} }"
                .to_string()
        } else if dangerous {
            "switch body is not braced and contains substitutions \u{2014} risk of code \
             injection. Use braces: { \u{2026} }"
                .to_string()
        } else {
            "switch body should be braced to prevent accidental substitution. Use braces: \
             { \u{2026} }"
                .to_string()
        };
        let severity = if dangerous {
            super::types::Severity::Error
        } else {
            super::types::Severity::Warning
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W106".to_string(),
            span,
            message,
            severity,
            fixes: Vec::new(),
        });
    }

    /// W212 (GAP-A8): a command argument that must be a variable
    /// *name* (`set $x 1`, `incr $x`, `info exists $x`, `upvar 1 a $b`)
    /// instead uses a `$`-substitution.  `args` / `arg_tokens` exclude
    /// the command name.  Fires when the resolved name-position argument
    /// is a `Var` token.  Mirrors `check_name_vs_value`.
    pub(super) fn emit_w212_name_vs_value(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        for idx in name_arg_indices(cmd_name, args) {
            let (Some(tok), Some(text)) = (arg_tokens.get(idx), args.get(idx)) else {
                continue;
            };
            if tok.kind != tcl_lexer::TokenType::Var {
                continue;
            }
            // `set ${token}(status) …` in a variable-name position is the
            // braced indirect-array-element idiom (`token` holds the array
            // name), not a `set $token` dynamic-name foot-gun.  Both the
            // W212 `did you mean token(status)` and the W216
            // `did you mean $token(status)` suggestions are wrong there, so
            // neither fires.  Mirrors `check_name_vs_value`'s
            // `is_braced_indirect_array_ref` carve-out.
            if tcl_syntax::naming::is_braced_indirect_array_ref(text) {
                continue;
            }
            let bare = text
                .trim_start_matches('$')
                .trim_start_matches('{')
                .trim_end_matches('}');
            let display_cmd = if cmd_name == "info" && !args.is_empty() {
                format!("info {}", args[0])
            } else {
                cmd_name.to_string()
            };
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W212".to_string(),
                span: tok.span,
                message: format!(
                    "'{display_cmd}' expects a variable name, got substitution (${bare}). \
                     Did you mean '{bare}'?"
                ),
                severity: super::types::Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W216** — broken brace-form variants of array element access.
    ///
    /// Two related shapes both look like array-element access but parse
    /// differently than the user intends:
    ///
    /// 1. `${arr}(foo)` — the lexer ends the variable substitution at the
    ///    `}`, so this is scalar `${arr}` followed by *literal* `(foo)`; no
    ///    array access happens.
    /// 2. `${arr($foo)}` — the brace form applies no further substitution to
    ///    its content, so `$foo` inside the braces is the literal four-char
    ///    string, not the value of `foo`.
    ///
    /// In a *variable-name* position (`set` / `incr` / `append` / `lappend` /
    /// `unset` / `info exists` / `vwait`) Pattern (1) is the legitimate
    /// indirect-array-element idiom (`token` holds the array name) and must
    /// not fire — see [`tcl_syntax::naming::is_braced_indirect_array_ref`].
    /// Mirrors `_emit_w216_for_command` in
    /// `analyser/_analyser/_diag_brace_then_paren.py`.
    pub(super) fn emit_w216_brace_then_paren(&mut self, cmd: &crate::segmenter::SegmentedCommand) {
        if cmd.texts.is_empty() {
            return;
        }
        let sm = SourceMap::new(&self.source);
        let source = self.source.as_bytes();
        // Word-start offsets the command reads as a variable name; a
        // `${name}(idx)` Pattern-(1) match starting there is the indirect
        // idiom and must not fire W216.
        let cmd_name = cmd.texts[0].as_str();
        let args = &cmd.texts[1..];
        let mut varname_word_starts: HashSet<u32> = HashSet::new();
        for ai in w216_varname_word_indices(cmd_name, args) {
            let wi = ai + 1;
            if let Some(tok) = cmd.argv.get(wi) {
                varname_word_starts.insert(tok.span.start());
            }
        }
        for &t1 in &cmd.all_tokens {
            // Token spans are absolute into the full document; skip any token
            // whose span exceeds the current source (synthetic unit-test
            // tokens, or a span past a truncated buffer) before slicing.
            if t1.span.end() as usize > source.len() {
                continue;
            }
            if !is_brace_form_var(&sm, t1) {
                continue;
            }
            let text = sm.token_text(t1).to_string();

            // Pattern (2) — `${arr($foo)}`: the VAR token's own text contains
            // `(...)` with `$`/`[` inside.
            if text.contains('(') && text.ends_with(')') {
                if let Some(paren_idx) = text.find('(') {
                    let name = &text[..paren_idx];
                    let inner = &text[paren_idx + 1..text.len() - 1];
                    if !name.is_empty() && index_has_substitution(inner) {
                        let corrected = build_w216_replacement(name, inner);
                        // Token span end is exclusive — it sits on the `}`.
                        let span = tcl_lexer::Span::new(t1.span.start(), t1.span.end() + 1);
                        let message = format!(
                            "`${{{name}({inner})}}` does not substitute `{inner}` \
(the brace form is documented to apply no further substitution to its \
content); use `{corrected}` to access the array element with index substitution"
                        );
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W216".to_string(),
                            span,
                            message,
                            severity: Severity::Warning,
                            fixes: vec![super::types::CodeFix {
                                span,
                                new_text: corrected.clone(),
                                description: format!("Replace with `{corrected}`"),
                            }],
                        });
                    }
                }
                continue;
            }

            // Pattern (1) — `${arr}(foo)`: the VAR token ends at the last
            // char before `}`; a literal `}` follows (the exclusive span end),
            // then `(` opens a paren group.
            let close_brace = t1.span.end() as usize;
            if close_brace >= source.len() || source[close_brace] != b'}' {
                continue;
            }
            let paren_start = close_brace + 1;
            if paren_start >= source.len() || source[paren_start] != b'(' {
                continue;
            }
            let Some(paren_end) = find_matching_close_paren(source, paren_start) else {
                continue;
            };
            // Variable-name position → indirect-array idiom, suppress.
            if varname_word_starts.contains(&t1.span.start()) {
                continue;
            }
            let inner = &self.source[paren_start + 1..paren_end];
            let corrected = build_w216_replacement(&text, inner);
            let span = tcl_lexer::Span::new(
                t1.span.start(),
                u32::try_from(paren_end + 1).unwrap_or(t1.span.end()),
            );
            let message = format!(
                "`${{{text}}}({inner})` is parsed as scalar `${{{text}}}` followed by \
literal text `({inner})`; did you mean `{corrected}` for array element access?"
            );
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W216".to_string(),
                span,
                message,
                severity: Severity::Warning,
                fixes: vec![super::types::CodeFix {
                    span,
                    new_text: corrected.clone(),
                    description: format!("Replace with `{corrected}`"),
                }],
            });
        }
    }

    /// W114 (GAP-A8): a nested `[expr …]` inside an argument that is
    /// *already* an expression context (`expr` / `if` / `while` / `for`
    /// conditions) is redundant.  `diag_span` is the source span of the
    /// expression argument; we scan its source slice for the first
    /// `[`-`expr`-whitespace sequence (the `_NESTED_EXPR_RE` pattern)
    /// and anchor the warning at the nested `[expr … ]`.  One warning
    /// per argument, mirroring Python's `re.search` (first match only).
    pub(super) fn emit_w114_redundant_nested_expr(
        &mut self,
        _text: &str,
        diag_span: tcl_lexer::Span,
    ) {
        let start = diag_span.start() as usize;
        let end = diag_span.end() as usize;
        if start >= end || end > self.source.len() {
            return;
        }
        let slice = &self.source[start..end];
        let Some((open, close)) = first_nested_expr(slice) else {
            return;
        };
        let nested_span = tcl_lexer::Span::new(
            u32::try_from(start + open).unwrap_or(diag_span.start()),
            u32::try_from(start + close + 1).unwrap_or(diag_span.end()),
        );
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W114".to_string(),
            span: nested_span,
            message: "Redundant nested [expr] \u{2014} already in expression context".to_string(),
            severity: super::types::Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **W110.** Emit "use `eq`/`ne` instead of `==`/`!=` for
    /// string comparison" hints on the EXPR-role argument of
    /// commands like `if`, `while`, `for`, `expr`.
    ///
    /// Mirrors ``check_string_compare_in_expr`` in
    /// ``core/analysis/checks/_style.py:740-834``.  Fires when at
    /// least one operand of a `==` / `!=` comparison is a string
    /// literal (`ExprString`, e.g. `"foo"`, `"1"`, `"true"`);
    /// comparisons between variables (`$x == $y`) are left alone.
    ///
    /// `expr_text` is the post-substitution body of the EXPR-role
    /// argument (already brace-stripped) — the caller is
    /// responsible for joining multi-arg `expr` invocations with
    /// spaces before calling.  `diag_span` is the source span the
    /// diagnostic anchors to (the source range of the argument
    /// token, or the full token range for `expr`).
    pub(super) fn emit_w110_string_eq_ne(&mut self, expr_text: &str, diag_span: tcl_lexer::Span) {
        // Quick bail-out: no equality operator at all.
        if !expr_text.contains("==") && !expr_text.contains("!=") {
            return;
        }
        let parsed = crate::parse_expr(expr_text.trim(), Some(self.dialect.as_str()));
        // ``ExprNode::Raw`` means the expression was unparseable —
        // mirror Python's ``isinstance(parsed, ExprRaw): continue``.
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }
        let matched_ops = find_string_eq_ne_ops(&parsed);
        if matched_ops.is_empty() {
            return;
        }
        let first_op = matched_ops[0];
        let (op_text, replacement) = match first_op {
            BinOp::Eq => ("==", "eq"),
            BinOp::Ne => ("!=", "ne"),
            _ => unreachable!("find_string_eq_ne_ops only returns Eq/Ne"),
        };
        // Only offer the regex-based code fix when every ``==``/
        // ``!=`` in the expression has a string-literal operand;
        // otherwise the blanket rewrite would incorrectly change
        // non-string comparisons too.
        let total = count_eq_ne_ops(&parsed);
        let mut fixes = Vec::new();
        if matched_ops.len() >= total {
            let rewritten = rewrite_string_compare_ops(expr_text);
            if rewritten != expr_text {
                fixes.push(super::types::CodeFix {
                    span: diag_span,
                    new_text: rewritten,
                    description: format!("Use '{replacement}' for string comparison"),
                });
            }
        }
        let message = format!(
            "Use '{replacement}' instead of '{op_text}' for string \
comparison in expressions to avoid ambiguous \
numeric/string coercion."
        );
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W110".to_string(),
            span: diag_span,
            message,
            severity: Severity::Hint,
            fixes,
        });
    }

    /// **W302.** Emit "catch without result variable" hint when a
    /// `catch BODY` invocation omits the optional `RESULTVAR`
    /// argument, silently swallowing any error the body raises.
    ///
    /// Mirrors the `IRCatch` arm of ``_check_statement`` in
    /// ``core/compiler/compiler_checks.py:491-504``.  Python only
    /// emits W302 for `IRCatch` (not `IRBarrier`) — the lowerer
    /// falls back to `IRBarrier` when the body argument is multi-token
    /// (e.g. ``catch $body``), so this Rust emit gates on
    /// ``arg_single[0]`` to mirror that suppression.  The diagnostic
    /// anchors at just the ``catch`` command token — the narrowest
    /// span that identifies the issue — matching the #464 narrowing
    /// (`compiler_checks.py` now uses ``range_from_token(argv[0])``
    /// rather than the whole-statement ``stmt.range``).
    pub(super) fn emit_w302_catch_no_result_var(
        &mut self,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        // Only fires when a result variable is absent.  Empty args
        // is "malformed catch" in Python's lowerer (IRBarrier path,
        // no W302).  ≥2 args means a result variable is present.
        if args.len() != 1 {
            return;
        }
        // Mirror Python's "catch with dynamic body" IRBarrier
        // suppression: a multi-token body word can't be statically
        // resolved to a script, so the lowerer drops it before
        // ``_check_statement`` ever sees it.
        if arg_single.first().copied() != Some(true) {
            return;
        }
        if arg_tokens.is_empty() {
            return;
        }
        // Suppress the hint on the documented "fire-and-forget" idiom:
        // ``catch {close $h}`` / ``catch {after cancel $h}`` etc.  These
        // commands error when the target is already gone, and a bare
        // ``catch {<cmd>}`` is the canonical Tcl idiom for "do this if
        // possible, ignore if not".
        if let Some(body) = args.first() {
            if catch_body_is_fire_and_forget(body) {
                return;
            }
        }
        let span = cmd_tok.span;
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W302".to_string(),
            span,
            message: "catch without a result variable silently swallows errors. \
Consider capturing the result: catch {\u{2026}} result"
                .to_string(),
            severity: Severity::Hint,
            fixes: Vec::new(),
        });
    }

    /// **W001.** Emit "Unknown subcommand" warning for commands
    /// whose registry signature is a [`SubcommandSig`](super::dispatch::SubcommandSig)
    /// when the first argument doesn't resolve to a known subcommand.
    ///
    /// Mirrors the `SubcommandSig` branch of `_check_arity` in
    /// ``core/compiler/compiler_checks.py:580-643``.  Skips:
    ///
    /// - commands the registry doesn't know (no signature),
    /// - simple-command signatures (no subcommand dispatch),
    /// - signatures with `allow_unknown == true` (generated
    ///   dialect packs),
    /// - first-arg values containing ``$`` / ``[`` (dynamic
    ///   substitution — runtime-resolved),
    /// - empty arg lists (handled by the E001 emitter, deferred).
    ///
    /// When emission is warranted, includes a "did you mean…?"
    /// suffix using [`crate::text::suggest_similar`] over the
    /// known subcommand set (max 1 suggestion within edit
    /// distance 3).
    ///
    /// **Known minor parity gap:** Python additionally skips when
    /// the subcommand position is ``{*}``-expanded
    /// (``arg_expand[0]``).  The Rust ``process_command`` does not
    /// currently thread the expansion flag through; the literal-
    /// text ``$`` / ``[`` gate covers the dynamic-substitution
    /// case, and ``{*}LITERAL`` for an unknown subcommand is rare
    /// enough in practice that the divergence is acceptable until
    /// expand-flag plumbing lands as its own chunk.
    pub(super) fn emit_w001_unknown_subcommand(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use super::dispatch::{signature_for_command, CommandSignature};
        use tcl_registry::prelude::DialectSet;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let Some(first_arg) = args.first() else {
            // Empty arg list — Python's E001 path; not in scope here.
            return;
        };
        // Dynamic-value subcommand position — can't resolve statically.
        if first_arg.contains('$') || first_arg.contains('[') {
            return;
        }
        let dialect = DialectSet::parse(&self.dialect).unwrap_or(DialectSet::ALL_TCL);
        let Some(CommandSignature::WithSubcommands(sig)) =
            signature_for_command(registry, cmd_name, dialect)
        else {
            return;
        };
        if sig.allow_unknown {
            return;
        }
        if sig.subcommands.contains_key(first_arg) {
            return;
        }
        let mut message = format!("Unknown subcommand '{first_arg}' for '{cmd_name}'");
        let candidates: Vec<&str> = sig.subcommands.keys().map(String::as_str).collect();
        let suggestions = crate::text::suggest_similar(first_arg, candidates.iter().copied(), 1, 3);
        let mut fixes: Vec<super::types::CodeFix> = Vec::new();
        if let Some(best) = suggestions.first() {
            use std::fmt::Write as _;
            let _ = write!(message, "; did you mean '{best}'?");
            if let Some(sub_tok) = arg_tokens.first() {
                // Target the *content* range of the subcommand
                // token rather than its full span.  Wrapper tokens
                // (`Str` braced, `Esc` quoted) carry the opening
                // delimiter via ``content_offset`` and intentionally
                // exclude the closing delimiter from ``span.end``;
                // replacing the full span would leave a stray
                // ``}`` / ``"`` behind (e.g. ``string {lenght}`` →
                // ``string length}``).  Using the content range
                // ([span.start + content_offset, span.end)) gives
                // ``{length}`` / ``"length"`` for the wrapped forms
                // and remains identical to the full span for bare
                // ``Esc`` words (``content_offset == 0``).
                let content_start = sub_tok.span.start() + u32::from(sub_tok.content_offset);
                let fix_span = tcl_lexer::Span::new(content_start, sub_tok.span.end());
                fixes.push(super::types::CodeFix {
                    span: fix_span,
                    new_text: (*best).to_string(),
                    description: format!("Replace with '{best}'"),
                });
            }
        }
        // Anchor at the command-head + subcommand-name range so
        // the squiggle covers ``cmd subname`` rather than the
        // entire invocation.  Mirrors Python's ``cmd_token_range``
        // which combines the command token with the subcommand
        // arg token.
        let span = match arg_tokens.first() {
            Some(sub_tok) => tcl_lexer::Span::new(cmd_tok.span.start(), sub_tok.span.end()),
            None => cmd_tok.span,
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W001".to_string(),
            span,
            message,
            severity: Severity::Warning,
            fixes,
        });
    }

    /// **E002 / E003.** Argument-count check for simple (non-
    /// subcommand) commands.  Mirrors `_check_simple_arity` in
    /// `core/compiler/compiler_checks.py`: skip leading declared
    /// option flags, then compare the positional-argument count
    /// against the registry signature's arity bounds.
    ///
    /// Option skipping uses the dialect-filtered
    /// [`CommandSig::leading_options`](super::dispatch::CommandSig::leading_options)
    /// set, so switches introduced in a later Tcl release (e.g.
    /// `regsub -command`, 9.0+) are only skipped under a dialect that
    /// declares them.  This is the SYNC-MAY21-3 fix: it prevents both
    /// the #455 false positive (declared switches counted as
    /// positional → spurious E003) and the #460 dialect leak (9.0-only
    /// switches skipped under 8.x).
    ///
    /// `arg_expand[i]` marks an argument preceded by the Tcl 8.5+
    /// `{*}` expansion prefix.  A `{*}`-expanded word contributes an
    /// unknown number of runtime arguments, so option skipping stops
    /// at the first such word and the positional upper bound becomes
    /// unbounded — only the count of *non-expanded* positional words
    /// can still trip E003, exactly as Python does.
    ///
    /// **Parity gaps (documented, intentional):**
    /// - Like Python's name-only `leading_options` skip, the *value*
    ///   of a value-taking leading option is **not** skipped (Python's
    ///   value-aware `skip_options` is used only for arg-role
    ///   resolution, not arity).  See the validation note in
    ///   `docs/rust-rewrite.md` (SYNC-MAY21-3).
    /// - Statically-resolvable literal `{*}` expansions (`{*}{a b c}`)
    ///   are not refined to their element count; the conservative form
    ///   here can miss a genuine over-arity but never invents a false
    ///   positive.
    ///
    /// Subcommand-dispatch commands are handled by
    /// [`Self::emit_w001_unknown_subcommand`] and skipped here;
    /// per-subcommand arity is a later follow-up.
    pub(super) fn emit_arity_diagnostics(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_expand_in: &[bool],
        cmd_tok: tcl_lexer::Token,
        scope_path: &[usize],
    ) {
        use super::dispatch::{signature_for_command, CommandSignature};
        use tcl_registry::prelude::DialectSet;

        // `arg_expand_in` is parallel to the full argv (command name at
        // index 0); drop that slot so it lines up with `args`.
        let arg_expand: &[bool] = arg_expand_in.get(1..).unwrap_or(&[]);

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let dialect = DialectSet::parse(&self.dialect).unwrap_or(DialectSet::ALL_TCL);
        match signature_for_command(registry, cmd_name, dialect) {
            Some(CommandSignature::Simple(sig)) => {
                self.check_simple_arity(
                    cmd_name, cmd_name, &sig, args, arg_tokens, arg_expand, cmd_tok, scope_path,
                );
            }
            Some(CommandSignature::WithSubcommands(sig)) => {
                // Per-subcommand arity, mirroring the Python
                // `_check_arity` → `_check_simple_arity` path on
                // `args[1:]` (compiler_checks.py:783-797).  The W001
                // unknown-subcommand path is handled separately by
                // [`Self::emit_w001_unknown_subcommand`].
                let Some(sub_name) = args.first() else {
                    // Missing subcommand — Python's E001 path; not here.
                    return;
                };
                // A `{*}`-expanded subcommand word resolves to an unknown
                // name at runtime; skip resolution and arity entirely.
                if arg_expand.first().copied().unwrap_or(false) {
                    return;
                }
                // Dynamic subcommand value — can't resolve statically.
                if sub_name.contains('$') || sub_name.contains('[') {
                    return;
                }
                let Some(sub_sig) = sig.subcommands.get(sub_name) else {
                    // Unknown subcommand — W001's job, not arity.
                    return;
                };
                let display_name = format!("{cmd_name} {sub_name}");
                self.check_simple_arity(
                    cmd_name,
                    &display_name,
                    sub_sig,
                    &args[1..],
                    arg_tokens.get(1..).unwrap_or(&[]),
                    arg_expand.get(1..).unwrap_or(&[]),
                    cmd_tok,
                    scope_path,
                );
            }
            None => {}
        }
    }

    /// Compare a positional-argument count against a single
    /// [`CommandSig`]'s arity bounds and queue an E002 / E003
    /// candidate.  Shared by the simple-command and per-subcommand
    /// arity paths in [`Self::emit_arity_diagnostics`]; mirrors
    /// `_check_simple_arity` in `core/compiler/compiler_checks.py`.
    ///
    /// `resolution_name` is the base command name used by the
    /// post-walk [`Self::flush_arity_diagnostics`] to honour a
    /// shadowing user proc / class / alias (e.g. `file` for the
    /// `file link` subcommand check), while `display_name` is the
    /// human-facing name shown in the message (`file link`).
    ///
    /// `args` / `arg_tokens` / `arg_expand` are the slices *after*
    /// whatever prefix the caller has already consumed (the command
    /// name for the simple path; the command name and subcommand word
    /// for the subcommand path), so the leading-option scan and
    /// positional count operate on the same coordinate system as
    /// `sig`.
    #[allow(clippy::too_many_arguments)]
    fn check_simple_arity(
        &mut self,
        resolution_name: &str,
        display_name: &str,
        sig: &super::dispatch::CommandSig,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_expand: &[bool],
        cmd_tok: tcl_lexer::Token,
        scope_path: &[usize],
    ) {
        let expanded = |i: usize| arg_expand.get(i).copied().unwrap_or(false);

        // Skip leading declared option flags.  Stop at the first
        // non-option word, the option terminator `--` (consumed), or
        // a `{*}`-expanded word (whose value can't be classified).
        let mut positional_start = 0usize;
        if !sig.leading_options.is_empty() {
            for (i, arg) in args.iter().enumerate() {
                if expanded(i) {
                    break;
                }
                if sig.leading_options.contains(arg) {
                    positional_start = i + 1;
                    if arg == "--" {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        let positional_any_expand = (positional_start..args.len()).any(expanded);
        // `nargs_min` is the *lower bound* on the positional-argument
        // count: the non-expanded words, since each `{*}` word
        // contributes 0..N more at runtime.  E003 ("too many") fires
        // when even this lower bound exceeds `max`.  E002 ("too few")
        // needs an *upper bound* on the count, which becomes unbounded
        // once any `{*}` expansion is present — so E002 only fires when
        // there is no expansion and the count is therefore exact.
        let nargs_min = if positional_any_expand {
            (positional_start..args.len())
                .filter(|&i| !expanded(i))
                .count()
        } else {
            args.len() - positional_start
        };
        let min = usize::from(sig.arity.min);
        let max = usize::from(sig.arity.max);

        let full_span = match arg_tokens.last() {
            Some(last) => tcl_lexer::Span::new(cmd_tok.span.start(), last.span.end()),
            None => cmd_tok.span,
        };

        // Capture the call-site command-resolution namespace so the
        // post-walk flush can resolve this command the Tcl way (current
        // namespace → global) and only suppress the arity check when
        // the call actually resolves to a user definition — not to any
        // same-tail-named proc elsewhere in the file. Uses the proc's
        // *defining* namespace (so `close` inside a body of
        // `proc ::ns::x` resolves through `::ns`), not just lexical
        // `namespace eval` nesting.
        let ns = self.command_resolution_namespace(scope_path);

        // Top-level calls (module body, `namespace eval` bodies, and
        // conditionals) execute in source order during load, so a
        // shadowing proc only silences the builtin arity check when its
        // definition lexically precedes the call.  Calls inside a proc
        // body resolve after the whole script has loaded, so order is
        // not enforced there.  Mirrors Python #475's `enforce_order`.
        let enforce_order = !self.scope_path_in_proc_body(scope_path);

        // Collect as a *candidate*; the post-walk
        // [`Self::flush_arity_diagnostics`] drops it if the call
        // resolves to a user proc / class / alias / ensemble / stub.
        // A class / alias / ensemble / stub match suppresses regardless
        // of definition order; a *proc* match additionally honours
        // `enforce_order` (in-order/reachability gate, #475).
        if !positional_any_expand && (args.len() - positional_start) < min {
            let got = args.len() - positional_start;
            self.pending_arity.push((
                resolution_name.to_string(),
                ns,
                enforce_order,
                super::types::Diagnostic {
                    code: "E002".to_string(),
                    span: full_span,
                    message: format!(
                        "Too few arguments for '{display_name}': expected at least {min}, got {got}"
                    ),
                    severity: Severity::Error,
                    fixes: Vec::new(),
                },
            ));
        } else if !sig.arity.is_unlimited() && nargs_min > max {
            self.pending_arity.push((
                resolution_name.to_string(),
                ns,
                enforce_order,
                super::types::Diagnostic {
                    code: "E003".to_string(),
                    span: full_span,
                    message: format!(
                        "Too many arguments for '{display_name}': expected at most {max}, got {nargs_min}"
                    ),
                    severity: Severity::Error,
                    fixes: Vec::new(),
                },
            ));
        }
    }

    /// Post-walk flush of the [`Self::pending_arity`] candidates
    /// collected by [`Self::emit_arity_diagnostics`].
    ///
    /// Runs after the command walk completes, when `all_procs`,
    /// `all_classes`, `command_aliases`, `ensemble_namespaces` and the
    /// inline stub set are fully populated.  A candidate is dropped
    /// only when the call **resolves to** a user definition rather than
    /// the builtin whose registry arity produced it — resolution
    /// follows Tcl's rule for unqualified commands (the call-site
    /// namespace, then global `::`), using the namespace captured at
    /// emit time.  So `proc ::ns::close {...}` suppresses a `close`
    /// call inside `::ns` (and a qualified `::ns::close ...`), but a
    /// `close` call in another namespace still resolves to the builtin
    /// and is checked.  Document-global declarations — inline
    /// `# tcl-lsp: stub`s — suppress by bare name regardless of
    /// namespace.
    ///
    /// Suppression by a shadowing **proc** also honours definition
    /// reachability (#475 / SYNC-MAY31-9): a top-level call (one whose
    /// `enforce_order` flag is set — module body, `namespace eval`
    /// body, or a conditional) is silenced only when the proc's
    /// definition lexically precedes it, since top-level commands run
    /// in source order during load (so a `close x y z` *before* a later
    /// `proc close` still reaches the builtin).  Proc-body calls run
    /// after load and are not order-gated.  Classes / aliases /
    /// ensembles / stubs always exist at run time and are never
    /// order-gated.  (Excluding *conditionally* defined procs would
    /// need the CFG dominator model and is deferred.)
    ///
    /// Idempotent: drains `pending_arity`, so a second call is a
    /// no-op.
    pub fn flush_arity_diagnostics(&mut self) {
        if self.pending_arity.is_empty() {
            return;
        }
        // Fully-qualified non-proc user-command names the calls may
        // resolve to (classes / aliases keyed by qualified name;
        // ensemble namespaces *are* the command name).  These always
        // exist by the time the script runs, so they suppress the
        // builtin arity check regardless of definition order.
        let mut non_proc_qnames: std::collections::HashSet<&str> = std::collections::HashSet::new();
        non_proc_qnames.extend(self.result.all_classes.keys().map(String::as_str));
        non_proc_qnames.extend(self.result.command_aliases.keys().map(String::as_str));
        non_proc_qnames.extend(self.ensemble_namespaces.iter().map(String::as_str));
        // Qualified proc name → definition offset (the proc-name
        // token start).  A shadowing proc only silences a *top-level*
        // call (`enforce_order`) when its definition lexically
        // precedes the call; proc-body calls are not order-gated.
        // Conditional / nested definitions are still treated as
        // shadowing here — distinguishing unconditionally-reachable
        // definitions needs the CFG dominator model and is deferred
        // per the SYNC-MAY31-9 doc note (#475).
        let proc_offsets: std::collections::HashMap<&str, u32> = self
            .result
            .all_procs
            .iter()
            .map(|(qname, def)| (qname.as_str(), def.name_span.start()))
            .collect();
        // Inline stubs are document-global and unqualified.
        let stub_names = super::utils::scan_stub_command_names(&self.source);

        // Qualify an unqualified command against a namespace, mirroring
        // `resolve_command_qualified_name` (`::` root → `::cmd`).
        let join = |ns: &str, cmd: &str| -> String {
            if ns == "::" {
                format!("::{cmd}")
            } else {
                format!("{ns}::{cmd}")
            }
        };

        let pending = std::mem::take(&mut self.pending_arity);
        for (cmd_name, ns, enforce_order, diag) in pending {
            let bare = cmd_name.rsplit("::").next().unwrap_or(&cmd_name);
            // Candidate qualified names this call could resolve to.
            let candidates: Vec<String> = if cmd_name.contains("::") {
                // Already qualified — absolutise like
                // `resolve_command_qualified_name` does.
                let abs = if cmd_name.starts_with("::") {
                    cmd_name.clone()
                } else {
                    format!("::{cmd_name}")
                };
                vec![abs]
            } else {
                // Unqualified — current namespace, then global.
                vec![join(&ns, &cmd_name), format!("::{cmd_name}")]
            };
            // A proc shadows only when reachable at the call: top-level
            // calls require the definition to lexically precede them
            // (`def_off < call_off`); proc-body calls accept any
            // same-named definition.  Classes / aliases / ensembles /
            // stubs are not order-gated.
            let call_off = diag.span.start();
            let resolves_to_user = candidates.iter().any(|c| {
                non_proc_qnames.contains(c.as_str())
                    || proc_offsets
                        .get(c.as_str())
                        .is_some_and(|&def_off| !enforce_order || def_off < call_off)
            }) || stub_names.contains(bare);
            if resolves_to_user {
                continue;
            }
            self.result.diagnostics.push(diag);
        }
    }

    /// **E004.** Emit "Malformed `if` command" / "Extra words after
    /// `else` clause" errors when an `if` invocation's structural
    /// shape doesn't match `if COND BODY ?elseif COND BODY ...?
    /// ?else BODY?`.
    ///
    /// Mirrors the `IRBarrier` arm of `_check_statement` in
    /// ``core/compiler/compiler_checks.py:506-525``, which fires
    /// when Python's `_lower_if`
    /// (``core/compiler/lowering.py:645-753``) returns an
    /// `IRBarrier` with `command == "if"` because the syntactic
    /// shape is invalid.  The reasons it produces:
    ///
    /// - `"malformed if"` — empty arg list, or no clauses after
    ///   the full walk.
    /// - `"malformed if else clause"` — bare `else` with no body
    ///   following.
    /// - `'extra words after "else" clause'` — `else BODY` with
    ///   one or more trailing words.
    /// - `"malformed if clause"` — condition with no body
    ///   (with or without an intervening `then` keyword).
    ///
    /// Detected analyser-side at the `if`-command dispatch site
    /// rather than by walking lowered IR — matches the established
    /// W302 / W001 dispatch-site pattern.  Also closes a latent
    /// parity gap in `lowering/structured.rs::lower_if`, which
    /// currently doesn't produce an "extra words after else"
    /// barrier at all (see `lowering.py:686-693` vs
    /// `structured.rs:147-162`).
    ///
    /// Severity: `Error`.  No code fixes (Python doesn't emit
    /// any).  Span anchors at the command-head token through the
    /// last argument-token end, mirroring Python's `cmd.range`
    /// (full command source range).
    pub(super) fn emit_e004_malformed_if(
        &mut self,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        let full_span = match arg_tokens.last() {
            Some(last) => tcl_lexer::Span::new(cmd_tok.span.start(), last.span.end()),
            None => cmd_tok.span,
        };
        let push_malformed = |this: &mut Self| {
            this.result.diagnostics.push(super::types::Diagnostic {
                code: "E004".to_string(),
                span: full_span,
                message: "Malformed 'if' command".to_string(),
                severity: Severity::Error,
                fixes: Vec::new(),
            });
        };
        let push_extra_words = |this: &mut Self| {
            this.result.diagnostics.push(super::types::Diagnostic {
                code: "E004".to_string(),
                span: full_span,
                message: "Extra words after \"else\" clause in \"if\" command".to_string(),
                severity: Severity::Error,
                fixes: Vec::new(),
            });
        };

        if args.is_empty() {
            push_malformed(self);
            return;
        }

        let mut i = 0;
        let mut clause_count: usize = 0;
        while i < args.len() {
            if args[i] == "elseif" {
                i += 1;
                continue;
            }
            if args[i] == "else" {
                if i + 1 >= args.len() {
                    // Bare ``else`` with no body following.
                    push_malformed(self);
                    return;
                }
                if i + 2 < args.len() {
                    // ``else BODY <extra...>``.
                    push_extra_words(self);
                    return;
                }
                // ``else BODY`` — well-formed terminator.  Note:
                // Python's ``_lower_if`` does *not* append to
                // ``clauses`` here (else-only sets ``else_body``);
                // the post-walk ``if not clauses`` check still
                // fires on ``if else BODY`` to produce a
                // ``"malformed if"`` barrier.  We mirror that by
                // leaving ``clause_count`` unchanged in this arm.
                break;
            }

            // Condition + (optional ``then``) + body shape.
            i += 1;
            if i < args.len() && args[i] == "then" {
                i += 1;
            }
            if i >= args.len() {
                // Condition with no following body.
                push_malformed(self);
                return;
            }
            clause_count += 1;
            i += 1;
        }

        if clause_count == 0 {
            // E.g. ``if elseif`` / ``if else`` after the elseif-skip
            // / else-skip branches consume their keywords without
            // producing a clause.  Mirrors the post-walk
            // ``if not clauses`` check in ``_lower_if``.
            push_malformed(self);
        }
    }

    /// **W304.** Emit "Missing option terminator (`--`)" diagnostics
    /// for option-bearing commands whose first positional argument
    /// could be misinterpreted as an option.
    ///
    /// Mirrors `core/analysis/checks/_style.py::check_missing_option_terminator`
    /// (`_style.py:516-679`).  Resolves the command's option-
    /// terminator profile via
    /// [`tcl_registry::CommandRegistry::resolve_option_terminator`],
    /// scans for the first positional argument that lacks a
    /// preceding `--`, and emits a tristate-severity diagnostic:
    ///
    /// - **OFF** (no diagnostic) — the value is provably non-`-`-
    ///   prefixed (a non-dynamic literal whose representative token
    ///   isn't a `Var`/`Cmd` and whose text doesn't start with `-`).
    /// - **INFO** — dynamic value (`Var` / `Cmd` token) with no
    ///   proof of starting with `-`.  When the value is a single-
    ///   token `Var` whose most recent literal `set` resolves to a
    ///   non-`-`-prefixed value, an additional "origin" diagnostic
    ///   is emitted at the resolution site to explain the INFO
    ///   downgrade.
    /// - **WARNING** — the value is known to start with `-`: either
    ///   a literal whose first character is `-`, or a `Var` whose
    ///   constant-propagated value starts with `-`.
    ///
    /// The diagnostic carries a code-fix that prepends `"-- "` to
    /// the positional-argument span (with a one-byte extension for
    /// `Cmd` tokens whose lexer span excludes the closing `]`).
    ///
    /// **Note on `warn_without_terminator`:** the registry's
    /// `Traits::WARN_WITHOUT_TERMINATOR` flag (set on `regexp` only
    /// today) is plumbed onto [`tcl_registry::ResolvedTerminator`]
    /// for API parity with Python, but Python's analyser-side
    /// `_style.py` doesn't actually consume it.  The OFF gate
    /// fires uniformly for non-dynamic, non-`-`-prefixed values
    /// regardless of the trait — see `_style.py:558-563`.
    pub(super) fn emit_w304_missing_option_terminator(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use tcl_registry::prelude::DialectSet;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }

        let dialect = DialectSet::parse(&self.dialect).unwrap_or(DialectSet::ALL_TCL);
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some(profile) = registry.resolve_option_terminator(cmd_name, &arg_strs, dialect) else {
            return;
        };

        // The braced pattern-list switch form ``switch $x { pat body … }``
        // is NOT a runtime hazard: Tcl unambiguously identifies the
        // trailing brace as the pattern list and never consumes the
        // preceding word as an option.  Detect the two-arg braced form
        // (the last arg is a brace-enclosed `Str` token) and exempt it
        // entirely.  The SPLIT form (`switch $x -nocase {body} …`, 3+
        // args) is still flagged.  Mirrors `_style.py` G12.
        if cmd_name == "switch"
            && arg_tokens.len() == 2
            && arg_tokens.last().map(|t| t.kind) == Some(tcl_lexer::TokenType::Str)
        {
            return;
        }

        let Some(positional_idx) = first_positional_without_terminator(args, &profile) else {
            return;
        };
        if positional_idx >= arg_tokens.len() {
            return;
        }

        let tok = arg_tokens[positional_idx];
        let text = &args[positional_idx];

        let is_dynamic = matches!(
            tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        );
        let looks_like_option = text.starts_with('-');

        // OFF — non-dynamic value that does not start with `-` can
        // never be confused with an option.
        if !is_dynamic && !looks_like_option {
            return;
        }

        let command_label = match profile.subcommand {
            Some(sub) => format!("{cmd_name} {sub}"),
            None => cmd_name.to_string(),
        };

        let (severity, message, origin) =
            self.classify_w304(tok, is_dynamic, looks_like_option, &command_label);

        // Build the code-fix span.  For ``Cmd`` (`[…]`) tokens the
        // lexer span covers ``[inner`` but excludes the closing
        // ``]``; extend by one byte when the byte after ``span.end``
        // is ``]`` so the replacement encompasses the bracket pair.
        let (fix_span, diag_end) = self.compute_w304_fix_span(tok);
        let fix_text = format!(
            "-- {}",
            &self.source[fix_span.start() as usize..fix_span.end() as usize]
        );
        let fixes = vec![super::types::CodeFix {
            span: fix_span,
            new_text: fix_text,
            description: "Insert '--' option terminator".to_string(),
        }];
        let diag_span = tcl_lexer::Span::new(tok.span.start(), diag_end);
        // Suppress unused-warning on the rare path where `cmd_tok`
        // isn't needed (the diagnostic anchors at the positional
        // arg's span, not the command head).
        let _ = cmd_tok;

        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W304".to_string(),
            span: diag_span,
            message,
            severity,
            fixes,
        });
        if let Some(origin_diag) = origin {
            self.result.diagnostics.push(origin_diag);
        }
    }

    /// **W101.** Emit "eval with string concatenation" warning
    /// when an `eval` invocation's argument list could be a
    /// substitution-driven injection vector.
    ///
    /// Mirrors `core/analysis/checks/_security.py:19-73::check_eval_string_concat`.
    /// Suppressed when:
    ///
    /// - every argument's representative token is `Str` (braced,
    ///   `eval {script}` / `eval {a} {b}` — the safe form), or
    /// - the single argument is a `Cmd` substitution whose inner
    ///   command head produces a canonical list (per
    ///   [`tcl_registry::CommandRegistry::is_canonical_list_command`]
    ///   — `eval [list ...]`, `eval [linsert ...]`, etc.).
    ///
    /// Otherwise fires `Severity::Warning` when any argument's
    /// representative token is `Var` / `Cmd` (substitution at the
    /// word level), or any argument is a multi-token word
    /// (substitution within the word — the single-token-word flag
    /// is `false`).  This is a sound approximation of Python's
    /// `all_tokens[1:]`-walk: `process_command` doesn't currently
    /// thread the full token stream, but multi-token-word implies
    /// inner substitution and the per-arg representative kind
    /// covers the single-token VAR / CMD cases.
    ///
    /// Diagnostic anchors at the first argument's range.
    pub(super) fn emit_w101_eval_string_concat(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        if cmd_name != "eval" || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        // ``eval {script}`` / ``eval {a} {b}`` — every word is a
        // braced literal, no substitution risk.
        if arg_tokens
            .iter()
            .all(|tok| matches!(tok.kind, tcl_lexer::TokenType::Str))
        {
            return;
        }
        // ``eval [list ...]`` and similar canonical-list idioms —
        // single-arg ``Cmd`` whose inner head produces a canonical
        // list.
        if arg_tokens.len() == 1 && self.is_canonical_list_substitution(arg_tokens[0]) {
            return;
        }
        // Substitution detection.  An argument carries substitution
        // when:
        //
        // - the representative token kind is ``Var`` / ``Cmd``
        //   (single-token substitution at the word level), or
        // - the word is multi-token AND its source range contains
        //   an unescaped ``$`` / ``[`` outside any ``{...}`` block.
        //
        // The multi-token-word flag alone is **not** equivalent to
        // substitution: the segmenter sets ``single_token_word=false``
        // for any adjacent-token concatenation, including pure-
        // literal shapes like ``eval foo{bar}`` (Esc+Str joined,
        // no inner Var/Cmd).  Mirroring Python's
        // ``all_tokens[1:]`` walk would require threading the full
        // token stream through ``process_command``; instead we do a
        // brace/backslash-aware source-byte scan over the word's
        // span, which is sound for the common cases and matches
        // Python's behaviour for every fixture in
        // ``tests/test_checks.py::TestEvalStringConcat``.  Known
        // approximation gap: ``"foo{$x}bar"`` (substitution inside
        // a brace pair within a quoted string — Tcl treats braces
        // as literal inside ``"…"``) is not detected.  Real W101
        // shapes don't hit that pattern; documented for posterity.
        let has_substitution = arg_tokens.iter().enumerate().any(|(i, tok)| {
            if matches!(
                tok.kind,
                tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
            ) {
                return true;
            }
            if arg_single.get(i).copied() == Some(true) {
                return false;
            }
            self.word_span_contains_substitution(tok.span)
        });
        if !has_substitution {
            return;
        }
        let first = arg_tokens[0];
        // Quick-fix the common single-line `eval "cmd $a …"` shape: rewrite
        // the quoted string to `eval [list cmd $a …]`.  `[list]` builds a
        // properly-quoted list so each substituted word is passed as exactly
        // one argument and never re-parsed.  Skip when the string spans
        // lines or carries backslash escapes (list re-quoting could differ).
        // Mirrors `checks/_security.py::check_eval_string_concat`.
        let fixes = self.eval_list_fix(first);
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W101".to_string(),
            span: first.span,
            message: "eval with substituted arguments risks code injection. \
Prefer direct invocation or {*}$cmdList to preserve argument boundaries."
                .to_string(),
            severity: Severity::Warning,
            fixes,
        });
    }

    /// Build the `eval [list …]` rewrite fix for a W101 diagnostic whose
    /// `eval` argument is a single-line double-quoted string
    /// (`eval "cmd $a"` → `eval [list cmd $a]`).  Returns an empty vec for
    /// any other shape (braced, multi-line, backslash-escaped).
    fn eval_list_fix(&self, first: tcl_lexer::Token) -> Vec<super::types::CodeFix> {
        let bytes = self.source.as_bytes();
        let open = first.span.start() as usize;
        if bytes.get(open) != Some(&b'"') {
            return Vec::new();
        }
        let Some(rel_close) = self.source[open + 1..].find('"') else {
            return Vec::new();
        };
        let close = open + 1 + rel_close;
        let inner = &self.source[open + 1..close];
        if inner.is_empty() || inner.contains('\n') || inner.contains('\\') {
            return Vec::new();
        }
        vec![super::types::CodeFix {
            span: tcl_lexer::Span::new(first.span.start(), u32::try_from(close + 1).unwrap_or(0)),
            new_text: format!("[list {inner}]"),
            description: "Rewrite to `eval [list …]` (passes each substituted \
word as one argument; no re-parsing)"
                .to_string(),
        }]
    }

    /// Scan the source bytes covered by `span` for an unescaped
    /// ``$`` or ``[`` outside any ``{...}`` brace block.  Used by
    /// [`Self::emit_w101_eval_string_concat`] to detect inner
    /// substitution within a multi-token word without requiring
    /// the full token stream to be threaded through
    /// ``process_command``.
    ///
    /// Brace tracking: ``{`` increments depth, ``}`` decrements;
    /// ``$`` / ``[`` only count as substitution when depth is
    /// zero.  Backslash escapes consume the next byte (so ``\$``
    /// is skipped).  Out-of-bounds spans return false rather than
    /// panicking.
    fn word_span_contains_substitution(&self, span: tcl_lexer::Span) -> bool {
        let start = span.start() as usize;
        let end = span.end() as usize;
        if end > self.source.len() || start >= end {
            return false;
        }
        let bytes = self.source.as_bytes();
        let mut i = start;
        let mut brace_depth: i32 = 0;
        while i < end {
            match bytes[i] {
                b'\\' if i + 1 < end => {
                    i += 2;
                    continue;
                }
                b'{' => brace_depth += 1,
                b'}' if brace_depth > 0 => brace_depth -= 1,
                b'$' | b'[' if brace_depth == 0 => return true,
                _ => {}
            }
            i += 1;
        }
        false
    }

    /// Helper for [`Self::emit_w101_eval_string_concat`].  Returns
    /// true when `tok` is a `Cmd` token whose inner script's
    /// command head (or `cmd subcmd` pair) produces a canonical
    /// list per the registry — the W101 safe-idiom suppression.
    ///
    /// Conservative: rejects multi-command scripts (containing `;`
    /// or newline) because `[list a b; set x $user]` returns the
    /// last command's result, which isn't necessarily a safe list.
    /// Mirrors `_security.py::_is_list_command_token`.
    fn is_canonical_list_substitution(&self, tok: tcl_lexer::Token) -> bool {
        if !matches!(tok.kind, tcl_lexer::TokenType::Cmd) {
            return false;
        }
        let Some(registry) = self.registry.as_ref() else {
            return false;
        };
        let start = tok.span.start() as usize + tok.content_offset as usize;
        let end = tok.span.end() as usize;
        if start >= end || end > self.source.len() {
            return false;
        }
        let script = self.source[start..end].trim();
        if script.is_empty() || script.contains(';') || script.contains('\n') {
            return false;
        }
        // ``parts[0]`` = command head; check both bare form and
        // ``"head sub"`` compound form.
        let mut iter = script.splitn(2, char::is_whitespace);
        let Some(head) = iter.next() else {
            return false;
        };
        if registry.is_canonical_list_command(head) {
            return true;
        }
        if let Some(rest) = iter.next() {
            let mut sub_iter = rest.trim_start().splitn(2, char::is_whitespace);
            if let Some(sub) = sub_iter.next() {
                let compound = format!("{head} {sub}");
                if registry.is_canonical_list_command(&compound) {
                    return true;
                }
            }
        }
        false
    }

    /// Shared substitution probe for the W3xx injection checks:
    /// returns `true` when any argument word carries a `$var` / `[cmd]`
    /// substitution.  A word counts as substituted when its
    /// representative token is `Var` / `Cmd` (single-token substitution
    /// at the word level) or — for a multi-token word — its source span
    /// contains an unescaped `$` / `[` outside any `{...}` block.  This
    /// is the same approximation of Python's `all_tokens[1:]` walk that
    /// [`Self::emit_w101_eval_string_concat`] uses (the analyser doesn't
    /// thread the full token stream through `process_command`); the
    /// representative-kind + brace-aware span scan covers every shape in
    /// the security fixtures.
    fn args_have_substitution(&self, arg_tokens: &[tcl_lexer::Token], arg_single: &[bool]) -> bool {
        arg_tokens.iter().enumerate().any(|(i, tok)| {
            if matches!(
                tok.kind,
                tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
            ) {
                return true;
            }
            if arg_single.get(i).copied() == Some(true) {
                return false;
            }
            self.word_span_contains_substitution(tok.span)
        })
    }

    /// Return the trimmed inner script of a `Cmd` substitution token
    /// (`[ … ]` with the brackets stripped), or `None` for a non-`Cmd`
    /// token or an out-of-bounds span.
    fn cmd_token_inner(&self, tok: tcl_lexer::Token) -> Option<&str> {
        if !matches!(tok.kind, tcl_lexer::TokenType::Cmd) {
            return None;
        }
        let start = tok.span.start() as usize + tok.content_offset as usize;
        let end = tok.span.end() as usize;
        if start >= end || end > self.source.len() {
            return None;
        }
        Some(self.source[start..end].trim())
    }

    /// **W300.** Emit "source with a variable path" when `source`'s
    /// file argument is a `$var` substitution — the path (and therefore
    /// the code executed) is dynamic.  Mirrors
    /// `core/analysis/checks/_security.py:388-429::check_source_variable`
    /// (skips a leading `-encoding ENC` option pair).
    pub(super) fn emit_w300_source_variable(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if cmd_name != "source" || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let mut file_idx = 0;
        if args[0] == "-encoding" && args.len() >= 3 {
            file_idx = 2;
        }
        let Some(tok) = arg_tokens.get(file_idx) else {
            return;
        };
        if matches!(tok.kind, tcl_lexer::TokenType::Var) {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W300".to_string(),
                span: tok.span,
                message: "source with a variable path executes arbitrary Tcl code. \
Ensure the path is not influenced by untrusted input."
                    .to_string(),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W309.** Emit "eval/uplevel with `[subst]` — double
    /// substitution" when an `eval` / `uplevel` argument is a `[subst …]`
    /// command substitution: `subst` expands `$var` / `[cmd]` once, then
    /// the outer command re-parses the result as Tcl — a classic
    /// double-decode injection.  Mirrors
    /// `_security.py:144-189::check_eval_subst_double_decode` (one
    /// diagnostic per command).  Approximation: only the per-word
    /// representative `Cmd` tokens are scanned, so a `[subst …]` buried
    /// inside a larger quoted word isn't detected (same limitation as
    /// W101 / W309 in the absence of the full token stream).
    pub(super) fn emit_w309_eval_subst_double_decode(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if !matches!(cmd_name, "eval" | "uplevel") || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        for tok in arg_tokens {
            let Some(inner) = self.cmd_token_inner(*tok) else {
                continue;
            };
            if inner == "subst" || inner.starts_with("subst ") || inner.starts_with("subst\t") {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W309".to_string(),
                    span: tok.span,
                    message: format!(
                        "{cmd_name} with [subst] creates double substitution: \
subst expands $var and [cmd], then {cmd_name} re-parses the result as Tcl. \
This is a code-injection risk. Use [format] or [string map] for safe templating."
                    ),
                    severity: Severity::Error,
                    fixes: Vec::new(),
                });
                break;
            }
        }
    }

    /// **W301.** Emit "uplevel with string-built script" when an
    /// `uplevel` script argument risks injection: either multiple script
    /// arguments (concatenated like `eval`) or a single unbraced script
    /// word carrying substitution.  Mirrors
    /// `_security.py:233-307::check_uplevel_injection` (skips a leading
    /// `?level?` argument; the `[list …]` idiom is the recognised safe
    /// form).
    pub(super) fn emit_w301_uplevel_injection(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        if cmd_name != "uplevel" || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let script_idx = usize::from(uplevel_has_level(&args[0]));
        if script_idx >= args.len() || script_idx >= arg_tokens.len() {
            return;
        }
        let remaining = &args[script_idx..];
        let remaining_toks = &arg_tokens[script_idx..];
        if remaining.len() > 1 {
            // Multiple args = concat behaviour = danger.
            if self.args_have_substitution(arg_tokens, arg_single) {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W301".to_string(),
                    span: remaining_toks[0].span,
                    message: "uplevel with multiple arguments concatenates them into \
a script (like eval). Use a single braced body or {*}$cmdList to avoid injection."
                        .to_string(),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        } else if let Some(tok) = remaining_toks.first() {
            // Single arg — unbraced + substituted (and not [list …]).
            if matches!(tok.kind, tcl_lexer::TokenType::Str)
                || self.is_canonical_list_substitution(*tok)
            {
                return;
            }
            // A single *pure* variable substitution (`uplevel 1 $body`) is the
            // safe single-substitution idiom: tclsh evaluates `$body` once in
            // the target frame, no concatenation / second substitution.  The
            // script word must be exactly one `Var` token — a concatenation
            // (`$a$b`, `pre$x`) is not a single token and stays flagged.
            if arg_single.get(script_idx).copied() == Some(true)
                && matches!(tok.kind, tcl_lexer::TokenType::Var)
            {
                return;
            }
            if self.args_have_substitution(arg_tokens, arg_single) {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W301".to_string(),
                    span: tok.span,
                    message: "uplevel with an unbraced script argument may cause \
double substitution. Use braces: uplevel 1 {...}"
                        .to_string(),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// **W312.** Emit "interp eval / invokehidden injection" when an
    /// `interp eval` / `interp invokehidden` script argument risks
    /// injection — the same shape as W301 but for the child-interpreter
    /// dispatch.  Mirrors
    /// `_security.py:579-663::check_interp_eval_injection`.
    pub(super) fn emit_w312_interp_eval_injection(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        if cmd_name != "interp" || args.is_empty() {
            return;
        }
        let sub = args[0].as_str();
        if !matches!(sub, "eval" | "invokehidden") || args.len() < 3 || arg_tokens.len() < 3 {
            return;
        }
        // Locate the first script word: `interp eval PATH script …` →
        // index 2; `interp invokehidden PATH ?-opt…? hiddenCmd …` → the
        // first non-option word from index 2.
        let script_start = if sub == "eval" {
            2
        } else {
            let mut i = 2;
            while i < args.len() && args[i].starts_with('-') {
                i += 1;
            }
            if i >= args.len() {
                return;
            }
            i
        };
        if script_start >= args.len() || script_start >= arg_tokens.len() {
            return;
        }
        let script_args = &args[script_start..];
        let script_toks = &arg_tokens[script_start..];
        if script_args.is_empty() || script_toks.is_empty() {
            return;
        }
        // `interp eval` with multiple script words concatenates them.
        if sub == "eval" && script_args.len() > 1 {
            if self.args_have_substitution(arg_tokens, arg_single) {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W312".to_string(),
                    span: script_toks[0].span,
                    message: format!(
                        "interp {sub} with multiple arguments concatenates \
them into a script (like eval). Use a single braced body to avoid injection."
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
            return;
        }
        let tok = script_toks[0];
        if matches!(tok.kind, tcl_lexer::TokenType::Str) || self.is_canonical_list_substitution(tok)
        {
            return;
        }
        if self.args_have_substitution(arg_tokens, arg_single) {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W312".to_string(),
                span: tok.span,
                message: format!(
                    "interp {sub} with an unbraced script argument may \
cause code injection. Use braces: interp {sub} $child {{...}}"
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W102.** Emit "subst on variable input" when `subst`'s template
    /// argument is a bare `$var` substitution — `subst` performs `$` /
    /// `[]` substitution on its argument, so a variable template enables
    /// code injection.  Mirrors
    /// `_security.py:79-138::check_subst_injection`: the message lists
    /// exactly the substitution kinds still active (`-nocommands` /
    /// `-novariables` narrow it) and is suppressed entirely when both
    /// flags are present (only backslash substitution remains).
    pub(super) fn emit_w102_subst_injection(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if cmd_name != "subst" || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let (template_idx, nocommands, novariables) = parse_subst_flags(args);
        let Some(idx) = template_idx else {
            return;
        };
        let Some(tok) = arg_tokens.get(idx) else {
            return;
        };
        if !matches!(tok.kind, tcl_lexer::TokenType::Var) {
            return;
        }
        if nocommands && novariables {
            // Only backslash substitution remains — low risk.
            return;
        }
        let mut active = String::new();
        if !nocommands {
            active.push_str("[cmd]");
        }
        if !nocommands && !novariables {
            active.push_str(" and ");
        }
        if !novariables {
            active.push_str("$var");
        }
        let mut mitigations: Vec<&str> = Vec::new();
        if !nocommands {
            mitigations.push("-nocommands");
        }
        if !novariables {
            mitigations.push("-novariables");
        }
        let message = format!(
            "subst with a variable argument enables code injection: any \
{active} in the string will be evaluated. Add {} to limit substitution \
scope, or use [format] / [string map] for safe templating.",
            mitigations.join(" ")
        );
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W102".to_string(),
            span: tok.span,
            message,
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **W103.** Emit "open with a pipeline" when `open`'s first
    /// argument requests a command pipeline (`open "|cmd"`) or is a
    /// variable that might.  Mirrors
    /// `_security.py:313-382::check_open_pipeline`: a `|`-prefixed
    /// argument carrying substitution is a WARNING (command injection),
    /// a literal `|`-pipeline is a HINT, and a bare `$var` argument is a
    /// WARNING (it may resolve to a `|`-pipeline).
    pub(super) fn emit_w103_open_pipeline(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        if cmd_name != "open" || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let tok = arg_tokens[0];
        if args[0].starts_with('|') {
            let (severity, message) = if self.args_have_substitution(arg_tokens, arg_single) {
                (
                    Severity::Warning,
                    "open with a pipeline containing variable/command \
substitution risks command injection. Validate and sanitize the command \
before passing to open."
                        .to_string(),
                )
            } else {
                (
                    Severity::Hint,
                    "open with a pipeline (\"|\") executes an external command. \
Ensure the command is not influenced by untrusted input."
                        .to_string(),
                )
            };
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W103".to_string(),
                span: tok.span,
                message,
                severity,
                fixes: Vec::new(),
            });
        } else if matches!(tok.kind, tcl_lexer::TokenType::Var) {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W103".to_string(),
                span: tok.span,
                message: "open with a variable argument: if the value starts with \
\"|\", it will execute a command pipeline. Validate input or use explicit \
I/O commands."
                    .to_string(),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W303.** Emit "regexp vulnerable to catastrophic backtracking
    /// (`ReDoS`)" when a *literal* regex pattern in `regexp` / `regsub` /
    /// `switch -regexp` contains a nested quantifier (`(a+)+`) or an
    /// overlapping alternation (`(a|a)+`).  Mirrors
    /// `_security.py:451-475::check_redos` driven by
    /// `_find_regex_patterns_in_command`; variable / command-substituted
    /// patterns are left alone (the literal text never matches the
    /// detector), matching Python's literal-only behaviour.
    pub(super) fn emit_w303_redos(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if !matches!(cmd_name, "regexp" | "regsub" | "switch") {
            return;
        }
        let patterns = find_regex_patterns_in_command(cmd_name, args, arg_tokens);
        if patterns.is_empty() {
            return;
        }
        // Nested quantifier `…+)+` / `…*)*` or overlapping alternation
        // `(a|a)+` — the `_REDOS_PATTERN` shape from `_security.py`.
        for (pattern, tok) in patterns {
            if has_redos_shape(&pattern) {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W303".to_string(),
                    span: tok.span,
                    message: "Regular expression may be vulnerable to catastrophic \
backtracking (ReDoS). Nested quantifiers like (a+)+ can cause exponential \
matching time on crafted input."
                        .to_string(),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// **W306.** Warn when a `regexp` / `regsub` *pattern* — a
    /// literal-expected position — contains a *live* substitution Tcl
    /// expands before the regex engine sees it.  A bare `$var` pattern is
    /// the canonical parameterised-pattern idiom and is exempt (there is
    /// no braced equivalent); a quoted `"$var"` / `"[cmd]"` or an unbraced
    /// `[cmd]` is the foot-gun.  `\[` / `\$` in a quoted pattern are
    /// literal regex characters, not substitutions.  Mirrors the
    /// `regexp` / `regsub` arm of `_domain.py::check_literal_expected`.
    pub(super) fn emit_w306_literal_expected(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if !matches!(cmd_name, "regexp" | "regsub") {
            return;
        }
        let Some(idx) = regexp_pattern_index(args) else {
            return;
        };
        let (Some(&tok), Some(text)) = (arg_tokens.get(idx), args.get(idx)) else {
            return;
        };
        if is_braced_word(&tok) || !has_substitution(text, &tok) {
            return;
        }
        let start = tok.span.start() as usize;
        let end = tok.span.end() as usize;
        if start >= end || end > self.source.len() {
            return;
        }
        let is_subst_token = matches!(
            tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        );
        // A quoted/literal token (not a single `$var` / `[cmd]` word) only
        // counts when the raw source carries a *live* (unescaped) `[`/`$`.
        if !is_subst_token && !raw_has_live_substitution(&self.source[start..end]) {
            return;
        }
        // Bare `$var` / `${var}` is the canonical idiom — a `Var` word has
        // no surrounding literal text, so it is exactly that form.
        if tok.kind == tcl_lexer::TokenType::Var {
            return;
        }
        let is_quoted = self.source.as_bytes().get(start) == Some(&b'"');
        let found = if text.contains('$') { "'$'" } else { "'['" };
        let advice = if is_quoted {
            ". Use braces '{...}' instead of quotes."
        } else {
            ". Use braces '{...}' to prevent substitution."
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W306".to_string(),
            span: tok.span,
            message: format!(
                "Literal expected in {cmd_name} pattern \u{2014} found {found}{advice}"
            ),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **IRULE2002.** Warn when a deprecated iRules command is used —
    /// the command's spec carries a `deprecated_replacement`.  Only fires
    /// under the `f5-irules` dialect.  Mirrors
    /// `core/analysis/checks/_domain.py::check_deprecated_irules_command`.
    pub(super) fn emit_irule2002_deprecated_command(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
    ) {
        if self.dialect != "f5-irules" {
            return;
        }
        let Some(replacement) = self
            .registry
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .and_then(|s| s.deprecated_replacement)
        else {
            return;
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "IRULE2002".to_string(),
            span: cmd_tok.span,
            message: format!("'{cmd_name}' is deprecated in iRules. Use '{replacement}' instead."),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **W310.** Emit "hardcoded credential" for a literal secret value.
    /// Mirrors both strategies of
    /// `_security.py:507-573::check_hardcoded_credentials` (one diagnostic
    /// per command):
    ///
    /// * **Strategy 1** — a credential-bearing option flag (the defaults
    ///   `-password` / `-pass` / `-secret` / `-token` / `-apikey`,
    ///   case-insensitive, unioned with the command's registry
    ///   `credential_options`, e.g. `http::geturl`'s `-headers`) followed
    ///   by a literal value.
    /// * **Strategy 2** — a subcommand whose registry `credential_arg` /
    ///   `sensitive_headers` mark a literal value at a sensitive header
    ///   (e.g. `HTTP::header insert authorization "Bearer …"`).
    pub(super) fn emit_w310_hardcoded_credentials(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        // Registry-augmented credential option flags (all `'static`, so
        // the `self.registry` borrow ends with this binding).
        let extra_opts: &'static [&'static str] = self
            .registry
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .map_or(&[], |s| s.credential_options);

        // Strategy 1: a credential option flag with a literal value.
        for (i, text) in args.iter().enumerate() {
            let lower = text.to_ascii_lowercase();
            if !DEFAULT_PASSWORD_OPTIONS.contains(&lower.as_str())
                && !extra_opts.contains(&lower.as_str())
            {
                continue;
            }
            let (Some(value), Some(val_tok)) = (args.get(i + 1), arg_tokens.get(i + 1)) else {
                continue;
            };
            if is_literal_credential_value(value, val_tok) {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W310".to_string(),
                    span: val_tok.span,
                    message: format!(
                        "Hardcoded credential in {text} argument. Store secrets in \
environment variables or a vault, not in source code."
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
                return; // one diagnostic per command
            }
        }

        // Strategy 2: a subcommand credential header with a literal value.
        if args.len() >= 3 {
            let sub = args[0].to_ascii_lowercase();
            // `(credential_arg, sensitive_headers)` — both copied out so
            // the registry borrow ends before we mutate `self.result`.
            let cred_info: Option<(usize, &'static [&'static str])> = self
                .registry
                .as_ref()
                .and_then(|r| r.get(cmd_name))
                .and_then(|s| s.subcommand(&sub))
                .and_then(|sc| {
                    sc.credential_arg
                        .map(|a| (a as usize, sc.sensitive_headers))
                });
            if let Some((cred_arg, sensitive)) = cred_info {
                let header_name = args[1].to_ascii_lowercase();
                if sensitive.contains(&header_name.as_str()) && cred_arg < arg_tokens.len() {
                    if let (Some(value), Some(val_tok)) =
                        (args.get(cred_arg), arg_tokens.get(cred_arg))
                    {
                        if is_literal_credential_value(value, val_tok) {
                            self.result.diagnostics.push(super::types::Diagnostic {
                                code: "W310".to_string(),
                                span: val_tok.span,
                                message: format!(
                                    "Hardcoded credential in {header_name} header value. \
Store secrets in environment variables or a vault, not in source code."
                                ),
                                severity: Severity::Warning,
                                fixes: Vec::new(),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Classify the positional value for W304: tristate severity,
    /// human-readable message, and an optional "origin" diagnostic
    /// for the constant-propagated INFO path.  Split out of
    /// [`Self::emit_w304_missing_option_terminator`] to keep that
    /// method's body within the clippy `too_many_lines` budget;
    /// mirrors the severity tree at ``_style.py:565-627``.
    fn classify_w304(
        &self,
        tok: tcl_lexer::Token,
        is_dynamic: bool,
        looks_like_option: bool,
        command_label: &str,
    ) -> (Severity, String, Option<super::types::Diagnostic>) {
        if is_dynamic && !looks_like_option {
            if matches!(tok.kind, tcl_lexer::TokenType::Var) {
                let var_name = self.var_name_from_token(tok);
                let resolved = var_name.and_then(|name| {
                    last_literal_set_value_for_var(
                        &self.source,
                        &name,
                        tok.span.start(),
                        self.lexer_config(),
                    )
                });
                if let Some((resolved_text, resolved_span, var_text)) = resolved {
                    if resolved_text.starts_with('-') {
                        let message = format!(
                            "'{command_label}' parses leading '-' as options. \
This value currently resolves to '{resolved_text}', so add '--' to force \
data parsing."
                        );
                        return (Severity::Warning, message, None);
                    }
                    let message = format!(
                        "'{command_label}' parses leading '-' as options. \
This value is reported at INFO because '{var_text}' currently resolves to \
static literal '{resolved_text}'. Keep '--' to guard against future \
option-injection regressions if the variable changes."
                    );
                    let origin = super::types::Diagnostic {
                        code: "W304".to_string(),
                        span: resolved_span,
                        message: format!(
                            "'{var_text}' is currently assigned static \
literal '{resolved_text}' here; this is why the diagnostic is INFO."
                        ),
                        severity: Severity::Suggestion,
                        fixes: Vec::new(),
                    };
                    return (Severity::Suggestion, message, Some(origin));
                }
            }
            // Command substitution / unresolved variable — INFO
            // with the substituted-input message.
            let message = format!(
                "'{command_label}' parses leading '-' as options. \
Insert '--' before substituted input to reduce option-injection risk."
            );
            return (Severity::Suggestion, message, None);
        }
        // ALWAYS: literal value that starts with `-`.
        let message = format!(
            "'{command_label}' argument starts with '-'. Add '--' \
before this value so it is treated as data, not an option."
        );
        (Severity::Warning, message, None)
    }

    /// Extract the variable name for a `Var` token using the
    /// lexer-provided token-text semantics
    /// ([`tcl_lexer::SourceMap::token_text`]).  Preserves the
    /// `Var`-specific normalisation rules (notably the trailing
    /// `}` strip for the `${}` degenerate case where the lexer
    /// extends the span by one byte to cover the closing brace),
    /// so this stays in sync with the rest of the analyser's
    /// token-text usage and avoids edge-case mismatches that a
    /// raw `self.source[..]` slice would introduce.  Returns
    /// `None` when the extracted text is empty.
    fn var_name_from_token(&self, tok: tcl_lexer::Token) -> Option<String> {
        let sm = tcl_lexer::SourceMap::new(&self.source);
        let text = sm.token_text(tok);
        if text.is_empty() {
            return None;
        }
        Some(text.to_string())
    }

    /// Compute the W304 code-fix span and diagnostic end position.
    ///
    /// For `Cmd` tokens (`[…]`) the lexer span excludes the closing
    /// `]`; we extend the span by one byte when the next character
    /// is `]` so the prepended ``-- `` doesn't split the bracket
    /// pair.  All other token kinds use the lexer span directly.
    fn compute_w304_fix_span(&self, tok: tcl_lexer::Token) -> (tcl_lexer::Span, u32) {
        let span_start = tok.span.start();
        let span_end = tok.span.end();
        if matches!(tok.kind, tcl_lexer::TokenType::Cmd) {
            let after = span_end as usize;
            if after < self.source.len() && self.source.as_bytes()[after] == b']' {
                let extended = span_end + 1;
                return (tcl_lexer::Span::new(span_start, extended), extended);
            }
        }
        (tcl_lexer::Span::new(span_start, span_end), span_end)
    }

    /// **W128 (SYNC-JUN02b-4, #519).** Flag a call to a command that was
    /// renamed or deleted earlier in the same file — it falls through to
    /// the `unknown` handler.
    ///
    /// Backed by the flow-sensitive command-binding lattice
    /// ([`crate::command_binding`]).  The lattice is seeded with every
    /// module procedure (canonically qualified) as `Proc` so a proc
    /// defined inside a `namespace eval` block — whose top-level CFG never
    /// sees the full qname — is still known, matching the optimiser's
    /// gating view.  A call fires W128 only when its resolved binding is
    /// `Opaque` *and* its name was actually perturbed somewhere in this
    /// file (`rebound_names`); a merely-undefined external command (always
    /// opaque, never rebound) does not.  A dynamic mutation collapses the
    /// lattice to the wildcard ⊤, under which every binding resolves to
    /// `Unknown` (not `Opaque`), so W128 conservatively goes quiet.
    pub(super) fn emit_w128_renamed_command(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::command_binding::{analyse_command_binding, Binding, BindingKind};
        use crate::ir::Statement;
        use crate::naming::normalise_qualified_name as nqn;

        let cfg = &cu.top_level.cfg;
        let seed: Vec<(String, Binding)> = cu
            .ir_module
            .procedures
            .keys()
            .map(|q| {
                (
                    q.clone(),
                    Binding {
                        kind: BindingKind::Proc,
                        target: Some(q.clone()),
                    },
                )
            })
            .collect();
        let binding = analyse_command_binding(cfg, registry, &seed);
        let rebound = binding.rebound_names();
        if rebound.is_empty() {
            return;
        }
        // Reverse-postorder for deterministic diagnostic ordering.
        for block_name in cfg.reverse_postorder() {
            let Some(block) = cfg.blocks.get(&block_name) else {
                continue;
            };
            for (idx, stmt) in block.statements.iter().enumerate() {
                let Statement::Call { command, span, .. } = stmt else {
                    continue;
                };
                // The mutating commands themselves are not flagged.
                if command.is_empty() || matches!(command.as_str(), "rename" | "interp" | "proc") {
                    continue;
                }
                if binding.binding_at(&block_name, idx, command).kind != BindingKind::Opaque {
                    continue;
                }
                if !rebound.contains(&nqn(command)) {
                    continue; // never bound here → an ordinary external command
                }
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W128".to_string(),
                    span: *span,
                    message: format!(
                        "Command '{command}' was renamed or deleted earlier in this \
file; this call falls through to the 'unknown' handler."
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// CFG/SSA-backed diagnostic orchestrator.
    ///
    /// Mirrors `_emit_cfg_ssa_diagnostics` in
    /// `_diagnostics.py:118-181`.  Builds a
    /// [`crate::compilation_unit::CompilationUnit`] for `source`,
    /// then walks the top-level + every procedure, dispatching
    /// per-function emitters.
    ///
    /// **C41d2 lands** the full ``_diag_var_lifecycle.py``
    /// emitter set (W220, W211, W214, W210, W213, H300).
    /// **C41d3 lands** the var-as-command post-pass (W307); W308
    /// awaits the class-hierarchy port.  W242 (interpolated-
    /// command resolution) lands in **C41d4**.
    pub fn emit_cfg_ssa_diagnostics(&mut self, source: &str) {
        use tcl_registry::prelude::DialectSet;
        use tcl_registry::CommandRegistry;

        let mut registry = CommandRegistry::build_default();
        if let Some(d) = DialectSet::parse(&self.dialect) {
            registry.load_dialect(d);
        }
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);

        // **W128 (SYNC-JUN02b-4).** Flag calls to commands renamed or
        // deleted earlier in the file via the flow-sensitive
        // command-binding lattice.  Independent of the CFG/SSA dead-store
        // machinery below, so run it up front against the same `cu`.
        self.emit_w128_renamed_command(&cu, &registry);

        // **C41e3 follow-up.** Compute the set of globals any
        // proc in this module writes to.  Top-level RBS (W210)
        // is suppressed for these variables — a helper proc may
        // populate them before the top-level read fires.
        // Mirrors `_globals_written_by_procs` in
        // `_diag_commands.py:264-296`.
        let globals_written = globals_written_by_procs(&cu);

        // **W220 call-by-name suppression (SYNC-JUN02d-2).** Build the
        // interprocedural proc-index once so a caller-local passed *by
        // name* to a proc that consumes it via `upvar` (`set tag "";
        // asnPeekTag data tag type dummy`) is not flagged as a dead
        // store.  `collect_call_by_name_reads` then yields the suppressed
        // names per function, merged into the dead-store `cross_event_vars`.
        let cbn_proc_index = {
            let ia = crate::interprocedural::build_interprocedural_analysis(
                &cu.ir_module,
                &registry,
                Some(self.dialect.as_str()),
            );
            crate::interprocedural::build_proc_index_from_summaries(&ia)
        };

        // **C41-default-on-followups-postpass W220-IR-paths.**
        // pkgIndex.tcl files have ``$dir`` set by the package
        // loader before the script body runs — suppress dead-
        // store / unused-variable diagnostics for it at the
        // top-level.  Mirrors `_diagnostics.py:147-149`.
        let mut top_level_cross_event_vars: HashSet<String> = if self
            .file_path
            .as_deref()
            .is_some_and(|p| p.ends_with("pkgIndex.tcl"))
        {
            HashSet::from(["dir".to_string()])
        } else {
            HashSet::new()
        };
        top_level_cross_event_vars.extend(crate::interprocedural::collect_call_by_name_reads(
            &cu.top_level.cfg,
            &cbn_proc_index,
        ));

        // Top-level first, then procedures in insertion order —
        // matches the iteration order of
        // ``CompilationUnit::functions``.
        // Iterate top-level explicitly so we can pass the IR
        // module through.
        self.emit_cfg_ssa_diagnostics_for_function_full(
            &cu.top_level,
            &cu.ir_module,
            &globals_written,
            &top_level_cross_event_vars,
        );
        self.emit_channel_diagnostics(&cu.top_level, &registry);
        for (qname, fu) in &cu.procedures {
            // **C41-default-on-followups-postpass W220-IR-paths.**
            // For ``::when::*`` procs, threaded
            // ``cross_event_defs | cross_event_imports`` from the
            // ConnectionScope so dead-store / unused-variable
            // diagnostics suppress vars that may be read in a
            // different iRule event.  Mirrors
            // `_diagnostics.py:165-167`.
            let mut cross_event_vars: HashSet<String> =
                if let Some(scope) = cu.connection_scope.as_ref() {
                    if qname.starts_with("::when::") {
                        scope
                            .cross_event_defs
                            .iter()
                            .chain(scope.cross_event_imports.iter())
                            .cloned()
                            .collect()
                    } else {
                        HashSet::new()
                    }
                } else {
                    HashSet::new()
                };
            // SYNC-JUN02d-2: suppress dead-store on caller-locals this
            // proc passes by name to an upvar callee.
            cross_event_vars.extend(crate::interprocedural::collect_call_by_name_reads(
                &fu.cfg,
                &cbn_proc_index,
            ));
            self.emit_cfg_ssa_diagnostics_for_function_full(
                fu,
                &cu.ir_module,
                &HashSet::new(),
                &cross_event_vars,
            );
            self.emit_channel_diagnostics(fu, &registry);
            // **C41d7.** IRULE4005 — racy ``static::``
            // cross-event flow.  Only fires for non-RULE_INIT
            // ``when`` procs when ``ConnectionScope::racy_static_defs``
            // is non-empty.  Mirrors Python's
            // ``_emit_racy_static_diagnostics`` call site in
            // ``_diagnostics.py:171-175``.
            if let Some(scope) = cu.connection_scope.as_ref() {
                if qname.starts_with("::when::") && !scope.racy_static_defs.is_empty() {
                    let event = crate::ir::when_event_name(qname);
                    if event != "RULE_INIT" {
                        self.emit_racy_static_diagnostics(fu, &scope.racy_static_defs);
                    }
                }
            }
        }

        // Cross-function post-pass: resolve $var-as-command sites
        // collected during the walk.  Mirrors
        // ``_emit_var_command_diagnostics`` in
        // ``_diag_var_command.py``.
        self.emit_var_command_diagnostics(&cu, &registry);

        // **C41 follow-up.** Suppress W123 for command-name
        // heads with partial interpolations like ``foo$suffix``
        // when ``$suffix`` resolves cleanly to a finite set of
        // known commands via SCCP.  Mirrors
        // ``_resolve_interpolated_commands`` in
        // ``_diag_commands.py:188-260``.
        self.resolve_interpolated_w123_diagnostics(&cu);
    }

    /// Per-function diagnostic dispatcher.
    ///
    /// Mirrors `_emit_cfg_ssa_diagnostics_for_function` in
    /// `_diagnostics.py:183-209`.  Called once for the top-level
    /// script and once per procedure.  Each per-emitter call is
    /// gated on its own predicate inside the helper.
    ///
    /// **C41d2 wires** all six ``_diag_var_lifecycle.py``
    /// emitters.  Each future C41d strip adds another emitter
    /// call here.
    pub fn emit_cfg_ssa_diagnostics_for_function(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        ir_module: &crate::ir::Module,
    ) {
        self.emit_cfg_ssa_diagnostics_for_function_full(
            function_unit,
            ir_module,
            &HashSet::new(),
            &HashSet::new(),
        );
    }

    /// Per-function diagnostic dispatcher with an extra
    /// "known-defined" set passed through to RBS suppression.
    ///
    /// Same as [`Self::emit_cfg_ssa_diagnostics_for_function`]
    /// but accepts an additional set of variable names that
    /// should be treated as already-defined for the W210
    /// (read-before-set) emitter.  Used at the top-level to
    /// suppress RBS for variables that any proc in the module
    /// writes — matches the
    /// ``extra_known_defined_vars=self._globals_written_by_procs(cu)``
    /// argument in `_diagnostics.py:154`.
    pub fn emit_cfg_ssa_diagnostics_for_function_with_extra(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        ir_module: &crate::ir::Module,
        extra_known_defined: &HashSet<String>,
    ) {
        self.emit_cfg_ssa_diagnostics_for_function_full(
            function_unit,
            ir_module,
            extra_known_defined,
            &HashSet::new(),
        );
    }

    /// Per-function diagnostic dispatcher with the full
    /// suppression context.
    ///
    /// Adds `cross_event_vars` on top of
    /// [`Self::emit_cfg_ssa_diagnostics_for_function_with_extra`].
    /// Used by the W220 IR-paths port to suppress dead-store
    /// diagnostics for variables a `::when::*` proc may carry
    /// across iRule events (`cu.connection_scope.cross_event_defs
    /// | cross_event_imports`) and for `pkgIndex.tcl` `$dir`,
    /// which the package loader assigns before the script body
    /// runs.
    ///
    /// Mirrors the `cross_event_vars=` arg threaded through
    /// `_emit_cfg_ssa_diagnostics_for_function` in
    /// `_diagnostics.py:159, 171`.
    pub fn emit_cfg_ssa_diagnostics_for_function_full(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        ir_module: &crate::ir::Module,
        extra_known_defined: &HashSet<String>,
        cross_event_vars: &HashSet<String>,
    ) {
        let defined = collect_defined_vars(&function_unit.cfg);
        let scope_aliases = crate::optimiser::elimination::scan_scope_aliases(&function_unit.cfg);
        let mut textually_referenced =
            crate::optimiser::elimination::collect_textual_var_references(
                &self.source,
                &function_unit.cfg,
            );
        // A var read in another iRule event, or consumed *by name* via a
        // call-by-name upvar callee (SYNC-JUN02d-2), is "used" — suppress
        // the unused-variable (W211) hint too, not just the dead store
        // (W220).  Mirrors Python threading `cross_event_vars` through
        // both `_dead_stores` and `_unused_variables`.
        textually_referenced.extend(cross_event_vars.iter().cloned());
        let ir_proc = ir_module.procedures.get(&function_unit.name);
        self.emit_dead_store_diagnostics(function_unit, &defined, &scope_aliases, cross_event_vars);
        self.emit_unused_variable_diagnostics(
            function_unit,
            &defined,
            &scope_aliases,
            &textually_referenced,
        );
        self.emit_possible_paste_error_diagnostics(function_unit);
        // Shared read-before-set context: the SCCP-executable block set and
        // the name-level suppression (`dict with` keys, qualified-`variable`
        // alias tails, dict vars), threaded through both the version-0
        // statement/branch emitter and the `Terminator::Return` pass.
        let considered: HashSet<String> = if function_unit.sccp.executable_blocks.is_empty() {
            function_unit.ssa.blocks.keys().cloned().collect()
        } else {
            function_unit.sccp.executable_blocks.clone()
        };
        let supp = build_undef_suppression(function_unit, &considered);
        let exists_guards = collect_existence_guards(function_unit);
        let rbs_params: HashSet<&str> = ir_proc
            .map(|p| p.params.iter().map(String::as_str).collect())
            .unwrap_or_default();
        self.emit_read_before_set_diagnostics(
            function_unit,
            ir_proc,
            &defined,
            &scope_aliases,
            extra_known_defined,
            &supp,
        );
        // Phi-from-undef on `return $v` reads (the def-use builder records
        // statement + branch-condition uses but NOT `Terminator::Return`
        // values).  Mirrors the `CFGReturn` arm of `_read_before_set`.
        self.emit_return_phi_undef_w210(
            function_unit,
            &rbs_params,
            &exists_guards,
            &scope_aliases,
            extra_known_defined,
            &defined,
            &considered,
            &supp,
        );
        // W210 on reads of a provably-no-match regexp / scan output var.
        self.emit_provably_unset_w210(function_unit, &considered, &defined);
        self.emit_constant_branch_diagnostics(function_unit);
        self.emit_existence_constant_branch_diagnostics(function_unit, ir_proc);
        self.emit_invalid_ip_diagnostics(function_unit);
        self.emit_w233_divide_by_zero(function_unit);
        self.emit_interval_bounds_diagnostics(function_unit);
        if let Some(ir_proc) = ir_proc {
            self.emit_unused_param_diagnostics(function_unit, ir_proc);
        }
    }

    /// Statements whose dead-store **W220** hint should be **suppressed**
    /// because their array-element / dict-path def place is observed by some
    /// read in the function (Phase 8 place-model precision, SYNC-MAY31-1b).
    ///
    /// Name-level SSA folds `a(k)` / `a(j)` / `$a` to the base name `a`, so a
    /// later `set a(j) 2` looks like it overwrites `set a(k) 1` before any read
    /// — a false dead store when `a(k)` is in fact read.  Delegates to the
    /// shared [`crate::place_bridge::element_writes_observed_by_reads`] (also
    /// used by the optimiser's O109), which resolves each element write to a
    /// [`Place`](crate::place::Place) and consults the over-approximating
    /// [`overlap`](crate::place::overlap).  Scalars keep the precise name-level
    /// verdict (they don't fold), so a genuine `set x 1; set x 2; puts $x` dead
    /// store still fires.  Empty when no registry is bound (e.g. the bare
    /// `emit_cfg_ssa_diagnostics` test path).
    fn place_suppressed_dead_stores(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) -> std::collections::HashSet<(String, i32)> {
        self.registry.as_ref().map_or_else(Default::default, |reg| {
            crate::place_bridge::element_writes_observed_by_reads(&fu.cfg, &fu.name, reg)
        })
    }

    /// W220 — dead-store hint.
    ///
    /// Mirrors `_emit_dead_store_diagnostics` in
    /// `_diag_var_lifecycle.py:29-72`, plus the
    /// IR-statement-type / SCCP path-sensitivity filters baked
    /// into Python's underlying `_dead_stores` analysis
    /// (`core_analyses.py:1105-1156`).  A *dead store* is an
    /// assignment whose value is overwritten before being read —
    /// some other SSA version of the same variable is live, so
    /// this version's value never reaches a user.
    ///
    /// Walks every dead [`Statement`](crate::ir::Statement) chain
    /// in `fu.def_use`, checks that another version of the same
    /// variable has live uses, and emits a Hint at the dead
    /// statement's span.  When the variable's name has a
    /// case-insensitive twin among `defined_vars`, the message
    /// includes a "did you mean…?" suggestion.
    ///
    /// Filters applied (each one mirrors a Python suppression):
    ///
    /// 1. **SCCP-unreachable blocks** — definitions in blocks
    ///    SCCP proved unreachable are reported as O107 by the
    ///    optimiser and intentionally suppressed here so we
    ///    don't double-up on dead-code calls.
    /// 2. **Scope aliases** (`global` / `upvar`) — writes are
    ///    visible in another scope; the local "no use" verdict
    ///    is unsafe.
    /// 3. **Cross-event vars** — for `pkgIndex.tcl` `$dir` and
    ///    iRules `::when::*` cross-event defs/imports, a write
    ///    in one event may be read in another at runtime.
    /// 4. **Globals (`::`-prefixed)** — externally consumed.
    ///    Python skips them in `_dead_stores`.
    /// 5. **Side-effecting stores** — only `AssignConst`,
    ///    `AssignValue` without `[`, and `AssignExpr` without a
    ///    command call are considered.  `Call.defs`, `Incr`, and
    ///    other side-effecting writes shouldn't be flagged
    ///    because removing the assignment would also drop the
    ///    side effect.
    fn emit_dead_store_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        cross_event_vars: &HashSet<String>,
    ) {
        use crate::def_use::DefKind;
        use crate::ir::Statement;
        use crate::ir_helpers::expr_has_command;
        use std::fmt::Write as _;
        // Phase 8 place-model precision: array-element / dict-path writes the
        // name-level SSA mis-folds but that a read actually observes.
        let place_suppressed = self.place_suppressed_dead_stores(fu);
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (var, version) = &chain.key;
            // Globals (``::``-prefixed) are externally consumed
            // — Python `_dead_stores` skips them.
            if var.starts_with("::") {
                continue;
            }
            // Scope-aliased vars (introduced via ``global`` or
            // ``upvar``) write through to a different scope — the
            // local "no use" verdict is unsafe.
            if scope_aliases.contains(var) {
                continue;
            }
            // Cross-event vars (iRules ``::when::*`` defs/imports
            // or ``pkgIndex.tcl`` ``$dir``) may be read in
            // another event/scope at runtime.
            if cross_event_vars.contains(var) {
                continue;
            }
            // Suppress dead stores in SCCP-unreachable blocks —
            // O107 already reports the whole block as dead, and
            // re-flagging individual stores inside it adds noise.
            if !fu.sccp.executable_blocks.contains(&chain.definition.block) {
                continue;
            }
            // ``any_other_live`` — another SSA version of this
            // variable has live uses, so this assignment is
            // overwritten.  When no other version is live, the
            // variable is truly unused — that's W211, handled
            // separately.
            let any_other_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == *var && k.1 != *version && !c.is_dead());
            if !any_other_live {
                continue;
            }
            let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
                continue;
            };
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            // IR-statement type filter — mirror Python's
            // `_dead_stores` shape (`core_analyses.py:1149-1155`).
            // Only pure assignments are reportable; side-effecting
            // writes (``Call``, ``Incr``, command-substitution
            // values, expressions invoking commands) are skipped
            // because dropping them would also drop the side
            // effect.
            match stmt {
                Statement::AssignConst { .. } => {}
                Statement::AssignValue { value, .. } => {
                    if value.contains('[') {
                        continue;
                    }
                }
                Statement::AssignExpr { expr, .. } => {
                    if expr_has_command(expr) {
                        continue;
                    }
                }
                _ => continue,
            }
            // Suppress when this element write is observed by a read the
            // name-level SSA can't see (place-model overlap, stage 4).
            if place_suppressed.contains(&(
                chain.definition.block.clone(),
                chain.definition.statement_index,
            )) {
                continue;
            }
            let span = stmt.span();
            if span.is_empty() {
                continue;
            }
            let mut message = format!("Assignment to '{var}' is never read");
            if let Some(similar) = find_case_mismatch(var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W220".to_string(),
                span,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// W211 — unused-variable hint.
    ///
    /// Mirrors `_emit_unused_variable_diagnostics` in
    /// `_diag_var_lifecycle.py:226-258`.  Fires when an
    /// assignment's variable has no live uses **and** no other
    /// SSA version is live (so the variable is entirely unused
    /// — distinct from W220's overwritten-before-read case).
    ///
    /// Three filters apply:
    ///
    /// 1. **Scope aliases** (``global`` / ``upvar``) — writes
    ///    are visible in the aliased scope, so a "no local use"
    ///    verdict is unsafe.
    /// 2. **Textual references** — variable names that appear
    ///    inside a ``"$x"`` string interpolation or a
    ///    ``Return`` value are kept live; the def-use builder
    ///    doesn't track those reads.
    /// 3. **Empty spans** — synthetic IR statements with no
    ///    user-visible source text.
    ///
    /// "Did you mean…?" suggestions use case-insensitive
    /// matching against the function's defined-variable set.
    fn emit_unused_variable_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        textually_referenced: &HashSet<String>,
    ) {
        use crate::def_use::DefKind;
        use std::fmt::Write as _;
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (var, version) = &chain.key;
            if scope_aliases.contains(var) {
                continue;
            }
            if textually_referenced.contains(var) {
                continue;
            }
            // Only emit when no other SSA version of this var is
            // live — the W220 path handles overwritten cases.
            let any_other_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == *var && k.1 != *version && !c.is_dead());
            if any_other_live {
                continue;
            }
            let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
                continue;
            };
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            let span = stmt.span();
            if span.is_empty() {
                continue;
            }
            let mut message = format!("Variable '{var}' is set but never used");
            if let Some(similar) = find_case_mismatch(var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W211".to_string(),
                span,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// H300 — possible paste error (duplicate dead-store with
    /// identical literal).
    ///
    /// Mirrors `_emit_possible_paste_error_diagnostics` in
    /// `_diag_var_lifecycle.py:74-121`.  When two consecutive
    /// statements in the same block are both dead stores AND
    /// share the same paste-fingerprint
    /// (same variable name + same trimmed literal value), emit
    /// a Hint at the *second* statement's span — the duplicate
    /// is the one that's almost certainly a paste error.
    ///
    /// Variables whose names start with ``_`` are excluded from
    /// the heuristic on the assumption that the leading
    /// underscore signals the user has flagged them as
    /// intentional.
    fn emit_possible_paste_error_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        use crate::def_use::DefKind;
        use std::collections::HashMap;

        // Pre-compute, per block, the set of statement indices
        // that are dead stores.  Walk every dead Statement-kind
        // chain in def_use, bucket by block.
        let mut dead_idx: HashMap<&str, HashSet<usize>> = HashMap::new();
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            dead_idx
                .entry(chain.definition.block.as_str())
                .or_default()
                .insert(idx);
        }

        for (block_name, block) in &fu.cfg.blocks {
            let Some(dead_indices) = dead_idx.get(block_name.as_str()) else {
                continue;
            };
            // Walk consecutive pairs (idx, idx + 1).  Only the
            // first must be dead — the second's
            // dead-status is irrelevant; what matters is whether
            // the value being assigned matches.
            for idx in 0..block.statements.len().saturating_sub(1) {
                if !dead_indices.contains(&idx) {
                    continue;
                }
                let Some(first) = super::utils::possible_paste_fingerprint(&block.statements[idx])
                else {
                    continue;
                };
                let Some(second) =
                    super::utils::possible_paste_fingerprint(&block.statements[idx + 1])
                else {
                    continue;
                };
                if first != second {
                    continue;
                }
                let (var_name, literal) = first;
                if var_name.starts_with('_') {
                    continue;
                }
                let span = block.statements[idx + 1].span();
                if span.is_empty() {
                    continue;
                }
                let pretty = super::utils::format_literal_for_message(&literal);
                let message = format!(
                    "Possible paste error: repeated assignment to '{var_name}' \
                     with static value '{pretty}'; \
                     did you mean to assign a different variable?"
                );
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "H300".to_string(),
                    span,
                    message,
                    severity: Severity::Hint,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// W214 — unused-parameter hint.
    ///
    /// Mirrors `_emit_unused_param_diagnostics` in
    /// `_diag_var_lifecycle.py:260-274`.  For every parameter
    /// declared in `ir_proc.params`, check whether any def-use
    /// chain for the parameter (any SSA version) has live uses.
    /// When all chains are dead, the parameter is unused —
    /// emit a Hint at the proc's span.
    ///
    /// Diverges slightly from Python's ``analysis.unused_params``:
    /// Python pre-computes the unused-params list during
    /// ``analyse_ir_module``; the Rust port inlines the same
    /// def-use scan here because the Rust ``FunctionAnalysis``
    /// builder hasn't been ported yet.  The check is equivalent —
    /// a parameter is unused iff no SSA version of its name has
    /// live uses.
    fn emit_unused_param_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: &crate::ir::Procedure,
    ) {
        // Empty-body procs (``proc foo {a b} {}``) are signature
        // placeholders — stubs declaring an API whose implementation
        // lives elsewhere.  Every parameter is necessarily "unused"
        // since there is no body to use it, so flagging is pure noise.
        // Mirrors the `not body.statements` early return in
        // `_diag_var_lifecycle.py::_emit_unused_param_diagnostics`.
        if ir_proc.body.statements.is_empty() {
            return;
        }
        let mut unused: Vec<String> = Vec::new();
        for param in &ir_proc.params {
            // Tcl's variadic ``args`` parameter is conventionally
            // declared even when unused (as a "consume the rest"
            // marker).  Skip it from W214.
            if param == "args" {
                continue;
            }
            // Positional keyword markers: a param whose name is itself a
            // quoted literal (snit-style ``{"as" ""}``) is a syntactic
            // placeholder consumed by being PRESENT in the call form, not
            // read as a variable.  Flagging it is noise.  Conservative:
            // only suppress params whose name starts AND ends with ``"``.
            if param.len() >= 2 && param.starts_with('"') && param.ends_with('"') {
                continue;
            }
            let any_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == *param && !c.is_dead());
            if any_live {
                continue;
            }
            // Fallback: the def-use builder doesn't track variable
            // references inside ``[expr {...}]`` command
            // substitutions or arbitrary nested ``[cmd ...]``
            // bodies that don't lower into a structured IR.
            // Mirror the Python ``infer_param_traits`` shallow
            // pass's ``$param`` text scan: if the body source
            // contains a ``$param`` / ``${param}`` reference
            // anywhere, treat the parameter as used and skip
            // W214.  Saves the W214 over-emit on ``proc f {x}
            // { return [expr {$x + 1}] }``-style bodies until
            // the full ``infer_param_traits`` port lands.
            if let Some(body_source) = ir_proc.body_source.as_deref() {
                if body_references_param(body_source, param) {
                    continue;
                }
            }
            unused.push(param.clone());
        }
        if unused.is_empty() {
            return;
        }
        // Dispatch-protocol suppression: when ≥3 peer procs in this
        // namespace share this proc's leading-param signature AND an
        // arity-compatible variable-command dispatcher exists, the leading
        // params are an external contract, not genuinely unused.  Mirrors
        // `_dispatch_protocol_signatures` + its W214 filter.  Computed only
        // when there is something to report.
        let ns = namespace_of(&ir_proc.qualified_name);
        let leading: Vec<String> = ir_proc
            .params
            .iter()
            .take_while(|p| *p != "args")
            .cloned()
            .collect();
        let protocol_params: HashSet<String> = if !leading.is_empty()
            && self
                .dispatch_protocol_signatures()
                .contains(&(ns, leading.clone()))
        {
            leading.into_iter().collect()
        } else {
            HashSet::new()
        };
        for param in unused {
            if protocol_params.contains(&param) {
                continue;
            }
            let message = format!(
                "Parameter '{param}' of proc '{name}' is unused",
                name = ir_proc.qualified_name,
            );
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W214".to_string(),
                span: ir_proc.span,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// Identify `(namespace, leading-param-list)` pairs that look like a
    /// **dispatch protocol** — ≥3 peer procs in the same namespace sharing a
    /// leading-param signature dictated by an arity-compatible
    /// variable-command dispatcher.  Mirrors `_dispatch_protocol_signatures`
    /// in `_diag_var_lifecycle.py`.
    fn dispatch_protocol_signatures(&self) -> HashSet<(String, Vec<String>)> {
        use std::collections::HashMap;
        // Group user procs by (namespace, leading-param-tuple stopping at `args`).
        let mut groups: HashMap<(String, Vec<String>), usize> = HashMap::new();
        for (qname, pdef) in &self.result.all_procs {
            let leading: Vec<String> = pdef
                .params
                .iter()
                .take_while(|p| p.name != "args")
                .map(|p| p.name.clone())
                .collect();
            if leading.is_empty() {
                continue;
            }
            *groups.entry((namespace_of(qname), leading)).or_insert(0) += 1;
        }
        let peer_protos: HashSet<(String, Vec<String>)> = groups
            .into_iter()
            .filter(|(_, n)| *n >= 3)
            .map(|(k, _)| k)
            .collect();
        if peer_protos.is_empty() {
            return HashSet::new();
        }
        // Dispatcher evidence: map each dispatcher namespace → the argument
        // counts observed at its variable-command sites.
        let mut dispatcher_ns_argc: HashMap<String, HashSet<usize>> = HashMap::new();
        for site in &self.var_command_sites {
            let off = site.cmd_span.start();
            let dns = self
                .result
                .all_procs
                .iter()
                .find(|(_, p)| p.body_span.start() <= off && off <= p.body_span.end())
                .map_or_else(|| "::".to_string(), |(q, _)| namespace_of(q));
            dispatcher_ns_argc.entry(dns).or_default().insert(site.argc);
        }
        peer_protos
            .into_iter()
            .filter(|(ns_key, params)| {
                let min_argc = params.len();
                dispatcher_ns_argc.iter().any(|(dns, argcs)| {
                    (dns == ns_key || dns.starts_with(&format!("{ns_key}::")))
                        && argcs.iter().any(|&a| a >= min_argc)
                })
            })
            .collect()
    }

    /// W210 + W213 — read-before-set / unset on possibly-undefined.
    ///
    /// Mirrors `_emit_read_before_set_diagnostics` in
    /// `_diag_var_lifecycle.py:159-224`.  Walks every
    /// version-0 chain (`DefKind::Parameter`) in `fu.def_use`
    /// — those are the synthetic defs the def-use builder
    /// emits when a variable is used without a preceding def.
    ///
    /// Distinguishes real proc parameters from synthetic RBS
    /// reads via `ir_proc.params`.  Only emits inside procedures
    /// (i.e. when `ir_proc` is `Some`) — top-level RBS would
    /// need the `globals_written_by_procs` filter Python uses
    /// (deferred to a later strip).
    ///
    /// Per use site:
    ///
    /// - **Phi-incoming uses** are skipped — they sit at block
    ///   boundaries and don't anchor on a real statement.
    /// - **`unset` without `-nocomplain`** emits W213 (the more
    ///   specific code) instead of W210.  W213 message tells
    ///   the user to add `-nocomplain` rather than initialise
    ///   the variable.
    /// - **`safe_on_uninit` calls** that initialise the variable
    ///   themselves (it's in their `defs`) are skipped —
    ///   commands like `lappend` / `incr` / `dict set` safely
    ///   initialise an uninitialised variable.
    /// - Everything else emits W210 with the canonical
    ///   "read before set" message + optional "did you mean…?"
    ///   suggestion.
    fn emit_read_before_set_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: Option<&crate::ir::Procedure>,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        extra_known_defined: &HashSet<String>,
        supp: &UndefSuppression,
    ) {
        use crate::def_use::{DefKind, UseKind};
        use crate::ir::Statement;
        use std::fmt::Write as _;

        // **C41e3 follow-up.** Top-level RBS now uses the
        // ``extra_known_defined`` set (computed from
        // ``globals_written_by_procs``) to suppress W210 on
        // globals that helper procs write.  Inside procs the
        // set is empty, matching Python's per-call argument.
        let params_owned: HashSet<&str> = match ir_proc {
            Some(p) => p.params.iter().map(String::as_str).collect(),
            None => HashSet::new(),
        };
        let params = &params_owned;

        // SYNC-MAY31-3: collect `[info exists X]` / `[array exists X]`
        // guards: `(var, guard_block)` where reads of `var` in any
        // block dominated by `guard_block` are guarded (X is known to
        // exist there).  Positive guards the true arm; `![info exists
        // X]` guards the false arm.
        let exists_guards = collect_existence_guards(fu);

        for chain in fu.def_use.chains.values() {
            if chain.definition.kind != DefKind::Parameter {
                continue;
            }
            let (var, _version) = &chain.key;
            if params.contains(var.as_str()) {
                continue;
            }
            if scope_aliases.contains(var) {
                continue;
            }
            if extra_known_defined.contains(var) {
                continue;
            }
            // `dict with`/`dict update` unpacking + qualified-`variable`
            // alias tails suppress version-0 reads of the unpacked / aliased
            // names (the `puts $a` inside `dict with d {…}` is not RBS).
            // Interproc constant propagation resolves an empty caller dict to
            // CONST("") (keys = ∅, not unknown), so the blanket variant fires
            // on a genuine missing-key read while still suppressing an
            // unknown-shape (mixed-caller / no-caller) dict.
            if supp.suppresses(var) {
                continue;
            }
            for use_site in &chain.uses {
                if matches!(use_site.kind, UseKind::PhiIncoming) {
                    continue;
                }
                let Some(block) = fu.cfg.blocks.get(&use_site.block) else {
                    continue;
                };
                let (span, stmt_opt): (tcl_lexer::Span, Option<&Statement>) =
                    if use_site.statement_index == -1 {
                        let Some(span) = block
                            .terminator
                            .as_ref()
                            .and_then(crate::cfg::Terminator::span)
                        else {
                            continue;
                        };
                        (span, None)
                    } else {
                        let Ok(idx) = usize::try_from(use_site.statement_index) else {
                            continue;
                        };
                        let Some(stmt) = block.statements.get(idx) else {
                            continue;
                        };
                        (stmt.span(), Some(stmt))
                    };
                if span.is_empty() {
                    continue;
                }
                // SYNC-MAY31-3: skip the existence-query word itself and
                // reads narrowed by an enclosing `[info exists X]` guard.
                if existence_exempt(stmt_opt, var, &exists_guards, &fu.ssa, &use_site.block) {
                    continue;
                }
                // ``unset`` without ``-nocomplain`` → W213.
                if let Some(Statement::Call { command, args, .. }) = stmt_opt {
                    if command == "unset" && !args.iter().any(|a| a == "-nocomplain") {
                        let message = format!(
                            "Variable '{var}' may not exist; \
                             use 'unset -nocomplain' to suppress the error",
                        );
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W213".to_string(),
                            span,
                            message,
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                        continue;
                    }
                }
                // A use site that itself safely initialises the variable
                // (`safe_on_uninit` calls like `lappend`/`dict set`, or an
                // `incr` of its own target) is not read-before-set.
                if use_site_safe_initialises(stmt_opt, var) {
                    continue;
                }
                let mut message = format!("Variable '{var}' is read before it is set");
                if let Some(similar) = find_case_mismatch(var, defined_vars) {
                    let _ = write!(message, "; did you mean '{similar}'?");
                }
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W210".to_string(),
                    span,
                    message,
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// W210 on `return $v` reads where `v`'s reaching version can be
    /// undefined on some executable path (phi-from-undef / `unset`-killed).
    /// Companion to [`Self::emit_read_before_set_diagnostics`]; see its
    /// trailing call site for why the def-use-chain pass cannot catch
    /// these (return values are terminator reads, not recorded uses).
    #[allow(clippy::too_many_arguments)]
    fn emit_return_phi_undef_w210(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        params: &HashSet<&str>,
        exists_guards: &[(String, String)],
        scope_aliases: &HashSet<String>,
        extra_known_defined: &HashSet<String>,
        defined_vars: &HashSet<String>,
        considered: &HashSet<String>,
        supp: &UndefSuppression,
    ) {
        use crate::var_refs::{VarReferenceScanner, VarScanOptions};
        use std::fmt::Write as _;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };

        let (phi_def, killed) = build_phi_undef_index(&fu.ssa, considered);

        let mut scanner = VarReferenceScanner::new(VarScanOptions {
            include_var_read_roles: false,
            recurse_cmd_substitutions: true,
        });

        let mut reported: HashSet<String> = HashSet::new();
        // Deterministic block order for stable diagnostics.
        let mut block_names: Vec<&String> = considered.iter().collect();
        block_names.sort();

        for bn in block_names {
            let Some(cfg_block) = fu.cfg.blocks.get(bn) else {
                continue;
            };
            let Some(crate::cfg::Terminator::Return { value, expr, .. }) = &cfg_block.terminator
            else {
                continue;
            };
            let Some(span) = cfg_block
                .terminator
                .as_ref()
                .and_then(crate::cfg::Terminator::span)
            else {
                continue;
            };
            if span.is_empty() {
                continue;
            }
            let Some(ssa_block) = fu.ssa.blocks.get(bn) else {
                continue;
            };

            // Collect the variable names read by the return value (word
            // substitutions + nested `[...]`) and any parsed expr.
            let mut reads: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            if let Some(v) = value {
                reads.extend(scanner.scan_script(v, registry));
            }
            if let Some(e) = expr {
                reads.extend(crate::var_refs::vars_in_expr(e));
            }

            for name in reads {
                if reported.contains(&name) {
                    continue;
                }
                let ver = ssa_block.exit_versions.get(&name).copied().unwrap_or(0);
                // Version-0 return reads are now recorded in def_use, so the
                // version-0 (`DefKind::Parameter`) emitter handles them with
                // the full suppression set — this pass only covers the
                // phi-from-undef / `unset`-killed (version > 0) cases, which
                // def-use can't express.  Skipping ver 0 avoids double-firing.
                if ver == 0 {
                    continue;
                }
                let mut seen = HashSet::new();
                if !phi_can_undef(
                    &name,
                    ver,
                    &phi_def,
                    &killed,
                    considered,
                    exists_guards,
                    &fu.ssa,
                    &mut seen,
                ) {
                    continue;
                }
                if params.contains(name.as_str())
                    || scope_aliases.contains(&name)
                    || extra_known_defined.contains(&name)
                    || is_implicit_var(&name)
                    || name.contains("::")
                    || supp.suppresses(&name)
                {
                    continue;
                }
                // A dominating existence guard proves the var exists here.
                if exists_guards
                    .iter()
                    .any(|(gv, gblk)| *gv == name && block_dominated_by(&fu.ssa, bn, gblk))
                {
                    continue;
                }
                reported.insert(name.clone());
                let mut message = format!("Variable '{name}' is read before it is set");
                if let Some(similar) = find_case_mismatch(&name, defined_vars) {
                    let _ = write!(message, "; did you mean '{similar}'?");
                }
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W210".to_string(),
                    span,
                    message,
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// **W210 (provably-unset regexp / scan output).** A `regexp` / `scan`
    /// with literal pattern + input that can be statically proven not to
    /// match leaves its output variables unset, so a later read of one is a
    /// real read-before-set.  Handles both the top-level call form and the
    /// call embedded in an `if` / `while` condition (firing only on the
    /// no-match branch).  Mirrors the `provably_unset` post-pass in
    /// `compiler/core_analyses.py::_read_before_set`.
    fn emit_provably_unset_w210(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        considered: &HashSet<String>,
        defined_vars: &HashSet<String>,
    ) {
        use crate::ir::Statement;
        use std::fmt::Write as _;

        // var name -> (def_block, def_stmt_idx); idx == -1 means "from the
        // start of the block" (the embedded-condition no-match target).
        let mut provably_unset: std::collections::HashMap<String, (String, i32)> =
            std::collections::HashMap::new();

        for bn in considered {
            let Some(block) = fu.cfg.blocks.get(bn) else {
                continue;
            };
            // Top-level regexp / scan calls.
            for (idx, stmt) in block.statements.iter().enumerate() {
                let Statement::Call {
                    command,
                    canonical_command,
                    args,
                    defs,
                    ..
                } = stmt
                else {
                    continue;
                };
                let canon = canonical_command.as_deref().unwrap_or(command);
                let is_regexp = canon == "::regexp" || command == "regexp";
                let is_scan = canon == "::scan" || command == "scan";
                if (!is_regexp && !is_scan) || defs.is_empty() {
                    continue;
                }
                if let Some(no_match) = regexp_scan_no_match(is_regexp, args) {
                    if no_match {
                        for d in defs {
                            provably_unset.entry(d.clone()).or_insert_with(|| {
                                (bn.clone(), i32::try_from(idx).unwrap_or(i32::MAX))
                            });
                        }
                    }
                }
            }
            // regexp / scan embedded in the branch condition.
            if let Some(crate::cfg::Terminator::Branch {
                condition,
                true_target,
                false_target,
                ..
            }) = &block.terminator
            {
                Self::collect_embedded_provably_unset(
                    condition,
                    true_target,
                    false_target,
                    &mut provably_unset,
                );
            }
        }

        if provably_unset.is_empty() {
            return;
        }

        // Fire on every executable use after the def (same block) or in a
        // block dominated by the def block.
        let mut reported: HashSet<String> = HashSet::new();
        let mut block_names: Vec<&String> = considered.iter().collect();
        block_names.sort();
        for bn in block_names {
            let Some(ssa_block) = fu.ssa.blocks.get(bn) else {
                continue;
            };
            for (idx, s) in ssa_block.statements.iter().enumerate() {
                for name in s.uses.keys() {
                    if reported.contains(name) {
                        continue;
                    }
                    let Some((def_block, def_idx)) = provably_unset.get(name) else {
                        continue;
                    };
                    let in_def_block_after =
                        bn == def_block && i32::try_from(idx).unwrap_or(i32::MAX) > *def_idx;
                    let dominated = bn != def_block && block_dominated_by(&fu.ssa, bn, def_block);
                    if !(in_def_block_after || dominated) {
                        continue;
                    }
                    let span = match fu.cfg.blocks.get(bn).and_then(|b| b.statements.get(idx)) {
                        Some(st) if !st.span().is_empty() => st.span(),
                        _ => continue,
                    };
                    reported.insert(name.clone());
                    let mut message = format!("Variable '{name}' is read before it is set");
                    if let Some(similar) = find_case_mismatch(name, defined_vars) {
                        let _ = write!(message, "; did you mean '{similar}'?");
                    }
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: "W210".to_string(),
                        span,
                        message,
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                }
            }
        }
    }

    /// Walk a branch `condition` for an embedded `[regexp …]` / `[scan …]`
    /// command substitution that provably can't match, recording its output
    /// variables as provably-unset on the no-match branch target (only when
    /// the condition is exactly `[cmd]` → false target, or `![cmd]` → true
    /// target; more complex shapes are skipped).
    fn collect_embedded_provably_unset(
        condition: &ExprNode,
        true_target: &str,
        false_target: &str,
        provably_unset: &mut std::collections::HashMap<String, (String, i32)>,
    ) {
        let (cmd_node, no_match_target) = match condition {
            ExprNode::Command { .. } => (condition, false_target),
            ExprNode::Unary {
                op: UnaryOp::Not | UnaryOp::WordNot,
                operand,
            } if matches!(operand.as_ref(), ExprNode::Command { .. }) => {
                (operand.as_ref(), true_target)
            }
            _ => return,
        };
        let ExprNode::Command { text, .. } = cmd_node else {
            return;
        };
        // Strip the surrounding `[` … `]` and segment the interior.
        let inner = text
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(text);
        let segs = crate::segmenter::segment_commands(inner);
        let Some(seg) = segs.first() else {
            return;
        };
        let Some(cmd) = seg.texts.first() else {
            return;
        };
        let bare = cmd
            .trim_start_matches(':')
            .rsplit("::")
            .next()
            .unwrap_or(cmd);
        let is_regexp = bare == "regexp";
        let is_scan = bare == "scan";
        if !is_regexp && !is_scan {
            return;
        }
        let args: Vec<String> = seg.texts[1..].to_vec();
        let pos = skip_options(&args, if is_regexp { &["-start"] } else { &[] });
        if pos + 2 > args.len() {
            return;
        }
        let out_vars = &args[(pos + 2).min(args.len())..];
        if out_vars.is_empty() {
            return;
        }
        if regexp_scan_no_match(is_regexp, &args) != Some(true) {
            return;
        }
        for v in out_vars {
            let name = crate::naming::normalise_var_name(v);
            if !name.is_empty() {
                provably_unset
                    .entry(name.to_string())
                    .or_insert_with(|| (no_match_target.to_string(), -1));
            }
        }
    }

    /// I230 / I231 — constant branch / switch-arm condition.
    ///
    /// Mirrors `_emit_constant_branch_diagnostics` in
    /// `core/analysis/_analyser/_diag_branches.py`.  For every
    /// branch SCCP folded to a constant, when the *not-taken*
    /// target is also unreachable (i.e. SCCP confirmed only one
    /// path is feasible), emit an Info-level diagnostic so the
    /// LSP can highlight the dead arm.
    ///
    /// Code selection follows the Python rules:
    /// - Block name starts with ``switch_`` → I231 (switch-arm).
    /// - Block name starts with ``if_`` → I230 (constant if).
    /// - Otherwise → I230 with the generic
    ///   ``"Branch condition '...' is constant"`` message.
    ///
    /// Severity is mapped to ``Hint`` because the Rust
    /// [`Severity`] enum has no ``Info`` variant — ``Hint`` is
    /// the closest non-actionable level.
    fn emit_constant_branch_diagnostics(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        for branch in &fu.sccp.constant_branches {
            // The Python check is "not_taken_target in
            // unreachable_blocks".  Rust SCCP exposes
            // ``executable_blocks`` (the complement); a block
            // is unreachable iff it's in ``cfg.blocks`` but
            // NOT in ``executable_blocks``.
            if fu.sccp.executable_blocks.contains(&branch.not_taken_target) {
                continue;
            }
            // Locate the branch's terminator span.
            let Some(block) = fu.cfg.blocks.get(&branch.block) else {
                continue;
            };
            let Some(crate::cfg::Terminator::Branch {
                span: Some(span), ..
            }) = &block.terminator
            else {
                continue;
            };
            let span = *span;

            let names = [
                branch.block.as_str(),
                branch.taken_target.as_str(),
                branch.not_taken_target.as_str(),
            ];
            let is_switch = names.iter().any(|n| n.starts_with("switch_"));
            let is_if = names.iter().any(|n| n.starts_with("if_"));

            let (code, message) = if is_switch {
                let code = "I231";
                let msg = if branch.value {
                    format!(
                        "Switch condition '{}' is always true here; \
                         subsequent switch arms are unreachable",
                        branch.condition,
                    )
                } else {
                    format!(
                        "Switch arm condition '{}' is always false; \
                         this arm is unreachable",
                        branch.condition,
                    )
                };
                (code, msg)
            } else if is_if {
                let msg = if branch.value {
                    format!(
                        "Condition '{}' is always true; \
                         the alternate branch is unreachable",
                        branch.condition,
                    )
                } else {
                    format!(
                        "Condition '{}' is always false; \
                         the alternate branch is unreachable",
                        branch.condition,
                    )
                };
                ("I230", msg)
            } else {
                let msg = format!(
                    "Branch condition '{}' is constant; one branch is unreachable",
                    branch.condition,
                );
                ("I230", msg)
            };

            self.result.diagnostics.push(super::types::Diagnostic {
                code: code.to_string(),
                span,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// I230 — fold `[info exists X]` / `[array exists X]` conditions.
    ///
    /// SYNC-MAY31-3.  SCCP can't fold these (the predicate lowers to an
    /// opaque `ExprNode::Command`, and SCCP has no parameter/existence
    /// facts), so the fold is computed by
    /// [`crate::sccp::existence_constant_branches`] using
    /// `ir_proc.params` — the same helper whose result
    /// `FunctionUnit::build` appends to `sccp.constant_branches` for the
    /// optimiser's O101 fold / DCE.  Emitting the I230 here (rather than
    /// via [`Self::emit_constant_branch_diagnostics`]) is deliberate:
    /// that emitter gates on the not-taken arm being unreachable in
    /// `executable_blocks`, which these post-pass folds don't update, so
    /// it skips them and there is no double emission.
    fn emit_existence_constant_branch_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: Option<&crate::ir::Procedure>,
    ) {
        let params: HashSet<&str> = match ir_proc {
            Some(p) => p.params.iter().map(String::as_str).collect(),
            None => HashSet::new(),
        };
        for cb in crate::sccp::existence_constant_branches(&fu.cfg, &params) {
            let Some(span) = cb.span else { continue };
            let message = if cb.value {
                format!(
                    "Condition '{}' is always true; the alternate branch is unreachable",
                    cb.condition,
                )
            } else {
                format!(
                    "Condition '{}' is always false; the alternate branch is unreachable",
                    cb.condition,
                )
            };
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "I230".to_string(),
                span,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// W126 — channel-argument validation.
    ///
    /// Mirrors `_emit_channel_diagnostics` in
    /// `core/analysis/_analyser/_diag_channel.py`.  Walks every
    /// SSA-annotated `Call` statement for commands that declare
    /// `ArgRole::Channel` arguments; for each channel-position
    /// argument, checks the SSA type lattice to determine whether
    /// the value is genuinely a channel.  Two failure modes:
    ///
    /// - **`$var` reference** with `TypeKind::Known` and a non-
    ///   `TclType::Channel` type — emits "passed as channel … has
    ///   type X, not CHANNEL".
    /// - **String literal** that isn't `stdin` / `stdout` /
    ///   `stderr` and contains no substitutions — emits
    ///   "String literal 'X' used as channel argument".
    ///
    /// The standard channels (`stdin`, `stdout`, `stderr`) are
    /// always accepted.  Unknown / overdefined types skip the
    /// check (could be anything).
    fn emit_channel_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::ir::Statement;
        use crate::types::TypeKind;
        use tcl_registry::ArgRole;

        const STANDARD_CHANNELS: &[&str] = &["stdout", "stderr", "stdin"];

        for block in fu.ssa.blocks.values() {
            for ssa_stmt in &block.statements {
                let Statement::Call {
                    command,
                    args,
                    span,
                    ..
                } = &ssa_stmt.statement
                else {
                    continue;
                };
                let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                let channel_indices =
                    registry.arg_indices_for_role(command, &arg_refs, ArgRole::Channel);
                if channel_indices.is_empty() {
                    continue;
                }
                for idx in channel_indices {
                    if idx >= args.len() {
                        continue;
                    }
                    let arg_text = &args[idx];
                    // Extract bare var name from ``$var`` / ``${var}``.
                    let var_name: Option<&str> =
                        if arg_text.starts_with("${") && arg_text.ends_with('}') {
                            Some(&arg_text[2..arg_text.len() - 1])
                        } else if let Some(rest) = arg_text.strip_prefix('$') {
                            Some(rest)
                        } else {
                            None
                        };

                    if let Some(name) = var_name {
                        let Some(&version) = ssa_stmt.uses.get(name) else {
                            continue;
                        };
                        let key: crate::ssa::ValueKey = (name.to_string(), version);
                        let Some(var_type) = fu.types.get(&key) else {
                            continue;
                        };
                        if var_type.kind != TypeKind::Known {
                            continue;
                        }
                        let Some(tcl_type) = var_type.tcl_type else {
                            continue;
                        };
                        if matches!(tcl_type, tcl_registry::TclType::Channel) {
                            continue;
                        }
                        let type_label = format!("{tcl_type:?}").to_uppercase();
                        let message = format!(
                            "Variable '${name}' passed as channel to '{command}' \
                             has type {type_label}, not CHANNEL.",
                        );
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W126".to_string(),
                            span: *span,
                            message,
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                    } else {
                        // Literal — strip surrounding braces / quotes.
                        let literal = arg_text
                            .trim_matches('"')
                            .trim_start_matches('{')
                            .trim_end_matches('}');
                        if STANDARD_CHANNELS.contains(&literal) {
                            continue;
                        }
                        // Only warn for clearly-not-substituted literals.
                        if arg_text.contains('$') || arg_text.contains('[') {
                            continue;
                        }
                        let message = format!(
                            "String literal '{literal}' used as channel argument to \
                             '{command}' — expected a channel from open/socket/chan create.",
                        );
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W126".to_string(),
                            span: *span,
                            message,
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                    }
                }
            }
        }
    }

    /// W124 — invalid IP address literal.
    ///
    /// Mirrors `_emit_invalid_ip_diagnostics` in
    /// `core/analysis/_analyser/_diag_ip.py`.  Walks every
    /// SSA-tracked constant string in the function's SCCP
    /// values; regex-searches for IPv4 dotted-quad and IPv6
    /// candidates and validates each.
    ///
    /// **Validation:**
    /// - **IPv4** — each octet must be 0..255; leading-zero
    ///   octets emit a Warning (interpreted as octal in some
    ///   contexts); over-255 octets emit an Error.  Patterns
    ///   preceded by ``/`` (CIDR / version-number context) are
    ///   skipped.
    /// - **IPv6** — parsed via [`std::net::Ipv6Addr`]; failure
    ///   emits an Error.
    ///
    /// Diagnostic anchors at the SSA def site (the assignment
    /// statement's span); seen-offsets dedup avoids duplicate
    /// emissions when multiple SSA versions share a def.
    /// **W233.** Division / modulo by a provably-zero divisor — raises
    /// "divide by zero" at runtime.  Walks every `[expr …]` AST reachable in
    /// the function (`AssignExpr` statements, `if`/`while` branch conditions,
    /// and `return [expr …]` values) over SCCP-executable blocks; a literal
    /// `0` divisor or a variable whose SCCP value is a constant zero fires.
    /// Mirrors the `find_divide_by_zero` arm of
    /// `analyser/_analyser/_diag_interval_bounds.py`.
    fn emit_w233_divide_by_zero(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        use crate::cfg::Terminator;
        use crate::ir::Statement;

        let considered: HashSet<&str> = if fu.sccp.executable_blocks.is_empty() {
            fu.ssa.blocks.keys().map(String::as_str).collect()
        } else {
            fu.sccp
                .executable_blocks
                .iter()
                .map(String::as_str)
                .collect()
        };
        let mut hits: Vec<(tcl_lexer::Span, BinOp)> = Vec::new();
        for bn in &considered {
            let Some(block) = fu.cfg.blocks.get(*bn) else {
                continue;
            };
            let ssa_block = fu.ssa.blocks.get(*bn);
            for (idx, stmt) in block.statements.iter().enumerate() {
                if let Statement::AssignExpr { expr, span, .. } = stmt {
                    let versions = ssa_block
                        .and_then(|sb| sb.statements.get(idx))
                        .map(|s| &s.uses);
                    if let Some(versions) = versions {
                        if let Some(op) = find_divide_by_zero(expr, versions, &fu.sccp.values) {
                            hits.push((*span, op));
                        }
                    }
                }
            }
            let exit = ssa_block.map(|sb| &sb.exit_versions);
            let Some(exit) = exit else { continue };
            match &block.terminator {
                Some(Terminator::Return {
                    expr: Some(e),
                    span: Some(sp),
                    ..
                }) => {
                    if let Some(op) = find_divide_by_zero(e, exit, &fu.sccp.values) {
                        hits.push((*sp, op));
                    }
                }
                Some(Terminator::Branch {
                    condition,
                    span: Some(sp),
                    ..
                }) => {
                    if let Some(op) = find_divide_by_zero(condition, exit, &fu.sccp.values) {
                        hits.push((*sp, op));
                    }
                }
                _ => {}
            }
        }
        for (span, op) in hits {
            if span.is_empty() {
                continue;
            }
            let verb = if op == BinOp::Div {
                "Division"
            } else {
                "Modulo"
            };
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W233".to_string(),
                span,
                message: format!(
                    "{verb} by a provably-zero divisor — raises 'divide by zero' at runtime."
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W230 / W231 / W232 (dynamic).** Interval-driven out-of-range index
    /// detection for a `$var` index whose [`crate::intervals`] range — guard-
    /// narrowed at the use site — proves the access is wholly out of range
    /// against a statically-established container length.  Complements the
    /// syntactic bounds checks (literal index + literal container only); the
    /// two never double-fire because the syntactic checks back off on any
    /// `$var` index.  Restricted to SCCP-reachable blocks so a dynamic index
    /// in dead code does not warn.  Mirrors
    /// `_diag_interval_bounds.py::_emit_interval_bounds_diagnostics`.
    fn emit_interval_bounds_diagnostics(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        let executable: HashSet<String> = if fu.sccp.executable_blocks.is_empty() {
            fu.ssa.blocks.keys().cloned().collect()
        } else {
            fu.sccp.executable_blocks.iter().cloned().collect()
        };
        let findings = crate::interval_bounds::find_interval_bounds(
            &fu.cfg,
            &fu.ssa,
            &fu.sccp.values,
            &executable,
        );
        for f in findings {
            if f.span.is_empty() {
                continue;
            }
            let bound = if f.reason == "negative" {
                "below 0".to_string()
            } else {
                format!("past the end ({})", f.length)
            };
            let rng = if f.reason == "negative" {
                "negative".to_string()
            } else if f.index_interval.lo == f.index_interval.hi {
                format!("is {}", f.index_interval.lo.map_or(0, |l| l))
            } else {
                let lo = f
                    .index_interval
                    .lo
                    .map_or("-inf".to_string(), |l| l.to_string());
                let hi = f
                    .index_interval
                    .hi
                    .map_or("+inf".to_string(), |h| h.to_string());
                format!("is in [{lo}, {hi}]")
            };
            let outcome = if f.code == "W231" {
                "raises 'index out of range' at runtime"
            } else {
                "silently returns the empty string"
            };
            self.result.diagnostics.push(super::types::Diagnostic {
                code: f.code,
                span: f.span,
                message: format!(
                    "{}: index ${} {rng}, {bound} \u{2014} {outcome}.",
                    f.command, f.index_var
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    fn emit_invalid_ip_diagnostics(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        use crate::analyses::{ConstValue, LatticeValue};
        use std::net::Ipv6Addr;
        use std::str::FromStr;

        let mut seen_offsets: HashSet<u32> = HashSet::new();
        for (key, lv) in &fu.sccp.values {
            let Some(text) = (match lv {
                LatticeValue::Const(ConstValue::String(s)) => Some(s.as_str()),
                _ => None,
            }) else {
                continue;
            };

            // ---- IPv4 candidates ----
            for quad in find_dotted_quads(text, 4) {
                let bytes = text.as_bytes();
                if quad.start > 0 && bytes[quad.start - 1] == b'/' {
                    continue;
                }
                // Skip OID-like patterns: the matched quad is a slice of a
                // longer dotted-digit chain (LDAP/SNMP OIDs like
                // ``1.3.6.1.4.1.4203.1.11.3``).  Detect a ``digit.<quad>``
                // before or a ``<quad>.digit`` after.
                let before_dot_digit = quad.start >= 2
                    && bytes[quad.start - 1] == b'.'
                    && bytes[quad.start - 2].is_ascii_digit();
                let after_dot_digit = quad.end + 1 < bytes.len()
                    && bytes[quad.end] == b'.'
                    && bytes[quad.end + 1].is_ascii_digit();
                if before_dot_digit || after_dot_digit {
                    continue;
                }
                let octets = quad.octets;
                let mut diag: Option<(String, Severity)> = None;
                for (i, octet) in octets.iter().enumerate() {
                    let v: u32 = octet.parse().unwrap_or(0);
                    if v > 255 {
                        diag = Some((
                            format!(
                                "IPv4 octet {} ({}) exceeds 255 — this is not a valid IP address.",
                                i + 1,
                                octet,
                            ),
                            Severity::Error,
                        ));
                        break;
                    }
                    if octet.len() > 1
                        && octet.starts_with('0')
                        && octet.bytes().all(|b| (b'0'..=b'7').contains(&b))
                    {
                        diag = Some((
                            format!(
                                "IPv4 octet {} ({}) has a leading zero — may be interpreted as octal in some contexts.",
                                i + 1,
                                octet,
                            ),
                            Severity::Warning,
                        ));
                        break;
                    }
                }
                if let Some((msg, sev)) = diag {
                    self.emit_ip_diag_at_def(fu, key, &msg, sev, &mut seen_offsets);
                    break;
                }
            }

            // ---- IPv6 candidates ----
            for candidate in find_ipv6_candidates(text) {
                if Ipv6Addr::from_str(candidate).is_err() {
                    let msg = format!("Invalid IPv6 address '{candidate}'.");
                    self.emit_ip_diag_at_def(fu, key, &msg, Severity::Error, &mut seen_offsets);
                    break;
                }
            }
        }
    }

    /// Helper for [`Self::emit_invalid_ip_diagnostics`].
    fn emit_ip_diag_at_def(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        key: &crate::ssa::ValueKey,
        message: &str,
        severity: Severity,
        seen_offsets: &mut HashSet<u32>,
    ) {
        let (var_name, version) = key;
        let Some(chain) = fu.def_use.chain_for(var_name, *version) else {
            return;
        };
        let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
            return;
        };
        let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
            return;
        };
        let Some(stmt) = block.statements.get(idx) else {
            return;
        };
        let span = stmt.span();
        if span.is_empty() {
            return;
        }
        if !seen_offsets.insert(span.start()) {
            return;
        }
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W124".to_string(),
            span,
            message: message.to_string(),
            severity,
            fixes: Vec::new(),
        });
    }

    /// W123 — unknown / unresolved command head.
    ///
    /// Mirrors `_emit_unresolved_command_diagnostics` in
    /// `core/analysis/_analyser/_diag_commands.py:39-186`.
    /// Walks every command invocation recorded during the
    /// analyser walk and emits W123 ("Unknown command 'X'")
    /// when no matching definition is in scope.
    ///
    /// Resolution paths checked in order — first match
    /// suppresses W123:
    ///
    /// - `cmd_name in registry_names` (built-in command).
    /// - `cmd_name` contains `::` (qualified — defer to
    ///   per-namespace logic, conservative skip).
    /// - `cmd_name` starts with `$` / `[` (interpolated /
    ///   substituted head — handled by W307 / W308).
    /// - User-defined proc tail or absolute name.
    /// - User-defined class tail or absolute name.
    /// - Command alias tail.
    /// - Ensemble namespace tail.
    ///
    /// Idempotency: ``self.unresolved_commands_emitted`` guards
    /// against double-emission when ``analyse`` is called twice
    /// or the chunked entry runs both passes.
    ///
    /// **Deferred** (Python parity gaps documented in the
    /// commit body): ``has_dynamic_providers`` early-return;
    /// the CONSTSET-driven interpolation suppression for
    /// ``$``-bearing command names.
    // Long-running analyser pass with many sequential phases over the CompilationUnit; splitting requires threading shared local state.
    #[allow(clippy::too_many_lines)]
    pub fn emit_unresolved_command_diagnostics(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.unresolved_commands_emitted {
            return;
        }
        self.unresolved_commands_emitted = true;
        if self.disabled_diagnostics.contains("W123") {
            return;
        }

        // Conservative gate: if any ``package require`` was seen,
        // suppress W123 entirely.  The package may load arbitrary
        // commands at runtime that the analyser can't see.
        if !self.result.package_requires.is_empty() {
            return;
        }

        // **C41e3 follow-up.** When the document defines a
        // user-level ``unknown`` proc with a *dynamic* dispatch
        // shape — chains the original handler, case-folds,
        // uses pattern (glob / regexp) dispatch, calls
        // ``exec``, or calls ``auto_load`` — the analyser can't
        // statically prove which commands are resolvable, so
        // suppress W123 entirely.  For the *non-dynamic* shape
        // (only explicit ``dispatch_targets`` listed), W123
        // still fires below; the per-invocation loop checks
        // ``dispatch_targets`` membership and lets unrelated
        // commands surface their warnings.  Empty-stub
        // ``unknown`` (``proc unknown {cmd args} {}``) resolves
        // nothing so we never hit this gate.
        if let Some(info) = self.result.unknown_proc_info.as_ref() {
            let is_dynamic = info.chains_original
                || info.case_insensitive
                || info.has_pattern_dispatch
                || info.has_exec
                || info.has_auto_load;
            if is_dynamic {
                return;
            }
        }

        let registry_names: HashSet<String> =
            registry.command_names().map(str::to_string).collect();
        // **C41 follow-up.** Inline ``# tcl-lsp: stub NAME ...``
        // declarations contribute to the candidate set and the
        // suppression set so users who declared a stub for a
        // command don't get spurious W123s.  Mirrors the
        // ``stub_names`` set in
        // ``_diag_commands.py:_emit_unresolved_command_diagnostics``.
        let stub_names: HashSet<String> = super::utils::scan_stub_command_names(&self.source);
        let proc_tail_names: HashSet<String> = self
            .result
            .all_procs
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let class_tail_names: HashSet<String> = self
            .result
            .all_classes
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let alias_names: HashSet<String> = self
            .result
            .command_aliases
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let ensemble_cmds: HashSet<String> = self
            .ensemble_namespaces
            .iter()
            .filter_map(|ns| ns.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();

        // Build the candidate set for "did you mean…?"
        // suggestions.  Mirrors Python's `candidates` set in
        // `_diag_commands.py:87-106` — every name a real command
        // could resolve to (including unknown-proc dispatch
        // targets and inline-stub declarations).
        let mut candidates: Vec<String> = Vec::new();
        candidates.extend(registry_names.iter().cloned());
        candidates.extend(proc_tail_names.iter().cloned());
        candidates.extend(class_tail_names.iter().cloned());
        candidates.extend(alias_names.iter().cloned());
        candidates.extend(ensemble_cmds.iter().cloned());
        candidates.extend(stub_names.iter().cloned());
        if let Some(info) = self.result.unknown_proc_info.as_ref() {
            for t in &info.dispatch_targets {
                candidates.push(t.clone());
            }
        }

        // Pre-compute the deduplicated ``Vec<&str>`` over the
        // candidate set once, instead of rebuilding it per
        // unresolved invocation.  ``candidates`` may carry
        // duplicates because each contributor (registry / proc
        // tails / class tails / aliases / ensemble cmds /
        // stubs / unknown-proc dispatch_targets) is unioned
        // independently — dedupe via a ``HashSet`` filter
        // while preserving stable iteration order.
        let mut seen_candidate_strs: HashSet<&str> = HashSet::new();
        let candidate_strs: Vec<&str> = candidates
            .iter()
            .map(String::as_str)
            .filter(|candidate| seen_candidate_strs.insert(*candidate))
            .collect();

        // Drain so the iteration loop can mutate
        // ``self.result.diagnostics`` freely; restore at the end
        // (matches the snapshot/restore round-trip contract).
        let invocations = std::mem::take(&mut self.result.command_invocations);
        for inv in &invocations {
            let name = &inv.name;
            if registry_names.contains(name) {
                continue;
            }
            if name.contains("::") {
                continue;
            }
            if name.starts_with('$') || name.starts_with('[') {
                continue;
            }
            if proc_tail_names.contains(name) {
                continue;
            }
            if class_tail_names.contains(name) {
                continue;
            }
            if alias_names.contains(name) {
                continue;
            }
            if ensemble_cmds.contains(name) {
                continue;
            }
            if stub_names.contains(name) {
                continue;
            }
            if let Some(info) = self.result.unknown_proc_info.as_ref() {
                if info.dispatch_targets.contains(name) {
                    continue;
                }
            }
            // Absolute-form fallback — ``cmd`` may be defined as
            // ``::cmd`` in the global namespace.
            if self.result.all_procs.contains_key(&format!("::{name}")) {
                continue;
            }
            if self.result.all_classes.contains_key(&format!("::{name}")) {
                continue;
            }

            // **C41 follow-up.** "Did you mean…?" suggestion
            // via Levenshtein.  Mirrors the
            // ``suggest_similar(cmd_name, candidates,
            // max_suggestions=1, max_distance=2)`` call in
            // ``_diag_commands.py:166``.  ``candidate_strs`` was
            // deduplicated above so every name in it is unique;
            // copying the slice per invocation is cheap (Vec of
            // ``&str`` references).
            let suggestions =
                crate::text::suggest_similar(name, candidate_strs.iter().copied(), 1, 2);
            let mut message = format!("Unknown command '{name}'");
            let mut fixes: Vec<super::types::CodeFix> = Vec::new();
            if let Some(best) = suggestions.first() {
                use std::fmt::Write as _;
                let _ = write!(message, "; did you mean '{best}'?");
                fixes.push(super::types::CodeFix {
                    span: inv.range,
                    new_text: (*best).to_string(),
                    description: format!("Replace with '{best}'"),
                });
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W123".to_string(),
                span: inv.range,
                message,
                severity: Severity::Hint,
                fixes,
            });
        }
        self.result.command_invocations = invocations;
    }

    /// W120 — command used without a corresponding
    /// `package require`.
    ///
    /// Mirrors `_check_missing_package_require` in
    /// `lsp/features/diagnostics.py`.  For every command
    /// invocation whose registry spec carries a
    /// `required_package`, emit W120 (once per command name)
    /// unless that package is already imported (a
    /// `package require` / `package provide` in this file).
    /// Attaches a `CodeFix` that inserts
    /// `package require <pkg>` after the last existing
    /// `package require`, or at the top of the file.
    ///
    /// Gated off entirely when:
    /// * the dialect has no `package` command (iRules);
    /// * the file loads packages dynamically
    ///   (`has_dynamic_providers`) — the runtime set of
    ///   commands is then unknowable;
    /// * W120 is in `disabled_diagnostics`.
    pub fn emit_missing_package_require_diagnostics(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.disabled_diagnostics.contains("W120") {
            return;
        }
        // Dialects without a `package` command (e.g. iRules)
        // can't `package require`, so W120 never applies.
        if registry.get("package").is_none() {
            return;
        }
        // Dynamic providers ⇒ unknowable command set ⇒ no W120.
        if self.result.has_dynamic_providers {
            return;
        }

        // Packages already available in this file: every
        // `package require` name plus every `package provide`
        // name (a file that provides a package needn't require
        // it).
        let mut imported: HashSet<&str> = HashSet::new();
        for pr in &self.result.package_requires {
            imported.insert(pr.name.as_str());
        }
        for pp in &self.result.package_provides {
            imported.insert(pp.name.as_str());
        }

        // Insertion point for the code fix: just after the last
        // `package require` line, else the top of the file.
        let insert_offset = self.package_require_insert_offset();

        let mut seen: HashSet<String> = HashSet::new();
        let mut new_diags: Vec<super::types::Diagnostic> = Vec::new();
        for inv in &self.result.command_invocations {
            let Some(spec) = registry.get(&inv.name) else {
                continue;
            };
            let Some(pkg) = spec.required_package else {
                continue;
            };
            if imported.contains(pkg) {
                continue;
            }
            // Emit once per command name to avoid flooding.
            if !seen.insert(inv.name.clone()) {
                continue;
            }
            let fix = super::types::CodeFix {
                span: tcl_lexer::Span::new(insert_offset, insert_offset),
                new_text: format!("package require {pkg}\n"),
                description: format!("Add 'package require {pkg}'"),
            };
            new_diags.push(super::types::Diagnostic {
                code: "W120".to_string(),
                span: inv.range,
                message: format!("\"{}\" requires `package require {pkg}`", inv.name),
                severity: Severity::Warning,
                fixes: vec![fix],
            });
        }
        self.result.diagnostics.extend(new_diags);
    }

    /// Byte offset at which a `package require <pkg>` line
    /// should be inserted: just past the newline after the
    /// last existing `package require`, else `0` (top of
    /// file).
    fn package_require_insert_offset(&self) -> u32 {
        let Some(last) = self
            .result
            .package_requires
            .iter()
            .max_by_key(|p| p.range.end())
        else {
            return 0;
        };
        let bytes = self.source.as_bytes();
        let mut off = last.range.end() as usize;
        while off < bytes.len() && bytes[off] != b'\n' {
            off += 1;
        }
        if off < bytes.len() {
            off += 1; // past the newline
        }
        u32::try_from(off).unwrap_or(0)
    }

    /// W307 — non-literal command name (variable / command-sub
    /// used as command head).
    ///
    /// Mirrors the W307 half of `_emit_var_command_diagnostics`
    /// in `core/analysis/_analyser/_diag_var_command.py:22-294`.
    /// Walks every recorded site in [`Self::var_command_sites`]
    /// and emits W307 unless the variable's value is statically
    /// resolvable to a finite set of known command names.
    ///
    /// **Resolution paths** (mirrors Python; first match
    /// suppresses W307):
    ///
    /// - Aggregate every CONSTSET / CONST entry in `cu`'s SCCP
    ///   results for the variable name; if every value in the
    ///   set is a known command, proc, class, or class-tail name,
    ///   the command head is statically resolvable — suppress.
    ///
    /// **Known limitations.**  W308 (unknown method on object)
    /// is deferred to a follow-up — it needs the
    /// `class_hierarchy` / MRO port (the C41e0 architectural
    /// decision still pending).  Likewise the
    /// `_cmd_command_sites` (``[cmd] method``) suppression via
    /// return-type analysis is deferred — that path needs the
    /// IR-level type-lattice plumbing extended into the
    /// analyser, which is a larger change than fits this strip.
    /// In-method W307 suppression and dict-with /
    /// dict-update barrier-range suppression also defer.
    #[allow(clippy::too_many_lines)]
    // Long-running analyser pass with many sequential phases over the CompilationUnit; splitting requires threading shared local state.
    fn emit_var_command_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::types::TypeKind;
        use std::collections::HashMap;

        if self.var_command_sites.is_empty() && self.cmd_command_sites.is_empty() {
            return;
        }
        // Aggregate type-lattice knowledge per variable name
        // across every FunctionUnit.  For each var with a
        // ``TclType::Object`` lattice entry that has a
        // ``class_name``, record the class qualified name so
        // W308 can validate the method against the class
        // hierarchy.  Mirrors the ``all_typed_vars`` /
        // ``all_types`` aggregation in
        // ``_diag_var_command.py:49-67``.
        let mut all_object_types: HashMap<String, HashSet<String>> = HashMap::new();
        let collect_object_types =
            |types: &HashMap<crate::ssa::ValueKey, crate::types::TypeLattice>,
             out: &mut HashMap<String, HashSet<String>>| {
                for ((var_name, _ver), tl) in types {
                    if tl.kind != TypeKind::Known {
                        continue;
                    }
                    if !matches!(tl.tcl_type, Some(tcl_registry::TclType::Object)) {
                        continue;
                    }
                    let Some(class_name) = &tl.class_name else {
                        continue;
                    };
                    out.entry(var_name.clone())
                        .or_default()
                        .insert(class_name.clone());
                }
            };
        collect_object_types(&cu.top_level.types, &mut all_object_types);
        for fu in cu.procedures.values() {
            collect_object_types(&fu.types, &mut all_object_types);
        }
        // Harvest `set x [Cls new]` / `set x [Cls create name]` where `Cls` is
        // a known TclOO class: `x` then holds an Object of class `Cls`, so a
        // later `$x method` dispatch resolves through the W308 method check
        // instead of firing W307.  The type lattice doesn't model the
        // constructor return type for a var assignment yet (the cmd-site path
        // below recognises the bare-class `new`/`create` pattern directly), so
        // mirror that recognition here for the var-assignment shape.
        let harvest_constructor_vars =
            |this: &Self,
             fu: &crate::compilation_unit::FunctionUnit,
             out: &mut HashMap<String, HashSet<String>>| {
                use crate::ir::Statement;
                for block in fu.cfg.blocks.values() {
                    for stmt in &block.statements {
                        let Statement::AssignValue { name, value, .. } = stmt else {
                            continue;
                        };
                        let Some((head, args)) =
                            crate::value_shapes::parse_command_substitution(value.trim())
                        else {
                            continue;
                        };
                        if !args.first().is_some_and(|s| s == "new" || s == "create") {
                            continue;
                        }
                        let class_qn = this.canonicalise_class_name(&head);
                        if this.result.all_classes.contains_key(&class_qn)
                            || this.result.all_classes.contains_key(&head)
                        {
                            out.entry(name.clone()).or_default().insert(class_qn);
                        }
                    }
                }
            };
        harvest_constructor_vars(self, &cu.top_level, &mut all_object_types);
        for fu in cu.procedures.values() {
            harvest_constructor_vars(self, fu, &mut all_object_types);
        }

        // Build the class hierarchy once for W308 method
        // resolution (uses the C41e0 ``ClassHierarchy``).
        let hierarchy = if self.result.all_classes.is_empty() {
            None
        } else {
            Some(super::class_hierarchy::build_class_hierarchy(
                self.result.all_classes.clone(),
            ))
        };

        // Aggregate constant-string knowledge per variable name
        // across every function in the CompilationUnit.  Python
        // uses ``_lattice_to_set`` which expands CONST and
        // CONSTSET into a flat set of values; we replicate that
        // shape here.
        let mut all_constsets: HashMap<String, HashSet<String>> = HashMap::new();
        let collect_from = |sccp: &crate::sccp::SccpResult,
                            out: &mut HashMap<String, HashSet<String>>| {
            for (key, lv) in &sccp.values {
                let (var_name, _ver) = key;
                let Some(values) = lattice_command_values(lv) else {
                    continue;
                };
                let entry = out.entry(var_name.clone()).or_default();
                for v in values {
                    entry.insert(v);
                }
            }
        };
        collect_from(&cu.top_level.sccp, &mut all_constsets);
        for fu in cu.procedures.values() {
            collect_from(&fu.sccp, &mut all_constsets);
        }

        // Harvest `array set arr {k1 v1 k2 v2 …}` literal element values into
        // the constset map keyed by `arr(key)`, so the W307 callback-array
        // suppression can check the *actual* value of `$arr(-command)` against
        // the known-command set.  Without this, the dash-prefixed /
        // callback-suffixed array-key heuristic fires even when SCCP-equivalent
        // literal evidence proves the value is (or isn't) a command.  Mirrors
        // `_diag_var_command.py:421-452`.
        let harvest_array_set =
            |fu: &crate::compilation_unit::FunctionUnit,
             out: &mut HashMap<String, HashSet<String>>| {
                use crate::ir::Statement;
                for block in fu.cfg.blocks.values() {
                    for stmt in &block.statements {
                        let (Statement::Call { command, args, .. }
                        | Statement::Barrier { command, args, .. }) = stmt
                        else {
                            continue;
                        };
                        let is_array =
                            command == "array" || stmt.canonical_command_or_source() == "::array";
                        if !is_array
                            || args.first().map(String::as_str) != Some("set")
                            || args.len() < 3
                        {
                            continue;
                        }
                        let arr_name = &args[1];
                        let items = crate::tcl_expr_eval::split_tcl_list(&args[2]);
                        if items.len() % 2 != 0 {
                            continue;
                        }
                        for pair in items.chunks_exact(2) {
                            let elem_name = format!("{arr_name}({})", pair[0]);
                            out.entry(elem_name).or_default().insert(pair[1].clone());
                        }
                    }
                }
            };
        harvest_array_set(&cu.top_level, &mut all_constsets);
        for fu in cu.procedures.values() {
            harvest_array_set(fu, &mut all_constsets);
        }

        // Harvest `dict with d { … }` unpacked variable values: when `d` is a
        // known literal dict (via SCCP CONST at param entry — usually from
        // call-site constant propagation), the body sees each dict key as a
        // local variable bound to its value.  Register those bindings so a
        // `$cmd hi` dispatch inside the body checks `cmd`'s value against the
        // known-command set.  Mirrors `_diag_var_command.py:380-420`.
        let harvest_dict_with =
            |fu: &crate::compilation_unit::FunctionUnit,
             out: &mut HashMap<String, HashSet<String>>| {
                use crate::ir::Statement;
                for block in fu.cfg.blocks.values() {
                    for stmt in &block.statements {
                        let (Statement::Barrier { command, args, .. }
                        | Statement::Call { command, args, .. }) = stmt
                        else {
                            continue;
                        };
                        let is_dict =
                            command == "dict" || stmt.canonical_command_or_source() == "::dict";
                        if !is_dict || args.first().map(String::as_str) != Some("with") {
                            continue;
                        }
                        let Some(dict_var) = args.get(1) else {
                            continue;
                        };
                        let dvar = crate::naming::normalise_var_name(dict_var);
                        // Look up the dict var's value at param entry (v0) — the
                        // call-site-propagated literal lands here.
                        let Some(crate::analyses::LatticeValue::Const(
                            crate::analyses::ConstValue::String(dict_text),
                        )) = fu.sccp.values.get(&(dvar.to_string(), 0))
                        else {
                            continue;
                        };
                        let items = crate::tcl_expr_eval::split_tcl_list(dict_text);
                        if items.len() % 2 != 0 {
                            continue;
                        }
                        for pair in items.chunks_exact(2) {
                            out.entry(pair[0].clone())
                                .or_default()
                                .insert(pair[1].clone());
                        }
                    }
                }
            };
        harvest_dict_with(&cu.top_level, &mut all_constsets);
        for fu in cu.procedures.values() {
            harvest_dict_with(fu, &mut all_constsets);
        }

        // Build the "known commands" universe — registry +
        // user-defined procs + class tail names.
        let known_cmds: HashSet<String> = registry.command_names().map(str::to_string).collect();
        let known_procs: HashSet<String> = self.result.all_procs.keys().cloned().collect();
        let known_proc_bare: HashSet<String> = known_procs
            .iter()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, tail)| tail.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let known_class_tails: HashSet<String> = self
            .result
            .all_classes
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, tail)| tail.to_string()))
            .filter(|s| !s.is_empty())
            .collect();

        let is_known_command = |v: &str| {
            known_cmds.contains(v)
                || known_procs.contains(v)
                || known_proc_bare.contains(v)
                || known_procs.contains(&format!("::{v}"))
                || known_class_tails.contains(v)
                || self.result.all_classes.contains_key(&format!("::{v}"))
        };

        // Per-SSA-version refinement (post-stage2 §A): map each
        // function to its source range + FunctionUnit so the W307
        // suppression can read the value at the dispatch's *exact* SSA
        // use-version instead of the merged set.  ``::top`` covers the
        // whole source; a proc's narrower range wins where it contains
        // the offset.  Mirrors Python's ``_func_ranges`` /
        // ``_all_fus_named`` (methods are ``in_method``-suppressed, so
        // — like Python's loop — they are left out).
        let mut func_ranges: Vec<(String, u32, u32)> = vec![("::top".to_string(), 0, u32::MAX)];
        let mut fu_by_qname: HashMap<String, &crate::compilation_unit::FunctionUnit> =
            HashMap::new();
        fu_by_qname.insert("::top".to_string(), &cu.top_level);
        for (qname, fu) in &cu.procedures {
            fu_by_qname.insert(qname.clone(), fu);
            if let Some(ir_proc) = cu.ir_module.procedures.get(qname) {
                func_ranges.push((qname.clone(), ir_proc.span.start(), ir_proc.span.end()));
            }
        }

        // Drain sites so we can borrow self.result mutably below.
        let sites = std::mem::take(&mut self.var_command_sites);
        let objdefined_vars = self.objdefined_vars.clone();

        // **Proc-parameter / multi-dispatch object-dispatch suppression**
        // (mirrors `_diag_var_command.py:807-859`).  A dispatch on a proc
        // *parameter* — `proc walk {tree} { $tree visit }` — is object
        // dispatch the user has documented as the proc's API contract, not a
        // static error.  A non-parameter local dispatched ≥2 times in the same
        // scope is likewise evidenced object usage (a single dispatch could be
        // a typo; repeated use is clearly designed).  Build, per enclosing
        // proc body, its parameter set and the per-var dispatch count, plus a
        // taint carve-out: a *tainted* var is never suppressed (dispatching a
        // user-controlled command name is an injection risk regardless of how
        // many times it appears).  `::top` is the sentinel for statements
        // outside any proc body.
        let mut proc_body_ranges: Vec<(u32, u32, String, HashSet<String>)> = self
            .result
            .all_procs
            .iter()
            .map(|(qname, pdef)| {
                let params: HashSet<String> = pdef.params.iter().map(|p| p.name.clone()).collect();
                (
                    pdef.body_span.start(),
                    pdef.body_span.end(),
                    qname.clone(),
                    params,
                )
            })
            .collect();
        // Innermost-enclosing wins: scan largest-start-first for a range that
        // contains the offset (procs don't nest, but `namespace eval` bodies
        // can wrap several, so this stays robust).  Returns the index into
        // `proc_body_ranges`, or `None` for the `::top` sentinel scope.
        proc_body_ranges.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        let enclosing_idx = |off: u32| -> Option<usize> {
            proc_body_ranges
                .iter()
                .enumerate()
                .rev()
                .find(|(_, (s, e, _, _))| *s <= off && off <= *e)
                .map(|(i, _)| i)
        };
        let scope_qname = |idx: Option<usize>| -> &str {
            idx.map_or(W307_TOP_SCOPE, |i| proc_body_ranges[i].2.as_str())
        };
        let mut dispatch_counts: HashMap<(String, String), usize> = HashMap::new();
        for site in &sites {
            let qname = scope_qname(enclosing_idx(site.cmd_span.start()));
            *dispatch_counts
                .entry((qname.to_owned(), site.var_name.clone()))
                .or_insert(0) += 1;
        }
        // Per-scope tainted var names — any tainted SSA version of a name
        // disqualifies it from dispatcher-suppression.  Keyed by qname, with
        // `::top` for the top-level scope.
        let tainted_names_of = |fu: &crate::compilation_unit::FunctionUnit| -> HashSet<String> {
            fu.taints
                .iter()
                .filter(|(_, tl)| tl.is_tainted())
                .map(|((var, _ver), _)| var.clone())
                .collect()
        };
        let mut tainted_by_scope: HashMap<String, HashSet<String>> = HashMap::new();
        let top_tainted = tainted_names_of(&cu.top_level);
        if !top_tainted.is_empty() {
            tainted_by_scope.insert(W307_TOP_SCOPE.to_owned(), top_tainted);
        }
        for (qname, fu) in &cu.procedures {
            let names = tainted_names_of(fu);
            if !names.is_empty() {
                tainted_by_scope.insert(qname.clone(), names);
            }
        }

        for site in &sites {
            // **W308 path.**  Variable known to hold an Object
            // — validate the method name against the class
            // hierarchy.  When the method isn't found and the
            // class doesn't have an external superclass that
            // could carry it, emit W308.
            if let Some(class_names) = all_object_types.get(&site.var_name) {
                if let (Some(method_name), Some(hierarchy)) = (&site.method_name, &hierarchy) {
                    let mut found = false;
                    let mut has_local_class = false;
                    for cls in class_names {
                        if hierarchy.method_target(cls, method_name).is_some() {
                            found = true;
                            break;
                        }
                        if let Some(cd) = self.result.all_classes.get(cls) {
                            has_local_class = true;
                            if cd.methods.contains_key(method_name)
                                || cd.class_methods.contains_key(method_name)
                                || matches!(
                                    method_name.as_str(),
                                    "new" | "create" | "destroy" | "configure" | "cget"
                                )
                                || cd.methods.contains_key("unknown")
                            {
                                found = true;
                                break;
                            }
                        }
                    }
                    // Inherited ``unknown`` handler via MRO.
                    if !found && has_local_class {
                        for cls in class_names {
                            if hierarchy.method_target(cls, "unknown").is_some() {
                                found = true;
                                break;
                            }
                        }
                    }
                    // External superclass: a method might come
                    // from a class outside the current index.
                    if !found && has_local_class {
                        const OO_BASE: &[&str] = &["oo::object", "oo::class"];
                        'cls_loop: for cls in class_names {
                            if let Some(cd) = self.result.all_classes.get(cls) {
                                for s in &cd.superclasses {
                                    if !self.result.all_classes.contains_key(s)
                                        && !OO_BASE.contains(&s.as_str())
                                    {
                                        found = true;
                                        break 'cls_loop;
                                    }
                                }
                            }
                        }
                    }
                    // ``oo::objdefine`` adds per-instance
                    // methods we can't see at the class level.
                    if !found && objdefined_vars.contains(&site.var_name) {
                        found = true;
                    }
                    if !found && has_local_class && !self.disabled_diagnostics.contains("W308") {
                        let mut classes_sorted: Vec<&str> =
                            class_names.iter().map(String::as_str).collect();
                        classes_sorted.sort_unstable();
                        let cls_display = classes_sorted.join(", ");
                        let message =
                            format!("Unknown method '{method_name}' on class '{cls_display}'");
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W308".to_string(),
                            span: site.cmd_span,
                            message,
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                    }
                }
                // W307 path doesn't fire when the var is a
                // known Object — the method-name check is the
                // load-bearing piece.
                continue;
            }

            // **W307 path.**  Variable not a known Object.
            // ``in_method`` short-circuits W307 because OO
            // methods routinely use ``$obj method`` patterns.
            // The Rust analyser doesn't track method context
            // yet (lands in C41e — pending a Method scope kind),
            // so this filter currently matches Python's
            // ``in_method=False`` always-fall-through behaviour.
            if site.in_method {
                continue;
            }
            // Prefer the value at the dispatch's exact SSA use-version;
            // fall back to the merged constset when no precise version
            // is found. This drops the merged-set false positive on a
            // variable reassigned from a non-command to a known command
            // before the dispatch (`set c x; set c puts; $c ...`).
            let precise = w307_precise_cmd_values(
                &func_ranges,
                &fu_by_qname,
                site.cmd_span.start(),
                &site.var_name,
            );
            let effective = precise
                .as_ref()
                .or_else(|| all_constsets.get(&site.var_name));
            if let Some(values) = effective {
                if !values.is_empty() && values.iter().all(|v| is_known_command(v)) {
                    continue;
                }
            }
            // Proc-parameter / multi-dispatch object-dispatch suppression: a
            // dispatch on a parameter of the enclosing proc (any count), or on
            // a non-parameter local dispatched ≥2 times in the same scope, is
            // evidenced object usage — suppress unless the var is tainted.
            let idx = enclosing_idx(site.cmd_span.start());
            let encl_qname = scope_qname(idx);
            let is_param = idx.is_some_and(|i| proc_body_ranges[i].3.contains(&site.var_name));
            let dispatch_count = dispatch_counts
                .get(&(encl_qname.to_owned(), site.var_name.clone()))
                .copied()
                .unwrap_or(0);
            let dispatcher_suppressed = is_param || dispatch_count >= 2;
            let tainted = tainted_by_scope
                .get(encl_qname)
                .is_some_and(|s| s.contains(&site.var_name));
            if dispatcher_suppressed && !tainted {
                continue;
            }
            // Namespaced-ensemble dispatch: `${ns}::tail` / `$ns::tail` where
            // `ns` holds a namespace prefix and `::tail` composes a qualified
            // command path (tcllib's logger / dns / irc modules use this).
            // When the prefix is an SCCP const and *every* composed name
            // `<value>::tail` resolves to a known command/proc/class, the
            // dispatch is statically resolvable — suppress.  A composition
            // that resolves to nothing (unknown proc) still fires.  Mirrors
            // the composed-name arm of `_diag_var_command.py:1200-1248`.
            if let Some((prefix, tail)) = parse_namespaced_ensemble(&self.source, site.cmd_span) {
                if let Some(values) = all_constsets.get(&prefix) {
                    if !values.is_empty()
                        && values
                            .iter()
                            .all(|v| is_known_command(&format!("{v}::{tail}")))
                    {
                        continue;
                    }
                }
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W307".to_string(),
                span: site.cmd_span,
                message: "Non-literal command name — cannot statically analyze".to_string(),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
        // Restore the sites list — snapshot/restore expects it
        // to round-trip independently of emission.
        self.var_command_sites = sites;

        // **C41 follow-up.** ``[cmd] method`` sites — emit
        // W307 only when the inner command's return type is
        // unknown AND the call isn't an OO self-dispatch
        // (``my`` / ``self``).  When the return type is a
        // known class, validate the method against the
        // hierarchy and emit W308 instead of W307.  This
        // mirrors the cmd_command_sites branch of
        // ``_emit_var_command_diagnostics`` in
        // ``_diag_var_command.py:296-375``.
        let cmd_sites = std::mem::take(&mut self.cmd_command_sites);
        for site in &cmd_sites {
            if site.in_method {
                continue;
            }
            // Parse the command-substitution text into
            // ``head ?args...``.  ``cmd_text`` is what the
            // analyser captured from
            // ``SourceMap::token_text``; the leading ``[`` /
            // trailing ``]`` are stripped already because
            // ``content_offset`` skipped them.
            let inner = site.cmd_text.trim();
            let inner = inner
                .strip_prefix('[')
                .map_or(inner, str::trim)
                .strip_suffix(']')
                .map_or(inner, str::trim);
            let mut parts = inner.split_whitespace();
            let Some(head) = parts.next() else {
                continue;
            };
            let arg_strs: Vec<&str> = parts.collect();

            // OO self-dispatch ⇒ suppress W307.
            let is_oo_self_dispatch = matches!(head, "my" | "self");
            if is_oo_self_dispatch {
                continue;
            }

            // **Codex P1 fix.** ``[Dog new]`` / ``[Dog create
            // name]`` produce an Object whose class is ``Dog``.
            // The registry lookup for the bare class name
            // returns Overdefined (the class isn't a built-in
            // command) so we recognise the constructor pattern
            // explicitly here — mirrors the Python
            // ``_return_type_for_command`` branch in
            // ``core/compiler/core_analyses.py`` that maps
            // ``known_class new/create`` to ``TclType.OBJECT``
            // with the class name attached.
            let class_qn = self.canonicalise_class_name(head);
            let head_is_known_class = self.result.all_classes.contains_key(&class_qn)
                || self.result.all_classes.contains_key(head);
            let is_constructor_call = head_is_known_class
                && arg_strs
                    .first()
                    .is_some_and(|sub| matches!(*sub, "new" | "create"));

            // Look up the return type via the registry.  When
            // the head is a user proc / class, fall back to
            // ``Overdefined`` (matches the registry behaviour
            // for unknown commands).
            let ret_type = if is_constructor_call {
                crate::types::TypeLattice {
                    kind: crate::types::TypeKind::Known,
                    tcl_type: Some(tcl_registry::TclType::Object),
                    from_type: None,
                    class_name: Some(class_qn.clone()),
                }
            } else {
                crate::type_infer::return_type_for_command(registry, head, &arg_strs)
            };

            // ``Object`` return type — suppress W307; if the
            // class is known, validate the method (W308).
            let is_object = ret_type.kind == crate::types::TypeKind::Known
                && matches!(ret_type.tcl_type, Some(tcl_registry::TclType::Object));
            if is_object {
                if !self.disabled_diagnostics.contains("W308") {
                    if let (Some(method), Some(class_name)) =
                        (site.method_name.as_ref(), ret_type.class_name.as_ref())
                    {
                        let cls_qn = self.canonicalise_class_name(class_name);
                        let cd = self.result.all_classes.get(&cls_qn).cloned();
                        let method_ok = self.validate_method_on_class(
                            &cls_qn,
                            method,
                            cd.as_ref(),
                            hierarchy.as_ref(),
                        );
                        if !method_ok {
                            self.result.diagnostics.push(super::types::Diagnostic {
                                code: "W308".to_string(),
                                span: site.cmd_span,
                                message: format!(
                                    "Unknown method '{method}' on class '{class_name}'"
                                ),
                                severity: Severity::Warning,
                                fixes: Vec::new(),
                            });
                        }
                    }
                }
                continue;
            }

            // Type is unknown — emit W307 (matching Python's
            // emit-then-suppress shape, but only the emit-half
            // for the residual unknown-type case).
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W307".to_string(),
                span: site.cmd_span,
                message: "Non-literal command name — cannot statically analyze".to_string(),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
        self.cmd_command_sites = cmd_sites;
    }

    /// Resolve a possibly-bare class name to its fully-qualified
    /// form keyed in `result.all_classes`.
    fn canonicalise_class_name(&self, name: &str) -> String {
        if name.starts_with("::") {
            return name.to_string();
        }
        let qualified = format!("::{name}");
        if self.result.all_classes.contains_key(&qualified) {
            qualified
        } else {
            name.to_string()
        }
    }

    /// Decide whether `method` is callable on `class_name`,
    /// consulting the class hierarchy + the class's local
    /// method tables.
    ///
    /// Mirrors the W308 method-resolution gate in
    /// ``_diag_var_command.py:341-361``.  A method is OK when
    /// the class's MRO produces a concrete provider, or the
    /// class is external (no local `ClassDef`), or the method
    /// is one of the OO standard hooks (``new`` / ``create`` /
    /// ``destroy`` / ``configure`` / ``cget``), or the class
    /// declares an ``unknown`` method, or the class extends an
    /// external superclass we can't introspect.
    fn validate_method_on_class(
        &self,
        class_name: &str,
        method: &str,
        cd: Option<&super::types::ClassDef>,
        hierarchy: Option<&super::class_hierarchy::ClassHierarchy>,
    ) -> bool {
        if hierarchy.is_some_and(|h| h.method_target(class_name, method).is_some()) {
            return true;
        }
        let Some(cd) = cd else {
            // External class — can't validate.
            return true;
        };
        if cd.methods.contains_key(method) || cd.class_methods.contains_key(method) {
            return true;
        }
        if matches!(method, "new" | "create" | "destroy" | "configure" | "cget") {
            return true;
        }
        if cd.methods.contains_key("unknown") {
            return true;
        }
        if hierarchy.is_some_and(|h| h.method_target(class_name, "unknown").is_some()) {
            return true;
        }
        // External superclass ⇒ skip W308.
        if !cd.superclasses.is_empty() {
            for s in &cd.superclasses {
                if !self.result.all_classes.contains_key(s) && !OO_BASE.contains(&s.as_str()) {
                    return true;
                }
            }
        }
        false
    }

    /// Suppress W123 diagnostics whose command-name contains a
    /// `$` interpolation that resolves cleanly via SCCP.
    ///
    /// Mirrors `_resolve_interpolated_commands` in
    /// `core/analysis/_analyser/_diag_commands.py:188-260`.
    /// Walks every emitted W123, extracts the command name
    /// from the message, and runs
    /// [`crate::text::fold_interpolation_set`] over the
    /// aggregated SCCP results.  When every resolved value is
    /// a known command, proc, class, or class-tail name, the
    /// W123 is removed.
    ///
    /// **Simplification vs. Python.**  Python builds a
    /// per-function SCCP map and uses range-based lookup so
    /// each W123 site sees only the variables in its enclosing
    /// function's scope.  The Rust port uses the union of
    /// every function's SCCP — slightly more permissive
    /// (over-suppresses if a same-named variable in a
    /// different function happens to resolve cleanly) but
    /// safe in practice.  Range-based lookup can land later
    /// when the parity gap surfaces.
    fn resolve_interpolated_w123_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        use crate::analyses::{ConstValue, LatticeValue};
        use std::collections::HashMap;

        // Bail early when no W123 carries a ``$`` — the common
        // case for non-iRules code.
        let has_interpolated = self
            .result
            .diagnostics
            .iter()
            .any(|d| d.code == "W123" && d.message.contains('$'));
        if !has_interpolated {
            return;
        }

        // Aggregate SCCP-resolved string sets per variable name
        // across every function in the CU.  Same shape as
        // ``emit_var_command_diagnostics``.
        let mut all_constsets: HashMap<String, HashSet<String>> = HashMap::new();
        let collect_from = |sccp: &crate::sccp::SccpResult,
                            out: &mut HashMap<String, HashSet<String>>| {
            for ((var_name, _ver), lv) in &sccp.values {
                let values: Option<Vec<String>> = match lv {
                    LatticeValue::Const(ConstValue::String(s)) => Some(vec![s.clone()]),
                    LatticeValue::ConstSet(set) => set
                        .iter()
                        .map(|cv| match cv {
                            ConstValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>(),
                    _ => None,
                };
                let Some(values) = values else { continue };
                let entry = out.entry(var_name.clone()).or_default();
                for v in values {
                    entry.insert(v);
                }
            }
        };
        collect_from(&cu.top_level.sccp, &mut all_constsets);
        for fu in cu.procedures.values() {
            collect_from(&fu.sccp, &mut all_constsets);
        }

        // Build the universe of names that count as "known
        // commands" for the resolution check.  Same set the
        // emitter used to skip suggestions in the first pass.
        let registry = tcl_registry::CommandRegistry::build_default();
        let known_cmds: HashSet<String> = registry.command_names().map(str::to_string).collect();
        let known_proc_tails: HashSet<String> = self
            .result
            .all_procs
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();

        // Walk W123 diagnostics and remove those whose
        // interpolated command name resolves cleanly.
        let drained = std::mem::take(&mut self.result.diagnostics);
        let mut kept: Vec<super::types::Diagnostic> = Vec::with_capacity(drained.len());
        for d in drained {
            if d.code != "W123" {
                kept.push(d);
                continue;
            }
            let Some(cmd_name) = extract_quoted_word(&d.message) else {
                kept.push(d);
                continue;
            };
            if !cmd_name.contains('$') {
                kept.push(d);
                continue;
            }
            let Some(resolved) = crate::text::fold_interpolation_set(&cmd_name, &all_constsets)
            else {
                kept.push(d);
                continue;
            };
            // All resolved candidates must be known commands.
            let all_known = resolved.iter().all(|name| {
                known_cmds.contains(name)
                    || known_proc_tails.contains(name)
                    || self.result.all_procs.contains_key(&format!("::{name}"))
                    || self.result.all_procs.contains_key(name)
            });
            if all_known {
                // Suppress this W123 — the interpolated head
                // statically resolves to a known command set.
                continue;
            }
            kept.push(d);
        }
        self.result.diagnostics = kept;
    }

    /// Drop exact-duplicate diagnostics + line-based suppression
    /// pairs.
    ///
    /// Mirrors `_dedupe_diagnostics` in
    /// `_diagnostics.py` (lives in `_core.py:595-630` — the
    /// orchestrator file imports it through the mixin
    /// hierarchy).  Two passes:
    ///
    /// 1. Compute the set of source lines on which `E101`
    ///    (missing-open-brace) and `W124` (SSA-based IP check)
    ///    fired.  These are sentinels for the related
    ///    redundant-message codes.
    /// 2. Walk diagnostics in source order, deduplicating by
    ///    `(code, span, message, severity)` and dropping:
    ///    - `E002` on a line where `E101` fired (the recovered
    ///      switch makes the arity message a false positive).
    ///    - `W122` on a line where `W124` fired (the SSA check
    ///      is more precise).
    ///
    /// Lines come from the [`SourceMap`] over `self.source`.
    pub fn dedupe_diagnostics(&mut self) {
        let sm = SourceMap::new(&self.source);
        let mut e101_lines: HashSet<u32> = HashSet::new();
        let mut w124_lines: HashSet<u32> = HashSet::new();
        for d in &self.result.diagnostics {
            let line = sm.range_positions(d.span).0.line;
            match d.code.as_str() {
                "E101" => {
                    e101_lines.insert(line);
                }
                "W124" => {
                    w124_lines.insert(line);
                }
                _ => {}
            }
        }

        let mut seen: HashSet<(String, u32, u32, String, Severity)> = HashSet::new();
        let drained = std::mem::take(&mut self.result.diagnostics);
        let mut deduped = Vec::with_capacity(drained.len());
        for d in drained {
            let key = (
                d.code.clone(),
                d.span.start(),
                d.span.end(),
                d.message.clone(),
                d.severity,
            );
            if seen.contains(&key) {
                continue;
            }
            let line = sm.range_positions(d.span).0.line;
            if d.code == "E002" && e101_lines.contains(&line) {
                continue;
            }
            if d.code == "W122" && w124_lines.contains(&line) {
                continue;
            }
            seen.insert(key);
            deduped.push(d);
        }
        self.result.diagnostics = deduped;
    }

    /// Filter out diagnostics whose codes are in
    /// [`Self::disabled_diagnostics`].
    ///
    /// Mirrors the per-emitter `if "Wxxx" in
    /// self._disabled_diagnostics:` early-returns in Python's
    /// emitter files.  Centralising the filter on the orchestrator
    /// side keeps the per-emitter code (in C41d2 / C41d3 / etc.)
    /// from having to thread the check at every emit site —
    /// emitters can push freely and the orchestrator drops the
    /// silenced codes at the end.
    ///
    /// Idempotent on an empty filter set (no allocations).
    pub fn apply_disabled_diagnostics(&mut self) {
        if self.disabled_diagnostics.is_empty() {
            return;
        }
        // Borrow-checker dance: `retain` closure can't capture
        // `&self.disabled_diagnostics` while ``self.result`` is
        // mut-borrowed; clone the set into a local first.  The
        // disabled set is small (LSP-config-scale) so the clone
        // cost is negligible vs. the rest of the diagnostics
        // pipeline.
        let disabled = self.disabled_diagnostics.clone();
        self.result
            .diagnostics
            .retain(|d| !disabled.contains(&d.code));
    }

    /// IRULE4005 — racy ``static::`` cross-event flow.
    ///
    /// Mirrors `_emit_racy_static_diagnostics` in
    /// `core/analysis/_analyser/_diag_racy.py`.  Walks every
    /// SSA statement in `fu` and emits IRULE4005 for any
    /// non-``unset`` def of a name in `racy_vars`.
    /// `racy_vars` comes from
    /// [`crate::connection_scope::ConnectionScope::racy_static_defs`]
    /// — built once per `CompilationUnit` and shared by every
    /// ``::when::*`` proc except `RULE_INIT`.
    fn emit_racy_static_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        racy_vars: &HashSet<String>,
    ) {
        if self.disabled_diagnostics.contains("IRULE4005") {
            return;
        }
        let mut emitted_spans: HashSet<u32> = HashSet::new();
        for block in fu.ssa.blocks.values() {
            for stmt in &block.statements {
                // Skip unset — not a real write.  Mirrors the
                // Python guard.
                if let crate::ir::Statement::Call { command, .. } = &stmt.statement {
                    if command == "unset" {
                        continue;
                    }
                }
                for name in stmt.defs.keys() {
                    if !racy_vars.contains(name) {
                        continue;
                    }
                    let span = stmt.statement.span();
                    if span.is_empty() || !emitted_spans.insert(span.start()) {
                        continue;
                    }
                    let message = format!(
                        "Potential race: '{name}' is written outside RULE_INIT and read in \
                         another event. static:: variables persist across all connections on \
                         the same virtual server; concurrent writes can produce unpredictable \
                         results."
                    );
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: "IRULE4005".to_string(),
                        span,
                        message,
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                }
            }
        }
    }

    /// **W004.** Emit "Command option is not available in the active
    /// dialect" warning for option-bearing commands invoked with an
    /// option whose registry entry restricts it to a dialect that
    /// doesn't include the active one.
    ///
    /// Mirrors `check_dialect_invalid_option` in
    /// `core/analysis/checks/_domain.py` (PR #433).  Examples:
    /// `lsearch -stride` on Tcl 8.4 / 8.5 (option is 8.6+),
    /// `regsub -command` / `clock scan -validate` /
    /// `fconfigure -nodelay` on Tcl 8.x (options are 9.0+).
    ///
    /// Walks args looking for `-foo`-shaped flags, asks the registry
    /// for the matching `OptionSpec`, and fires when
    /// `OptionSpec::supports_dialect` returns false.  Substituted
    /// flag values (`-foo $bar`, `-foo [cmd]`) are skipped because
    /// the dispatching is only on the *flag name*; we don't have to
    /// inspect the value.  `--` terminates the scan.
    ///
    /// Subcommand-scoped options consult the subcommand's
    /// `OptionSpec` table when the first arg matches a known
    /// subcommand.
    pub(super) fn emit_w004_dialect_invalid_option(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use tcl_registry::dialects::DialectSet;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let Some(active) = DialectSet::parse(&self.dialect) else {
            return;
        };
        let Some(spec) = registry.get(cmd_name) else {
            return;
        };

        // Resolve subcommand-level options when the first arg names
        // one (mirrors Python's `if first in spec.subcommands`).
        let sub_match = (!spec.subcommands.is_empty())
            .then(|| spec.subcommands.iter().find(|s| s.name == args[0].as_str()))
            .flatten();
        let (options, parent_dialects, start_idx) = if let Some(sub) = sub_match {
            (sub.options, sub.dialects.or(spec.dialects), 1usize)
        } else {
            (spec.options, spec.dialects, 0usize)
        };

        if options.is_empty() {
            return;
        }

        let mut i = start_idx;
        while i < args.len() {
            let arg = args[i].as_str();
            if arg == "--" {
                break;
            }
            if !arg.starts_with('-') || arg.len() < 2 {
                i += 1;
                continue;
            }
            // Skip negative number literals (`-1`, `-1.5`).
            let rest = &arg[1..].trim_start_matches('-');
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '.') {
                i += 1;
                continue;
            }
            // Skip dynamic-value args (Var / Cmd tokens).  The flag
            // name itself comes from the arg text, but if the
            // representative token is a substitution we can't know
            // it's actually `-foo`.
            if i < arg_tokens.len() {
                let tok = arg_tokens[i];
                if matches!(
                    tok.kind,
                    tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
                ) {
                    i += 1;
                    continue;
                }
            }
            // Find a matching OptionSpec; if found and dialect-gated
            // out, emit W004.
            if let Some(opt) = options.iter().find(|o| o.name == arg) {
                if !opt.supports_dialect(Some(active), parent_dialects) {
                    let span = if i < arg_tokens.len() {
                        arg_tokens[i].span
                    } else {
                        continue;
                    };
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: "W004".to_string(),
                        span,
                        message: format!(
                            "Option '{}' on command '{}' is not available in dialect '{}'.",
                            arg, cmd_name, self.dialect
                        ),
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                }
            }
            i += 1;
        }
    }

    /// **W003.** Emit "Expression operator not available in active
    /// dialect" warning for expressions that use a Tcl 9.0 string-
    /// comparison operator (`lt` / `le` / `gt` / `ge`, TIP 461) in a
    /// pre-9.0 dialect, or `in` / `ni` (TIP 201, Tcl 8.5+) in
    /// Tcl 8.4 / f5-irules.
    ///
    /// Mirrors `check_dialect_invalid_expr_operator` in
    /// `core/analysis/checks/_domain.py` (PR #433).
    pub(super) fn emit_w003_dialect_invalid_expr_operator(
        &mut self,
        expr_text: &str,
        diag_span: tcl_lexer::Span,
    ) {
        use tcl_registry::dialects::DialectSet;

        // Quick lexical bail-out — the gated operators are short
        // word-shaped keywords; if none appear as a whole word we
        // can skip the parse.  Boundary check uses ASCII identifier
        // continuation so `tab`-, `newline`-, and start/end-of-text
        // boundaries all count (mirrors Tcl expr's whitespace
        // tolerance — `$x\tlt\t$y` and a wrapped `in` expression
        // both qualify).
        if !contains_gated_word(expr_text) {
            return;
        }
        let Some(active) = DialectSet::parse(&self.dialect) else {
            return;
        };
        // Pre-Tcl-8.5 dialects don't accept `in` / `ni` (TIP 201).
        let pre_85 = !DialectSet::TCL85_PLUS.contains(active);
        // Pre-Tcl-9.0 dialects don't accept `lt` / `le` / `gt` / `ge`
        // (TIP 461).
        let pre_90 = !DialectSet::from_iter([DialectSet::TCL90]).contains(active);
        if !pre_85 && !pre_90 {
            return;
        }

        let parsed = crate::parse_expr(expr_text.trim(), Some(self.dialect.as_str()));
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }
        let mut found: Vec<&'static str> = Vec::new();
        walk_dialect_invalid_ops(&parsed, pre_85, pre_90, &mut found);
        for op_name in found {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W003".to_string(),
                span: diag_span,
                message: format!(
                    "Expression operator '{op_name}' is not available in dialect '{}'.",
                    self.dialect
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }
}

/// Walk an expression AST and collect dialect-gated operator
/// occurrences.  Mirrors `_find_dialect_invalid_ops` in
/// `core/analysis/checks/_domain.py` (PR #433).
/// Return `true` if `text` contains any of the dialect-gated
/// expression operator keywords (`lt`, `le`, `gt`, `ge`, `in`, `ni`)
/// as a whole word — i.e. surrounded by non-identifier bytes or
/// the text boundary.  Used as a fast prefilter to skip the
/// expression parse for expressions that obviously can't trigger
/// W003.
///
/// Whitespace-aware: tabs, newlines, and any other non-identifier
/// byte (parentheses, operators, comparison glyphs, etc.) count
/// as word boundaries.  Matches Tcl expr's tolerance for
/// arbitrary whitespace between tokens.
fn contains_gated_word(text: &str) -> bool {
    const GATED: &[&[u8]] = &[b"lt", b"le", b"gt", b"ge", b"in", b"ni"];
    let bytes = text.as_bytes();
    for needle in GATED {
        let n = needle.len();
        let mut i = 0;
        while i + n <= bytes.len() {
            if &bytes[i..i + n] == *needle {
                let before_ok = i == 0 || !is_ident_continue(bytes[i - 1]);
                let after_ok = i + n == bytes.len() || !is_ident_continue(bytes[i + n]);
                if before_ok && after_ok {
                    return true;
                }
            }
            i += 1;
        }
    }
    false
}

fn walk_dialect_invalid_ops(
    node: &ExprNode,
    pre_85: bool,
    pre_90: bool,
    found: &mut Vec<&'static str>,
) {
    match node {
        ExprNode::Binary { op, left, right } => {
            walk_dialect_invalid_ops(left, pre_85, pre_90, found);
            walk_dialect_invalid_ops(right, pre_85, pre_90, found);
            match op {
                BinOp::In if pre_85 => found.push("in"),
                BinOp::Ni if pre_85 => found.push("ni"),
                BinOp::StrLt if pre_90 => found.push("lt"),
                BinOp::StrLe if pre_90 => found.push("le"),
                BinOp::StrGt if pre_90 => found.push("gt"),
                BinOp::StrGe if pre_90 => found.push("ge"),
                _ => {}
            }
        }
        ExprNode::Unary { operand, .. } => {
            walk_dialect_invalid_ops(operand, pre_85, pre_90, found);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_dialect_invalid_ops(condition, pre_85, pre_90, found);
            walk_dialect_invalid_ops(true_branch, pre_85, pre_90, found);
            walk_dialect_invalid_ops(false_branch, pre_85, pre_90, found);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                walk_dialect_invalid_ops(arg, pre_85, pre_90, found);
            }
        }
        _ => {}
    }
}

/// Expand a CONST / CONSTSET lattice value into the flat set of its
/// string values, or `None` for any non-string-constant lattice state.
/// Mirrors Python's `_lattice_to_set` as consumed by the W307 emitter.
fn lattice_command_values(lv: &crate::analyses::LatticeValue) -> Option<Vec<String>> {
    use crate::analyses::{ConstValue, LatticeValue};
    match lv {
        LatticeValue::Const(ConstValue::String(s)) => Some(vec![s.clone()]),
        LatticeValue::ConstSet(set) => set
            .iter()
            .map(|cv| match cv {
                ConstValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect::<Option<Vec<_>>>(),
        _ => None,
    }
}

/// The SCCP value set of `var_name` at the SSA use-version that reaches
/// the dispatch statement at `offset` (W307 per-SSA-version refinement).
///
/// The merged `all_constsets` map unions every version of a variable,
/// so `set c notacommand; set c parse; $c x` wrongly keeps
/// `notacommand` in the set even though only the `parse` version
/// reaches the dispatch. Reading the value at the use site's exact
/// version removes that false positive.
///
/// Purely additive: returns a set only when a CFG statement containing
/// `offset` that *uses* `var_name` is found and its version has a
/// concrete CONST / CONSTSET value — otherwise `None`, and the caller
/// falls back to the merged-set logic. Never broadens a fire into a
/// suppression unsoundly — the value is the exact one flowing into the
/// dispatch. Mirrors Python's `_precise_cmd_values`.
fn w307_precise_cmd_values(
    func_ranges: &[(String, u32, u32)],
    fu_by_qname: &std::collections::HashMap<String, &crate::compilation_unit::FunctionUnit>,
    offset: u32,
    var_name: &str,
) -> Option<HashSet<String>> {
    // Narrowest function range containing `offset` (mirrors the
    // scoping `_constsets_for_offset` uses).
    let mut best: Option<(u32, &str)> = None;
    for (qname, start, end) in func_ranges {
        if *start <= offset && offset <= *end {
            let width = end - start;
            if best.map_or(true, |(bw, _)| width < bw) {
                best = Some((width, qname.as_str()));
            }
        }
    }
    let fu = fu_by_qname.get(best?.1)?;

    // Narrowest CFG statement containing `offset` that uses `var_name`,
    // reading its SSA use-version (CFG / SSA blocks are parallel-indexed).
    let mut best_width: Option<u32> = None;
    let mut best_version: Option<u32> = None;
    for (block_name, block) in &fu.cfg.blocks {
        let Some(ssa_block) = fu.ssa.blocks.get(block_name) else {
            continue;
        };
        for (idx, stmt) in block.statements.iter().enumerate() {
            let span = stmt.span();
            if !(span.start() <= offset && offset <= span.end()) {
                continue;
            }
            let Some(ssa_stmt) = ssa_block.statements.get(idx) else {
                continue;
            };
            let Some(version) = ssa_stmt.uses.get(var_name) else {
                continue;
            };
            let width = span.end() - span.start();
            if best_width.map_or(true, |bw| width < bw) {
                best_width = Some(width);
                best_version = Some(*version);
            }
        }
    }
    let version = best_version?;
    let lv = fu.sccp.values.get(&(var_name.to_string(), version))?;
    Some(lattice_command_values(lv)?.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::types::Diagnostic;
    use tcl_lexer::Span;

    fn w114_codes(src: &str) -> usize {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == "W114")
            .count()
    }

    fn code_sevs(src: &str, code: &str) -> Vec<String> {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == code)
            .map(|d| format!("{:?}", d.severity))
            .collect()
    }

    fn has_code(src: &str, dialect: &str, code: &str) -> bool {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, dialect)
            .diagnostics
            .iter()
            .any(|d| d.code == code)
    }

    #[test]
    fn w311_flags_binary_encoding_with_translation() {
        assert!(has_code(
            "fconfigure $ch -encoding binary -translation lf\n",
            "tcl8.6",
            "W311",
        ));
        assert!(has_code(
            "chan configure $ch -encoding binary -translation crlf\n",
            "tcl8.6",
            "W311",
        ));
        // `-translation binary` is consistent — no warning.
        assert!(!has_code(
            "fconfigure $ch -encoding binary -translation binary\n",
            "tcl8.6",
            "W311",
        ));
    }

    #[test]
    fn w200_binary_modifier_is_dialect_gated() {
        // `cu` / `su` modifiers need Tcl 8.5+; flagged under 8.4 only.
        assert!(has_code("binary format cu1 $x\n", "tcl8.4", "W200"));
        assert!(has_code("binary scan $d su v\n", "tcl8.4", "W200"));
        assert!(!has_code("binary format cu1 $x\n", "tcl8.6", "W200"));
        // No modifier — never flagged.
        assert!(!has_code("binary format c1 $x\n", "tcl8.4", "W200"));
    }

    #[test]
    fn w121_flags_noncontiguous_subnet_mask() {
        assert!(has_code("set m 255.255.255.1\n", "tcl8.6", "W121"));
        assert!(has_code("set m 255.0.255.0\n", "tcl8.6", "W121"));
        // Valid contiguous masks are fine.
        assert!(!has_code("set m 255.255.255.0\n", "tcl8.6", "W121"));
        assert!(!has_code("set m 255.255.254.0\n", "tcl8.6", "W121"));
    }

    #[test]
    fn subnet_mask_helpers() {
        assert!(is_valid_subnet_mask(255, 255, 255, 0));
        assert!(is_valid_subnet_mask(0, 0, 0, 0));
        assert!(!is_valid_subnet_mask(255, 255, 255, 1));
        assert!(looks_like_subnet_mask(255, 255, 255, 1));
        assert!(!looks_like_subnet_mask(10, 0, 0, 1)); // ordinary IP
                                                       // 24 leading 1-bits → /24.
        assert_eq!(
            nearest_valid_mask(255, 255, 255, 1).as_deref(),
            Some("255.255.255.0")
        );
    }

    #[test]
    fn dotted_quad_scanner_matches_regex_behaviour() {
        let q = |t, n| {
            super::find_dotted_quads(t, n)
                .into_iter()
                .map(|q| (q.start, q.octets))
                .collect::<Vec<_>>()
        };
        // A clean quad is found with its octet substrings and start.
        assert_eq!(q("ip 192.168.1.1!", 3), vec![(3, ["192", "168", "1", "1"])]);
        // A 4-digit octet defeats the 3-digit cap (no leading boundary
        // realignment), exactly like `\b\d{1,3}`.
        assert!(q("1234.1.1.1", 3).is_empty());
        // The 4-digit cap accepts `999` and a 4-digit octet.
        assert_eq!(q("192.168.1.999", 4), vec![(0, ["192", "168", "1", "999"])]);
        // Two quads, non-overlapping; an embedding word char blocks the
        // boundary (`a10.0.0.1` has no leading `\b`).
        assert!(q("a10.0.0.1", 3).is_empty());
    }

    #[test]
    fn ipv6_candidate_scanner_extracts_runs() {
        let c = super::find_ipv6_candidates("addr fe80::1 end");
        assert_eq!(c, vec!["fe80::1"]);
        // A bare hextet pair (only one colon → <2 groups) is not a
        // candidate; a full address is.
        assert!(super::find_ipv6_candidates("ab:cd").is_empty());
        assert_eq!(
            super::find_ipv6_candidates("2001:db8::8a2e:370:7334"),
            vec!["2001:db8::8a2e:370:7334"]
        );
    }

    #[test]
    fn redos_shape_detector() {
        assert!(super::has_redos_shape("(a+)+"));
        assert!(super::has_redos_shape("(a*)*"));
        assert!(super::has_redos_shape("(a|a)+"));
        assert!(super::has_redos_shape("(foo|bar){2}"));
        // No nested quantifier / overlapping alternation → safe.
        assert!(!super::has_redos_shape("^[a-z]+$"));
        assert!(!super::has_redos_shape("(abc)+"));
        assert!(!super::has_redos_shape("a|b|c"));
    }

    fn w108(src: &str, dialect: &str) -> Vec<(u32, usize)> {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code == "W108")
            .map(|d| {
                let ch = d
                    .message
                    .chars()
                    .find(|c| !c.is_ascii())
                    .map_or(0, |c| c as u32);
                (ch, d.fixes.len())
            })
            .collect()
    }

    #[test]
    fn w108_flags_confusables_and_artifacts() {
        // Smart quotes (auto-fix artifacts) → two W108 with fixes.
        assert_eq!(
            w108("set x \u{201c}hi\u{201d}\n", "tcl8.6"),
            vec![(0x201c, 1), (0x201d, 1)],
        );
        // NBSP and em-dash → W108 with an ASCII fix.
        assert_eq!(w108("set x \u{a0}y\n", "tcl8.6"), vec![(0xa0, 1)]);
        assert_eq!(w108("set x \u{2014}\n", "tcl8.6"), vec![(0x2014, 1)]);
    }

    #[test]
    fn w108_confusables_mode_ignores_benign_unicode() {
        // `é` is not a confusable / artifact → silent in confusables mode.
        assert!(w108("puts caf\u{e9}\n", "tcl8.6").is_empty());
        // Plain ASCII → silent.
        assert!(w108("set x hello\n", "tcl8.6").is_empty());
        // The command word itself is not scanned.
        assert!(w108("\u{440}uts x\n", "tcl8.6").is_empty());
    }

    #[test]
    fn w108_strict_mode_flags_all_non_ascii_for_irules() {
        // F5 iRules default to strict — every non-ASCII char fires,
        // including `é` (which has no ASCII equivalent → no fix).
        assert_eq!(w108("puts caf\u{e9}\n", "f5-irules"), vec![(0xe9, 0)]);
    }

    fn w108_mode(src: &str, dialect: &str, mode: crate::analyser::NonAsciiMode) -> Vec<u32> {
        let mut a = crate::analyser::Analyser::new().with_non_ascii_mode(mode);
        let mut out: Vec<u32> = a
            .analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code == "W108")
            .map(|d| {
                d.message
                    .chars()
                    .find(|c| !c.is_ascii())
                    .map_or(0, |c| c as u32)
            })
            .collect();
        out.sort_unstable();
        out
    }

    #[test]
    fn is_benign_unicode_matches_reference() {
        // Cross-checked against `_style.py::_is_benign_unicode`.
        for cp in [
            0x00E9, 0x00B0, 0x00B5, 0x2212, 0x4E2D, 0xFFFD, 0x2014, 0x2026,
        ] {
            assert!(
                super::is_benign_unicode(char::from_u32(cp).unwrap()),
                "U+{cp:04X}"
            );
        }
        for cp in [0x200B, 0x00A0, 0x0007, 0x202E] {
            assert!(
                !super::is_benign_unicode(char::from_u32(cp).unwrap()),
                "U+{cp:04X}"
            );
        }
    }

    #[test]
    fn w108_off_mode_disables_entirely() {
        use crate::analyser::NonAsciiMode::Off;
        // Even smart quotes / NBSP are silent when W108 is off.
        assert!(w108_mode("set x \u{201c}hi\u{201d}\u{a0}\n", "tcl8.6", Off).is_empty());
        // ...and off wins even for iRules (which would otherwise be strict).
        assert!(w108_mode("puts caf\u{e9}\n", "f5-irules", Off).is_empty());
    }

    #[test]
    fn w108_strict_mode_explicit_flags_all_in_plain_tcl() {
        use crate::analyser::NonAsciiMode::Strict;
        // Explicit strict flags `é` even in a non-F5 dialect.
        assert_eq!(w108_mode("puts caf\u{e9}\n", "tcl8.6", Strict), vec![0xe9]);
    }

    #[test]
    fn w108_common_mode_allows_intentional_unicode() {
        use crate::analyser::NonAsciiMode::Common;
        // Benign letters / symbols / punctuation in any script are allowed.
        assert!(w108_mode("set x caf\u{e9}\n", "tcl8.6", Common).is_empty()); // é (Ll)
        assert!(w108_mode("set x 90\u{b0}\n", "tcl8.6", Common).is_empty()); // ° (So)
        assert!(w108_mode("set x \u{4e2d}\n", "tcl8.6", Common).is_empty()); // 中 (Lo)
    }

    #[test]
    fn w108_common_mode_flags_confusables_and_non_benign() {
        use crate::analyser::NonAsciiMode::Common;
        // Confusables / auto-fix artifacts still fire in common mode.
        assert_eq!(
            w108_mode("set x \u{201c}\n", "tcl8.6", Common),
            vec![0x201c]
        );
        // Non-benign characters (control / zero-width / format) fire even
        // without being confusables — these are the encoding-issue chars
        // `common` mode is meant to catch.
        assert_eq!(
            w108_mode("set x a\u{200b}b\n", "tcl8.6", Common),
            vec![0x200b]
        ); // ZWSP (Cf)
        assert_eq!(
            w108_mode("set x a\u{202e}b\n", "tcl8.6", Common),
            vec![0x202e]
        ); // RLO (Cf)
    }

    #[test]
    fn w104_flags_space_padded_append() {
        assert_eq!(code_sevs("append x \" foo\"\n", "W104"), vec!["Hint"]);
        assert_eq!(code_sevs("append result \"item \"\n", "W104"), vec!["Hint"]);
        assert!(code_sevs("append x foo\n", "W104").is_empty());
        assert!(code_sevs("lappend x foo\n", "W104").is_empty());
    }

    #[test]
    fn w106_flags_unbraced_switch_body() {
        // Alternating unbraced body (no sub → WARNING; sub → ERROR).
        assert_eq!(code_sevs("switch $v a body\n", "W106"), vec!["Warning"]);
        assert_eq!(code_sevs("switch $v $pat $body\n", "W106"), vec!["Error"]);
        // Braced forms are fine.
        assert!(code_sevs("switch $v {a {x} b {y}}\n", "W106").is_empty());
        assert!(code_sevs("switch -regexp $v {a {x}}\n", "W106").is_empty());
        assert!(code_sevs("switch $v { a body }\n", "W106").is_empty());
    }

    fn w100_sev(src: &str) -> Vec<String> {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == "W100")
            .map(|d| format!("{:?}", d.severity))
            .collect()
    }

    #[test]
    fn w100_flags_unbraced_expr_with_substitution() {
        // Matches the live Python analyser (ERROR when a `$`/`[` sub).
        assert_eq!(w100_sev("if $x {puts hi}\n"), vec!["Error"]);
        assert_eq!(w100_sev("while $cond {}\n"), vec!["Error"]);
        assert_eq!(w100_sev("expr $a + $b\n"), vec!["Error"]);
        assert_eq!(w100_sev("expr \"$a == $b\"\n"), vec!["Error"]);
        assert_eq!(w100_sev("for {set i 0} $i<10 {incr i} {}\n"), vec!["Error"]);
    }

    #[test]
    fn w100_skips_braced_and_safe_literals() {
        assert!(w100_sev("if {$x} {puts hi}\n").is_empty());
        assert!(w100_sev("expr {$a + $b}\n").is_empty());
        assert!(w100_sev("expr 1+2\n").is_empty());
        assert!(w100_sev("if 1 {puts hi}\n").is_empty());
        assert!(w100_sev("if {1} {puts hi}\n").is_empty());
    }

    #[test]
    fn is_safe_literal_expr_classifies() {
        assert!(is_safe_literal("42"));
        assert!(is_safe_literal("true"));
        assert!(!is_safe_literal("$x"));
        assert!(is_safe_literal_expr("1 + 2", "tcl8.6"));
        assert!(!is_safe_literal_expr("$a + $b", "tcl8.6"));
    }

    fn w212_count(src: &str) -> usize {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == "W212")
            .count()
    }

    #[test]
    fn w212_flags_name_position_substitution() {
        // Matches the live Python analyser.
        assert_eq!(w212_count("set $x 1\n"), 1);
        assert_eq!(w212_count("incr $counter\n"), 1);
        assert_eq!(w212_count("info exists $v\n"), 1);
        assert_eq!(w212_count("upvar 1 a $b\n"), 1);
    }

    #[test]
    fn w212_ignores_plain_names() {
        assert_eq!(w212_count("set x 1\n"), 0);
        assert_eq!(w212_count("info exists v\n"), 0);
        // A `$`-value in a non-name position is fine.
        assert_eq!(w212_count("set x $y\n"), 0);
    }

    #[test]
    fn name_arg_indices_resolvers() {
        assert_eq!(name_arg_indices("set", &["a".into(), "b".into()]), vec![0]);
        assert_eq!(
            name_arg_indices("unset", &["-nocomplain".into(), "a".into(), "b".into()]),
            vec![1, 2],
        );
        assert_eq!(
            name_arg_indices("info", &["exists".into(), "v".into()]),
            vec![1]
        );
        assert_eq!(
            name_arg_indices("info", &["level".into()]),
            Vec::<usize>::new()
        );
        assert_eq!(
            name_arg_indices("upvar", &["1".into(), "a".into(), "b".into()]),
            vec![2],
        );
    }

    #[test]
    fn w114_flags_nested_expr_in_expr_context() {
        // Matches the live Python analyser.
        assert_eq!(w114_codes("expr {[expr {$x + 1}]}\n"), 1);
        assert_eq!(w114_codes("if {[expr {$x}]} {puts hi}\n"), 1);
    }

    #[test]
    fn w114_ignores_non_expr_context_and_plain_expr() {
        // `set y [expr {…}]` is a command substitution value, not a
        // nested expr context — no W114.
        assert_eq!(w114_codes("set y [expr {1+2}]\n"), 0);
        // A plain braced expr is fine.
        assert_eq!(w114_codes("expr {$x + 1}\n"), 0);
    }

    #[test]
    fn first_nested_expr_finds_bracketed_expr() {
        assert_eq!(first_nested_expr("{[expr {$x}]}"), Some((1, 11)));
        assert_eq!(first_nested_expr("{$x + 1}"), None);
        assert_eq!(first_nested_expr("[express]"), None); // not `expr` + ws
    }

    #[test]
    fn body_references_param_bare_dollar() {
        assert!(body_references_param("set y $x", "x"));
        assert!(body_references_param("return [expr {$a + $b}]", "a"));
        assert!(body_references_param("return [expr {$a + $b}]", "b"));
        assert!(body_references_param("puts [list $val 1]", "val"));
    }

    #[test]
    fn body_references_param_braced_dollar() {
        assert!(body_references_param("set y ${x}", "x"));
        assert!(body_references_param("puts \"got ${val}!\"", "val"));
    }

    #[test]
    fn body_references_param_no_match_for_substring_only() {
        // ``$abc`` must not match ``ab`` (boundary check).
        assert!(!body_references_param("set y $abc", "ab"));
        assert!(!body_references_param("puts $foobar", "foo"));
    }

    #[test]
    fn body_references_param_skips_backslash_escape() {
        // ``\$x`` is a literal dollar — not a substitution.
        assert!(!body_references_param("puts \\$x", "x"));
    }

    #[test]
    fn body_references_param_handles_multiple_uses() {
        assert!(body_references_param("set y $x; set z $x", "x"));
    }

    #[test]
    fn body_references_param_misses_when_unused() {
        assert!(!body_references_param("puts hello", "x"));
        assert!(!body_references_param("return 42", "y"));
    }

    #[test]
    fn body_references_param_braced_with_punct_after() {
        // ``${x}foo`` is a valid substitution — boundary not
        // required inside braces.
        assert!(body_references_param("set y ${x}foo", "x"));
    }

    #[test]
    fn body_references_param_namespace_qualified() {
        // ``$ns::var`` is a qualified variable; the param name
        // is the leading identifier.  Boundary on ``::`` is
        // OK — both are part of the qualified name; the W214
        // emitter passes the bare param so this is a non-issue
        // in practice.  Test pins the boundary semantics.
        assert!(!body_references_param("set y $ns::var", "ns"));
    }

    fn diag(code: &str, span: Span, msg: &str) -> Diagnostic {
        Diagnostic {
            code: code.to_string(),
            span,
            message: msg.to_string(),
            severity: Severity::Warning,
            fixes: Vec::new(),
        }
    }

    #[test]
    fn w004_fires_on_regsub_command_in_tcl86() {
        // `regsub -command` is Tcl 9.0+ (TIP 463); on Tcl 8.6 it
        // should produce a W004 dialect-availability warning.
        let mut a = Analyser::new();
        let result = a.analyse("regsub -command {[A-Z]+} foo {bar} out", "tcl8.6");
        let w004: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W004")
            .collect();
        assert!(
            !w004.is_empty(),
            "expected W004 on tcl8.6 regsub -command, got {:?}",
            result.diagnostics
        );
        assert!(w004[0].message.contains("-command"));
        assert!(w004[0].message.contains("regsub"));
    }

    // -- SYNC-MAY21-3 (#460 / #455): E002 / E003 arity ---------------

    #[test]
    fn e003_not_emitted_for_leading_switches() {
        // Regression for #455: declared option flags must be skipped
        // before counting positional args.  `regsub` (max arity 4)
        // previously tripped a false E003 once any switch appeared.
        // These switches exist in every supported dialect.
        for snippet in [
            "regsub -all -line {x} $args {} str",
            "regsub -all {a} $b {} c",
            "regsub -nocase -all -- $pat $s {} out",
        ] {
            let mut a = Analyser::new();
            let result = a.analyse(snippet, "tcl8.6");
            let e003: Vec<&Diagnostic> = result
                .diagnostics
                .iter()
                .filter(|d| d.code == "E003")
                .collect();
            assert!(e003.is_empty(), "unexpected E003 for {snippet:?}: {e003:?}");
        }
    }

    #[test]
    fn e003_fires_on_genuine_over_arity() {
        // 5 positional args for `regsub` (max 4) is a real error.
        let mut a = Analyser::new();
        let result = a.analyse("regsub a b c d e", "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "E003"),
            "expected E003, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn e003_switch_options_are_dialect_filtered() {
        // `regsub -command` is Tcl 9.0+ (TIP 463).
        // Under 9.0 it is a real switch → skipped → 4 positional → OK.
        let mut a = Analyser::new();
        let r9 = a.analyse("regsub -command a b c d", "tcl9.0");
        assert!(
            !r9.diagnostics.iter().any(|d| d.code == "E003"),
            "unexpected E003 under tcl9.0: {:?}",
            r9.diagnostics
        );
        // Under 8.6 `-command` is unknown → counted positional →
        // 5 > max 4 → E003 (the #460 dialect-leak guard).
        let mut a2 = Analyser::new();
        let r8 = a2.analyse("regsub -command a b c d", "tcl8.6");
        assert!(
            r8.diagnostics.iter().any(|d| d.code == "E003"),
            "expected E003 under tcl8.6, got {:?}",
            r8.diagnostics
        );
    }

    #[test]
    fn e003_suppressed_by_expanded_word() {
        // `{*}$rest` expands to an unknown count, so the expanded word
        // is excluded from the positional lower bound: `regsub a b c d
        // {*}$rest` has 4 non-expanded positional words (≤ max 4) and
        // must not trip E003, whereas the same five literal words do.
        let mut a = Analyser::new();
        let expanded = a.analyse("regsub a b c d {*}$rest", "tcl8.6");
        assert!(
            !expanded.diagnostics.iter().any(|d| d.code == "E003"),
            "expansion should suppress E003: {:?}",
            expanded.diagnostics
        );
        let mut b = Analyser::new();
        let literal = b.analyse("regsub a b c d e", "tcl8.6");
        assert!(
            literal.diagnostics.iter().any(|d| d.code == "E003"),
            "control: five literal words should fire E003: {:?}",
            literal.diagnostics
        );
    }

    #[test]
    fn e003_arity_is_dialect_aware_via_expand_syntax() {
        // SYNC-MAY19-dialect-contextvar: end-to-end proof that the
        // document dialect reaches the analyser's segmenter (and thus the
        // lexer's `expand_syntax` flag).  `{*}` is the expansion operator
        // on 8.5+ but a literal brace word on 8.4, so for
        // `regsub a b c d {*}$rest`:
        //   * tcl8.4 — `{*}$rest` is a 5th literal positional word; 5 > max
        //     4 → E003 fires.
        //   * tcl9.0 — `{*}$rest` expands, contributing an unbounded count;
        //     the 4 non-expanded words are ≤ max 4 → E003 is suppressed.
        // Before the dialect → `LexerConfig` wiring the analyser always
        // lexed with `expand_syntax` on, so 8.4 wrongly behaved like 9.0
        // (no E003) — this asserts the two now diverge.
        let codes = |dialect: &str| -> Vec<String> {
            let mut a = Analyser::new();
            a.analyse("regsub a b c d {*}$rest", dialect)
                .diagnostics
                .iter()
                .map(|d| d.code.clone())
                .collect()
        };
        let on_84 = codes("tcl8.4");
        assert!(
            on_84.iter().any(|c| c == "E003"),
            "8.4 treats `{{*}}` as a literal word → 5 positional args → E003: {on_84:?}",
        );
        let on_90 = codes("tcl9.0");
        assert!(
            !on_90.iter().any(|c| c == "E003"),
            "9.0 expands `{{*}}` → 4 positional words ≤ max → no E003: {on_90:?}",
        );
    }

    // -- subcommand-level E003 arity (per-subcommand signatures) -----

    #[test]
    fn e003_fires_on_subcommand_over_arity() {
        // `string length` takes exactly one argument — three positional
        // words must trip E003.
        let mut a = Analyser::new();
        let result = a.analyse("string length a b c", "tcl8.6");
        let e003: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == "E003")
            .collect();
        assert!(
            !e003.is_empty(),
            "expected E003 for `string length a b c`, got {:?}",
            result.diagnostics
        );
        assert!(
            e003[0].message.contains("string length"),
            "message should name the subcommand: {:?}",
            e003[0].message
        );
    }

    #[test]
    fn e003_fires_on_file_link_over_arity() {
        // `file link ?-linktype? linkName ?target?` — `link` accepts at
        // most two positional args, so three literal targets is E003.
        let mut a = Analyser::new();
        let result = a.analyse("file link $a $b $c", "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "E003"),
            "expected E003 for `file link $a $b $c`, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn e003_silent_for_subcommand_leading_options() {
        // Per-subcommand options (`file link -symbolic` / `-hard`,
        // `string match -nocase`) must be skipped before counting
        // positionals, so these well-formed calls stay silent.
        for snippet in [
            "file link -symbolic $a $b",
            "file link -hard $a $b",
            "string match -nocase $a $b",
        ] {
            let mut a = Analyser::new();
            let result = a.analyse(snippet, "tcl8.6");
            let e003: Vec<&Diagnostic> = result
                .diagnostics
                .iter()
                .filter(|d| d.code == "E003")
                .collect();
            assert!(e003.is_empty(), "unexpected E003 for {snippet:?}: {e003:?}");
        }
    }

    #[test]
    fn subcommand_arity_skips_unknown_and_dynamic_subcommands() {
        // An unknown subcommand is W001's job, not E003; a dynamic
        // subcommand word (`$sub`) can't be resolved, so neither path
        // should emit E003.
        for snippet in ["string $sub a b c", "string [x] a b c"] {
            let mut a = Analyser::new();
            let result = a.analyse(snippet, "tcl8.6");
            assert!(
                !result.diagnostics.iter().any(|d| d.code == "E003"),
                "unexpected E003 for {snippet:?}: {:?}",
                result.diagnostics
            );
        }
    }

    #[test]
    fn e002_fires_on_too_few_args() {
        // `regsub` requires at least 3 args (exp string subSpec).
        let mut a = Analyser::new();
        let result = a.analyse("regsub a b", "tcl8.6");
        let e002: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == "E002")
            .collect();
        assert!(
            !e002.is_empty(),
            "expected E002 for `regsub a b`, got {:?}",
            result.diagnostics
        );
        assert!(e002[0].message.contains("at least 3"));
    }

    #[test]
    fn e003_shadow_is_namespace_scoped() {
        // PR #472 review (Codex): a namespaced proc named `close` must
        // NOT suppress arity checks on a *global* `close` call (which
        // resolves to the builtin, max 2), but must suppress a `close`
        // call inside its own namespace (which resolves to the proc).
        let src = "proc ::ns::close {a b c d} {}\n\
                   close x y z\n\
                   namespace eval ::ns { close x y z }\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let e003: Vec<&Diagnostic> = r.diagnostics.iter().filter(|d| d.code == "E003").collect();
        assert_eq!(
            e003.len(),
            1,
            "expected exactly one E003 (the global close), got {:?}",
            r.diagnostics
        );
        // The flagged call must be the top-level one, before the
        // `namespace eval` body (both call sites share the same text).
        let ns_eval_off = src.find("namespace eval").unwrap();
        let span = e003[0].span;
        assert!(
            (span.start() as usize) < ns_eval_off,
            "flagged the namespaced call instead of the global one: {:?}",
            &src[span.start() as usize..span.end() as usize],
        );
    }

    // -- SYNC-MAY31-9 (#475): reachable, in-order shadow gating -------

    #[test]
    fn e003_top_level_call_before_shadowing_proc_fires() {
        // A top-level `close x y z` *before* `proc close` resolves to
        // the builtin at load time (the proc does not exist yet), so the
        // builtin arity check must fire even though a same-named proc is
        // defined later in the file.  Regression target for #475's
        // in-order gate (without it the post-walk flush silenced this).
        let src = "close x y z\nproc close {a b c d} {}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let e003: Vec<&Diagnostic> = r.diagnostics.iter().filter(|d| d.code == "E003").collect();
        assert_eq!(
            e003.len(),
            1,
            "expected E003 on the top-level close before its shadowing proc, got {:?}",
            r.diagnostics
        );
        // The flagged call is the top-level one (offset 0), not the proc.
        assert_eq!(e003[0].span.start(), 0, "wrong call flagged");
    }

    #[test]
    fn e003_top_level_call_after_shadowing_proc_suppressed() {
        // The mirror image: once `proc close` is defined, a later
        // top-level `close x y z` resolves to the 4-param user proc, so
        // the builtin arity check is suppressed.
        let src = "proc close {a b c d} {}\nclose x y z\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "E003"),
            "no E003 expected — the call follows its shadowing proc, got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn e003_proc_body_call_not_order_gated() {
        // A call inside a proc body resolves when that proc is *invoked*
        // — after the whole script has loaded — so a shadowing proc
        // defined later in the file still suppresses the builtin check.
        // Order is only enforced for top-level calls.
        let src = "proc foo {} { close x y z }\nproc close {a b c d} {}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "E003"),
            "no E003 expected — proc-body calls are not order-gated, got {:?}",
            r.diagnostics
        );
    }

    // -- SYNC-MAY31-4 (#501): BODY role on iRules nesting scripts ------

    #[test]
    fn analyser_recurses_into_irules_nesting_script_bodies() {
        // clientside / serverside / peer / after now carry an
        // `ArgRole::Body`, so the analyser descends into the nesting
        // script and flags problems inside it.  A nested `set` with no
        // arguments trips E002 only when the body is actually analysed —
        // i.e. the generic body-walk picks the role up automatically.
        for src in [
            "when CLIENT_DATA { clientside { set } }",
            "when CLIENT_ACCEPTED { serverside { set } }",
            "when CLIENT_ACCEPTED { peer { set } }",
            "when RULE_INIT { after 1000 { set } }",
        ] {
            let mut a = Analyser::new();
            let r = a.analyse(src, "f5-irules");
            assert!(
                r.diagnostics
                    .iter()
                    .any(|d| d.code == "E002" && d.message.contains("'set'")),
                "expected E002 from the nested `set` (body must be analysed) in {src:?}, got {:?}",
                r.diagnostics
            );
        }
    }

    #[test]
    fn w004_fires_on_lsearch_stride_in_tcl85() {
        // PR #441 review (Codex): the W004 coverage requires the
        // option to exist in the registry.  `lsearch -stride` was
        // populated as part of this review fix.
        let mut a = Analyser::new();
        let result = a.analyse("lsearch -stride 2 {a b c d} b", "tcl8.5");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W004"),
            "expected W004 on tcl8.5 lsearch -stride, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w004_silent_on_lsearch_stride_in_tcl86() {
        let mut a = Analyser::new();
        let result = a.analyse("lsearch -stride 2 {a b c d} b", "tcl8.6");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "W004"),
            "W004 must not fire on tcl8.6 lsearch -stride, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w004_fires_on_clock_scan_validate_in_tcl86() {
        // `clock scan -validate` is Tcl 9.0+ (TIP 532); the
        // subcommand-scoped option table consults the active
        // dialect via the W004 emitter's `sub_match` branch.
        let mut a = Analyser::new();
        let result = a.analyse("clock scan {today} -validate 1", "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W004"),
            "expected W004 on tcl8.6 clock scan -validate, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w004_fires_on_fconfigure_nodelay_in_tcl86() {
        // `fconfigure -nodelay` is Tcl 9.0+ (TIP 528).
        let mut a = Analyser::new();
        let result = a.analyse("fconfigure $chan -nodelay 1", "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W004"),
            "expected W004 on tcl8.6 fconfigure -nodelay, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w004_fires_on_chan_configure_inputmode_in_tcl86() {
        // Subcommand-scoped option: `chan configure -inputmode` is
        // Tcl 9.0+ (TIP 160).
        let mut a = Analyser::new();
        let result = a.analyse("chan configure $chan -inputmode raw", "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W004"),
            "expected W004 on tcl8.6 chan configure -inputmode, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w004_silent_on_regsub_command_in_tcl9() {
        // Same input on Tcl 9.0 — option is supported, no W004.
        let mut a = Analyser::new();
        let result = a.analyse("regsub -command {[A-Z]+} foo {bar} out", "tcl9.0");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "W004"),
            "W004 should not fire on tcl9.0, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w003_fires_on_string_compare_in_tcl84() {
        // `lt` / `le` / `gt` / `ge` are Tcl 9.0+ (TIP 461); on
        // Tcl 8.4 / 8.5 / 8.6 they should produce W003.
        let mut a = Analyser::new();
        let result = a.analyse("if {$x lt $y} { puts hi }", "tcl8.4");
        let w003: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W003")
            .collect();
        assert!(
            !w003.is_empty(),
            "expected W003 on tcl8.4 'lt' operator, got {:?}",
            result.diagnostics
        );
        assert!(w003[0].message.contains("'lt'"));
    }

    #[test]
    fn w003_silent_on_string_compare_in_tcl9() {
        let mut a = Analyser::new();
        let result = a.analyse("if {$x lt $y} { puts hi }", "tcl9.0");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "W003"),
            "W003 should not fire on tcl9.0, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w003_fires_on_in_operator_in_tcl84() {
        // `in` / `ni` are Tcl 8.5+ (TIP 201).
        let mut a = Analyser::new();
        let result = a.analyse("if {$x in {a b c}} { puts hi }", "tcl8.4");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W003"),
            "expected W003 on tcl8.4 'in' operator, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w003_fires_on_tab_separated_operator() {
        // PR #441 review (Codex): the prefilter must tolerate any
        // whitespace, not just literal spaces.  `if {$x\tlt\t$y}` is
        // valid Tcl 8.4 syntax that the expr parser handles — the
        // analyser must not skip it because we only checked for
        // space-delimited operators.
        let mut a = Analyser::new();
        let result = a.analyse("if {$x\tlt\t$y} { puts hi }", "tcl8.4");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W003"),
            "W003 must fire on tab-separated 'lt', got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w003_fires_on_newline_separated_operator() {
        // Same shape with a newline boundary — also valid Tcl.
        let mut a = Analyser::new();
        let result = a.analyse("if {$x\nin\n{a b c}} { puts hi }", "tcl8.4");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W003"),
            "W003 must fire on newline-separated 'in', got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn contains_gated_word_handles_boundaries() {
        // No false positives on identifiers that contain the keyword.
        assert!(!contains_gated_word("$alt"));
        assert!(!contains_gated_word("$align"));
        assert!(!contains_gated_word("inner"));
        assert!(!contains_gated_word("$gem"));
        // Real matches at word boundaries.
        assert!(contains_gated_word("$x lt $y"));
        assert!(contains_gated_word("$x\tlt\t$y"));
        assert!(contains_gated_word("($x)lt($y)"));
        assert!(contains_gated_word("lt $y"));
        assert!(contains_gated_word("$x lt"));
    }

    #[test]
    fn w003_silent_on_in_operator_in_tcl85() {
        let mut a = Analyser::new();
        let result = a.analyse("if {$x in {a b c}} { puts hi }", "tcl8.5");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "W003"),
            "W003 should not fire on tcl8.5, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn emit_variable_usage_diagnostics_is_a_noop() {
        // Hook is intentionally empty — running it must leave
        // the diagnostics list untouched.
        let mut a = Analyser::new();
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "x"));
        a.emit_variable_usage_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 1);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_runs_without_panicking_on_empty_source() {
        // Smoke test — the orchestrator handles empty input
        // gracefully (an empty CompilationUnit yields no
        // diagnostics).
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("");
        assert!(a.result.diagnostics.is_empty());
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_no_w220_on_simple_assignment() {
        // ``set x 1`` — single assignment, no overwrite, no
        // W220.  Smoke test that pipeline runs without
        // emitting spurious W codes for clean code.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set x 1");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W220"),
            "W220 must not fire on a single assignment; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w220_dead_store_overwritten() {
        // ``set x 1\nset x 2\nputs $x`` — the first ``set x 1``
        // is overwritten before being read.  W220 should fire
        // at the first assignment.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set x 1\nset x 2\nputs $x");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            !w220s.is_empty(),
            "W220 expected for overwritten ``set x 1``; got {:?}",
            a.result.diagnostics,
        );
        assert!(w220s.iter().any(|d| d.message.contains("'x'")));
        assert_eq!(w220s[0].severity, Severity::Hint);
    }

    #[test]
    fn w220_array_element_overwrite_not_dead() {
        // SYNC-MAY31-1b (place model): `set a(k) 1` is NOT a dead store even
        // though the later `set a(j) 2` bumps the name-level SSA version of the
        // base `a`.  The place model sees `a(k)` is read by `puts $a(k)` and
        // that `a(k)` ≠ `a(j)`, so the false W220 on the first write is
        // suppressed.  Goes through `analyse` (the production path) so the
        // registry — which the place bridge needs — is bound.
        let mut a = Analyser::new();
        let r = a.analyse("proc f {} { set a(k) 1; set a(j) 2; puts $a(k) }", "tcl8.6");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W220"),
            "no W220 expected — a(k) is read by `puts $a(k)`; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w220_scalar_overwrite_still_fires_via_analyse() {
        // Regression guard for the element-granular scope of the suppression:
        // a genuine *scalar* overwrite must still fire W220 with the place
        // model active (scalars don't fold, so the name-level verdict stands).
        let mut a = Analyser::new();
        let r = a.analyse("proc f {} { set x 1; set x 2; puts $x }", "tcl8.6");
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == "W220" && d.message.contains("'x'")),
            "scalar dead store must still fire; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w220_braced_literal_arg_is_not_a_read() {
        // A braced word performs no `$`-substitution, so `puts {$a(k)}` does
        // NOT read `a(k)` — the place bridge must not treat the de-braced IR
        // text as a read and wrongly suppress the genuine dead store on the
        // first `set a(k)`.  (`puts $a(j)` keeps the base `a` live so `a(k)`
        // is a W220 overwrite candidate rather than a W211.)
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc f {} { set a(k) 1; set a(j) 2; puts $a(j); puts {$a(k)} }",
            "tcl8.6",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "W220"),
            "braced literal must not suppress the a(k) dead store; got {:?}",
            r.diagnostics,
        );
    }

    /// W220-IR-paths.  Variables prefixed with ``::`` are
    /// externally consumed (other namespaces, the global frame
    /// outside this file) — Python's ``_dead_stores`` skips
    /// them in `core_analyses.py:1147-1148`, and the Rust port
    /// must too.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_skips_global_qualified_var() {
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set ::x 1\nset ::x 2\nputs $::x");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.is_empty(),
            "W220 must skip ``::``-prefixed globals; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  ``set x [foo]`` is a side-effecting
    /// store: dropping the assignment would also drop the call
    /// to ``foo``.  Python's ``_dead_stores`` filters
    /// ``IRAssignValue`` containing ``[`` (`core_analyses.py:1152`).
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_skips_command_substitution_value() {
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set x [clock seconds]\nset x 2\nputs $x");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.is_empty(),
            "W220 must skip ``set x [cmd]`` side-effecting stores; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  ``set x [expr {[foo]}]`` lowers as
    /// ``IRAssignExpr`` with a command call inside — same
    /// side-effecting reasoning as command-substitution
    /// values.  Python's ``_dead_stores`` filters
    /// ``IRAssignExpr`` whose tree contains an
    /// ``IRExprCommand`` (`core_analyses.py:1154`).
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_skips_expr_with_command_call() {
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set x [expr {[clock seconds] + 1}]\nset x 2\nputs $x");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.is_empty(),
            "W220 must skip ``IRAssignExpr`` containing a command call; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  ``incr x`` is a side-effecting write
    /// (it reads the current value first).  Python's
    /// ``_dead_stores`` only matches ``IRAssignConst`` /
    /// ``IRAssignValue`` / ``IRAssignExpr`` — ``IRIncr`` and
    /// ``IRCall.defs`` are skipped by exclusion.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_skips_incr_writes() {
        let mut a = Analyser::new();
        // ``incr x`` reads x then writes x+1; even when later
        // overwritten, dropping the incr would also drop the
        // implicit read.  Of the three writes to ``x``, only
        // the ``incr`` qualifies as overwritten-before-read
        // (``set x 0`` is read by incr, ``set x 5`` is read
        // by puts), so any W220 on x must be from the incr,
        // and the IR-statement-type filter must drop it.
        a.emit_cfg_ssa_diagnostics("set x 0\nincr x\nset x 5\nputs $x");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220" && d.message.contains("'x'"))
            .collect();
        assert!(
            w220s.is_empty(),
            "W220 must skip ``incr`` side-effecting writes; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  ``lassign $list a b`` defines ``a`` and
    /// ``b`` via ``IRCall.defs`` — a side-effecting write that
    /// can't be dropped without also dropping the call.
    /// Python's ``_dead_stores`` only matches the three
    /// pure-assign IR shapes; ``IRCall`` is skipped by
    /// exclusion.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_skips_call_defs() {
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("lassign {1 2} a b\nset a 5\nputs $a");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.iter().all(|d| !d.message.contains("'a'")),
            "W220 must skip ``IRCall.defs`` side-effecting writes; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  In a ``pkgIndex.tcl`` file, ``$dir`` is
    /// set by the Tcl package loader before the script body
    /// runs — even when the script reassigns it, the original
    /// store can't be considered dead (the loader-supplied
    /// value is the relevant initial state).  Mirrors
    /// `_diagnostics.py:147-149`.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_pkgindex_dir_var_suppressed() {
        let mut a = Analyser::new();
        a.file_path = Some("/some/path/pkgIndex.tcl".to_string());
        a.emit_cfg_ssa_diagnostics("set dir foo\nset dir bar\nputs $dir");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.is_empty(),
            "W220 must suppress ``$dir`` in pkgIndex.tcl; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  Outside ``pkgIndex.tcl``, ``$dir`` is
    /// just a regular variable — no special suppression.
    /// Negative control for the pkgIndex special-case.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_dir_var_not_suppressed_outside_pkgindex() {
        let mut a = Analyser::new();
        a.file_path = Some("/some/path/script.tcl".to_string());
        a.emit_cfg_ssa_diagnostics("set dir foo\nset dir bar\nputs $dir");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            !w220s.is_empty(),
            "W220 must fire on ``$dir`` outside pkgIndex.tcl; got {:?}",
            a.result.diagnostics,
        );
        assert!(w220s.iter().any(|d| d.message.contains("'dir'")));
    }

    /// W220-IR-paths.  Variables shared across iRule events
    /// via ``::when::*`` procs (collected in
    /// ``ConnectionScope::cross_event_imports``) may be read
    /// in a different event from where they're set — the
    /// local "no use" verdict is unsafe.  Mirrors
    /// `_diagnostics.py:165-167`.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_irules_cross_event_var_suppressed() {
        let mut a = Analyser::new();
        a.dialect = "f5-irules".to_string();
        // ``HTTP_REQUEST`` writes ``v``, ``HTTP_RESPONSE``
        // reads ``v`` — ``v`` is a cross-event def.  The
        // ``set v 1\nset v 2`` shape inside ``HTTP_REQUEST``
        // would normally fire W220 on the first ``set v 1``,
        // but cross-event suppression should drop it.
        a.emit_cfg_ssa_diagnostics(
            "when HTTP_REQUEST {\n  set v 1\n  set v 2\n}\nwhen HTTP_RESPONSE {\n  log local0. $v\n}",
        );
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.iter().all(|d| !d.message.contains("'v'")),
            "W220 must suppress vars shared across iRule events; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  Negative control: a proc-local variable
    /// (NOT shared across events) inside a ``::when::*`` proc
    /// is still subject to W220.  Confirms the cross-event
    /// filter is targeted, not a blanket
    /// "skip everything in `::when::`*" rule.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_irules_proc_local_still_flagged() {
        let mut a = Analyser::new();
        a.dialect = "f5-irules".to_string();
        // ``local`` is only used inside HTTP_REQUEST — not a
        // cross-event var, so W220 should still fire on the
        // overwritten first assignment.
        a.emit_cfg_ssa_diagnostics(
            "when HTTP_REQUEST {\n  set local 1\n  set local 2\n  log local0. $local\n}",
        );
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.iter().any(|d| d.message.contains("'local'")),
            "W220 must still fire for proc-local vars in ::when::*; got {:?}",
            a.result.diagnostics,
        );
    }

    /// W220-IR-paths.  Dead stores in SCCP-unreachable blocks
    /// are reported as O107 by the optimiser; the analyser
    /// must not double-report them as W220.  Mirrors Python's
    /// ``_dead_stores`` `executable_blocks` filter
    /// (`core_analyses.py:1112-1140`).
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_skips_unreachable_block() {
        let mut a = Analyser::new();
        // ``if {0} { ... }`` makes the then-branch unreachable
        // under SCCP.  Any dead store inside is suppressed.
        a.emit_cfg_ssa_diagnostics("if {0} {\n  set x 1\n  set x 2\n  puts $x\n}\nputs done");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.is_empty(),
            "W220 must skip dead stores in SCCP-unreachable blocks; got {w220s:?}",
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w214_unused_param() {
        // ``proc foo {x y} { puts $x }`` — parameter ``y`` is
        // declared but never read in the body.  W214 should
        // fire on it.  Parameter ``x`` is read, so no W214.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {x y} { puts $x }");
        let w214s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W214")
            .collect();
        assert_eq!(
            w214s.len(),
            1,
            "expected exactly one W214 for unused param ``y``; got {:?}",
            a.result.diagnostics,
        );
        assert!(w214s[0].message.contains("'y'"));
        assert!(w214s[0].message.contains("'::foo'"));
        assert_eq!(w214s[0].severity, Severity::Hint);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w211_unused_variable() {
        // ``proc foo {} { set y 1 }`` — y is set, never read,
        // and there's no other version → W211 fires.
        // Top-level test would be subject to global-scope
        // assumptions, so use a proc body where the local-only
        // verdict is safe.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set y 1 }");
        let w211s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W211")
            .collect();
        assert!(
            !w211s.is_empty(),
            "W211 expected for unused var ``y`` in proc foo; got {:?}",
            a.result.diagnostics,
        );
        assert!(w211s[0].message.contains("'y'"));
        assert!(w211s[0].message.contains("set but never used"));
        assert_eq!(w211s[0].severity, Severity::Hint);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w211_w220_skipped_for_traced_var() {
        // A write trace makes `x` observable on every `set`, so neither
        // W211 (unused) nor W220 (dead store) may fire.  Both the 8.5+
        // `trace add variable` and 8.4 `trace variable` spellings count.
        for src in [
            "proc f {} { trace add variable x write cb; set x 1 }",
            "proc f {} { trace variable x w cb; set x 1 }",
        ] {
            let mut a = Analyser::new();
            a.emit_cfg_ssa_diagnostics(src);
            assert!(
                !a.result
                    .diagnostics
                    .iter()
                    .any(|d| matches!(d.code.as_str(), "W211" | "W220")),
                "traced var must not fire W211/W220 for {src:?}; got {:?}",
                a.result.diagnostics,
            );
        }
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w211_skipped_for_textually_referenced() {
        // ``proc foo {} { set msg hello; puts "got $msg" }`` —
        // ``msg`` is referenced inside a quoted string; the
        // textual-reference filter should suppress W211 because
        // the def-use builder doesn't track ``"$msg"`` reads.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set msg hello; puts \"got $msg\" }");
        let w211s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W211" && d.message.contains("'msg'"))
            .collect();
        assert!(
            w211s.is_empty(),
            "W211 must not fire on var referenced via $-interpolation; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w211_skipped_for_global_aliased() {
        // ``proc foo {} { global config; set config 1 }`` —
        // ``config`` is global-aliased; the write goes to the
        // outer scope, so the local "no use" verdict is unsafe.
        // W211 must not fire.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { global config; set config 1 }");
        let w211s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W211" && d.message.contains("'config'"))
            .collect();
        assert!(
            w211s.is_empty(),
            "W211 must not fire on global-aliased var; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_h300_repeated_assignment() {
        // ``proc foo {} { set x 1; set x 1 }`` — same var,
        // same literal value, consecutive statements.  The
        // first is a dead store; H300 fires on the second.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set x 1\nset x 1 }");
        let h300s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "H300")
            .collect();
        assert!(
            !h300s.is_empty(),
            "H300 expected for repeated ``set x 1``; got {:?}",
            a.result.diagnostics,
        );
        assert!(h300s[0].message.contains("'x'"));
        assert!(h300s[0].message.contains("Possible paste error"));
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_h300_skips_underscore_vars() {
        // Vars starting with ``_`` are excluded (the convention
        // for "intentionally unused").
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set _x 1\nset _x 1 }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "H300"),
            "H300 must not fire on underscore-prefixed vars",
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_h300_skips_distinct_values() {
        // Same var, different literal → not a paste error.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set x 1\nset x 2 }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "H300"),
            "H300 must not fire when literal values differ",
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_read_before_set() {
        // ``proc foo {} { puts $undef }`` — undef is not a
        // parameter and not in scope; W210 fires at the use.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { puts $undef }");
        let w210s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W210" && d.message.contains("'undef'"))
            .collect();
        assert!(
            !w210s.is_empty(),
            "W210 expected for read of undef ``$undef``; got {:?}",
            a.result.diagnostics,
        );
        assert_eq!(w210s[0].severity, Severity::Warning);
        assert!(w210s[0].message.contains("read before it is set"));
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_skipped_for_real_param() {
        // ``proc foo {x} { puts $x }`` — x IS a real parameter,
        // so W210 must not fire.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {x} { puts $x }");
        let w210s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W210" && d.message.contains("'x'"))
            .collect();
        assert!(
            w210s.is_empty(),
            "W210 must not fire on real param ``x``; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w213_unset_on_possibly_undef() {
        // ``proc foo {} { unset xs }`` — ``xs`` may not exist;
        // ``unset`` without ``-nocomplain`` would error at
        // runtime.  W213 fires (instead of W210) at the unset
        // statement.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { unset xs }");
        let w213s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W213")
            .collect();
        assert!(
            !w213s.is_empty(),
            "W213 expected for ``unset xs`` on possibly-undef var; got {:?}",
            a.result.diagnostics,
        );
        assert!(w213s[0].message.contains("'xs'"));
        assert!(w213s[0].message.contains("unset -nocomplain"));
        assert_eq!(w213s[0].severity, Severity::Warning);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w213_skipped_with_nocomplain() {
        // ``unset -nocomplain xs`` is the safe form — W213
        // must not fire.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { unset -nocomplain xs }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W213"),
            "W213 must not fire when ``-nocomplain`` is present; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_fires_at_top_level() {
        // **C41e3 follow-up.** Top-level RBS now fires when no
        // proc writes the variable.  ``puts $undef`` reads
        // ``undef`` without any preceding write.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("puts $undef");
        assert!(
            a.result.diagnostics.iter().any(|d| d.code == "W210"),
            "W210 must fire at top-level when no proc writes the var; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_suppressed_when_proc_writes_global() {
        // A helper proc ``init`` writes ``::counter`` via ``set``,
        // so the top-level read should not flag W210 — the proc
        // may run before the read.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc init {} { set ::counter 0 }\nputs $counter");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W210"),
            "W210 must be suppressed for globals written by procs; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_suppressed_via_global_alias() {
        // ``proc init {} { global counter; set counter 0 }`` — the
        // ``global`` declaration aliases the proc-local ``counter``
        // to the global.  Top-level read should not flag W210.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc init {} { global counter; set counter 0 }\nputs $counter");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W210"),
            "W210 must be suppressed via global-alias case; got {:?}",
            a.result.diagnostics,
        );
    }

    // ── SYNC-MAY31-3: info exists / array exists ──────────────────

    fn codes_for(src: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics(src);
        a.result
            .diagnostics
            .iter()
            .map(|d| d.code.clone())
            .collect()
    }

    #[test]
    fn info_exists_control_still_flags_w210() {
        // Baseline: a plain read of an unset local flags W210.
        assert!(codes_for("proc f {} { puts $u }").contains(&"W210".to_string()));
    }

    #[test]
    fn info_exists_guard_narrows_read_in_then_arm() {
        // SYNC-MAY31-3(narrowing): reads inside `if {[info exists X]}`
        // are guarded — X provably exists there, so no W210.
        let codes = codes_for("proc f {} { if {[info exists u]} { puts $u } }");
        assert!(
            !codes.contains(&"W210".to_string()),
            "guarded read must not flag W210; got {codes:?}",
        );
    }

    #[test]
    fn info_exists_read_outside_guard_still_flags_w210() {
        // The narrowing is scoped to the guarded arm: a read after the
        // `if` (not dominated by the guard) still flags W210.
        let codes = codes_for("proc f {} { if {[info exists u]} { puts hi }\nputs $u }");
        assert!(
            codes.contains(&"W210".to_string()),
            "read outside the guarded arm must still flag W210; got {codes:?}",
        );
    }

    #[test]
    fn info_exists_negated_guard_narrows_false_arm() {
        // SYNC-MAY31-3(narrowing): the false arm of `![info exists X]`
        // is guarded.
        let codes = codes_for("proc f {} { if {![info exists u]} { puts no } else { puts $u } }");
        assert!(
            !codes.contains(&"W210".to_string()),
            "false-arm read of `![info exists X]` must not flag W210; got {codes:?}",
        );
    }

    #[test]
    fn info_exists_query_word_not_read_before_set() {
        // SYNC-MAY31-3(W210 suppression): the existence-query word is
        // not a read-before-set — bare call and command-sub forms.
        assert!(!codes_for("proc f {} { info exists u }").contains(&"W210".to_string()));
        assert!(!codes_for("proc f {} { array exists u }").contains(&"W210".to_string()));
        let codes = codes_for("proc f {} { set y [info exists u]; puts $y }");
        assert!(
            !codes.contains(&"W210".to_string()),
            "`set y [info exists u]` must not flag W210 on u; got {codes:?}",
        );
    }

    #[test]
    fn info_exists_folds_false_for_never_defined_local() {
        // SYNC-MAY31-3(fold): a never-defined non-parameter never
        // exists → predicate folds false → I230.
        let codes = codes_for("proc f {a} { if {[info exists b]} { puts hi } }");
        assert!(
            codes.contains(&"I230".to_string()),
            "`info exists` of a never-defined local should fold to I230; got {codes:?}",
        );
    }

    #[test]
    fn info_exists_folds_true_for_parameter() {
        // A parameter always exists → predicate folds true → I230.
        let codes = codes_for("proc f {a} { if {[info exists a]} { puts hi } }");
        assert!(
            codes.contains(&"I230".to_string()),
            "`info exists` of a parameter should fold to I230; got {codes:?}",
        );
    }

    #[test]
    fn info_exists_does_not_fold_conditionally_set_var() {
        // A var that is set on some path is not provably set/unset —
        // no fold, no false I230.
        let codes = codes_for(
            "proc f {flag} { if {$flag} { set u 1 } ; if {[info exists u]} { puts $u } }",
        );
        assert!(
            !codes.contains(&"I230".to_string()),
            "conditionally-set var must not fold; got {codes:?}",
        );
    }

    #[test]
    fn info_exists_does_not_fold_namespaced_or_array() {
        // Array elements / namespaced vars may be populated outside the
        // function's view — never fold them.
        assert!(
            !codes_for("proc f {} { if {[info exists ::env(PATH)]} { puts hi } }")
                .contains(&"I230".to_string())
        );
    }

    #[test]
    fn info_exists_does_not_fold_unset_parameter() {
        // A parameter that is `unset` before the check can't be assumed
        // to exist.
        let codes = codes_for("proc f {a} { unset a; if {[info exists a]} { puts hi } }");
        assert!(
            !codes.contains(&"I230".to_string()),
            "unset parameter must not fold true; got {codes:?}",
        );
    }

    #[test]
    fn analyse_w307_suppressed_for_known_class_constructor_chain() {
        // ``[Dog new] bark`` — ``Dog`` is a user class so
        // ``new`` returns an Object whose class is ``Dog``.
        // The W307 cmd-sub suppression should kick in.  Since
        // ``bark`` is declared on ``Dog``, no W308 either.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Dog { method bark {} { return woof } }\n[Dog new] bark",
            "tcl",
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W307"),
            "W307 must not fire for [KnownClass new] method chain; got {:?}",
            r.diagnostics,
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W308"),
            "W308 must not fire when method is declared on the class; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w308_emitted_for_unknown_method_on_known_class_constructor() {
        // ``[Dog new] fly`` — ``fly`` isn't declared on ``Dog``.
        // W307 is suppressed (constructor returns Object) but
        // W308 fires for the missing method.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Dog { method bark {} { return woof } }\n[Dog new] fly",
            "tcl",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == "W308" && d.message.contains("fly")),
            "W308 expected for unknown method on known class; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w307_emitted_for_cmd_substitution_with_unknown_return_type() {
        // ``[bogus_cmd] foo`` — the inner command isn't in the
        // registry, so the return type is unknown.  W307 should
        // fire for the cmd-as-command site.
        let src = "[bogus_cmd] foo";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            r.diagnostics.iter().any(|d| d.code == "W307"),
            "W307 expected for [unknown] method pattern; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w307_suppressed_for_my_self_dispatch() {
        // ``[my method]`` is OO self-dispatch — never trips W307.
        let src = "[my m] arg";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W307"),
            "W307 must not fire for OO self-dispatch; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_suppressed_for_partial_interpolation_resolving_to_known_proc() {
        // ``set suffix _hi`` makes ``$suffix`` resolve to ``_hi``;
        // ``foo$suffix`` therefore resolves to ``foo_hi``, which
        // is a known proc.  W123 should not fire.
        let src = "\
proc foo_hi {} {}
set suffix _hi
foo$suffix
";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 should be suppressed when partial interpolation resolves to a known proc; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_kept_when_partial_interpolation_resolves_to_unknown() {
        // ``set suffix _missing`` makes ``foo$suffix`` resolve
        // to ``foo_missing`` — not a known command — so W123
        // should still fire.
        let src = "\
set suffix _missing
foo$suffix
";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 expected when partial interpolation resolves to an unknown command",
        );
    }

    #[test]
    fn analyse_w123_emits_did_you_mean_suggestion() {
        // ``puta`` is one edit away from ``puts`` — the
        // emitter should attach a suggestion and a CodeFix.
        let mut a = Analyser::new();
        let r = a.analyse("puta hi", "tcl");
        let w123 = r
            .diagnostics
            .iter()
            .find(|d| d.code == "W123")
            .expect("W123 emitted");
        assert!(
            w123.message.contains("did you mean 'puts'"),
            "expected suggestion in message, got: {}",
            w123.message,
        );
        assert!(!w123.fixes.is_empty(), "expected CodeFix payload");
        let fix = &w123.fixes[0];
        assert_eq!(fix.new_text, "puts");
        assert!(fix.description.contains("puts"));
    }

    #[test]
    fn analyse_w123_suppressed_for_inline_stub_declared_command() {
        // ``my_cmd`` is declared via inline stub — W123 must
        // not fire even though it isn't in the registry.
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub my_cmd {arg1:var body:body}
# tcl-lsp: stubs-end
my_cmd $x foo
";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must not fire for stub-declared commands; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_dispatch_target_from_unknown_proc_suppresses() {
        // ``foo`` is one of the switch arms inside a
        // user-defined ``unknown`` proc — the empty-stub gate
        // doesn't fire (body is non-empty), so W123 is
        // already suppressed.  Add a fixture that verifies
        // the dispatch_targets are also in the suggestion
        // candidate set when an empty-stub unknown is in play.
        let src = "\
proc unknown {cmd args} {}
foo
";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        // Empty unknown means W123 still fires — but the
        // dispatch_targets membership doesn't apply (set is
        // empty).  Just sanity-check the test runs.
        assert!(r.diagnostics.iter().any(|d| d.code == "W123"));
    }

    #[test]
    fn analyse_w123_no_suggestion_when_far_from_any_known_command() {
        let mut a = Analyser::new();
        let r = a.analyse("xyzzy_unknown_cmd", "tcl");
        let w123 = r
            .diagnostics
            .iter()
            .find(|d| d.code == "W123")
            .expect("W123 emitted");
        assert!(
            !w123.message.contains("did you mean"),
            "no suggestion expected for far-away command name; got: {}",
            w123.message,
        );
        assert!(w123.fixes.is_empty());
    }

    #[test]
    fn analyse_irule4005_racy_static_emitted_for_per_request_writes() {
        // ``static::counter`` written in HTTP_REQUEST and read
        // in HTTP_RESPONSE — both per-request events; the
        // cross-event flow is racy ⇒ IRULE4005 fires.
        let mut a = Analyser::new();
        let r = a.analyse(
            "when HTTP_REQUEST { incr static::counter }\n\
             when HTTP_RESPONSE { log local0. \"$static::counter\" }",
            "f5-irules",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "IRULE4005"),
            "IRULE4005 expected for racy static cross-event flow; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_irule4005_no_emit_for_rule_init_writes() {
        // ``static::config`` written in RULE_INIT is racy-safe
        // (RULE_INIT runs once at iRule load) — IRULE4005 must
        // not fire.
        let mut a = Analyser::new();
        let r = a.analyse(
            "when RULE_INIT { set static::config 1 }\n\
             when HTTP_REQUEST { log local0. \"$static::config\" }",
            "f5-irules",
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "IRULE4005"),
            "IRULE4005 must not fire for RULE_INIT writes; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w124_ipv4_octet_overflow() {
        // ``proc foo {} { set ip 192.168.1.999 }`` — 999 > 255,
        // not a valid IP.  W124 fires at the assignment.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set ip 192.168.1.999 }", "tcl");
        let w124s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W124").collect();
        assert!(
            !w124s.is_empty(),
            "W124 expected for IPv4 octet > 255; got {:?}",
            r.diagnostics,
        );
        assert!(w124s[0].message.contains("999"));
        assert!(w124s[0].message.contains("exceeds 255"));
        assert_eq!(w124s[0].severity, Severity::Error);
    }

    #[test]
    fn analyse_no_w124_for_valid_ipv4() {
        // ``proc foo {} { set ip 192.168.1.1 }`` — valid IP.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set ip 192.168.1.1 }", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W124"),
            "W124 must not fire on valid IPv4; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w124_ipv4_leading_zero_warning() {
        // ``proc foo {} { set ip 192.168.01.1 }`` — leading
        // zero on octet 3; might be octal in some contexts.
        // Severity is Warning.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set ip 192.168.01.1 }", "tcl");
        let w124s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W124").collect();
        assert!(
            !w124s.is_empty(),
            "W124 expected for IPv4 leading-zero octet; got {:?}",
            r.diagnostics,
        );
        assert_eq!(w124s[0].severity, Severity::Warning);
        assert!(w124s[0].message.contains("leading zero"));
    }

    #[test]
    fn analyse_no_w124_for_oid_chain() {
        // FP-STY-06: an LDAP PEN OID (`1.3.6.1.4.1.4203.1.11.3`) is a
        // hierarchical dotted chain, not IPv4 — the embedded `4203.1.11.3`
        // slice must NOT fire W124.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set oid 1.3.6.1.4.1.4203.1.11.3 }", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W124"),
            "W124 must not fire on an OID dotted chain; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w124_real_ipv4_shaped_still_fires() {
        // TP control: a genuine four-component dotted quad with an
        // out-of-range octet (not part of a longer chain) still fires.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set ip 10.0.0.300 }", "tcl");
        assert!(
            r.diagnostics.iter().any(|d| d.code == "W124"),
            "W124 must fire on a genuine over-255 IPv4 quad; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w302_fire_and_forget_bare_close_silent() {
        // FP-STY-05: `catch {close $fh}` is the documented fire-and-forget
        // idiom — no W302.
        let mut a = Analyser::new();
        let r = a.analyse("proc f {} { catch {close $fh} }", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W302"),
            "W302 must be suppressed on `catch {{close ...}}`; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w302_fire_and_forget_ensemble_chan_close_silent() {
        let mut a = Analyser::new();
        let r = a.analyse("proc f {} { catch {chan close $fh} }", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W302"),
            "W302 must be suppressed on `catch {{chan close ...}}`; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w304_braced_switch_form_silent() {
        // FP-NAB-05: the two-arg braced switch form is unambiguous — no W304.
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc f {x} { switch $x { -nocase {puts a} default {puts b} } }",
            "tcl",
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W304"),
            "W304 must not fire on a two-arg braced switch; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w304_split_switch_form_still_fires() {
        // TP control: the split (3+ arg) switch form with a dynamic string
        // before an explicit option still warrants `--`.
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc f {x} { switch $x -nocase {puts a} default {puts b} }",
            "tcl",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "W304"),
            "W304 must still fire on the split switch form; got {:?}",
            r.diagnostics,
        );
    }

    /// Helper: W210 codes for a snippet.
    fn w210_codes(src: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl")
            .diagnostics
            .iter()
            .filter(|d| d.code == "W210")
            .map(|d| d.message.clone())
            .collect()
    }

    fn w233_codes(src: &str) -> usize {
        let mut a = Analyser::new();
        a.analyse(src, "tcl")
            .diagnostics
            .iter()
            .filter(|d| d.code == "W233")
            .count()
    }

    #[test]
    fn w233_divide_by_zero_literal_and_const_var() {
        assert_eq!(w233_codes("proc f {} { return [expr {1 / 0}] }"), 1);
        assert_eq!(w233_codes("proc f {} { return [expr {10 % 0}] }"), 1);
        assert_eq!(
            w233_codes("proc f {} { set d 0\n return [expr {10 / $d}] }"),
            1
        );
    }

    #[test]
    fn w233_silent_on_nonzero_unknown_and_guarded() {
        // Non-zero const + unknown divisor never fire.
        assert_eq!(
            w233_codes("proc f {} { set d 3\n return [expr {10 / $d}] }"),
            0
        );
        assert_eq!(w233_codes("proc f {n} { return [expr {10 / $n}] }"), 0);
        // Short-circuit / dead-arm guards make the division unreachable.
        assert_eq!(w233_codes("proc f {} { return [expr {0 && 1/0}] }"), 0);
        assert_eq!(w233_codes("proc f {} { return [expr {0 ? 1/0 : 7}] }"), 0);
        assert_eq!(w233_codes("proc f {c} { return [expr {$c && 1/0}] }"), 0);
        // Constant-truthy guard forces the arm — fires.
        assert_eq!(w233_codes("proc f {} { return [expr {1.0 && 1/0}] }"), 1);
        assert_eq!(w233_codes("proc f {} { return [expr {-1 && 1/0}] }"), 1);
    }

    #[test]
    fn w210_phi_undef_if_arm_only_def_return() {
        // `v` is defined only when `$x > 0`; the unconditional `return $v`
        // reads it on the no-set path too.
        let got = w210_codes("proc f {x} { if {$x > 0} { set v 1 }\n return $v }");
        assert!(
            got.iter().any(|m| m.contains("'v'")),
            "phi-from-undef merge read must fire W210; got {got:?}"
        );
    }

    #[test]
    fn w210_phi_undef_switch_no_default_return() {
        let got =
            w210_codes("proc f {x} { switch $x { a { set v 1 } b { set v 2 } }\n return $v }");
        assert!(
            got.iter().any(|m| m.contains("'v'")),
            "switch-no-default + return must fire W210; got {got:?}"
        );
    }

    #[test]
    fn w210_interproc_dict_with_caller_literal() {
        // A caller passing a literal dict propagates to the callee's
        // `dict with $param` key check (interproc constant propagation).
        // Key present → silent.
        assert!(
            w210_codes("proc f {d} { dict with d { return $missing } }\nf {missing ok}\n")
                .is_empty()
        );
        // Empty dict → no keys → the read fires.
        assert!(
            w210_codes("proc f {d} { dict with d { return $missing } }\nf {}\n")
                .iter()
                .any(|m| m.contains("'missing'"))
        );
        // Mixed callers → unknown shape → conservatively silent.
        assert!(w210_codes(
            "proc f {d} { dict with d { return $missing } }\nf {}\nf {missing X}\n"
        )
        .is_empty());
    }

    #[test]
    fn w210_provably_no_match_regexp_scan() {
        // Provably-no-match output reads fire.
        assert!(w210_codes("proc f {} { scan abc %d n\n puts $n }")
            .iter()
            .any(|m| m.contains("'n'")));
        assert!(w210_codes("proc f {} { regexp {x} y -> v\n puts $v }")
            .iter()
            .any(|m| m.contains("'v'")));
        // Embedded in a negated condition fires on the no-match arm.
        assert!(
            w210_codes("proc f {} { if {![regexp {x} y -> v]} { puts $v } }")
                .iter()
                .any(|m| m.contains("'v'"))
        );
    }

    #[test]
    fn w210_regexp_expanded_whitespace_pattern_silent() {
        // `-expanded` ignores whitespace, so `{a b}` matches `ab` and writes
        // v — the no-match proof must bail (no false W210).
        assert!(w210_codes("proc f {} { regexp -expanded {a b} ab v\n puts $v }").is_empty());
        // A whitespace-free literal under -expanded is still safe → fires.
        assert!(
            w210_codes("proc f {} { regexp -expanded {x} X v\n puts $v }")
                .iter()
                .any(|m| m.contains("'v'"))
        );
    }

    #[test]
    fn w210_matchable_regexp_scan_silent() {
        // A matchable / nocase-matchable regexp output is set — no W210.
        assert!(w210_codes("proc f {} { regexp -nocase {x} X v\n puts $v }").is_empty());
        assert!(w210_codes("proc f {} { scan 42 %d n\n puts $n }").is_empty());
        // The success arm of a positive condition reads a set var.
        assert!(w210_codes("proc f {} { if {[regexp {x} y -> v]} { puts $v } }").is_empty());
        // An unknown / unsafe switch can't prove no-match → silent.
        assert!(w210_codes("proc f {} { regexp -about {x} y v\n puts $v }").is_empty());
    }

    #[test]
    fn w210_incr_on_uninit_is_silent() {
        // `incr z` initialises z to 0 (Tcl 8.5+) — not read-before-set.
        assert!(w210_codes("proc f {} { incr z\n return $z }").is_empty());
        // A genuine bare read of an unset local still fires.
        assert!(w210_codes("proc f {} { puts $z }")
            .iter()
            .any(|m| m.contains("'z'")));
    }

    #[test]
    fn w210_phi_undef_use_after_unset_return() {
        let got = w210_codes("proc f {} { set v 1\n unset v\n return $v }");
        assert!(
            got.iter().any(|m| m.contains("'v'")),
            "use-after-unset return must fire W210; got {got:?}"
        );
    }

    #[test]
    fn w210_phi_undef_loop_body_only_init_return() {
        let got = w210_codes("proc f {items} { foreach i $items { lappend r $i }\n return $r }");
        assert!(
            got.iter().any(|m| m.contains("'r'")),
            "loop-body-only init + return must fire W210; got {got:?}"
        );
    }

    #[test]
    fn w210_no_fire_when_both_merge_arms_define() {
        // Control: every merge predecessor defines `v` — not read-before-set.
        let got = w210_codes("proc f {x} { if {$x > 0} { set v 1 } else { set v 2 }\n return $v }");
        assert!(
            got.is_empty(),
            "both-arms-defined merge must be silent; got {got:?}"
        );
    }

    #[test]
    fn w210_empty_dict_with_return_fires_but_known_key_silent() {
        // FP-DS-08: empty dict unpacks nothing — `return $missing` fires.
        let empty = w210_codes("proc f {} { set d {}\n dict with d {}\n return $missing }");
        assert!(
            empty.iter().any(|m| m.contains("'missing'")),
            "empty dict-with return must fire W210; got {empty:?}"
        );
        // Known-key dict unpacks `missing` — silent.
        let known =
            w210_codes("proc f {} { set d {missing ok}\n dict with d {}\n return $missing }");
        assert!(
            known.is_empty(),
            "known-key dict-with return must be silent; got {known:?}"
        );
        // Unknown-shape dict (param) — conservatively silent.
        let unknown = w210_codes("proc f {d} { dict with d {}\n return $missing }");
        assert!(
            unknown.is_empty(),
            "unknown dict-with return must be silent; got {unknown:?}"
        );
    }

    #[test]
    fn w210_qualified_variable_alias_tail_return_silent() {
        // FP-RBS-04: `variable ${name}::graphAttr` declares the local alias
        // `graphAttr`; the bare tail read is not read-before-set.
        let got = w210_codes(
            "proc ::ns::get {name key} { variable ${name}::graphAttr\n return $graphAttr }",
        );
        assert!(
            got.is_empty(),
            "qualified variable-alias tail read must be silent; got {got:?}"
        );
    }

    #[test]
    fn w210_no_false_fire_on_many_var_scan_return() {
        // D4-F2: the dynamic scan arg-role resolver marks every trailing
        // varName as a write, so `return $a19` is not read-before-set.
        let src = "proc f {} { scan {0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19} \
{%s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s} \
a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15 a16 a17 a18 a19\n return $a19 }";
        assert!(
            w210_codes(src).is_empty(),
            "20-var scan must not false-fire W210 on the tail var"
        );
    }

    #[test]
    fn w210_no_false_fire_on_many_var_lassign_return() {
        let src = "proc f {l} { lassign $l a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 \
a15 a16 a17 a18 a19 a20\n return $a20 }";
        assert!(
            w210_codes(src).is_empty(),
            "21-var lassign must not false-fire W210 on the tail var"
        );
    }

    #[test]
    fn w214_empty_body_stub_silent() {
        // FP-STY-08: `proc stub {a b} {}` is a signature placeholder — no
        // W214 on its necessarily-unused params.
        let mut a = Analyser::new();
        let r = a.analyse("proc stub {a b} {}", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W214"),
            "W214 must be suppressed on an empty-body stub; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w214_quoted_keyword_marker_silent() {
        // FP-STY-08: a param named `"as"` is a snit-style keyword marker.
        let mut a = Analyser::new();
        let r = a.analyse("proc xyz {\"as\" v} { return $v }", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W214"),
            "W214 must not fire on a quoted-keyword param; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w214_dispatch_protocol_suppresses_peer_family() {
        // ≥3 peers sharing `{ctx token}` + an arity-compatible dispatcher
        // (`$cmd $ctx $token`, 2 args) — `token` is a protocol contract.
        let src = "namespace eval ::n {\n\
                   proc a {ctx token} { puts $ctx }\n\
                   proc b {ctx token} { puts $ctx }\n\
                   proc c {ctx token} { puts $ctx }\n\
                   proc dispatch {cmd ctx token} { $cmd $ctx $token }\n\
                   }\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == "W214" && d.message.contains("'token'")),
            "dispatch-protocol family must suppress W214 on protocol params; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w214_no_dispatcher_still_fires_on_peer_family() {
        // TP control: 3 peers sharing `{ctx token}` but NO dispatcher — the
        // shared shape is coincidence, so `token` still fires.
        let src = "namespace eval ::n {\n\
                   proc a {ctx token} { puts $ctx }\n\
                   proc b {ctx token} { puts $ctx }\n\
                   proc c {ctx token} { puts $ctx }\n\
                   }\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == "W214" && d.message.contains("'token'")),
            "without a dispatcher, an unused protocol-shaped param still fires; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w214_genuine_unused_param_still_fires() {
        // TP control: a normal unused param in a non-empty body still fires.
        let mut a = Analyser::new();
        let r = a.analyse("proc f {a b} { return $a }", "tcl");
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == "W214" && d.message.contains("'b'")),
            "W214 must still fire on a genuine unused param; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w302_constructive_subcommand_still_fires() {
        // TP control: `chan configure` is constructive, not fire-and-forget.
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc f {} { catch {chan configure $fh -blocking 0} }",
            "tcl",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "W302"),
            "W302 must still fire on a constructive `chan configure`; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_i230_constant_if_branch() {
        // ``proc foo {} { if {1} { puts hi } }`` — the ``if 1``
        // condition is constant, the false branch is unreachable.
        // I230 should fire.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { if {1} { puts hi } }", "tcl");
        let i230s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "I230").collect();
        assert!(
            !i230s.is_empty(),
            "I230 expected for constant ``if 1``; got {:?}",
            r.diagnostics,
        );
        assert!(i230s[0].message.contains("always true"));
    }

    #[test]
    fn analyse_no_i230_for_dynamic_condition() {
        // ``proc foo {x} { if {$x > 0} {} }`` — ``$x > 0`` is
        // not constant; no I230.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {x} { if {$x > 0} { puts hi } }", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "I230"),
            "I230 must not fire on dynamic condition; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_unknown_command() {
        // ``no_such_cmd hello`` — bare name that's not a
        // built-in / proc / class / alias.  W123 fires.
        let mut a = Analyser::new();
        let r = a.analyse("no_such_cmd hello", "tcl");
        let w123s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W123").collect();
        assert!(
            !w123s.is_empty(),
            "W123 expected for unknown command; got {:?}",
            r.diagnostics,
        );
        assert!(w123s[0].message.contains("'no_such_cmd'"));
        assert_eq!(w123s[0].severity, Severity::Hint);
    }

    #[test]
    fn analyse_no_w123_for_builtin_command() {
        // ``puts hello`` — ``puts`` is a built-in; no W123.
        let mut a = Analyser::new();
        let r = a.analyse("puts hello", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must not fire on built-in command; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_no_w123_for_user_proc() {
        // User-defined proc, then call it.  Both go through
        // the analyser walk; the call site must NOT trip W123.
        let mut a = Analyser::new();
        let r = a.analyse("proc greet {} { puts hi }\ngreet", "tcl");
        let w123s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W123").collect();
        assert!(
            w123s.is_empty(),
            "W123 must not fire on user-defined proc call; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_no_w123_for_qualified_command_name() {
        // Qualified names (``a::b``) skip W123 — defer to
        // per-namespace logic.
        let mut a = Analyser::new();
        let r = a.analyse("ns::cmd hello", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must not fire on qualified command name; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_package_require_gate_suppresses_when_recorded() {
        // The ``package_requires`` gate suppresses W123 entirely
        // when any package require has been recorded.  The
        // analyser walk doesn't yet record ``package require``
        // (deferred — handler not landed), so we exercise the
        // gate by pre-populating ``result.package_requires``
        // and re-running the post-pass directly.
        use crate::signature_scan::types::SignaturePackageRequire;
        use tcl_lexer::Span;
        let mut a = Analyser::new();
        a.result.package_requires.push(SignaturePackageRequire {
            name: "Tcl".to_string(),
            version: Some("8.6".to_string()),
            range: Span::new(0, 24),
            conditional: false,
        });
        // Seed an invocation that would otherwise trip W123.
        a.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                name: "random_cmd".to_string(),
                range: Span::new(25, 35),
                resolved_qualified_name: None,
            },
        );
        let registry = tcl_registry::CommandRegistry::build_default();
        a.emit_unresolved_command_diagnostics(&registry);
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must be fully suppressed when package_requires is non-empty; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_filtered_by_disabled_diagnostics() {
        // ``# tcl-lsp: disable=W123`` at top of file silences
        // the diagnostic via the existing disable filter.
        let mut a = Analyser::new();
        let r = a.analyse("# tcl-lsp: disable=W123\nno_such_cmd hello", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must be silenced by file-suppression directive; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w307_var_as_command() {
        // ``proc foo {} { $cmd arg1 }`` — ``$cmd`` (a non-parameter local
        // dispatched once) is used as command head with no static knowledge
        // of what it holds, so W307 fires.  Must go through ``analyse`` (not
        // raw ``emit_cfg_ssa_diagnostics``) because ``var_command_sites`` is
        // populated by the analyser's walk dispatch, not the emitter pipeline.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { $cmd arg1 }", "tcl");
        let w307s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W307").collect();
        assert!(
            !w307s.is_empty(),
            "W307 expected for ``$cmd arg1``; got {:?}",
            r.diagnostics,
        );
        assert_eq!(w307s[0].severity, Severity::Warning);
        assert!(w307s[0].message.contains("Non-literal command name"));
    }

    #[test]
    fn analyse_w307_suppressed_for_proc_param_dispatch() {
        // A dispatch on a *parameter* of the enclosing proc is object dispatch
        // the user documented as the proc's API contract — W307 must stay
        // silent (mirrors `_diag_var_command.py`'s proc-parameter
        // suppression).  `$self configure` is the canonical method-dispatch
        // idiom on an opaque handle.
        let mut a = Analyser::new();
        let r = a.analyse("proc p {self} { $self configure -x 1 }", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W307"),
            "W307 must be suppressed for a dispatch on proc parameter; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w307_suppressed_for_multi_dispatch_local() {
        // A non-parameter local dispatched ≥2 times demonstrates intent
        // (object usage), so W307 is suppressed even without a known value.
        let mut a = Analyser::new();
        let r = a.analyse("proc p {} { $tree visit\n$tree leaves }", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W307"),
            "W307 must be suppressed for a local dispatched ≥2 times; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w307_fires_for_tainted_dispatch_despite_multi_use() {
        // Taint carve-out: a user-controlled command name dispatched multiple
        // times is still a command-injection risk — the dispatcher-suppression
        // must NOT apply, so W307 fires.
        let mut a = Analyser::new();
        let r = a.analyse("proc p {} { set c [gets stdin]\n$c one\n$c two }", "tcl");
        assert!(
            r.diagnostics.iter().any(|d| d.code == "W307"),
            "W307 must fire for a tainted dispatched command name; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_no_w307_for_static_known_command() {
        // ``proc foo {} { set cmd puts; $cmd hello }`` — ``cmd``
        // has constant value "puts" which IS a known command, so
        // W307 must be suppressed.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set cmd puts\n$cmd hello }", "tcl");
        let w307s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W307").collect();
        assert!(
            w307s.is_empty(),
            "W307 must be suppressed when var holds known command name; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w307_suppressed_per_ssa_version_after_reassignment() {
        // Per-SSA-version refinement (SYNC-JUN02-1 strip 5): `cmd` is
        // reassigned from a non-command to a known command before the
        // dispatch.  The merged const-set {notacommand, puts} would
        // wrongly keep W307 alive; reading the value at the dispatch's
        // exact SSA use-version ("puts") suppresses it.
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc foo {} { set cmd notacommand\nset cmd puts\n$cmd hello }",
            "tcl",
        );
        let w307s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W307").collect();
        assert!(
            w307s.is_empty(),
            "W307 must read the precise reaching version (\"puts\"); got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w307_still_fires_when_precise_version_not_a_command() {
        // The mirror case: the reaching version is a non-command, so
        // W307 must still fire (the refinement only suppresses when the
        // exact value is provably a known command).
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc foo {} { set cmd puts\nset cmd notacommand\n$cmd hello }",
            "tcl",
        );
        let w307s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W307").collect();
        assert!(
            !w307s.is_empty(),
            "W307 should fire when the reaching version isn't a command; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_var_command_sites_recorded_during_walk() {
        // Smoke: confirm the recording infrastructure populates
        // ``var_command_sites`` for ``$var`` heads.  Run analyse
        // (not just emit) so the apply_disabled_diagnostics +
        // dedupe don't matter — we inspect post-analyse state.
        let mut a = Analyser::new();
        let _ = a.analyse("proc foo {x} { $x arg }", "tcl");
        // After analyse, var_command_sites is consumed by the
        // post-pass but restored at the end (snapshot/restore
        // contract).
        assert!(
            a.var_command_sites.iter().any(|s| s.var_name == "x"),
            "var_command_sites should record ``$x`` head; got {:?}",
            a.var_command_sites,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_cmd_command_sites_recorded_during_walk() {
        // ``[cmd] arg`` records to ``cmd_command_sites`` even
        // though no W307 emitter consumes it yet.
        let mut a = Analyser::new();
        let _ = a.analyse("proc foo {} { [puts hi] arg }", "tcl");
        assert!(
            !a.cmd_command_sites.is_empty(),
            "cmd_command_sites should be populated for ``[cmd] arg``; got {:?}",
            a.cmd_command_sites,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w214_skips_args_param() {
        // The variadic ``args`` is conventional and frequently
        // declared without use; W214 must not fire on it.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {x args} { puts $x }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W214"),
            "W214 should not fire on ``args``; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn dedupe_drops_exact_duplicates() {
        // Same code + span + message + severity → kept once.
        let mut a = Analyser::new();
        a.source = "set x 1".to_string();
        a.result
            .diagnostics
            .push(diag("W210", Span::new(4, 5), "x not set"));
        a.result
            .diagnostics
            .push(diag("W210", Span::new(4, 5), "x not set"));
        a.dedupe_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 1);
    }

    #[test]
    fn dedupe_keeps_distinct_diagnostics_at_different_spans() {
        let mut a = Analyser::new();
        a.source = "set x 1\nset y 2".to_string();
        a.result
            .diagnostics
            .push(diag("W210", Span::new(4, 5), "x"));
        a.result
            .diagnostics
            .push(diag("W210", Span::new(12, 13), "y"));
        a.dedupe_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 2);
    }

    #[test]
    fn dedupe_drops_e002_on_e101_line() {
        // E101 fires on a line; any E002 on the same line is
        // a false positive (arity check confused by the
        // recovered switch) and gets dropped.
        let mut a = Analyser::new();
        a.source = "switch $x { foo {puts foo}".to_string();
        let switch_span = Span::new(0, 6);
        a.result
            .diagnostics
            .push(diag("E101", switch_span, "missing open brace"));
        a.result
            .diagnostics
            .push(diag("E002", switch_span, "too few args"));
        a.dedupe_diagnostics();
        assert!(a.result.diagnostics.iter().any(|d| d.code == "E101"));
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "E002"));
    }

    #[test]
    fn dedupe_drops_w122_on_w124_line() {
        // W124 (SSA-based IP check) on a line → W122 (regex IP
        // check) on the same line is redundant.
        let mut a = Analyser::new();
        a.source = "if {[IP::addr $ip]} {}".to_string();
        let ip_span = Span::new(15, 18);
        a.result
            .diagnostics
            .push(diag("W124", ip_span, "invalid IP"));
        a.result
            .diagnostics
            .push(diag("W122", ip_span, "regex IP check"));
        a.dedupe_diagnostics();
        assert!(a.result.diagnostics.iter().any(|d| d.code == "W124"));
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "W122"));
    }

    #[test]
    fn dedupe_keeps_e002_on_unrelated_line() {
        // E101 on line 0, E002 on line 1 — different lines, so
        // the suppression rule doesn't fire.
        let mut a = Analyser::new();
        a.source = "switch $x {\nset y 1".to_string();
        a.result
            .diagnostics
            .push(diag("E101", Span::new(0, 6), "missing brace"));
        a.result
            .diagnostics
            .push(diag("E002", Span::new(12, 15), "too few args"));
        a.dedupe_diagnostics();
        assert!(a.result.diagnostics.iter().any(|d| d.code == "E002"));
    }

    #[test]
    fn apply_disabled_diagnostics_removes_listed_codes() {
        let mut a = Analyser::with_disabled_diagnostics(
            ["W113"].iter().map(|s| (*s).to_string()).collect(),
        );
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "shadows"));
        a.result
            .diagnostics
            .push(diag("W210", Span::new(0, 3), "unset"));
        a.apply_disabled_diagnostics();
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "W113"));
        assert!(a.result.diagnostics.iter().any(|d| d.code == "W210"));
    }

    #[test]
    fn apply_disabled_diagnostics_no_op_when_empty() {
        let mut a = Analyser::new();
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "x"));
        a.apply_disabled_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 1);
    }

    // -- W120: missing package require ------------------------------

    #[test]
    fn w120_fires_for_package_gated_command_without_require() {
        // `tcl::idna` carries `required_package = "tcl::idna"`.
        // Using it without a `package require` emits W120.
        let mut a = Analyser::new();
        let r = a.analyse("tcl::idna decode example.com\n", "tcl9.0");
        let w120: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W120").collect();
        assert_eq!(w120.len(), 1, "expected one W120; got {:?}", r.diagnostics);
        assert!(w120[0].message.contains("package require tcl::idna"));
        // Carries a fix that inserts the require at the top.
        assert_eq!(w120[0].fixes.len(), 1);
        assert_eq!(w120[0].fixes[0].new_text, "package require tcl::idna\n");
        assert!(w120[0].fixes[0]
            .description
            .contains("Add 'package require"));
    }

    #[test]
    fn w120_suppressed_when_package_required() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "package require tcl::idna\ntcl::idna decode example.com\n",
            "tcl9.0",
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W120"),
            "W120 must not fire when the package is required; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn w120_fix_inserts_after_existing_require() {
        // With an unrelated `package require` present, the fix
        // inserts on the line after it.
        let src = "package require Tcl 8.6\ntcl::idna decode x\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let w120 = r
            .diagnostics
            .iter()
            .find(|d| d.code == "W120")
            .expect("W120 expected");
        let fix = &w120.fixes[0];
        // Insertion offset is past the first line's newline
        // (byte 23 = start of line 1).
        let off = fix.span.start() as usize;
        assert_eq!(&src[..off], "package require Tcl 8.6\n");
    }

    #[test]
    fn w120_emitted_once_per_command_name() {
        let src = "tcl::idna decode a\ntcl::idna encode b\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let w120: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W120").collect();
        assert_eq!(w120.len(), 1, "expected one W120 per name; got {w120:?}");
    }

    #[test]
    fn w120_disabled_via_directive() {
        let mut a = Analyser::new();
        let r = a.analyse("# tcl-lsp: disable=W120\ntcl::idna decode x\n", "tcl9.0");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W120"),
            "{:?}",
            r.diagnostics
        );
    }

    // -- GAP-A2 security-injection checks (W300 / W301 / W309 / W312) --
    //
    // Each fixture's diagnostic set is cross-checked against the live
    // Python analyser (`core/analysis/checks/_security.py`).

    fn sec_codes(src: &str, code: &str) -> usize {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == code)
            .count()
    }

    #[test]
    fn w300_source_with_variable_path() {
        assert_eq!(sec_codes("source $path\n", "W300"), 1);
        // `-encoding ENC` is skipped to find the file argument.
        assert_eq!(sec_codes("source -encoding utf-8 $path\n", "W300"), 1);
        // A literal path is fine.
        assert_eq!(sec_codes("source ./lib.tcl\n", "W300"), 0);
    }

    #[test]
    fn w309_eval_uplevel_with_subst() {
        let mut a = Analyser::new();
        let r = a.analyse("eval [subst $template]\n", "tcl8.6");
        let w309: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W309").collect();
        assert_eq!(w309.len(), 1);
        assert_eq!(w309[0].severity, Severity::Error);
        assert!(w309[0].message.starts_with("eval with [subst]"));
        assert_eq!(sec_codes("uplevel [subst {$x}]\n", "W309"), 1);
        // No `[subst]` → no W309.
        assert_eq!(sec_codes("eval [list set x $y]\n", "W309"), 0);
    }

    #[test]
    fn w301_uplevel_injection() {
        // Single unbraced substituted script.
        assert_eq!(sec_codes("uplevel 1 \"set x $y\"\n", "W301"), 1);
        // Multiple args concatenate.
        assert_eq!(sec_codes("uplevel $a $b\n", "W301"), 1);
        // Braced body and the `[list …]` idiom are safe.
        assert_eq!(sec_codes("uplevel 1 {set x 1}\n", "W301"), 0);
        assert_eq!(sec_codes("uplevel 1 [list set x $y]\n", "W301"), 0);
        // A single *pure* variable script is the safe single-substitution
        // idiom — no W301; a concatenation still fires.
        assert_eq!(sec_codes("proc f {body} { uplevel 1 $body }\n", "W301"), 0);
        assert_eq!(sec_codes("uplevel 1 $body\n", "W301"), 0);
        assert_eq!(sec_codes("uplevel 1 pre$body\n", "W301"), 1);
    }

    #[test]
    fn w312_interp_eval_injection() {
        assert_eq!(sec_codes("interp eval $child $script\n", "W312"), 1);
        assert_eq!(sec_codes("interp eval $child \"set x $y\"\n", "W312"), 1);
        // Multiple script words concatenate.
        assert_eq!(sec_codes("interp eval $foo $a $b\n", "W312"), 1);
        // invokehidden flags the hidden command word.
        assert_eq!(
            sec_codes("interp invokehidden $child $cmd $arg\n", "W312"),
            1
        );
        // Braced body is safe.
        assert_eq!(sec_codes("interp eval $child {set x 1}\n", "W312"), 0);
    }

    #[test]
    fn w312_message_names_subcommand() {
        let mut a = Analyser::new();
        let r = a.analyse("interp eval $child $script\n", "tcl8.6");
        let w312 = r.diagnostics.iter().find(|d| d.code == "W312").unwrap();
        assert!(
            w312.message.contains("interp eval $child {...}"),
            "{w312:?}"
        );
    }

    #[test]
    fn w102_subst_variable_argument() {
        // Bare `$var` template fires; the message lists both kinds.
        let mut a = Analyser::new();
        let r = a.analyse("subst $x\n", "tcl8.6");
        let w102 = r.diagnostics.iter().find(|d| d.code == "W102").unwrap();
        assert!(
            w102.message.contains("any [cmd] and $var in the string"),
            "{w102:?}"
        );
        assert!(w102
            .message
            .contains("Add -nocommands -novariables to limit"));
        // A braced or quoted template is fine; both flags suppress it.
        assert_eq!(sec_codes("subst {literal $y}\n", "W102"), 0);
        assert_eq!(sec_codes("subst \"$x\"\n", "W102"), 0);
        assert_eq!(sec_codes("subst -nocommands -novariables $x\n", "W102"), 0);
    }

    #[test]
    fn w102_message_narrows_with_flags() {
        let mut a = Analyser::new();
        let r = a.analyse("subst -nocommands $x\n", "tcl8.6");
        let w102 = r.diagnostics.iter().find(|d| d.code == "W102").unwrap();
        // Only `$var` remains active; only `-novariables` is suggested.
        assert!(w102.message.contains("any $var in the string"), "{w102:?}");
        assert!(!w102.message.contains("[cmd]"), "{w102:?}");
        assert!(w102.message.contains("Add -novariables to limit"));
    }

    #[test]
    fn w103_open_pipeline() {
        // `|`-pipeline with substitution → WARNING (injection).
        let mut a = Analyser::new();
        let r = a.analyse("open \"|$cmd\"\n", "tcl8.6");
        let w103 = r.diagnostics.iter().find(|d| d.code == "W103").unwrap();
        assert_eq!(w103.severity, Severity::Warning);
        assert!(w103.message.contains("command injection"), "{w103:?}");
        // Literal `|`-pipeline → HINT.
        assert_eq!(code_sevs("open |ls\n", "W103"), vec!["Hint"]);
        assert_eq!(code_sevs("open \"|cat file\"\n", "W103"), vec!["Hint"]);
        // Bare `$var` argument → WARNING (may resolve to a pipeline).
        assert_eq!(code_sevs("open $f\n", "W103"), vec!["Warning"]);
        // A literal filename is fine.
        assert_eq!(sec_codes("open \"file.txt\"\n", "W103"), 0);
    }

    #[test]
    fn w303_redos_nested_quantifiers() {
        // Nested quantifier and overlapping alternation, in regexp /
        // regsub and a `switch -regexp` braced case list.
        assert_eq!(sec_codes("regexp {(a+)+} $str\n", "W303"), 1);
        assert_eq!(sec_codes("regexp {(a|a)+} $str\n", "W303"), 1);
        assert_eq!(sec_codes("regsub {(x*)*} $s y out\n", "W303"), 1);
        assert_eq!(
            sec_codes("switch -regexp $x {(a+)+ {body} default {x}}\n", "W303"),
            1
        );
        // Option flags before the pattern are skipped.
        assert_eq!(sec_codes("regexp -nocase {(a+)+} $s\n", "W303"), 1);
        // Safe patterns don't fire.
        assert_eq!(sec_codes("regexp {abc} $str\n", "W303"), 0);
        assert_eq!(sec_codes("regexp {[0-9]+} $s\n", "W303"), 0);
    }

    #[test]
    fn w303_message_and_severity() {
        let mut a = Analyser::new();
        let r = a.analyse("regexp {(a+)+} $s\n", "tcl8.6");
        let w303 = r.diagnostics.iter().find(|d| d.code == "W303").unwrap();
        assert_eq!(w303.severity, Severity::Warning);
        assert!(w303.message.contains("catastrophic"), "{w303:?}");
    }

    #[test]
    fn w310_hardcoded_credential_option() {
        // A literal value after a default credential option fires.
        assert_eq!(sec_codes("mycmd -password literalsecret123\n", "W310"), 1);
        assert_eq!(sec_codes("mycmd -token abc123\n", "W310"), 1);
        // Case-insensitive option matching.
        assert_eq!(sec_codes("mycmd -Password hunter2\n", "W310"), 1);
        // Only one diagnostic per command.
        assert_eq!(sec_codes("mycmd -pass a -secret b\n", "W310"), 1);
        // A `$var` / `[cmd]` value is not a hardcoded credential.
        assert_eq!(sec_codes("mycmd -password $env_pw\n", "W310"), 0);
        assert_eq!(sec_codes("mycmd -password [getpw]\n", "W310"), 0);
        // No credential option → nothing.
        assert_eq!(sec_codes("mycmd -name literalvalue\n", "W310"), 0);
    }

    #[test]
    fn irule2002_flags_deprecated_irules_command() {
        // `HTTP::class` is deprecated → `CLASSIFY::application`.
        let mut a = Analyser::new();
        let r = a.analyse("when HTTP_REQUEST {\n  HTTP::class\n}\n", "f5-irules");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "IRULE2002")
            .expect("IRULE2002");
        assert_eq!(d.severity, Severity::Warning);
        assert!(
            d.message.contains(
                "'HTTP::class' is deprecated in iRules. Use 'CLASSIFY::application' instead."
            ),
            "{d:?}"
        );
    }

    #[test]
    fn irule2002_silent_in_plain_tcl_dialect() {
        // The deprecation check is iRules-only.
        assert!(!has_code("HTTP::class\n", "tcl8.6", "IRULE2002"));
    }

    #[test]
    fn w310_message_names_option() {
        let mut a = Analyser::new();
        let r = a.analyse("mycmd -password literalsecret\n", "tcl8.6");
        let w310 = r.diagnostics.iter().find(|d| d.code == "W310").unwrap();
        assert!(
            w310.message
                .starts_with("Hardcoded credential in -password argument."),
            "{w310:?}"
        );
    }

    #[test]
    fn w310_registry_credential_option() {
        // `http::geturl`'s registry `credential_options` adds `-headers`
        // to the default flag set (Strategy 1 augmentation).
        let mut a = Analyser::new();
        let r = a.analyse(
            "http::geturl $url -headers {Authorization \"Bearer abc123def456\"}\n",
            "tcl8.6",
        );
        let w310 = r.diagnostics.iter().find(|d| d.code == "W310").unwrap();
        assert!(
            w310.message
                .starts_with("Hardcoded credential in -headers argument."),
            "{w310:?}"
        );
    }

    #[test]
    fn w310_subcommand_sensitive_header() {
        // `HTTP::header insert authorization <literal>` — the subcommand's
        // registry credential_arg + sensitive_headers (Strategy 2).
        let mut a = Analyser::new();
        let r = a.analyse(
            "HTTP::header insert authorization \"Bearer secrettoken123\"\n",
            "f5-irules",
        );
        let w310 = r.diagnostics.iter().find(|d| d.code == "W310").unwrap();
        assert!(
            w310.message
                .starts_with("Hardcoded credential in authorization header value."),
            "{w310:?}"
        );
        // A non-sensitive header is fine; a `$var` value is not literal.
        assert!(!a
            .analyse(
                "HTTP::header insert content-type \"text/html\"\n",
                "f5-irules"
            )
            .diagnostics
            .iter()
            .any(|d| d.code == "W310"));
        assert!(!a
            .analyse("HTTP::header insert authorization $tok\n", "f5-irules")
            .diagnostics
            .iter()
            .any(|d| d.code == "W310"));
    }

    #[test]
    fn w306_literal_expected_in_regexp_pattern() {
        fn has_w306(src: &str) -> bool {
            let mut a = Analyser::new();
            a.analyse(src, "tcl8.6")
                .diagnostics
                .iter()
                .any(|d| d.code == "W306")
        }
        // Quoted `"$var"` / `"[cmd]"` patterns fire (Tcl substitutes them
        // before the regex engine sees the value).
        assert!(has_w306("regexp \"$pat\" $s\n"));
        assert!(has_w306("regexp \"[clock seconds]\" $s\n"));
        // A bare `$var` is the canonical parameterised-pattern idiom — exempt.
        assert!(!has_w306("regexp $pat $s\n"));
        // A braced pattern suppresses substitution — exempt.
        assert!(!has_w306("regexp {[abc]+} $s\n"));
        // An escaped `\[` in a quoted pattern is a literal regex char — exempt.
        assert!(!has_w306("regexp \"\\[abc\\]+\" $s\n"));
        // A bare `[cmd]` pattern is the foot-gun (parsed as command sub) — fires.
        assert!(has_w306("regexp [join $parts] $s\n"));
    }

    #[test]
    fn w304_does_not_cross_proc_param_shadow() {
        // The outer `set path -force` must NOT be attributed to the inner
        // `$path` use — the proc param `path` shadows it.  W304 may still
        // fire on the substituted `file delete $path`, but never claiming
        // the value is `-force`.
        let mut a = Analyser::new();
        let r = a.analyse(
            "set path -force\nproc useit {path} { file delete $path }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == "W304" && d.message.contains("-force")),
            "{:?}",
            r.diagnostics
        );
        // Control: a top-level `$path` use *after* a complete proc still
        // resolves to the outer literal (no shadow crossing).
        let r2 = a.analyse(
            "set path -force\nproc p {path} {}\nfile delete $path\n",
            "tcl8.6",
        );
        assert!(
            r2.diagnostics
                .iter()
                .any(|d| d.code == "W304" && d.message.contains("-force")),
            "{:?}",
            r2.diagnostics
        );
    }
}
