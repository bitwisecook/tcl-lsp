//! Behaviour-driven coverage of the `tcl-lsp-core` references and rename
//! providers (`references::references`, `rename::rename`,
//! `rename::prepare_rename`, plus `is_safe_symbol_name`).
//!
//! There is no single upstream pytest source — these tests are derived from
//! the providers' public API (see `src/references.rs` / `src/rename.rs`) and
//! from Tcl name-resolution semantics, with the Tcl-semantic facts pinned to
//! real C-Tcl (tclsh8.6 / tclsh9.0 via `scripts/dev/tclsh_check.sh`).
//!
//! C-Tcl proof model
//! -----------------
//! A "reference" is a genuine call/use site in runnable Tcl, and a rename is
//! correct exactly when the renamed script still resolves the same way Tcl
//! itself would. Every snippet below is a COMPLETE runnable Tcl script in
//! which the proc/method is actually invoked, or the variable actually read,
//! at the sites asserted as references. The reference/rename SET (which
//! positions) is an editor-structural fact about the parse, so it is asserted
//! structurally; the proof that the set is *right* is that those positions are
//! exactly the call/use sites Tcl's own name resolution binds together — and
//! the sites it keeps APART (a same-named local in another proc, a same-named
//! proc in another namespace) are left untouched. Each test cites the
//! corresponding `tclsh:` observation.
//!
//! Verified C-Tcl facts (tclsh8.6 + tclsh9.0, identical output):
//!   * `proc greet {} {return hi}; puts [greet]; puts [greet]`  -> hi / hi
//!     (the two `greet` words are real invocations of the one proc).
//!   * `set x 1; puts $x; puts $x`  -> 1 / 1  (two real reads of `x`).
//!   * `proc a {} {set v 10; return $v}; proc b {} {set v 20; return $v}`
//!     -> a=10, b=20  (each `v` is a distinct local; renaming a's `v` must
//!     not touch b's `v`).
//!   * namespace: `::myns::greet` and a short `greet` inside
//!     `namespace eval ::myns` both invoke the one proc -> ns-hi / ns-hi;
//!     and `::a::helper` vs `::b::helper` are two independent procs
//!     -> a / b  (renaming one must not touch the other).
//!   * renamed forms still run: `salute`/`salute`, `$y`/`$y`,
//!     `::myns::hello` + short `hello` -> identical output to the original.

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::references::references;
use tcl_lsp_core::rename::{TextEdit, is_safe_symbol_name, prepare_rename, rename};
use tcl_registry::CommandRegistry;

/// Build an analysis exactly the way the existing port files do
/// (`call_hierarchy.rs`, `selection_range.rs`).
fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

/// The set of start lines touched by a list of reference ranges, sorted and
/// de-duplicated — the load-bearing structural fact for most assertions.
fn ref_lines(ranges: &[LspRange]) -> Vec<u32> {
    let mut v: Vec<u32> = ranges.iter().map(|r| r.start_line).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// The set of start lines touched by a list of rename edits.
fn edit_lines(edits: &[TextEdit]) -> Vec<u32> {
    let mut v: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    v.sort_unstable();
    v.dedup();
    v
}

// ---------------------------------------------------------------------------
// references — procs
// ---------------------------------------------------------------------------

#[test]
fn references_proc_from_definition_includes_decl_and_both_calls() {
    // tclsh: `proc greet {} {return hi}; puts [greet]; puts [greet]` -> hi/hi.
    // The two `greet` words (lines 1 and 2) are genuine invocations of the
    // proc declared on line 0 — so references from the declaration must be
    // exactly {decl line 0, call line 1, call line 2}.
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    // Cursor on `greet` in the declaration (line 0, col 6).
    let refs = references(src, "tcl", 0, 6, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1, 2],
        "decl + both call sites expected; got {refs:?}",
    );
    // First entry is the declaration on line 0 (include_declaration = true).
    assert_eq!(refs[0].start_line, 0, "decl should lead: {refs:?}");
}

#[test]
fn references_proc_from_call_site_finds_the_same_set() {
    // Triggering Find-All-References from a call site resolves to the same
    // proc and therefore the same reference set as from the declaration.
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    // Cursor on the first `greet` CALL (line 1, inside `[greet]`).
    let refs = references(src, "tcl", 1, 7, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1, 2],
        "call-site Find-References should match the proc's whole set; got {refs:?}",
    );
}

