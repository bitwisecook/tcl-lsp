// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Code lens, document links, formatting, and inlay hints — end-to-end against
//! the packaged server. A cross-implementation conformance surface (raw
//! JSON-RPC): the Rust/Zed port must meet this spec.

use crate::common::{Lsp, unique_uri};

use serde_json::{Value, json};

/// A whole-document formatter returns a single replace edit; return its text.
fn full_text(edits: &Value) -> Option<String> {
    let arr = edits.as_array()?;
    let first = arr.first()?;
    first
        .get("newText")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// The lens array of a `code_lens` result (never null).
fn lenses(lsp: &mut Lsp, uri: &str) -> Vec<Value> {
    match lsp.code_lens(uri) {
        Value::Array(a) => a,
        _ => Vec::new(),
    }
}

/// A lens's `data.qname`, if present.
fn qname(lens: &Value) -> Option<&str> {
    lens.get("data")
        .and_then(|d| d.get("qname"))
        .and_then(Value::as_str)
}

/// Resolve the lens whose `data.qname` equals `qname`.
fn resolve_for_qname(lsp: &mut Lsp, lenses: &[Value], want: &str) -> Value {
    let lens = lenses
        .iter()
        .find(|l| qname(l) == Some(want))
        .unwrap_or_else(|| panic!("no lens for qname {want:?}"))
        .clone();
    lsp.code_lens_resolve(lens)
}

/// An inlay hint's `label`: a string or a list of `{value}` parts.
fn label_text(hint: &Value) -> String {
    match hint.get("label") {
        Some(Value::Array(parts)) => parts
            .iter()
            .map(|p| {
                p.get("value")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned()
            })
            .collect::<String>(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// `(character, label)` of every parameter-kind hint on `line`, sorted.
fn param_labels_on_line(hints: &Value, line: i64) -> Vec<(i64, String)> {
    let mut out = Vec::new();
    for h in hints.as_array().cloned().unwrap_or_default() {
        if h.get("kind").and_then(Value::as_i64) != Some(2) {
            continue;
        }
        let pos = h.get("position").cloned().unwrap_or(Value::Null);
        let char = pos.get("character").and_then(Value::as_i64);
        if pos.get("line").and_then(Value::as_i64) == Some(line)
            && let Some(c) = char
        {
            out.push((c, label_text(&h)));
        }
    }
    out.sort();
    out
}

// -- TestCodeLens --------------------------------------------------------

#[test]
fn test_proc_gets_reference_count_lens() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {} { return }\ngreet\ngreet\n");
    let ls = lenses(&mut lsp, &uri);
    let on_proc: Vec<&Value> = ls
        .iter()
        .filter(|l| l["range"]["start"]["line"].as_i64() == Some(0))
        .collect();
    assert!(!on_proc.is_empty(), "{ls:?}");
    assert!(
        on_proc.iter().any(|l| qname(l) == Some("::greet")),
        "{on_proc:?}"
    );
}

#[test]
fn test_no_lens_without_procs() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\nputs $x\n");
    assert!(lenses(&mut lsp, &uri).is_empty());
}

#[test]
fn test_count_matches_reference_list_for_forward_call() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "fwd637\nproc fwd637 {} { return }\n");
    let ls = lenses(&mut lsp, &uri);
    let resolved = resolve_for_qname(&mut lsp, &ls, "::fwd637");
    assert_eq!(resolved["command"]["title"], json!("1 reference"));
    // The reference list at the proc name (declaration excluded) agrees.
    let refs = match lsp.references(&uri, 1, 5, false) {
        Value::Array(a) => a,
        _ => Vec::new(),
    };
    assert_eq!(refs.len(), 1, "{refs:?}");
    let title = resolved["command"]["title"].as_str().unwrap_or("");
    assert!(title.starts_with(&refs.len().to_string()), "{title:?}");
}

#[test]
fn test_unresolved_call_scoped_to_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval nsa644 {\n    dup644\n    proc dup644 {} {}\n}\n\
         namespace eval nsb644 {\n    proc dup644 {} {}\n}\n",
    );
    let ls = lenses(&mut lsp, &uri);
    let a = resolve_for_qname(&mut lsp, &ls, "::nsa644::dup644");
    let b = resolve_for_qname(&mut lsp, &ls, "::nsb644::dup644");
    assert_eq!(a["command"]["title"], json!("1 reference"));
    assert_eq!(b["command"]["title"], json!("0 references"));
}

