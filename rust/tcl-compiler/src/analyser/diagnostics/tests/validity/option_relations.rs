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

//! W146, W147 and W152: literal-argument validation and the relations between a call's options.

use super::*;

#[test]
fn w146_validates_modern_trace_operation_lists_by_registry_declared_type() {
    for (invalid, valid) in [
        (
            "trace add variable item {read rename write} callback\n",
            "trace add variable item {array read unset write} callback\n",
        ),
        (
            "trace remove command worker {delete enter rename} callback\n",
            "trace remove command worker {delete rename} callback\n",
        ),
        (
            "trace add execution worker {enter write leavestep} callback\n",
            "trace add execution worker {enter leave enterstep leavestep} callback\n",
        ),
    ] {
        assert!(
            codes_for_dialect(invalid, "tcl9.0").contains(&"W146".to_owned()),
            "invalid type-specific operation must fire: {invalid}"
        );
        assert!(
            !codes_for_dialect(valid, "tcl9.0").contains(&"W146".to_owned()),
            "the complete Tcl 9 operation set must stay silent: {valid}"
        );
    }
}

#[test]
fn w146_mixed_operation_list_fix_preserves_valid_members_and_requires_review() {
    let source = "trace add variable item {read bogus write} callback\n";
    let result = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W146)
        .expect("mixed valid/invalid list fires W146");
    assert!(diagnostic.message.contains("'bogus'"));
    assert!(diagnostic.message.contains("array, read, unset, write"));
    assert_eq!(diagnostic.fixes.len(), 1);
    assert_eq!(diagnostic.fixes[0].new_text, "{read write}");
    assert_eq!(
        diagnostic.fixes[0].safety,
        crate::irules_checks::FixSafety::RequiresReview
    );
    assert_eq!(
        &source[diagnostic.fixes[0].span.as_range()],
        "{read bogus write}",
        "the edit replaces the whole written Tcl word, including delimiters"
    );
}

#[test]
fn w146_abstains_when_a_modern_operation_list_is_not_safely_fixable() {
    for source in [
        "trace add variable item $operations callback\n",
        "trace add variable item [operations] callback\n",
        "trace add variable item {*}$operations callback\n",
        "trace add variable item {read {unterminated} callback\n",
        "trace add not-a-type item {read bogus} callback\n",
        "trace add variable item\n",
    ] {
        assert!(
            !codes_for_dialect(source, "tcl9.0").contains(&"W146".to_owned()),
            "dynamic, malformed, incomplete, or invalid-discriminator input must abstain: {source:?}"
        );
    }

    for source in [
        "trace add command worker {} callback\n",
        "trace add command worker {enter write} callback\n",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W146)
            .expect("a complete empty/all-invalid list is still diagnosed");
        assert_eq!(
            diagnostic.fixes.len(),
            0,
            "removing every member would leave Tcl's invalid empty operation list"
        );
    }
}

#[test]
fn w146_honours_trace_prefix_and_legacy_version_rules() {
    assert!(
        !codes_for_dialect("trace add var item {read write} callback\n", "tcl9.0")
            .contains(&"W146".to_owned()),
        "the type discriminator accepts Tcl's unique-prefix rule"
    );
    assert!(
        codes_for_dialect("trace add variable item w callback\n", "tcl9.0")
            .contains(&"W146".to_owned()),
        "operation members use exact matching and may not be abbreviated"
    );
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        assert!(
            codes_for_dialect("trace variable item rwx callback\n", dialect)
                .contains(&"W146".to_owned()),
            "the deprecated rwua form is validated on {dialect}"
        );
    }
    assert!(
        !codes_for_dialect("trace variable item rwx callback\n", "tcl9.0")
            .contains(&"W146".to_owned()),
        "Tcl 9 removed the legacy form, so subcommand availability owns it"
    );
}

#[test]
fn w146_drops_registry_finding_when_a_user_command_shadows_trace() {
    let source =
        "proc trace args { return user-command }\ntrace add variable item {read bogus} callback\n";
    assert!(
        !codes_for_dialect(source, "tcl9.0").contains(&"W146".to_owned()),
        "post-walk command resolution must suppress registry argument facts for a shadowed builtin"
    );
}