#[test]
fn references_proc_exclude_declaration_drops_the_decl_line() {
    // include_declaration = false omits the defining span; only call sites
    // remain (lines 1 and 2).
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    let with_decl = references(src, "tcl", 0, 6, &analysis, true);
    let without_decl = references(src, "tcl", 0, 6, &analysis, false);
    assert_eq!(
        ref_lines(&without_decl),
        vec![1, 2],
        "only call sites when decl excluded; got {without_decl:?}",
    );
    assert_eq!(
        with_decl.len(),
        without_decl.len() + 1,
        "excluding the declaration drops exactly one entry",
    );
}

#[test]
fn references_proc_only_at_real_call_sites_not_substrings() {
    // tclsh: `greeter` is a *different* command from `greet`; only the two
    // bare `greet` words are real invocations of the proc. The provider must
    // not match the `greet` substring inside the proc name `greeter`.
    // Snippet is runnable: both procs return, top level calls each once.
    let src = "proc greet {} { return a }\nproc greeter {} { return b }\ngreet\ngreeter\n";
    let analysis = analyse(src);
    // References for `greet` (cursor on its decl, line 0).
    let refs = references(src, "tcl", 0, 6, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&0), "decl missing: {refs:?}");
    assert!(
        lines.contains(&2),
        "the `greet` call on line 2 missing: {refs:?}"
    );
    assert!(
        !lines.contains(&1),
        "must NOT reference the `greeter` declaration on line 1: {refs:?}",
    );
    assert!(
        !lines.contains(&3),
        "must NOT match the distinct `greeter` call on line 3: {refs:?}",
    );
}

// ---------------------------------------------------------------------------
// references — variables
// ---------------------------------------------------------------------------

#[test]
fn references_var_includes_definition_and_every_read() {
    // tclsh: `set x 1; puts $x; puts $x` -> 1/1. The two `$x` reads (lines 1
    // and 2) are genuine uses of the variable defined on line 0.
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    // Cursor inside the first `$x` read (line 1).
    let refs = references(src, "tcl", 1, 6, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1, 2],
        "definition + both read sites expected; got {refs:?}",
    );
}

#[test]
fn references_var_exclude_declaration_keeps_only_reads() {
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    let refs = references(src, "tcl", 1, 6, &analysis, false);
    assert_eq!(
        ref_lines(&refs),
        vec![1, 2],
        "only the two read sites when declaration excluded; got {refs:?}",
    );
}

#[test]
fn references_var_scoped_to_its_proc_not_a_same_named_local_elsewhere() {
    // tclsh: `proc a {} {set v 10; return $v}` and
    //        `proc b {} {set v 20; return $v}` -> a=10, b=20.
    // The two `v`s are independent locals (one per call frame). References to
    // proc a's `v` must stay inside proc a (lines 1-2) and never reach proc
    // b's `v` (lines 5-6).
    let src = "proc a {} {\n    set v 10\n    return $v\n}\nproc b {} {\n    set v 20\n    return $v\n}\n";
    let analysis = analyse(src);
    // Cursor on `$v` in proc a's body (line 2).
    let refs = references(src, "tcl", 2, 12, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(
        lines.iter().all(|&l| l == 1 || l == 2),
        "proc a's `v` references must stay within proc a (lines 1-2); got {refs:?}",
    );
    assert!(
        !lines.contains(&5) && !lines.contains(&6),
        "must NOT reference proc b's same-named local `v`; got {refs:?}",
    );
}

// ---------------------------------------------------------------------------
// references — namespaces (qualified vs short, cross-namespace isolation)
// ---------------------------------------------------------------------------

#[test]
fn references_namespaced_proc_matches_qualified_and_short_calls() {
    // tclsh: a proc in `::myns` is invoked both as `::myns::greet` and as a
    // short `greet` inside `namespace eval ::myns` -> ns-hi/ns-hi. Both are
    // real invocations of the one proc, so both are references.
    let src = "namespace eval ::myns {\n    proc greet {} { return ns-hi }\n}\n::myns::greet\nnamespace eval ::myns {\n    greet\n}\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1).
    let refs = references(src, "tcl", 1, 9, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&1), "declaration missing: {refs:?}");
    assert!(
        lines.contains(&3),
        "qualified `::myns::greet` call missing: {refs:?}"
    );
    assert!(
        lines.contains(&5),
        "short in-namespace `greet` call missing: {refs:?}"
    );
}