/// Resolve the method / member lens anchored on `line` (0-based) and return
/// the resolved `command`.  Since issue #956, member lenses resolve lazily
/// the same way proc/class lenses do (range + `data`, no `command` until
/// `codeLens/resolve` — see `tcl-lsp-server`'s `code_lens` handler and the
/// `#724` "reference is not active" defect it fixed for proc/class lenses),
/// so a raw, unresolved listing has no `command` for a caller to read
/// directly; this always resolves first.
fn resolve_member_lens_on_line(lsp: &mut Lsp, ls: &[Value], line: i64) -> Value {
    let lens = ls
        .iter()
        .find(|l| l["range"]["start"]["line"].as_i64() == Some(line))
        .unwrap_or_else(|| panic!("no lens anchored at line {line}: {ls:?}"))
        .clone();
    lsp.code_lens_resolve(lens)["command"].clone()
}

#[test]
fn test_method_lens_counts_external_obj_dispatch_issue_864() {
    // Regression for issue #864: the lens above `method get` must count the
    // external `$b get foo` dispatch (`set b [Bar new]`), reading
    // "1 reference" rather than the "0 references" the old heuristic showed.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Bar {\n\
         \x20  variable _options\n\
         \x20   constructor {args} {\n\
         \x20        set _options $args\n\
         \x20   }\n\
         \n\
         \x20   method get {key} {\n\
         \x20       return [dict get $_options $key]\n\
         \x20   }\n\
         \n\
         }\n\
         set b [Bar new]\n\
         puts [$b get foo]\n",
    );
    let ls = lenses(&mut lsp, &uri);
    // `method get` is on line 6 (0-based).
    let command = resolve_member_lens_on_line(&mut lsp, &ls, 6);
    assert_eq!(command["title"], json!("1 reference"), "{ls:?}");
    // Regression for issue #956: the lens must resolve to a *clickable*
    // command, not the empty-id inert shape (the `#724` defect recurring
    // for methods — the count above was already correct before the fix;
    // only the command was empty).
    assert_eq!(
        command["command"],
        json!("tcl-lsp.showReferences"),
        "method lens resolved to an inert command: {command:?}"
    );
    // The lens count must equal Find All References (declaration excluded) on
    // the `get` method-name token (`    method get` → col 11).
    let refs = match lsp.references(&uri, 6, 11, false) {
        Value::Array(a) => a,
        _ => Vec::new(),
    };
    assert_eq!(refs.len(), 1, "peek disagrees with lens: {refs:?}");
}

#[test]
fn test_method_lens_zero_when_uncalled() {
    // TN: an instance method with no dispatch site reads "0 references" but
    // still resolves to a real, clickable command (an empty peek is a valid
    // target — the count being zero must not leave the lens inert).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Solo {\n    method lonely {} {}\n}\nSolo new\n",
    );
    let ls = lenses(&mut lsp, &uri);
    let command = resolve_member_lens_on_line(&mut lsp, &ls, 1);
    assert_eq!(command["title"], json!("0 references"), "{ls:?}");
    assert_eq!(
        command["command"],
        json!("tcl-lsp.showReferences"),
        "{command:?}"
    );
}

#[test]
fn test_classmethod_lens_resolves_to_clickable_command() {
    // TP: a classmethod lens (a distinct member map from instance methods)
    // resolves the same way, with its own disambiguated qname.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Factory {\n    classmethod make {} {\n        return [Factory new]\n    }\n}\nFactory make\n",
    );
    let ls = lenses(&mut lsp, &uri);
    let command = resolve_member_lens_on_line(&mut lsp, &ls, 1);
    assert_eq!(command["title"], json!("1 reference"), "{ls:?}");
    assert_eq!(
        command["command"],
        json!("tcl-lsp.showReferences"),
        "{command:?}"
    );
}