#[test]
fn w147_reports_registry_declared_mutually_exclusive_options() {
    let mut a = Analyser::new();
    let result = a.analyse("source -encoding utf-8 -nopkg file.tcl\n", "tcl9.0");
    let conflicts: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W147)
        .collect();
    assert_eq!(
        conflicts.len(),
        1,
        "expected one W147: {:?}",
        result.diagnostics
    );
    assert!(conflicts[0].message.contains("-encoding, -nopkg"));
    assert_eq!(conflicts[0].fixes.len(), 0, "intent is ambiguous");
    let mut legacy = Analyser::new();
    let legacy_result = legacy.analyse("source -encoding utf-8 -nopkg file.tcl\n", "tcl8.6");
    assert!(
        !legacy_result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W147),
        "Tcl 9-only relationship must not leak into Tcl 8.6"
    );
}

/// `subst`'s two switch families cannot be combined (tclsh 9.1b0:
/// `cannot combine positive and negative options`): the registry declares the
/// exclusion as option relations, so the existing relation check reports it
/// as W147 at the call site — no `subst` logic in the analyser.
#[test]
fn w147_reports_mixed_subst_switch_families() {
    let w147 = |src: &str, dialect: &str| -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W147)
            .map(|d| d.message.clone())
            .collect()
    };
    let mixed = w147("subst -nocommands -variables $x\n", "tcl9.1");
    assert_eq!(mixed.len(), 1, "{mixed:?}");
    assert!(
        mixed[0].contains("cannot combine positive and negative options"),
        "{mixed:?}"
    );
    // One family alone is legal, and below 9.1 the positive switches are not
    // options at all, so there is no family to mix.
    assert_eq!(
        w147("subst -nocommands -novariables $x\n", "tcl9.1"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        w147("subst -variables -commands $x\n", "tcl9.1"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        w147("subst -nocommands -variables $x\n", "tcl9.0"),
        [] as [std::string::String; 0]
    );
}

/// `regexp -inline` returns the match data and rejects a match variable
/// (tclsh 8.4–9.1: `regexp match variables not allowed when using
/// -inline`): the trailing word is the relation's finding, never a write.
#[test]
fn w147_reports_a_match_variable_after_regexp_inline() {
    let w147 = |src: &str| -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W147)
            .map(|d| d.message.clone())
            .collect()
    };
    let found = w147("regexp -inline {a(b)} ab v\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("regexp match variables not allowed when using -inline"),
        "{found:?}"
    );
    assert_eq!(
        w147("set t {a b}\nregexp -all -inline {\\S+} $t\n"),
        [] as [std::string::String; 0]
    );
    assert_eq!(w147("regexp {a(b)} ab v\n"), [] as [std::string::String; 0]);
}

#[test]
fn w147_is_generic_and_covers_glob_without_source_logic() {
    let mut a = Analyser::new();
    let result = a.analyse("glob -directory root -path prefix *.tcl\n", "tcl8.6");
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W147)
            .count(),
        1,
        "expected generic option conflict: {:?}",
        result.diagnostics
    );
}

/// **E-R14 real case 1** — `bibtex::parse`'s `-command` requires `-channel`
/// (`bibtex.tcl:213-241`), the census gap G2 directional half.
#[test]
fn w152_reports_a_directional_option_requirement() {
    let messages = |src: &str| -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W152)
            .map(|d| d.message.clone())
            .collect()
    };
    // `-command` with no `-channel`: the library raises "Option -command and
    // text exclude each other" at run time.
    let unmet =
        messages("package require bibtex\n::bibtex::parse -command handle -recordcommand rec\n");
    assert!(
        unmet
            .iter()
            .any(|m| m.contains("-command") && m.contains("-channel")),
        "{unmet:?}"
    );
    // With `-channel` present the relation is satisfied and nothing is said.
    let met = messages("package require bibtex\n::bibtex::parse -channel $chan -command handle\n");
    assert!(
        !met.iter().any(|m| m.contains("-channel")),
        "a satisfied requirement must be silent: {met:?}"
    );
}