#[test]
fn references_namespaced_proc_does_not_leak_to_same_name_in_other_namespace() {
    // tclsh: `::a::helper` -> "a" and `::b::helper` -> "b" are two distinct
    // procs. References to `::a::helper` must include its decl and its own
    // call, and must NOT include `::b::helper`'s decl or call.
    let src = "namespace eval ::a {\n    proc helper {} { return a }\n}\nnamespace eval ::b {\n    proc helper {} { return b }\n}\n::a::helper\n::b::helper\n";
    let analysis = analyse(src);
    // Cursor on `::a`'s helper declaration (line 1).
    let refs = references(src, "tcl", 1, 9, &analysis, true);
    let lines = ref_lines(&refs);
    assert_eq!(
        lines,
        vec![1, 6],
        "exactly ::a::helper decl (line 1) + its call (line 6); got {refs:?}",
    );
    assert!(
        !lines.contains(&4) && !lines.contains(&7),
        "must not reference ::b::helper (decl line 4 / call line 7): {refs:?}",
    );
}

// ---------------------------------------------------------------------------
// references — non-symbols / robustness
// ---------------------------------------------------------------------------

#[test]
fn references_builtin_command_yields_nothing() {
    // `puts` is a built-in, not a user proc — no references are surfaced.
    let src = "puts hello\nputs world\n";
    let analysis = analyse(src);
    assert!(
        references(src, "tcl", 0, 1, &analysis, true).is_empty(),
        "built-in `puts` should have no provider references",
    );
}

#[test]
fn references_unrelated_word_yields_nothing() {
    // A bare literal argument (`hello`) is neither a proc, class, nor var.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(references(src, "tcl", 0, 6, &analysis, true).is_empty());
}

#[test]
fn references_out_of_range_and_empty_file_do_not_panic() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    // Way past EOF — must return empty, not panic.
    assert!(references(src, "tcl", 99, 0, &analysis, true).is_empty());
    assert!(references(src, "tcl", 0, 999, &analysis, true).is_empty());
    // Empty document.
    let empty = analyse("");
    assert!(references("", "tcl", 0, 0, &empty, true).is_empty());
}

// ---------------------------------------------------------------------------
// rename — procs
// ---------------------------------------------------------------------------

#[test]
fn rename_proc_updates_definition_and_all_call_sites() {
    // tclsh: renaming `greet`->`salute` everywhere keeps the script runnable
    // (`proc salute {} {return hi}; puts [salute]; puts [salute]` -> hi/hi).
    // The rename set must be exactly the decl + both calls.
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    // Cursor on the declaration (line 0).
    let edits = rename(src, "tcl", 0, 6, "salute", &analysis, None);
    assert_eq!(
        edit_lines(&edits),
        vec![0, 1, 2],
        "decl + both call sites rewritten; got {edits:?}",
    );
    assert!(
        edits.iter().all(|e| e.new_text == "salute"),
        "every top-level edit is the bare new name; got {edits:?}",
    );
    // The declaration edit leads and replaces just `greet` -> `salute`
    // (col 5 = start of the proc name in `proc greet`).
    assert_eq!(edits[0].range.start_line, 0);
    assert_eq!(edits[0].range.start_character, 5);
}

#[test]
fn rename_proc_from_call_site_rewrites_declaration_too() {
    // Renaming from a call site rewrites the declaration as well — same set.
    let src = "proc greet {} { return hi }\ngreet\ngreet\n";
    let analysis = analyse(src);
    // Cursor on the first call (line 1).
    let edits = rename(src, "tcl", 1, 2, "salute", &analysis, None);
    assert_eq!(edit_lines(&edits), vec![0, 1, 2], "{edits:?}");
    assert!(
        edits.iter().any(|e| e.range.start_line == 0),
        "decl rewritten: {edits:?}"
    );
}

#[test]
fn rename_proc_rejected_when_new_name_collides_with_existing_proc() {
    // tclsh: `hello` already names a distinct proc. Renaming `greet`->`hello`
    // would shadow it (two procs would claim `::hello`), so the provider
    // refuses with an empty edit set rather than merge them.
    let src = "proc greet {} { return a }\nproc hello {} { return b }\ngreet\n";
    let analysis = analyse(src);
    let edits = rename(src, "tcl", 0, 6, "hello", &analysis, None);
    assert!(
        edits.is_empty(),
        "collision with existing proc must be refused: {edits:?}"
    );
}

// ---------------------------------------------------------------------------
// rename — variables (scoping is the load-bearing correctness fact)
// ---------------------------------------------------------------------------

