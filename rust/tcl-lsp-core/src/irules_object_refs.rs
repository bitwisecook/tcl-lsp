//! iRules BIG-IP object-reference extraction (GAP-C2 / `_bigip.py`).
//!
//! Port of the token-relevant half of `core/bigip/irules_refs.py` —
//! `extract_irules_object_references` — used by the semantic-tokens
//! provider to highlight BIG-IP object names (pools, data groups,
//! virtuals, …) referenced from iRules code as the `object` token type.
//!
//! The walk segments the source, recognises the object-reference
//! commands (`pool` / `snatpool` / `virtual` / `node` / `class` /
//! `persist` / …) by name + argument shape, and recurses into `BODY`
//! arguments (`when` / `if` bodies) and `[…]` command substitutions so
//! refs nested inside event handlers are found.  Only the *name range*
//! is needed for highlighting, so the Python `kinds` / `rule_module`
//! machinery (used by completion / diagnostics) is dropped — with
//! `rule_module = None`, every `rule_module != "gtm"` branch fires, so
//! all LTM references are recognised.

use tcl_compiler::segmenter::{segment_commands_with_offset, SegmentedCommand};
use tcl_lexer::{Span, TokenType};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::CommandRegistry;

/// `pool` subcommands that aren't a pool-name reference.
const POOL_NON_REFERENCE_KEYWORDS: &[&str] = &["member", "members"];

/// `persist` heads that aren't a profile-name reference (unless `/`-pathed).
const PERSIST_NON_REFERENCE_KEYWORDS: &[&str] = &[
    "add",
    "delete",
    "lookup",
    "replace",
    "none",
    "simple",
    "sticky",
    "source_addr",
    "dest_addr",
    "hash",
    "cookie",
    "ssl",
    "msrdp",
    "sip",
    "uie",
    "carp",
    "universal",
];

/// `class match`/`search` comparison operators.
const CLASS_OPERATORS: &[&str] = &[
    "equals",
    "starts_with",
    "ends_with",
    "contains",
    "matches_glob",
    "matches_regex",
];

/// Return the sorted, de-duplicated byte spans of every BIG-IP object
/// name referenced from `source`.  Mirrors
/// `extract_irules_object_references` (name ranges only).
#[must_use]
pub fn object_ref_spans(source: &str, registry: &CommandRegistry) -> Vec<Span> {
    let mut out = Vec::new();
    walk(source, source, 0, registry, &mut out);
    out.sort_by_key(|s| (s.start(), s.end()));
    out.dedup();
    out
}

/// Segment `slice` (a substring of `full` starting at byte `base`) and
/// collect object refs, recursing into body / command-substitution
/// arguments.  Token spans are absolute into `full`.
fn walk(full: &str, slice: &str, base: u32, registry: &CommandRegistry, out: &mut Vec<Span>) {
    for cmd in segment_commands_with_offset(slice, base) {
        collect_command_refs(full, &cmd, out);

        let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();
        let body_indices = registry.arg_indices_for_role(cmd.name(), &args, ArgRole::Body);
        for body_idx in body_indices {
            recurse_into(
                full,
                &cmd,
                body_idx + 1,
                &[TokenType::Str, TokenType::Cmd],
                registry,
                out,
            );
        }
        // EXPR-role words (`if` / `while` / `expr` conditions, …) are
        // braced/quoted (`Str` / `Esc`) words whose `[…]` substitutions
        // are *not* separate `Cmd` tokens in `all_tokens`.  Recurse into
        // the expression text so object refs nested in a condition —
        // `if {[class match $h equals dg]} { … }` — are found; the
        // inner `[…]` is then picked up by the `all_tokens` scan one
        // level down.  Mirrors the EXPR re-lex in `_walk_irules_commands`.
        let expr_indices = registry.arg_indices_for_role(cmd.name(), &args, ArgRole::Expr);
        for expr_idx in expr_indices {
            recurse_into(
                full,
                &cmd,
                expr_idx + 1,
                &[TokenType::Str, TokenType::Esc],
                registry,
                out,
            );
        }
        // `[…]` command substitutions anywhere in the command.
        for tok in &cmd.all_tokens {
            if matches!(tok.kind, TokenType::Cmd) {
                recurse_token(full, tok, registry, out);
            }
        }
    }
}