/// A declared option relation is checked over a proven option word as over
/// a written one, for a core command and a pack's command alike (#2453).
#[test]
fn option_relations_read_a_proven_option_word() {
    let codes = |src: &str, dialect: &str| -> Vec<(DiagCode, String)> {
        let mut a = Analyser::new();
        let mut found: Vec<_> = a
            .analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code, DiagCode::W147 | DiagCode::W152))
            .map(|d| (d.code, d.message.clone()))
            .collect();
        found.sort();
        found
    };
    for (written, proven, dialect) in [
        (
            "glob -directory root -path prefix *.tcl\n",
            "set o -path\nglob -directory root $o prefix *.tcl\n",
            "tcl9.0",
        ),
        (
            "package require bibtex\n::bibtex::parse -command handle -recordcommand rec\n",
            "package require bibtex\nset o [string range -recordcommandxx 0 13]\n\
             ::bibtex::parse -command handle $o rec\n",
            "tcl8.6",
        ),
    ] {
        let want = codes(written, dialect);
        assert!(!want.is_empty(), "{written}");
        let got = codes(proven, dialect);
        assert!(
            want.iter()
                .filter(|(code, _)| *code == DiagCode::W147)
                .all(|w| got.contains(w)),
            "{proven}: {got:?} vs {want:?}"
        );
    }
    let unknown = codes(
        "proc p {o} {\n glob -directory root $o prefix *.tcl\n}\n",
        "tcl9.0",
    );
    assert_eq!(unknown, Vec::new());
}

/// **E-R14 real case 2** — `http::geturl`'s conditional pairs
/// (`http.n`: `-queryprogress` is made "in a POST request, i.e. a call with
/// `-query` or `-querychannel`").
///
/// It also pins the `OptionPlacement::Anywhere` half: `geturl` takes the URL
/// positionally *first* and only then loops over flag/value pairs, so a
/// leading-run walk would find no options at all here.
#[test]
fn w152_reports_a_requires_one_of_after_a_positional_word() {
    let codes = |src: &str| -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W152)
            .map(|d| d.message.clone())
            .collect()
    };
    let lone =
        codes("package require http\n::http::geturl http://example.invalid/ -queryprogress cb\n");
    assert!(
        lone.iter()
            .any(|m| m.contains("-queryprogress") && m.contains("-query")),
        "{lone:?}"
    );
    let paired = codes(
        "package require http\n\
         ::http::geturl http://example.invalid/ -query a=1 -queryprogress cb\n",
    );
    assert_eq!(
        paired.len(),
        0,
        "the pair satisfies the relation: {paired:?}"
    );
}

/// **E-R14 real case 3** — `struct::tree walk`'s `-order in` is illegal with
/// `-type bfs` (`tree_tcl.tcl`'s `WalkOptions`).
///
/// A relation between one option's **value** and another option's value, on
/// an object-instance method — neither of which the retired `OptionConstraint`
/// could reach.
#[test]
fn w147_reports_a_cross_option_value_relation_on_an_instance_method() {
    let messages = |src: &str| -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W147)
            .map(|d| d.message.clone())
            .collect()
    };
    let illegal = messages(
        "package require struct::tree\n\
         set t [::struct::tree]\n\
         $t walk root -order in -type bfs v script\n",
    );
    assert!(
        illegal
            .iter()
            .any(|m| m.contains("in-order breadth first walk")),
        "{illegal:?}"
    );
    // Every other order/type pairing the library accepts stays silent — the
    // FP control, because a relation that fired on `-order pre` would make
    // the whole walk vocabulary unusable.
    for legal in [
        "$t walk root -order pre -type bfs v script\n",
        "$t walk root -order in -type dfs v script\n",
        "$t walk root -order post v script\n",
    ] {
        let src = format!("package require struct::tree\nset t [::struct::tree]\n{legal}");
        assert_eq!(messages(&src).len(), 0, "{legal:?}: {:?}", messages(&src));
    }
}