#[test]
fn rename_var_updates_definition_and_reads_with_dollar_preserved() {
    // tclsh: `set y 1; puts $y; puts $y` -> 1/1 — the renamed script runs.
    // The declaration becomes the bare name; each `$x` read keeps its `$`.
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    // Cursor in the first `$x` read.
    let edits = rename(src, "tcl", 1, 6, "y", &analysis, None);
    assert_eq!(
        edit_lines(&edits),
        vec![0, 1, 2],
        "decl + both reads: {edits:?}"
    );
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"y"),
        "declaration rewrite `y` missing: {texts:?}"
    );
    assert_eq!(
        texts.iter().filter(|t| **t == "$y").count(),
        2,
        "both `$x` reads should become `$y`: {texts:?}",
    );
}

#[test]
fn rename_var_from_definition_site_resolves_without_dollar() {
    // Cursor on the bare `x` in `set x 1` (no `$`). The definition-site
    // resolver must still find the variable and rewrite decl + reads.
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    // Cursor on `x` in `set x` (line 0, col 4).
    let edits = rename(src, "tcl", 0, 4, "y", &analysis, None);
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains(&"y"), "decl edit missing: {edits:?}");
    assert!(texts.contains(&"$y"), "ref edit missing: {edits:?}");
}

#[test]
fn rename_local_var_is_scoped_and_leaves_same_named_var_in_other_proc_intact() {
    // tclsh: `proc a {} {set v 10; return $v}` / `proc b {} {set v 20;
    // return $v}` -> a=10, b=20. Renaming proc a's `v`->`w` must rewrite ONLY
    // proc a's def+read (lines 1-2) and never proc b's `v` (lines 5-6) — the
    // two locals are independent, so clobbering b's `v` would be a real bug.
    //
    // C-Tcl proof of the *renamed* form: `proc a {} {set w 10; return $w}`
    // alongside the untouched `proc b {} {set v 20; return $v}` -> a=10, b=20
    // (verified: identical output to the original).
    let src = "proc a {} {\n    set v 10\n    return $v\n}\nproc b {} {\n    set v 20\n    return $v\n}\n";
    let analysis = analyse(src);
    // Cursor on the def site `v` in proc a (line 1, col 8).
    let edits = rename(src, "tcl", 1, 8, "w", &analysis, None);
    assert!(
        !edits.is_empty(),
        "expected a scoped rename of proc a's `v`"
    );
    let lines = edit_lines(&edits);
    assert!(
        lines.iter().all(|&l| l == 1 || l == 2),
        "rename must stay inside proc a (lines 1-2); got {edits:?}",
    );
    assert!(
        !lines.contains(&5) && !lines.contains(&6),
        "rename must NOT clobber proc b's same-named local `v`; got {edits:?}",
    );
    // The read site keeps its `$`.
    assert!(
        edits.iter().any(|e| e.new_text == "$w"),
        "the `$v` read should become `$w`: {edits:?}",
    );
}

#[test]
fn rename_var_rejected_on_same_scope_collision() {
    // Renaming `x`->`y` when a sibling `y` already lives in the same proc
    // scope would merge two distinct variables — refuse.
    let src = "proc demo {} {\n    set x 1\n    set y 2\n    puts $x\n}\n";
    let analysis = analyse(src);
    // Rename `x` (its read site on line 3) to the already-present `y`.
    let edits = rename(src, "tcl", 3, 10, "y", &analysis, None);
    assert!(
        edits.is_empty(),
        "same-scope collision must be refused: {edits:?}"
    );
}

// ---------------------------------------------------------------------------
// rename — namespaces (qualified call keeps its qualifier; isolation holds)
// ---------------------------------------------------------------------------

#[test]
fn rename_namespaced_proc_rewrites_qualified_and_short_forms() {
    // tclsh: renaming `::myns::greet`->`hello` keeps both the qualified call
    // (`::myns::hello`) and the short in-namespace call (`hello`) runnable
    // -> ns-hi/ns-hi. The qualified call must keep its `::myns::` qualifier;
    // the short call stays short.
    let src = "namespace eval ::myns {\n    proc greet {} { return ns-hi }\n}\n::myns::greet\nnamespace eval ::myns {\n    greet\n}\n";
    let analysis = analyse(src);
    // Cursor on the declaration (line 1).
    let edits = rename(src, "tcl", 1, 9, "hello", &analysis, None);
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"::myns::hello"),
        "qualified call must keep its namespace qualifier; got {texts:?}",
    );
    assert!(
        texts.contains(&"hello"),
        "short in-namespace call must stay short; got {texts:?}",
    );
}