#[test]
fn test_find_references_on_classmethod_finds_bare_class_command_dispatch() {
    // The `textDocument/references` peek (not just the lens) must also see
    // the bare `Factory make` dispatch — including through an inheriting
    // subclass's own command (`SubFactory make`), which never overrides
    // `make` so its dispatch still targets `Factory::make`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Factory {\n    classmethod make {} { return [Factory new] }\n}\noo::class create SubFactory {\n    superclass Factory\n}\nFactory make\nSubFactory make\n",
    );
    // cursor on the `make` declaration name (line 1, col 16).
    let refs = match lsp.references(&uri, 1, 16, true) {
        Value::Array(a) => a,
        _ => Vec::new(),
    };
    let lines: std::collections::BTreeSet<i64> = refs
        .iter()
        .filter_map(|l| l["range"]["start"]["line"].as_i64())
        .collect();
    assert_eq!(
        lines,
        std::collections::BTreeSet::from([1, 6, 7]),
        "decl + `Factory make` + inheriting `SubFactory make`: {refs:?}"
    );
}

#[test]
fn test_property_lens_counts_my_dispatch_and_resolves_clickable() {
    // TP mirroring `test_method_lens_counts_external_obj_dispatch_issue_864`:
    // a `property`'s auto-generated accessor is dispatched via `my <name>`,
    // just like a method, so its lens must count those sites and resolve to
    // a clickable command the same way (issue #992).
    // `property` is Tcl 9.0+, so pin the dialect via the in-source directive
    // (shifts every line below down by one).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl9.0\noo::class create Widget {\n    property size -get {return $mySize} -set {set mySize $value}\n    method bump {} { my size ; my size }\n}\n",
    );
    let ls = lenses(&mut lsp, &uri);
    // `property size` is on line 2 (0-based).
    let command = resolve_member_lens_on_line(&mut lsp, &ls, 2);
    assert_eq!(command["title"], json!("2 references"), "{ls:?}");
    assert_eq!(
        command["command"],
        json!("tcl-lsp.showReferences"),
        "property lens resolved to an inert command: {command:?}"
    );
    // The lens count must equal Find All References (declaration excluded) on
    // the `size` property-name token (`    property size` → col 13).
    let refs = match lsp.references(&uri, 2, 13, false) {
        Value::Array(a) => a,
        _ => Vec::new(),
    };
    assert_eq!(refs.len(), 2, "peek disagrees with lens: {refs:?}");
}

#[test]
fn test_property_lens_zero_when_unused() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl9.0\noo::class create Widget {\n    property size\n}\n",
    );
    let ls = lenses(&mut lsp, &uri);
    let command = resolve_member_lens_on_line(&mut lsp, &ls, 2);
    assert_eq!(command["title"], json!("0 references"), "{ls:?}");
    assert_eq!(
        command["command"],
        json!("tcl-lsp.showReferences"),
        "{command:?}"
    );
}

#[test]
fn test_property_method_constructor_and_class_all_get_lenses_issue_992() {
    // Repro from issue #992: a class with a `property`, a `constructor`, and
    // a `method` gets a lens for every one of them — the class itself, the
    // property, the constructor, and the method. The constructor's lens is
    // scoped to the next-chain relationship (issue #992's own "Constructors
    // / destructors" follow-up), not a general dispatch count, so it reads
    // "0 references" here (nothing chains into it).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl9.0\noo::class create Widget {\n    property size -get {return $mySize} -set {set mySize $value}\n    constructor {} { set mySize 10 }\n    method use {} { return 1 }\n}\nset w [Widget new]\n$w use\n",
    );
    let ls = lenses(&mut lsp, &uri);
    assert_eq!(
        ls.len(),
        4,
        "class + property + constructor + method: {ls:?}"
    );

    let class_lens = resolve_member_lens_on_line(&mut lsp, &ls, 1);
    assert_eq!(class_lens["title"], json!("1 reference"), "{ls:?}");

    let property_lens = resolve_member_lens_on_line(&mut lsp, &ls, 2);
    assert_eq!(property_lens["title"], json!("0 references"), "{ls:?}");
    assert_eq!(property_lens["command"], json!("tcl-lsp.showReferences"));

    let constructor_lens = resolve_member_lens_on_line(&mut lsp, &ls, 3);
    assert_eq!(constructor_lens["title"], json!("0 references"), "{ls:?}");
    assert_eq!(constructor_lens["command"], json!("tcl-lsp.showReferences"));

    let method_lens = resolve_member_lens_on_line(&mut lsp, &ls, 4);
    assert_eq!(method_lens["title"], json!("1 reference"), "{ls:?}");
    assert_eq!(method_lens["command"], json!("tcl-lsp.showReferences"));
}