/// **The P-B measurement** (redesign §0.05): an option-heavy document is
/// checked entirely in Rust.
///
/// The corpus below is 40 option-bearing call sites across the commands whose
/// option tables are the densest in the tree — `glob`, `source`, `lsort`,
/// `string match`, `regsub`, `switch`, `http::geturl`, `bibtex::parse`, and a
/// `struct::tree` walk. The assertion is not "hooks are fast": it is that the
/// count of call sites reaching a hook is **zero**, which is what principle
/// P-B demands and what a regression would break loudly.
#[test]
fn an_option_heavy_document_is_checked_with_no_vm_entry() {
    const CORPUS: &str = "\
package require http\n\
package require bibtex\n\
package require struct::tree\n\
set t [::struct::tree]\n\
proc probe {chan pattern items t} {\n\
    glob -directory /tmp -nocomplain -types f *.tcl\n\
    glob -path /tmp/pre -nocomplain *.tcl\n\
    glob -directory /tmp -path /tmp/pre *.tcl\n\
    source -encoding utf-8 file.tcl\n\
    source -encoding utf-8 -nopkg file.tcl\n\
    lsort -integer -decreasing $items\n\
    lsort -dictionary -unique $items\n\
    lsort -command cmp -index 0 $items\n\
    string match -nocase $pattern abc\n\
    regsub -all -nocase -- $pattern abc def out\n\
    regsub -start 0 -line -- $pattern abc def out\n\
    switch -exact -- $pattern { a { } default { } }\n\
    switch -glob -nocase -- $pattern { a { } default { } }\n\
    switch -regexp -- $pattern { a { } default { } }\n\
    ::http::geturl http://example.invalid/ -query a=1 -queryprogress cb\n\
    ::http::geturl http://example.invalid/ -querychannel $chan -queryblocksize 4096\n\
    ::http::geturl http://example.invalid/ -timeout 5000 -headers {}\n\
    ::http::geturl http://example.invalid/ -query a=1 -querychannel $chan\n\
    ::bibtex::parse -channel $chan -recordcommand rec\n\
    ::bibtex::parse -channel $chan -preamblecommand pre -stringcommand str\n\
    ::bibtex::parse -command handle -channel $chan\n\
    ::bibtex::parse -command handle -recordcommand rec -channel $chan\n\
    $t walk root -order pre -type dfs v script\n\
    $t walk root -order in -type bfs v script\n\
    $t walkproc root -order post -type dfs cmdprefix\n\
    file link -symbolic a b\n\
    file link -hard a b\n\
    string map -nocase {a b} $pattern\n\
    lsearch -exact -sorted $items a\n\
    lsearch -glob -inline -all $items a\n\
    lsearch -regexp -not $items a\n\
    array get arr *\n\
    fconfigure $chan -blocking 0 -buffering line\n\
    open /tmp/x r\n\
    exec -ignorestderr -- ls\n\
    binary scan abc cH2 code ident\n\
    clock format 0 -format {%Y} -gmt 1\n\
    clock scan {2026} -format {%Y} -timezone :UTC\n\
    interp create -safe child\n\
    zlib compress abc 6\n\
}\n\
";

    tcl_registry::spec::reset_relation_check_stats();
    let before = tcl_registry::pack_hooks::cache_stats();
    let cold = std::time::Instant::now();
    let mut analyser = Analyser::new();
    let result = analyser.analyse(CORPUS, "tcl8.6");
    let cold = cold.elapsed();
    let stats = tcl_registry::spec::relation_check_stats();
    let after = tcl_registry::pack_hooks::cache_stats();

    // A second pass over the same document, to show what a re-analysis after
    // an unrelated edit costs when nothing a relation reads changed.
    let warm = std::time::Instant::now();
    let mut again = Analyser::new();
    let _ = again.analyse(CORPUS, "tcl8.6");
    let warm = warm.elapsed();

    // THE NUMBER. Every relation on this corpus is declarative, so the count
    // of call sites that entered the hook seam — and therefore the tclvm — is
    // zero, and no hook dispatch happened at all.
    assert_eq!(
        stats.hook_entries, 0,
        "an option-heavy document must not enter the hook VM: {stats:?}"
    );
    assert_eq!(
        (after.hits - before.hits, after.misses - before.misses),
        (0, 0),
        "no pack-hook dispatch may happen while checking option relations"
    );
    assert!(
        stats.option_bearing_sites >= 30,
        "the corpus must actually exercise the checker: {stats:?}"
    );
    assert!(stats.relations_evaluated >= stats.judged_sites, "{stats:?}");
    // The three real cases are all found in one pass over the corpus.
    let reported = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code, DiagCode::W147 | DiagCode::W152))
        .count();
    assert!(reported >= 3, "{:?}", result.diagnostics);

    println!(
        "E-R14 measurement: {} option-bearing call sites walked, \
         {} of them judged against declared relations, \
         {} relations evaluated natively, {} entered tclvm; \
         cold analysis {cold:?}, second pass {warm:?}",
        stats.option_bearing_sites,
        stats.judged_sites,
        stats.relations_evaluated,
        stats.hook_entries,
    );
}