/// Recurse into the representative token of word `word_index` when its
/// kind is one of `kinds`.
fn recurse_into(
    full: &str,
    cmd: &SegmentedCommand,
    word_index: usize,
    kinds: &[TokenType],
    registry: &CommandRegistry,
    out: &mut Vec<Span>,
) {
    if let Some(tok) = cmd.argv.get(word_index) {
        if kinds.contains(&tok.kind) {
            recurse_token(full, tok, registry, out);
        }
    }
}

/// Recurse into a single token's inner content (offset-adjusted).
fn recurse_token(
    full: &str,
    tok: &tcl_lexer::Token,
    registry: &CommandRegistry,
    out: &mut Vec<Span>,
) {
    let cstart = tok.span.start() as usize + tok.content_offset as usize;
    let cend = (tok.span.end() as usize).min(full.len());
    if cstart >= cend {
        return;
    }
    let inner = &full[cstart..cend];
    if inner.trim().is_empty() {
        return;
    }
    walk(
        full,
        inner,
        u32::try_from(cstart).unwrap_or(0),
        registry,
        out,
    );
}

/// Recognise an object-reference command and push its name-arg span(s).
/// Mirrors `_collect_command_references` (with `rule_module = None`).
fn collect_command_refs(full: &str, cmd: &SegmentedCommand, out: &mut Vec<Span>) {
    let args = cmd.args();
    if args.is_empty() {
        return;
    }
    let name = cmd.name().to_ascii_lowercase();
    let head = args[0].to_ascii_lowercase();
    let push = |idx: usize, out: &mut Vec<Span>| {
        if let Some(span) = literal_arg_span(full, cmd, idx) {
            out.push(span);
        }
    };
    match name.as_str() {
        "pool" => {
            if !POOL_NON_REFERENCE_KEYWORDS.contains(&head.as_str()) {
                push(0, out);
            }
        }
        "active_members" | "members" | "snatpool" | "virtual" | "node" => push(0, out),
        "clone" => {
            push(0, out);
            if args.len() >= 2 {
                push(1, out);
            }
        }
        // `snat pool NAME` / `LB::status pool NAME` — pool-name at arg 1.
        "snat" | "lb::status" => {
            if args.len() >= 2 && head == "pool" {
                push(1, out);
            }
        }
        "persist" => {
            if !PERSIST_NON_REFERENCE_KEYWORDS.contains(&head.as_str()) || args[0].starts_with('/')
            {
                push(0, out);
            }
        }
        "class" => collect_class_refs(full, cmd, out),
        "matchclass" => {
            if args.len() >= 2 {
                push(args.len() - 1, out);
            }
        }
        "lb::reselect" => {
            if args.len() >= 2 && matches!(head.as_str(), "pool" | "snatpool" | "virtual") {
                push(1, out);
            }
        }
        _ => {}
    }
}

/// Recognise `class match`/`search`/`lookup`/`exists`/… data-group
/// references.  Mirrors `_collect_class_references`.
fn collect_class_refs(full: &str, cmd: &SegmentedCommand, out: &mut Vec<Span>) {
    let args = cmd.args();
    if args.is_empty() {
        return;
    }
    let sub = args[0].to_ascii_lowercase();
    let push = |idx: usize, out: &mut Vec<Span>| {
        if let Some(span) = literal_arg_span(full, cmd, idx) {
            out.push(span);
        }
    };
    match sub.as_str() {
        "match" | "search" => {
            let mut idx = 1;
            while idx < args.len() && args[idx].starts_with('-') {
                idx += 1;
            }
            if idx < args.len() && args[idx] == "--" {
                idx += 1;
            }
            idx += 1; // item
            if idx >= args.len()
                || !CLASS_OPERATORS.contains(&args[idx].to_ascii_lowercase().as_str())
            {
                return;
            }
            idx += 1;
            if idx < args.len() {
                push(idx, out);
            }
        }
        "lookup" => {
            let mut idx = 1;
            if idx < args.len() && args[idx] == "--" {
                idx += 1;
            }
            idx += 1; // key
            if idx < args.len() {
                push(idx, out);
            }
        }
        "exists" | "size" | "type" | "get" | "startsearch" => {
            if args.len() >= 2 {
                push(1, out);
            }
        }
        _ => {}
    }
}