#[test]
fn test_constructor_lens_counts_and_resolves_subclass_next_chain() {
    // TP for issue #992's own follow-up: a subclass constructor chaining to
    // its superclass's via `next` is a name-independent but still
    // meaningful reference — the superclass constructor's lens must count
    // it and resolve to a clickable command, the same as every other lens
    // kind.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { next }\n}\n",
    );
    let ls = lenses(&mut lsp, &uri);
    // `Base`'s constructor is on line 1.
    let command = resolve_member_lens_on_line(&mut lsp, &ls, 1);
    assert_eq!(command["title"], json!("1 reference"), "{ls:?}");
    assert_eq!(
        command["command"],
        json!("tcl-lsp.showReferences"),
        "constructor lens resolved to an inert command: {command:?}"
    );
    // The lens count must equal Find All References (declaration excluded)
    // on the `constructor` keyword (`    constructor` → col 4).
    let refs = match lsp.references(&uri, 1, 4, false) {
        Value::Array(a) => a,
        _ => Vec::new(),
    };
    assert_eq!(refs.len(), 1, "peek disagrees with lens: {refs:?}");
}

// -- TestDocumentLinks ---------------------------------------------------

#[test]
fn test_source_command_is_linked() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "source other.tcl\nputs done\n");
    let links = match lsp.document_links(&uri) {
        Value::Array(a) => a,
        _ => Vec::new(),
    };
    assert!(
        links.iter().any(|l| l
            .get("tooltip")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("other.tcl")),
        "{links:?}"
    );
}

#[test]
fn test_package_require_is_linked() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "package require Tcl\n");
    let links = match lsp.document_links(&uri) {
        Value::Array(a) => a,
        _ => Vec::new(),
    };
    assert!(
        links
            .iter()
            .any(|l| l["range"]["start"]["line"].as_i64() == Some(0)),
        "{links:?}"
    );
}

// -- TestFormatting ------------------------------------------------------

#[test]
fn test_full_document_formatting_normalises_spacing() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc  f {}    {\nputs   hi\n}\n");
    let formatted = full_text(&lsp.formatting(&uri, 4, true));
    let formatted = formatted.expect("formatting produced no edit");
    assert!(formatted.contains("proc f {} {"), "{formatted:?}");
    assert!(formatted.contains("    puts hi"), "{formatted:?}");
}

#[test]
fn test_already_formatted_is_stable() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    puts hi\n}\n";
    lsp.open_ready(&uri, src);
    let edits = lsp.formatting(&uri, 4, true);
    // No change needed → either no edits, or an edit reproducing the text.
    let non_empty = edits.as_array().is_some_and(|a| !a.is_empty());
    if non_empty {
        assert_eq!(full_text(&edits).as_deref(), Some(src));
    }
}

/// Issue #1186 — the formatting engine consumes registry grammar, so the
/// absolute global spellings C Tcl resolves to the same commands
/// (`namespace which -command ::if` → `::if`) format identically to their
/// bare forms. The old `name == "if"` / `"for"` / `"try"` comparisons did
/// not fire for them at all.
#[test]
fn test_formatting_qualified_control_flow_matches_bare_form() {
    let mut lsp = Lsp::tcl();
    for (bare_src, head, qualified) in [
        (
            "if {$x} then {\nputs yes\n} else {\nputs no\n}\n",
            "if",
            "::if",
        ),
        (
            "for {set i 0} {$i < 3} {incr i} {\nputs $i\n}\n",
            "for",
            "::for",
        ),
        (
            "try {\nrisky\n} on error {msg opts} {\nputs $msg\n}\n",
            "try",
            "::try",
        ),
    ] {
        let bare_uri = unique_uri("tcl");
        lsp.open_ready(&bare_uri, bare_src);
        let bare = full_text(&lsp.formatting(&bare_uri, 4, true)).expect("formatting edit");

        let qualified_src = bare_src.replacen(head, qualified, 1);
        let q_uri = unique_uri("tcl");
        lsp.open_ready(&q_uri, &qualified_src);
        let qualified_out = full_text(&lsp.formatting(&q_uri, 4, true)).expect("formatting edit");

        assert_eq!(
            qualified_out,
            bare.replacen(head, qualified, 1),
            "{qualified} formatted differently from {head}"
        );
    }
}