#[test]
fn rename_namespaced_proc_from_its_own_decl_isolates_to_that_namespace() {
    // tclsh: `::a::helper` -> "a", `::b::helper` -> "b" are distinct procs.
    // Renaming `::a::helper`->`assist` from ::a's OWN declaration must rewrite
    // only ::a's decl + its own call (`::a::assist`), leaving ::b::helper
    // completely intact — otherwise we would clobber an unrelated proc. This
    // is the working case: ::a::helper is the first proc named `helper` in the
    // table, which is also the one the cursor sits on.
    //
    // C-Tcl proof of the renamed form: `::a::assist` -> "a" alongside the
    // untouched `::b::helper` -> "b" (the two are independent name bindings).
    let src = "namespace eval ::a {\n    proc helper {} { return a }\n}\nnamespace eval ::b {\n    proc helper {} { return b }\n}\n::a::helper\n::b::helper\n";
    let analysis = analyse(src);
    // Cursor on ::a's helper declaration (line 1).
    let edits = rename(src, "tcl", 1, 9, "assist", &analysis, None);
    assert_eq!(
        edit_lines(&edits),
        vec![1, 6],
        "exactly ::a's decl (line 1) + its call (line 6); got {edits:?}",
    );
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"::a::assist"),
        "the ::a::helper call must become ::a::assist; got {texts:?}",
    );
    // No edit may land on ::b::helper's decl (line 4) or call (line 7).
    assert!(
        edits
            .iter()
            .all(|e| e.range.start_line != 4 && e.range.start_line != 7),
        "::b::helper must be untouched; got {edits:?}",
    );
}

// FIXED: `rename` now disambiguates same-named procs across namespaces by the
// cursor's name span (not first-by-name), so renaming the SECOND same-named
// proc hits that proc and leaves the first one — an unrelated symbol Tcl keeps
// distinct — untouched. tclsh: `::a::helper`->"a", `::b::helper`->"b" are
// independent. `rename_proc` now prefers the proc whose `name_span` covers the
// cursor (matching `references::references`), falling back to first-by-name
// only for call-site / unqualified cursors.
#[test]
fn rename_second_same_named_proc_resolves_to_that_proc_not_the_first() {
    let src = "namespace eval ::a {\n    proc helper {} { return a }\n}\nnamespace eval ::b {\n    proc helper {} { return b }\n}\n::a::helper\n::b::helper\n";
    let analysis = analyse(src);
    // Cursor on ::b's helper declaration (line 4) — should rename ::b::helper.
    let edits = rename(src, "tcl", 4, 9, "assist", &analysis, None);
    assert_eq!(
        edit_lines(&edits),
        vec![4, 7],
        "cursor on ::b::helper must rename ::b's decl (line 4) + its call (line 7); got {edits:?}",
    );
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"::b::assist"),
        "expected ::b::assist; got {texts:?}"
    );
    // ::a::helper (lines 1+6) must be untouched.
    assert!(
        edits
            .iter()
            .all(|e| e.range.start_line != 1 && e.range.start_line != 6),
        "::a::helper must be untouched; got {edits:?}",
    );
}

// ---------------------------------------------------------------------------
// rename — safety gating (shape + builtin shadow)
// ---------------------------------------------------------------------------

#[test]
fn rename_rejects_syntactically_unsafe_new_names() {
    // A new name that isn't a valid Tcl identifier is refused (empty edits)
    // so the editor never applies a partially-broken rename.
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    for bad in [
        "bad name",
        "1lead",
        "with-dash",
        "has::colon",
        "with$dollar",
        "",
    ] {
        assert!(
            rename(src, "tcl", 0, 6, bad, &analysis, None).is_empty(),
            "unsafe new name {bad:?} must yield no edits",
        );
    }
}

#[test]
fn rename_var_also_rejects_unsafe_new_name() {
    // The shape gate applies to variable renames too.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    assert!(rename(src, "tcl", 1, 6, "bad name", &analysis, None).is_empty());
}

#[test]
fn rename_proc_to_builtin_command_name_is_blocked_with_registry() {
    // tclsh: `puts` is a built-in command. Renaming a proc to `puts` would
    // make the editor's calls dispatch to the built-in instead of the proc,
    // so with a registry supplied the rename is refused.
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(src, "tcl", 0, 6, "puts", &analysis, Some(&registry));
    assert!(
        edits.is_empty(),
        "rename to built-in `puts` must be blocked: {edits:?}"
    );
}