#[test]
fn w147_requires_two_distinct_canonical_options() {
    for snippet in [
        "glob -directory root -directory other *.tcl\n",
        "glob -path root -path other *.tcl\n",
    ] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::W147),
            "repeating one constrained option is not a relationship conflict: {:?}",
            result.diagnostics
        );
    }
}

fn w146_messages(source: &str, dialect: &str) -> Vec<String> {
    crate::analyser::Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W146)
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn a_leading_tilde_in_a_file_name_reports_w146_from_tcl_9_0() {
    // Item 192, `tcl9-changes/t03`: tclsh 8.6.18 reads `file pathtype ~/x` as
    // absolute and 9.0.4 as relative; `file join a ~b` is `~b` and `a/~b`.
    for source in [
        "file pathtype ~/x\n",
        "file join a ~b\n",
        "file exi ~/x\n",
        "file copy -force ~/a ~/b\n",
    ] {
        assert!(w146_messages(source, "tcl8.6").is_empty(), "{source}");
        assert!(!w146_messages(source, "tcl9.0").is_empty(), "{source}");
    }
    for source in [
        "file split ~/x\n",
        "file tildeexpand ~/x\n",
        "file join a b\n",
        "file pathtype $x\n",
    ] {
        assert!(w146_messages(source, "tcl9.0").is_empty(), "{source}");
    }
    assert_eq!(w146_messages("file join a ~b ~c\n", "tcl9.1").len(), 1);
}

#[test]
fn a_removed_encoding_name_reports_w146_from_tcl_9_0() {
    // Item 192, `tcl9-changes/t08`: `binary` and `identity` raise `unknown
    // encoding` in tclsh 9.0.4 and 9.1.0; 8.6.18 reads `identity` and the
    // channel `-encoding binary`.
    for source in [
        "encoding convertto binary abc\n",
        "encoding convertfrom identity abc\n",
        "encoding convertfrom -profile strict binary abc\n",
        "encoding system binary\n",
        "fconfigure stdout -encoding binary\n",
        "chan configure stdout -encoding identity\n",
    ] {
        assert_eq!(w146_messages(source, "tcl9.0").len(), 1, "{source}");
        assert!(w146_messages(source, "tcl8.6").is_empty(), "{source}");
    }
    for source in [
        "encoding convertto utf-8 binary\n",
        "encoding convertfrom binary\n",
        "encoding system\n",
        "fconfigure stdout -encoding utf-8\n",
        "fconfigure stdout -translation binary\n",
    ] {
        assert!(w146_messages(source, "tcl9.0").is_empty(), "{source}");
    }
}