/// Issue #1275 — the formatter lays a command out under the grammar of the
/// command it **is**, not the one it is spelled as, end-to-end through the
/// packaged server.
///
/// tclsh-proof, byte-identical on 8.6.16 and 9.0.4: after
/// `interp alias {} guard {} if`, `guard {1} {puts a}` runs the `if`; after
/// `rename if guard` the same holds and `if` is gone
/// (`info commands if` → empty); after `proc if {c b} {…}` the call runs the
/// user procedure.
#[test]
fn test_formatting_follows_effective_command_identity() {
    let mut lsp = Lsp::tcl();
    // A body-role argument is expanded onto its own lines; an unrecognised
    // command's braced word is left exactly as written.  That difference is
    // the witness.
    let mut expanded = |src: &str| -> bool {
        let uri = unique_uri("tcl");
        lsp.open_ready(&uri, src);
        // An already-normalised document yields no edit at all, which for
        // these inputs means "left as written".
        full_text(&lsp.formatting(&uri, 4, true))
            .unwrap_or_else(|| src.to_owned())
            .contains("{\n    puts a\n}")
    };

    // TP — an alias of `if`, and the `::`-qualified spelling of that alias.
    assert!(expanded(
        "interp alias {} guard {} if\nguard {$x} {puts a}\n"
    ));
    assert!(expanded(
        "interp alias {} guard {} if\n::guard {$x} {puts a}\n"
    ));
    // TP — a renamed `if`.
    assert!(expanded("rename if guard\nguard {$x} {puts a}\n"));

    // FP guard — the vacated name must not keep the built-in's layout.
    assert!(!expanded("rename if guard\nif {$x} {puts a}\n"));
    // FP guard — a user `proc if` takes the name over.
    assert!(!expanded("proc if {c b} { return 1 }\nif {$x} {puts a}\n"));
    // TN — a dynamic binding proves nothing in either direction.
    assert!(!expanded("rename $old guard\nguard {$x} {puts a}\n"));
    assert!(expanded("rename $old guard\nif {$x} {puts a}\n"));
    // Baseline — an unbound `guard` has no body argument at all.
    assert!(!expanded("set y 1\nguard {$x} {puts a}\n"));
}

/// Issue #1186 — `for`'s `start` / `next` scripts stay on the header line
/// (registry `ArgPresentation::InlineScript`) while only the body expands,
/// and range formatting agrees with whole-document formatting.
#[test]
fn test_range_formatting_keeps_for_header_inline() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "for {set i 0} {$i < 3} {incr i} {\nputs $i\n}\n");
    let range = json!({
        "start": { "line": 0, "character": 0 },
        "end": { "line": 2, "character": 1 },
    });
    let edits = lsp.range_formatting(&uri, range, 4);
    let arr = edits.as_array().cloned().unwrap_or_default();
    let text: String = arr
        .iter()
        .filter_map(|e| e["newText"].as_str())
        .collect::<Vec<_>>()
        .join("");
    if !text.is_empty() {
        assert!(
            text.contains("for {set i 0} {$i < 3} {incr i} {"),
            "for header was split across lines: {text:?}"
        );
    }
}

/// Issue #1196 — formatting must never change a proc's arity. C Tcl 9
/// collapses the backslash-newline in a pre-pass before the parameter word is
/// list-parsed (even inside braces), so this proc has two required
/// parameters; the old formatter emitted `{a\ b}`, which is one *optional*
/// parameter `a` defaulting to `b`.
#[test]
fn test_formatting_preserves_proc_arity_across_backslash_newline() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc f {a\\\n b} {return}\n");
    let formatted = full_text(&lsp.formatting(&uri, 4, true)).expect("formatting produced no edit");
    assert!(
        formatted.contains("proc f {a b} {"),
        "arity-changing reflow: {formatted:?}"
    );
    assert!(
        !formatted.contains("a\\ b"),
        "two parameters fused into one defaulted parameter: {formatted:?}"
    );
}