/// The trimmed byte span of a single-token literal argument's name, or
/// `None` when the argument isn't a usable literal (`$var` / `[cmd]` /
/// multi-token / whitespace-bearing).  Mirrors `_literal_arg_value` +
/// `_normalise_literal_name`.
fn literal_arg_span(full: &str, cmd: &SegmentedCommand, arg_index: usize) -> Option<Span> {
    let word_index = arg_index + 1;
    if !cmd
        .single_token_word
        .get(word_index)
        .copied()
        .unwrap_or(false)
    {
        return None;
    }
    let tok = cmd.argv.get(word_index)?;
    if !matches!(tok.kind, TokenType::Esc | TokenType::Str) {
        return None;
    }
    let cstart = tok.span.start() as usize + tok.content_offset as usize;
    let cend = (tok.span.end() as usize).min(full.len());
    let raw = full.get(cstart..cend)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('$') || trimmed.starts_with('[') {
        return None;
    }
    if trimmed.chars().any(char::is_whitespace) {
        return None;
    }
    let lead = raw.len() - raw.trim_start().len();
    let abs_start = cstart + lead;
    let abs_end = abs_start + trimmed.len();
    Some(Span::new(
        u32::try_from(abs_start).unwrap_or(0),
        u32::try_from(abs_end).unwrap_or(0),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(src: &str) -> Vec<(usize, &str)> {
        let mut reg = CommandRegistry::build_default();
        reg.load_dialect(tcl_registry::dialects::DialectSet::IRULES);
        object_ref_spans(src, &reg)
            .into_iter()
            .map(|s| (s.start() as usize, &src[s.as_range()]))
            .collect()
    }

    #[test]
    fn pool_reference_in_event_body() {
        // `pool` inside a `when` body is found via BODY recursion.
        let got = spans("when HTTP_REQUEST {\n  pool web_pool\n}\n");
        assert!(got.iter().any(|(_, n)| *n == "web_pool"), "{got:?}");
    }

    #[test]
    fn pool_member_subcommand_is_not_a_reference() {
        let got = spans("when HTTP_REQUEST {\n  pool members foo\n}\n");
        assert!(!got.iter().any(|(_, n)| *n == "foo"), "{got:?}");
    }

    #[test]
    fn virtual_and_node_and_snatpool_refs() {
        let got = spans("when CLIENT_ACCEPTED {\n  virtual vs1\n  node n1\n  snatpool sp1\n}\n");
        let names: Vec<&str> = got.iter().map(|(_, n)| *n).collect();
        assert!(names.contains(&"vs1"), "{names:?}");
        assert!(names.contains(&"n1"), "{names:?}");
        assert!(names.contains(&"sp1"), "{names:?}");
    }

    #[test]
    fn class_lookup_data_group_ref() {
        let got = spans("when HTTP_REQUEST {\n  class lookup [HTTP::host] my_dg\n}\n");
        assert!(got.iter().any(|(_, n)| *n == "my_dg"), "{got:?}");
    }

    #[test]
    fn data_group_ref_inside_if_condition() {
        // `class match` inside an `if` *condition* (an EXPR-role word)
        // lives in a braced expression, not a top-level `[…]` token; the
        // EXPR recursion must still find the `dg` data-group reference.
        let got = spans(
            "when HTTP_REQUEST {\n  if {[class match [HTTP::host] equals dg]} {\n    pool p\n  }\n}\n",
        );
        let names: Vec<&str> = got.iter().map(|(_, n)| *n).collect();
        assert!(names.contains(&"dg"), "{names:?}");
        // The body ref is still found too.
        assert!(names.contains(&"p"), "{names:?}");
    }

    #[test]
    fn substituted_pool_name_is_not_a_reference() {
        // `pool $p` — the name is a variable, not a literal.
        let got = spans("when HTTP_REQUEST {\n  pool $p\n}\n");
        assert!(got.is_empty(), "{got:?}");
    }
}