#[test]
fn rename_proc_to_non_builtin_succeeds_with_registry() {
    // The registry gate only blocks built-in names; a free name still renames.
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(src, "tcl", 0, 6, "salut", &analysis, Some(&registry));
    assert!(
        !edits.is_empty(),
        "non-built-in rename should succeed with a registry"
    );
    assert!(edits.iter().all(|e| e.new_text == "salut"), "{edits:?}");
}

#[test]
fn rename_var_to_builtin_name_is_allowed() {
    // The built-in-shadow gate is proc-only — variable names occupy a
    // separate Tcl namespace, so renaming a var to `puts` is fine.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(src, "tcl", 1, 6, "puts", &analysis, Some(&registry));
    assert!(
        !edits.is_empty(),
        "variable rename to `puts` should succeed"
    );
}

// ---------------------------------------------------------------------------
// rename — non-symbols / robustness
// ---------------------------------------------------------------------------

#[test]
fn rename_unknown_word_yields_no_edits() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    // `hello` is a bare literal — nothing to rename.
    assert!(rename(src, "tcl", 0, 6, "x", &analysis, None).is_empty());
}

#[test]
fn rename_builtin_without_registry_still_yields_no_edits() {
    // `puts` has no user proc def, so even without a registry there is no
    // symbol whose definition/calls could be rewritten.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(rename(src, "tcl", 0, 1, "show", &analysis, None).is_empty());
}

#[test]
fn rename_out_of_range_and_empty_file_do_not_panic() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    assert!(rename(src, "tcl", 99, 0, "z", &analysis, None).is_empty());
    assert!(rename(src, "tcl", 0, 999, "z", &analysis, None).is_empty());
    let empty = analyse("");
    assert!(rename("", "tcl", 0, 0, "z", &empty, None).is_empty());
}

// ---------------------------------------------------------------------------
// prepare_rename — gates the rename UI
// ---------------------------------------------------------------------------

#[test]
fn prepare_rename_offers_proc_name_and_placeholder() {
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 0, 6, &analysis).expect("proc name is renameable");
    assert_eq!(p.placeholder, "greet");
    // Anchored at the proc name span (`proc greet` -> name starts col 5).
    assert_eq!(p.range.start_line, 0);
    assert_eq!(p.range.start_character, 5);
}

#[test]
fn prepare_rename_offers_variable_name_and_placeholder() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 1, 6, &analysis).expect("variable is renameable");
    assert_eq!(p.placeholder, "x");
}

#[test]
fn prepare_rename_rejects_builtin_command() {
    // `puts` is a built-in, not a renameable user symbol.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(
        prepare_rename(src, 0, 1, &analysis).is_none(),
        "built-in `puts` is not renameable",
    );
}

#[test]
fn prepare_rename_rejects_whitespace_and_bare_literal() {
    // The space between `set` and `x`, and the bare literal `hello`, are not
    // renameable symbols.
    let src = "set x 1\nputs hello\n";
    let analysis = analyse(src);
    // Column 3 on line 0 is the space in `set x`.
    assert!(
        prepare_rename(src, 0, 3, &analysis).is_none(),
        "whitespace is not renameable"
    );
    // The literal `hello` argument on line 1.
    assert!(
        prepare_rename(src, 1, 6, &analysis).is_none(),
        "bare literal is not renameable"
    );
}

#[test]
fn prepare_rename_out_of_range_and_empty_file_return_none() {
    let src = "set x 1\n";
    let analysis = analyse(src);
    assert!(prepare_rename(src, 99, 0, &analysis).is_none());
    assert!(prepare_rename(src, 0, 999, &analysis).is_none());
    let empty = analyse("");
    assert!(prepare_rename("", 0, 0, &empty).is_none());
}

// ---------------------------------------------------------------------------
// is_safe_symbol_name — the shape predicate behind the gate
// ---------------------------------------------------------------------------

#[test]
fn is_safe_symbol_name_accepts_identifiers_and_rejects_the_rest() {
    for ok in ["foo", "Foo", "_under", "a1", "snake_case_42"] {
        assert!(is_safe_symbol_name(ok), "{ok:?} should be accepted");
    }
    for bad in [
        "",
        "1lead",
        "has space",
        "has-dash",
        "has::colon",
        "with$dollar",
        "dotted.name",
    ] {
        assert!(!is_safe_symbol_name(bad), "{bad:?} should be rejected");
    }
}