/// Issue #1196 — the same document formatted twice is a fixed point, and the
/// escaped-space form (genuinely one element) keeps its identity.
#[test]
fn test_formatting_param_lists_are_idempotent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc f {a\\ b   {c 1}   d} {return}\n");
    let once = full_text(&lsp.formatting(&uri, 4, true)).expect("formatting produced no edit");
    assert!(
        once.contains("proc f {{a b} {c 1} d} {"),
        "escaped-space element lost: {once:?}"
    );

    let uri2 = unique_uri("tcl");
    lsp.open_ready(&uri2, &once);
    if let Some(twice) = full_text(&lsp.formatting(&uri2, 4, true)) {
        assert_eq!(twice, once, "formatting is not idempotent");
    }
}

#[test]
fn test_range_formatting_returns_edits() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc  f {}    {\nputs   hi\n}\n");
    let range = json!({
        "start": { "line": 0, "character": 0 },
        "end": { "line": 2, "character": 1 },
    });
    let edits = lsp.range_formatting(&uri, range, 4);
    let arr = edits.as_array().cloned().unwrap_or_default();
    assert!(!arr.is_empty(), "{edits:?}");
    assert!(
        arr[0]["newText"]
            .as_str()
            .unwrap_or("")
            .contains("proc f {} {"),
        "{:?}",
        arr[0]
    );
}

// -- TestInlayHints ------------------------------------------------------

#[test]
fn test_provider_responds_with_a_list() {
    // Inlay hints are gated off by default; the provider must still answer with
    // a well-formed (here empty) list, never an error.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\n");
    let result = lsp.inlay_hints(&uri, (0, 0), (1, 0));
    assert!(result.is_array(), "{result:?}");
}

#[test]
fn test_hint_kinds_are_valid_when_present() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc add {a b} { expr {$a + $b} }\nadd 1 2\n");
    for hint in lsp
        .inlay_hints(&uri, (0, 0), (2, 0))
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        let k = hint.get("kind").and_then(Value::as_i64);
        assert!(k == Some(1) || k == Some(2), "{hint:?}");
    }
}

// -- TestInlayHintOptionalPositionals ------------------------------------
// These run against `Lsp::inlay()` (inlay hints on) so the provider actually
// produces hints — the default server keeps them off.

#[test]
fn test_single_positional_binds_required_string_slot() {
    let mut lsp = Lsp::inlay();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts hello\n");
    let labels = param_labels_on_line(&lsp.inlay_hints(&uri, (0, 0), (1, 0)), 0);
    let names: Vec<String> = labels.iter().map(|(_, n)| n.clone()).collect();
    assert!(names.contains(&"string:".to_owned()), "{labels:?}");
    assert!(!names.contains(&"channelId:".to_owned()), "{labels:?}");
}

#[test]
fn test_two_positionals_label_channel_then_string() {
    let mut lsp = Lsp::inlay();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts $chan hello\n");
    let labels = param_labels_on_line(&lsp.inlay_hints(&uri, (0, 0), (1, 0)), 0);
    let names: Vec<String> = labels.iter().map(|(_, n)| n.clone()).collect();
    assert_eq!(
        names,
        vec!["channelId:".to_owned(), "string:".to_owned()],
        "{labels:?}"
    );
}

#[test]
fn test_leading_flag_does_not_shift_required_slot() {
    let mut lsp = Lsp::inlay();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts -nonewline hello\n");
    let labels = param_labels_on_line(&lsp.inlay_hints(&uri, (0, 0), (1, 0)), 0);
    let names: Vec<String> = labels.iter().map(|(_, n)| n.clone()).collect();
    assert!(names.contains(&"string:".to_owned()), "{names:?}");
    assert!(!names.contains(&"channelId:".to_owned()), "{names:?}");
}