#[test]
fn an_lseq_operand_that_is_an_expression_reports_w146() {
    // Item 192, `tcl9-changes/t14` and `t32`: tclsh 9.0.4 and 9.1.0 raise
    // `expected number but got "1+2"`.
    for source in ["lseq 0 {1+2}\n", "lseq 1 {2+1}\n", "lseq 1 to foo by 2\n"] {
        for dialect in ["tcl9.0", "tcl9.1"] {
            assert_eq!(w146_messages(source, dialect).len(), 1, "{source}");
        }
    }
    for source in [
        "lseq 4\n",
        "lseq 1 to 10 by 3\n",
        "lseq 5 count 3\n",
        "lseq 0 0.5 by 0.1\n",
        "lseq 1 .. $n\n",
    ] {
        assert!(w146_messages(source, "tcl9.0").is_empty(), "{source}");
    }
}

#[test]
fn a_write_to_a_constant_reports_e007_on_the_selected_cell() {
    // Both pinned Tcl 9 handlers reject incrementing a constant before
    // converting the amount, even when the literal amount is invalid.
    for command in [
        "set PI 4",
        "incr PI",
        "incr PI 2",
        "incr PI invalid",
        "append PI x",
        "lappend PI y",
    ] {
        let source = format!("const PI 3\n{command}\n");
        for profile in ["tcl9.0", "tcl9.1"] {
            let result = crate::analyser::Analyser::new().analyse(&source, profile);
            let reports = result
                .diagnostics
                .iter()
                .filter(|diagnostic| matches!(diagnostic.code, DiagCode::E007 | DiagCode::W146))
                .collect::<Vec<_>>();
            let [diagnostic] = reports.as_slice() else {
                panic!("expected one constant-write report for {command}: {reports:?}");
            };
            assert_eq!(diagnostic.code, DiagCode::E007, "{command}");
            assert_eq!(&source[diagnostic.span.as_range()], "PI", "{command}");
        }
    }
    for source in [
        "const PI 3; const PI 3; puts $PI",
        "const PI 3; proc f {} {set PI 4}; set Q 1; set Q 2",
        "set PI 3; catch {const PI 3}; set PI 4",
        "const PI 3; append PI",
        "const PI 3; lappend PI",
        "const PI \\{; lappend PI y",
        "const PI 3; proc inspect args {}; trace add variable PI read inspect; lappend PI y",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| { !matches!(diagnostic.code, DiagCode::E007 | DiagCode::W146) }),
            "{source}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn caught_constant_store_after_stdout_reports_at_the_written_target() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/work/name-handling/handover/oo-dispatch-probe/tcl9-changes/t13-const.tcl"
    ));
    for profile in ["tcl9.0", "tcl9.1"] {
        let result = crate::analyser::Analyser::new().analyse(source, profile);
        let reports: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|diagnostic| matches!(diagnostic.code, DiagCode::E007 | DiagCode::W146))
            .collect();
        let [diagnostic] = reports.as_slice() else {
            panic!("{profile}: expected one constant-write report: {reports:?}");
        };
        assert_eq!(diagnostic.code, DiagCode::E007, "{profile}");
        assert_eq!(
            usize::try_from(diagnostic.span.start()).unwrap(),
            source.find("set PI 4").unwrap() + "set ".len(),
            "{profile}"
        );
        assert_eq!(&source[diagnostic.span.as_range()], "PI", "{profile}");
    }
}

#[test]
fn the_variables_the_new_list_and_encoding_commands_write_are_defined() {
    // Item 192: `encoding -failindex` and `ledit` write their variable, so a
    // later read is not a read before set; `lremove` and `lseq` write none.
    for source in [
        "encoding convertfrom -failindex fi utf-8 abc\nputs $fi\n",
        "encoding convertto -failindex fi utf-8 abc\nputs $fi\n",
    ] {
        assert!(
            !codes_for_dialect(source, "tcl9.0").contains(&"W210".to_owned()),
            "{source}"
        );
    }
    for source in ["lremove $l 0\nputs $l\n", "set s [lseq 3]\nputs $l\n"] {
        assert!(
            codes_for_dialect(source, "tcl9.0").contains(&"W210".to_owned()),
            "{source}"
        );
    }
}