#[test]
fn test_no_documentation_placeholder_labels() {
    let mut lsp = Lsp::inlay();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "lsearch $mylist needle\n");
    let labels = param_labels_on_line(&lsp.inlay_hints(&uri, (0, 0), (1, 0)), 0);
    let names: Vec<String> = labels.iter().map(|(_, n)| n.clone()).collect();
    assert!(names.contains(&"list:".to_owned()), "{names:?}");
    assert!(names.contains(&"pattern:".to_owned()), "{names:?}");
    assert!(
        !names.iter().any(|n| n == "options:" || n == "switches:"),
        "{names:?}"
    );
}

// -- TestInlayToggleIndependenceE2E --------------------------------------
// The single `inlayHints` toggle was split into two independent options, both
// off by default: `inlayTypeHints` and `inlayParameterHints`. Enabling one must
// not turn on the other. Each test owns its server, so `apply_configuration_settle`
// needs no matching restore.

#[test]
fn test_parameter_hints_only_produce_labels_no_type_hints() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.apply_configuration_settle(
        json!({ "features": { "inlayTypeHints": false, "inlayParameterHints": true } }),
        &uri,
        |c| {
            c["features"].get("inlayParameterHints") == Some(&json!(true))
                && c["features"].get("inlayTypeHints") == Some(&json!(false))
        },
    );
    lsp.open_ready(&uri, "set x 42\nputs hello\n");
    let hints = lsp.inlay_hints(&uri, (0, 0), (2, 0));
    let arr = hints.as_array().cloned().unwrap_or_default();
    assert!(
        arr.iter()
            .any(|h| h.get("kind").and_then(Value::as_i64) == Some(2)),
        "{arr:?}"
    ); // Parameter present
    assert!(
        arr.iter()
            .all(|h| h.get("kind").and_then(Value::as_i64) != Some(1)),
        "{arr:?}"
    ); // no Type leaked in
}

#[test]
fn test_type_hints_only_emit_no_parameter_labels() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.apply_configuration_settle(
        json!({ "features": { "inlayTypeHints": true, "inlayParameterHints": false } }),
        &uri,
        |c| {
            c["features"].get("inlayTypeHints") == Some(&json!(true))
                && c["features"].get("inlayParameterHints") == Some(&json!(false))
        },
    );
    lsp.open_ready(&uri, "set x 42\nputs hello\n");
    let hints = lsp.inlay_hints(&uri, (0, 0), (2, 0));
    let arr = hints.as_array().cloned().unwrap_or_default();
    assert!(
        arr.iter()
            .any(|h| h.get("kind").and_then(Value::as_i64) == Some(1)),
        "{arr:?}"
    ); // Type present
    assert!(
        arr.iter()
            .all(|h| h.get("kind").and_then(Value::as_i64) != Some(2)),
        "{arr:?}"
    ); // no Parameter leaked in
}

// -- TestInlayLegacyAliasE2E ---------------------------------------------
// The retired `features.inlayHints` key is a backward-compatible alias that
// enables *type* hints only — parameter hints stay off.

#[test]
fn test_legacy_inlay_hints_alias_enables_type_only() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.apply_configuration_settle(json!({ "features": { "inlayHints": true } }), &uri, |c| {
        c["features"].get("inlayTypeHints") == Some(&json!(true))
    });
    let eff = lsp.effective_config(&uri);
    assert_eq!(
        eff["features"].get("inlayTypeHints"),
        Some(&json!(true)),
        "{:?}",
        eff["features"]
    );
    assert_eq!(
        eff["features"].get("inlayParameterHints"),
        Some(&json!(false)),
        "{:?}",
        eff["features"]
    );
    lsp.open_ready(&uri, "set x 42\nputs hello\n");
    let hints = lsp.inlay_hints(&uri, (0, 0), (2, 0));
    let arr = hints.as_array().cloned().unwrap_or_default();
    assert!(
        arr.iter()
            .any(|h| h.get("kind").and_then(Value::as_i64) == Some(1)),
        "{arr:?}"
    ); // Type present
    assert!(
        arr.iter()
            .all(|h| h.get("kind").and_then(Value::as_i64) != Some(2)),
        "{arr:?}"
    ); // no Parameter
}
