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

use super::*;
use tcl_compiler::analyser::Analyser;

/// Run the transform over the byte range of `needle` in `src`.
fn at(src: &str, needle: &str) -> Option<Refactoring> {
    at_dialect(src, needle, "tcl9.0")
}

/// [`at`] against a document analysed under a named release — the
/// `${…}` close rule is release-dependent.
fn at_dialect(src: &str, needle: &str, dialect: &str) -> Option<Refactoring> {
    let registry = super::super::test_registry();
    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(src, dialect).clone();
    let start = u32::try_from(src.find(needle).expect("needle in source")).unwrap();
    let end = start + u32::try_from(needle.len()).unwrap();
    extract_proc(src, (start, end), &analysis, &registry)
}

/// The rewritten document, or the refusal reason.
fn outcome(src: &str, needle: &str) -> Result<String, String> {
    let refactoring = at(src, needle).expect("a selection to extract");
    match &refactoring.disabled {
        Some(reason) => Err(reason.clone()),
        None => Ok(refactoring.apply(src)),
    }
}

#[test]
fn logical_written_roles_keep_literal_sigil_names_and_exact_element_shape() {
    for (source, names) in [
        ("set {$literal} 1", vec!["$literal"]),
        ("set {[literal]} 1", vec!["[literal]"]),
        ("set a(b 1", vec!["a(b"]),
        ("set {a)b} 1", vec!["a)b"]),
        (
            "foreach {{$literal} {[literal]} a(b} {} {}",
            vec!["$literal", "[literal]", "a(b"],
        ),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let walk = super::super::FrameWalk::new(source, &analysis).unwrap();
        let commands = walk.segment_document(source);
        let [command] = commands.as_slice() else {
            panic!("one original command");
        };
        let mut roles = VariableRoles {
            read: BTreeMap::new(),
            written: BTreeMap::new(),
        };
        classify_command(source, command, &walk, &mut roles).unwrap();
        assert_eq!(
            roles.written.keys().map(String::as_str).collect::<Vec<_>>(),
            names,
            "{source}"
        );
    }
    for source in ["set $unknown 1", "set {a(key)} 1"] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let walk = super::super::FrameWalk::new(source, &analysis).unwrap();
        let commands = walk.segment_document(source);
        let mut roles = VariableRoles {
            read: BTreeMap::new(),
            written: BTreeMap::new(),
        };
        assert!(classify_command(source, &commands[0], &walk, &mut roles).is_err());
    }
}

#[test]
fn extraction_original_components_keep_literal_sigil_data_and_refuse_array_captures() {
    let logical_outcome = |source: &str, needle: &str| -> Result<String, String> {
        let action = at_dialect(source, needle, "tcl").unwrap();
        action
            .disabled
            .clone()
            .map_or_else(|| Ok(action.apply(source)), Err)
    };
    let source = "set value TEXT\nputs \"escaped=\\$notavar $value\"\nputs done\n";
    // tclsh 8.4.20–9.1.0 print `escaped=$notavar TEXT` then `done` for
    // this source and for its rewrite, so a release extracts it as the
    // unversioned profile does.
    for result in [
        outcome(source, "puts \"escaped=\\$notavar $value\"").unwrap(),
        logical_outcome(source, "puts \"escaped=\\$notavar $value\"").unwrap(),
    ] {
        assert!(result.contains("proc extracted_proc {value}"), "{result}");
        assert!(!result.contains("{notavar value}"), "{result}");
        assert!(result.contains("escaped=\\$notavar $value"), "{result}");
    }
    let source = "set {$literal} TEXT\nputs ${$literal}\nputs done\n";
    let result = logical_outcome(source, "puts ${$literal}").unwrap();
    assert!(result.contains("${$literal}"), "{result}");
    let source = "set array(key) TEXT\nputs $array(key)\nputs done\n";
    assert!(
        logical_outcome(source, "puts $array(key)")
            .unwrap_err()
            .contains("array element read")
    );
}

/// Implementation contract: naming.refactor.original-extraction-motion-boundary
/// docs/design/analysis/name-resolution-proofs/refactor-original-extraction-motion-boundary.md
///
/// A release refuses the extraction where the interpreter observes the
/// motion. tclsh 8.4.20–9.1.0, before and after the rewrite each case
/// would produce: `puts [info level]` prints 0 then 1; a write trace whose
/// handler prints `[info level]` prints 1 then 2; `eval $script` with
/// `set made 1` leaves `made` in the caller before and in the new
/// procedure's frame after.
#[test]
fn a_release_refuses_an_extraction_the_interpreter_can_observe() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for (source, needle, reason) in [
            (
                "puts [info level]\nputs done\n",
                "puts [info level]",
                "call frame",
            ),
            (
                "proc h args {puts [info level]}\nset v 0\ntrace add variable v write h\nset v 1\nputs done\n",
                "set v 1",
                "trace",
            ),
            (
                "set script {set made 1}\neval $script\nputs [info exists made]\n",
                "eval $script",
                "call frame",
            ),
        ] {
            // A selection the walk cannot offer at all is refused too.
            let Some(action) = at_dialect(source, needle, release) else {
                continue;
            };
            let refusal = action.disabled.as_deref().unwrap_or_else(|| {
                panic!(
                    "{release}: {needle} must be refused: {}",
                    action.apply(source)
                )
            });
            assert!(refusal.contains(reason), "{release}: {needle}: {refusal}");
        }
    }
}

// TP: a pure selection with live-ins and no live-outs.

#[test]
fn tp_extracts_a_read_only_selection_as_value_parameters() {
    let src = "set x 5\nputs $x\nputs done\n";
    let result = outcome(src, "puts $x").unwrap();
    assert!(
        result.contains("proc extracted_proc {x} {\n    puts $x\n}"),
        "{result}"
    );
    assert!(result.contains("extracted_proc $x"), "{result}");
}

#[test]
fn generated_required_formals_preserve_the_actual_local_names() {
    use tcl_dialect::ParameterGrammar::{Jim, Tcl};
    let source = "set {two words} hello\nputs ${two words}\n";
    // Plain Tcl analysis has to preserve either selected engine; otherwise it refuses.
    let logical = at_dialect(source, "puts ${two words}", "tcl");
    assert!(logical.is_none_or(|action| action.disabled.is_some()));
    let tcl = tcl_lexer::LexerConfig::for_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
    assert_eq!(
        required_formal_word(&["two words".into()], tcl, Some(Tcl)).as_deref(),
        Some("{{{two words}}}")
    );
    assert!(required_formal_word(&["two words".into()], tcl, Some(Jim)).is_none());
    assert!(required_formal_word(&["args".into()], tcl, Some(Tcl)).is_none());
}

/// The captured-variable set decides the generated proc's
/// parameters and its call-site arguments, so a brace-bearing `${…}`
/// name must be read by the **document's** release rule, not always the
/// 8.x first-`}` one.
///
/// Oracle: with `a{b}c` set to `NINE` and `a{b` set to `EIGHT`,
/// `puts ${a{b}c}` prints `NINE` on tclsh 9.0.4 (one variable named
/// `a{b}c`) and `EIGHTc}` on 8.6.16 (the name ends at the first `}` and
/// `c}` is ordinary word text).
///
/// The configured shared component projection supplies the same names
/// independently of extraction's frame and insertion prerequisites.
#[test]
fn variable_references_read_braced_names_by_the_documents_release() {
    let text = "puts ${a{b}c}";
    // 9.x: brace depth, so the whole `a{b}c` is the name.
    let nine = variable_references(text, BracedVarStyle::Tcl9Nesting);
    assert_eq!(
        nine.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["a{b}c"]
    );
    // 8.x: the name ends at the first `}`.
    let eight = variable_references(text, BracedVarStyle::FirstClose);
    assert_eq!(
        eight.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["a{b"]
    );
}

/// An unterminated `${` names nothing — inventing a name for it would
/// put that name into the generated proc's parameter list.
#[test]
fn variable_references_reject_an_unterminated_braced_name() {
    assert!(variable_references("puts ${abc", BracedVarStyle::Tcl9Nesting).is_empty());
    assert!(variable_references("puts ${abc", BracedVarStyle::FirstClose).is_empty());
}

/// The **plumb**: the style must come from the document's own dialect,
/// not a constant. The reference sits inside a braced body so the whole
/// command's span is balanced and the segmenter hands over the complete
/// `${a{b}c}`.
///
/// Oracle: `puts ${a{b}c}` reads the variable `a{b}c` on tclsh 9.0.4 and
/// `a{b` on 8.6.16, so the two releases capture different names — and
/// the generated proc's parameter list differs accordingly.
#[test]
fn the_documents_dialect_decides_the_captured_braced_name() {
    let src = "if {1} {puts ${a{b}c}}\nputs done\n";
    let nine = at_dialect(src, "if {1} {puts ${a{b}c}}", "tcl9.0")
        .expect("a selection")
        .apply(src);
    assert!(
        nine.contains("proc extracted_proc {a{b}c} {"),
        "9.x must capture the whole name: {nine}"
    );
    let eight = at_dialect(src, "if {1} {puts ${a{b}c}}", "tcl8.6")
        .expect("a selection")
        .apply(src);
    // `a{b` carries an unbalanced brace, so it reaches the parameter list
    // as the quoted element tclsh itself writes (`list a{b` → `a\{b`).
    // Emitting it raw made `proc extracted_proc {a{b} {…}` — an unbalanced
    // brace that breaks the whole generated command.
    assert!(
        eight.contains(r"proc extracted_proc {a\{b} {"),
        "8.x must capture only `a{{b`, quoted: {eight}"
    );
}

#[test]
fn render_call_braces_a_non_bare_captured_name() {
    // A bare-writable name keeps the bare form…
    assert_eq!(
        render_call("p", &["plain".to_string()], &[]),
        "p $plain",
        "a bare name needs no braces"
    );
    // …and one that cannot be written bare gets the `${…}` spelling.
    // `$a{b}c` would parse as `$a` followed by the literal `{b}c` on both
    // 8.6.16 and 9.0.4, so passing it bare passes the wrong value.
    assert_eq!(
        render_call("p", &["a{b}c".to_string()], &[]),
        "p ${a{b}c}",
        "a brace-bearing name must be passed braced"
    );
    // A by-name argument is a word, not a reference, so it is list-quoted
    // rather than braced.
    assert_eq!(
        render_call("p", &[], &["a{b".to_string()]),
        r"p a\{b",
        "an unbalanced-brace name must reach the call as one word"
    );
    assert_eq!(
        render_call("p", &[], &["plain".to_string()]),
        "p plain",
        "a bare by-name argument stays raw"
    );
}

#[test]
fn extracted_call_passes_a_brace_bearing_name_braced() {
    // End-to-end, with the reference inside a braced body so the command
    // span is balanced and the segmenter hands over the complete word.
    let src = "if {1} {puts ${a{b}c}}\nputs done\n";
    let out = at_dialect(src, "if {1} {puts ${a{b}c}}", "tcl9.0")
        .expect("a selection")
        .apply(src);
    assert!(
        out.contains("extracted_proc ${a{b}c}"),
        "the generated call must brace the name: {out}"
    );
    assert!(
        !out.contains("extracted_proc $a{b}c"),
        "the bare form parses as `$a` plus literal text: {out}"
    );
}

#[test]
fn tp_a_write_never_read_again_becomes_a_proc_local() {
    // `tmp` is not used after the selection, so it is a genuine local:
    // it becomes a proc local and stops leaking into the caller at all.
    let src = "set tmp 1\nputs done\n";
    let result = outcome(src, "set tmp 1").unwrap();
    assert!(
        result.contains("proc extracted_proc {} {\n    set tmp 1\n}"),
        "{result}"
    );
    assert!(!result.contains("upvar"), "no upvar needed: {result}");
}

// TP: caller-frame writes survive via upvar.
/// The same command with its variable substitution switched off reads
/// nothing: `-novariables` leaves `$name` as four literal characters, so
/// asking the caller for `name` would invent a parameter. Which kinds a
/// call runs is the registry's per-call answer, not a switch spelling
/// matched in this module.
///
/// Oracle (tclsh 9.0.4): original and extraction both print the literal
/// `hello $name`.
#[test]
fn tp_a_substituting_call_can_switch_its_variable_reads_off() {
    let src = "set name world\nset msg [subst -novariables {hello $name}]\nputs $msg\n";
    let result = outcome(src, "set msg [subst -novariables {hello $name}]").unwrap();
    assert!(
        result.contains("proc extracted_proc {msgName} {"),
        "a disabled substitution reads nothing: {result}"
    );
    assert!(!result.contains("$name msg"), "{result}");
}

/// A `[…]` inside that argument runs even when variable substitution is
/// off — the manpage's "nested inside a command still runs" — and its
/// contents are ordinary script, so the read is the caller's after all.
#[test]
fn tp_a_substituted_bracket_reads_the_caller_even_with_variables_off() {
    let src =
        "set name world\nset msg [subst -novariables {hello [string toupper $name]}]\nputs $msg\n";
    let result = outcome(
        src,
        "set msg [subst -novariables {hello [string toupper $name]}]",
    )
    .unwrap();
    assert!(
        result.contains("proc extracted_proc {name msgName} {"),
        "the bracket still runs: {result}"
    );
}

/// That bracket is script in the caller's own frame, so a write it makes
/// has to leave by name like any other. The script lexer sees one braced
/// word here, so only the substitution grammar reaches inside it.
///
/// Oracle (tclsh 9.0.4): original and extraction both print `1`; a proc
/// local instead leaves the caller's `total` at `0`.
#[test]
fn tp_a_substituted_bracket_writes_through_to_the_caller() {
    let src = "set total 0\nset msg [subst {x[incr total]}]\nputs $total\n";
    let result = outcome(src, "set msg [subst {x[incr total]}]").unwrap();
    assert!(
        result.contains("upvar 1 $totalName total"),
        "the substituted bracket writes the caller's variable: {result}"
    );
    assert!(result.contains("extracted_proc total\n"), "{result}");
}

/// A command that performs Tcl substitution reads through its own braced
/// argument, so that word is not the inert literal a braced word usually
/// is: `subst {hello $name}` substitutes `$name`.
///
/// Oracle (tclsh 8.6.18 and 9.0.4 alike): the original prints
/// `hello world`; so does the extraction.  Dropping the `name` parameter
/// makes the rewritten call die with `can't read "name"`.
#[test]
fn tp_a_substituting_command_reads_through_its_braced_word() {
    let src = "set name world\nset msg [subst {hello $name}]\nputs $msg\n";
    let result = outcome(src, "set msg [subst {hello $name}]").unwrap();
    assert!(
        result.contains("proc extracted_proc {name msgName} {"),
        "the substituted name is the caller's: {result}"
    );
    assert!(result.contains("extracted_proc $name msg"), "{result}");
}

/// A braced word substitutes nothing, so the `$name` spelled inside one
/// is not a variable the caller has to supply.
///
/// Oracle (tclsh 8.6.18 and 9.0.4 alike): the original prints the four
/// characters `$notavar`; so does the extraction.  A `notavar` parameter
/// makes the rewritten call die with `can't read "notavar"`.
#[test]
fn tp_a_braced_literal_is_not_a_variable_read() {
    let src = "set msg {$notavar}\nputs $msg\n";
    let result = outcome(src, "set msg {$notavar}").unwrap();
    assert!(
        result.contains("proc extracted_proc {msgName} {"),
        "a braced word reads nothing: {result}"
    );
    assert!(result.contains("extracted_proc msg\n"), "{result}");
}

/// The same rule reaches a lambda inside a callback word. `lsort`'s
/// `-command` value is data to the script lexer, so the lambda's own
/// parameters are neither the selection's reads nor the caller's to pass.
///
/// Oracle: the original prints `1 2 3`; so does the extraction.
#[test]
fn tp_a_lambda_in_a_callback_word_is_not_a_parameter() {
    let src = "set items {3 1 2}\nset sorted [lsort -command {apply {{a b} {expr {$a - $b}}}} $items]\nputs $sorted\n";
    let result = outcome(
        src,
        "set sorted [lsort -command {apply {{a b} {expr {$a - $b}}}} $items]",
    )
    .unwrap();
    assert!(
        result.contains("proc extracted_proc {items sortedName} {"),
        "only the list is the caller's: {result}"
    );
    assert!(result.contains("extracted_proc $items sorted"), "{result}");
}

/// A name the selection reads *before* it writes is still an input. The
/// list word of `foreach x $x` is evaluated before the loop binds `x`, so
/// the caller has to hand that list over.
///
/// Oracle (tclsh 8.6.18 and 9.0.4 alike): the original prints `a` then
/// `b`; so does the extraction.  A proc with no `x` parameter dies with
/// `can't read "x": no such variable`.
#[test]
fn tp_a_read_before_a_loop_binding_stays_a_value_parameter() {
    let src = "set x {a b}\nforeach x $x {\n    puts $x\n}\nputs done\n";
    let result = outcome(src, "foreach x $x {\n    puts $x\n}").unwrap();
    assert!(
        result.contains("proc extracted_proc {x} {"),
        "the list the loop iterates comes from the caller: {result}"
    );
    assert!(result.contains("extracted_proc $x\n"), "{result}");
}

/// The same rule without a loop: a command evaluates its arguments before
/// its own write lands, so `$y` reads what the caller set.
///
/// Oracle: the original leaves `y` at 2; so does the extraction.
#[test]
fn tp_a_read_in_the_writing_command_stays_a_value_parameter() {
    let src = "set y 1\nset y [expr {$y + 1}]\nputs done\n";
    let result = outcome(src, "set y [expr {$y + 1}]").unwrap();
    assert!(
        result.contains("proc extracted_proc {y} {"),
        "the operand is the caller's value: {result}"
    );
    assert!(result.contains("extracted_proc $y\n"), "{result}");
}

/// A `[…]` inside a *braced* expression argument is script the script
/// lexer cannot see: `expr` substitutes it, in the caller's own frame. The
/// write it makes therefore has to leave by name like any other.
///
/// Oracle: the original prints `1`; so does the extraction.  Without the
/// `upvar` the caller's `x` stays `0`.
#[test]
fn tp_a_write_inside_a_braced_expression_is_carried_by_upvar() {
    let src = "set x 0\nif {[set x 1]} {\n    puts hi\n}\nputs $x\n";
    let result = outcome(src, "if {[set x 1]} {\n    puts hi\n}").unwrap();
    assert!(
        result.contains("upvar 1 $xName x"),
        "the expression's substitution writes the caller's variable: {result}"
    );
    assert!(result.contains("extracted_proc x\n"), "{result}");
}

/// A selection made *inside* a loop body still writes the caller's
/// variable: a control-flow body shares the frame it sits in, so the
/// "read after the selection?" question runs past the body's closing
/// brace to the frame that owns the variable.
///
/// Oracle (tclsh 8.6.18 and 9.0.4 alike): original and extraction both
/// print `6`.  Classifying `total` as a proc local prints `0`.
#[test]
fn tp_a_write_inside_a_loop_body_reaches_the_caller() {
    let src = "set total 0\nforeach n {1 2 3} {\n    incr total $n\n}\nputs $total\n";
    let result = outcome(src, "incr total $n").unwrap();
    assert!(
        result.contains("proc extracted_proc {n totalName} {\n    upvar 1 $totalName total\n"),
        "the loop body's write is the caller's write: {result}"
    );
    assert!(result.contains("extracted_proc $n total\n"), "{result}");
}

/// The enclosing body counts in full, not only the part after the
/// selection: a loop runs again, so a read *above* the selection reads
/// what the previous iteration assigned.
///
/// Oracle: the original prints `0`, `1`, `3`; so does the extraction.
#[test]
fn tp_a_loop_carried_read_above_the_selection_keeps_the_upvar() {
    let src = "set total 0\nforeach n {1 2 3} {\n    puts $total\n    incr total $n\n}\n";
    let result = outcome(src, "incr total $n").unwrap();
    assert!(
        result.contains("upvar 1 $totalName total"),
        "the next iteration reads what this one wrote: {result}"
    );
    assert!(result.contains("extracted_proc $n total\n"), "{result}");
}

/// A `$name` inside a body that opens its own variable frame names *that*
/// frame's variable, so it is neither a parameter of the extracted proc
/// nor an argument at the call site — the caller has no such variable to
/// pass.
///
/// Oracle: the original defines `helper` and prints `done`; so does the
/// extraction.  Asking the caller for `$name` dies with
/// `can't read "name": no such variable`.
#[test]
fn tp_a_read_inside_a_nested_procs_body_is_not_a_parameter() {
    let src = "proc helper {name} {\n    puts \"hello $name\"\n}\nputs done\n";
    let result = outcome(src, "proc helper {name} {\n    puts \"hello $name\"\n}").unwrap();
    assert!(
        result.contains("proc extracted_proc {} {"),
        "the nested proc's parameter is nobody else's: {result}"
    );
    assert!(
        !result.contains("extracted_proc $name"),
        "the caller has no `name` to pass: {result}"
    );
}

/// A write nested in a control-flow body is still the caller's write.
///
/// Classifying only the selection's *top-level* commands hides the `set
/// total` inside the `foreach` body: `total` reads as never written and
/// leaves as an ordinary value parameter whose assignment the caller
/// never sees.  The loop variable is the mirror case — a `LoopVarList`
/// binding is not a `VarWrite` — and would leave as a second value
/// parameter, making the generated call read a `$n` the caller does not
/// have.
///
/// Oracle (tclsh 8.6.18 and 9.0.4 alike): the original prints `6` and so
/// does the extraction below, while `proc extracted_proc {n total}` /
/// `extracted_proc $n $total` dies with `can't read "n": no such
/// variable`.
#[test]
fn tp_a_write_nested_in_a_loop_body_is_carried_by_upvar() {
    let src =
        "set total 0\nforeach n {1 2 3} {\n    set total [expr {$total + $n}]\n}\nputs $total\n";
    let result = outcome(
        src,
        "foreach n {1 2 3} {\n    set total [expr {$total + $n}]\n}",
    )
    .unwrap();
    assert!(
        result.contains("proc extracted_proc {totalName} {\n    upvar 1 $totalName total\n"),
        "the nested write must reach the caller's frame: {result}"
    );
    assert!(
        result.contains("extracted_proc total\n"),
        "the call passes the variable's *name*: {result}"
    );
    assert!(
        !result.contains("extracted_proc $n"),
        "the loop variable is bound by the loop, not supplied by the caller: {result}"
    );
}

/// The same nesting through a `switch` clause body, which is not an
/// `ArgRole::Body` argument at all: its arms are reached through the
/// registry's own `CaseListSpec`, so the walk covers them only because it
/// asks the shared same-frame walker rather than descending braces itself.
#[test]
fn tp_a_write_nested_in_a_switch_arm_is_carried_by_upvar() {
    let src = "set total 0\nswitch x {\n    x {\n        set total 5\n    }\n}\nputs $total\n";
    let result = outcome(src, "switch x {\n    x {\n        set total 5\n    }\n}").unwrap();
    assert!(
        result.contains("upvar 1 $totalName total"),
        "a switch arm's write must reach the caller's frame: {result}"
    );
    assert!(result.contains("extracted_proc total\n"), "{result}");
}

/// A `dict for` binds its key and value names on every iteration, so they
/// are the loop's own locals rather than parameters the caller supplies —
/// while the `lappend` target nested in the body still leaves by name.
///
/// Oracle: the extraction below prints `a b` on tclsh 8.6.18 and 9.0.4,
/// as the original does.
#[test]
fn tp_a_loops_pair_bindings_are_locals_not_parameters() {
    let src =
        "set d {a 1 b 2}\nset out {}\ndict for {k v} $d {\n    lappend out $k\n}\nputs $out\n";
    let result = outcome(src, "dict for {k v} $d {\n    lappend out $k\n}").unwrap();
    assert!(
        result.contains("proc extracted_proc {d outName} {\n    upvar 1 $outName out\n"),
        "only the dictionary is a value parameter: {result}"
    );
    assert!(result.contains("extracted_proc $d out"), "{result}");
}

/// The nesting stops at a fresh variable frame.  A `proc` body inside the
/// selection writes *that* proc's local, so classifying it as the
/// selection's write would emit an `upvar` for a variable the moved code
/// never touches.
#[test]
fn tp_a_nested_procs_own_writes_are_not_the_selections() {
    let src = "set inner 0\nproc helper {} {\n    set inner 1\n}\nputs $inner\n";
    let result = outcome(src, "proc helper {} {\n    set inner 1\n}").unwrap();
    assert!(
        result.contains("proc extracted_proc {} {"),
        "the nested proc's local is nobody's parameter: {result}"
    );
    assert!(
        !result.contains("upvar"),
        "no caller-frame write happened here: {result}"
    );
}

/// The mirror question — "is this variable read *after* the selection?" —
/// is asked over the tail's statement tree too.  `incr total` carries no
/// `$total` for the text scan to find, so a top-level-only tail scan
/// would classify the write as a proc local and lose the caller's
/// value.
#[test]
fn tp_a_role_named_read_nested_after_the_selection_keeps_the_upvar() {
    let src = "set total 0\nset total 5\nif {1} {\n    incr total\n}\n";
    let result = outcome(src, "set total 5").unwrap();
    assert!(
        result.contains("upvar 1 $totalName total"),
        "the later nested read makes `total` a live-out: {result}"
    );
    assert!(result.contains("extracted_proc total\n"), "{result}");
}

#[test]
fn tp_a_write_read_after_the_selection_is_carried_by_upvar() {
    // With a value parameter the original
    // printed `after=1` and the refactored one printed `after=0`.
    let src = "set x 0\nset x 1\nputs $x\nputs \"after=$x\"\n";
    let result = outcome(src, "set x 1\nputs $x").unwrap();
    assert!(
        result.contains("upvar 1 $xName x"),
        "the write must reach the caller's frame: {result}"
    );
    assert!(
        result.contains("extracted_proc x\n"),
        "the call passes the variable's *name*: {result}"
    );
    assert!(result.contains("puts \"after=$x\""), "{result}");
}

#[test]
fn tp_the_definition_lands_after_the_file_prologue() {
    // Not at line 0: a definition inserted above `package require` would
    // sit before the dialect/package setup it may depend on.
    let src = "package require Tcl\nset x 5\nputs $x\n";
    let result = outcome(src, "puts $x").unwrap();
    let package_line = result.find("package require Tcl").unwrap();
    let proc_line = result.find("proc extracted_proc").unwrap();
    assert!(package_line < proc_line, "{result}");
}

#[test]
fn tp_the_generated_name_avoids_a_collision() {
    let src = "proc extracted_proc {} {}\nset x 5\nputs $x\n";
    let result = outcome(src, "puts $x").unwrap();
    assert!(result.contains("proc extracted_proc_2 {x}"), "{result}");
}

// FP/TN: refusals that keep behaviour.

#[test]
fn fp_refuses_a_selection_containing_return() {
    let src = "proc f {} {\n    set x 1\n    return $x\n}\n";
    let reason = outcome(src, "return $x").unwrap_err();
    assert!(reason.contains("call frame"), "{reason}");
}

#[test]
fn fp_refuses_a_selection_containing_break() {
    let src = "foreach i {1 2 3} {\n    break\n}\n";
    let reason = outcome(src, "break").unwrap_err();
    assert!(reason.contains("call frame"), "{reason}");
}

#[test]
fn fp_refuses_a_selection_containing_continue() {
    let src = "foreach i {1 2 3} {\n    continue\n}\n";
    let reason = outcome(src, "continue").unwrap_err();
    assert!(reason.contains("call frame"), "{reason}");
}

#[test]
fn fp_refuses_a_selection_containing_global() {
    let src = "proc f {} {\n    global config\n    puts $config\n}\n";
    let reason = outcome(src, "global config").unwrap_err();
    assert!(reason.contains("call frame"), "{reason}");
}

#[test]
fn fp_refuses_a_selection_containing_upvar() {
    let src = "proc f {n} {\n    upvar 1 $n local\n}\n";
    let reason = outcome(src, "upvar 1 $n local").unwrap_err();
    assert!(reason.contains("call frame"), "{reason}");
}

#[test]
fn fp_refuses_a_computed_variable_write() {
    let src = "set n counter\nset $n 1\nputs $counter\n";
    let reason = outcome(src, "set $n 1").unwrap_err();
    assert!(reason.contains("computed"), "{reason}");
}

#[test]
fn fp_refuses_an_array_element_write() {
    let src = "set a(x) 1\nputs $a(x)\n";
    let reason = outcome(src, "set a(x) 1").unwrap_err();
    assert!(reason.contains("array element"), "{reason}");
}

#[test]
fn fp_refuses_a_selection_inside_a_namespace_eval() {
    let src = "namespace eval app {\n    set x 1\n    puts $x\n}\n";
    let reason = outcome(src, "puts $x").unwrap_err();
    assert!(reason.contains("namespace"), "{reason}");
}

#[test]
fn tn_no_action_for_an_empty_selection() {
    let registry = super::super::test_registry();
    let mut analyser = Analyser::new();
    let src = "set x 1\n";
    let analysis = analyser.analyse(src, "tcl9.0").clone();
    assert!(extract_proc(src, (0, 0), &analysis, &registry).is_none());
}

#[test]
fn tn_no_action_for_a_selection_covering_no_whole_command() {
    // Half a word is not a command: nothing is offered rather than
    // something wrong being offered.
    let src = "set x 1\n";
    assert!(at(src, "et x").is_none());
}

#[test]
fn tn_no_action_for_a_whitespace_only_selection() {
    let src = "set x 1\n\n\nputs $x\n";
    assert!(at(src, "\n\n\n").is_none());
}

// Dialect drift.

/// iRules' `}{` ghost word separator (no space between an `if`'s
/// condition and its body — the idiom every real iRule uses) must be
/// read under the document's own grammar, never the segmenter's
/// default.  Under the default grammar `{$x}{ … }` reads as one
/// *composite* word (`{cond}` welded to `{body}`), so `if` sees a single
/// argument instead of the required condition + body pair and
/// `enclosing_scope`'s `ArgRole::Body` descent cannot find a body to
/// descend into at all; under `f5-irules` it splits at the `}{`
/// boundary into the ordinary two-word `if {cond} {body}` shape.
#[test]
fn extracts_an_irule_selection_across_the_brace_separator_idiom() {
    let snippet = "if {$x}{ set y 1 }";
    // Document the drift this test guards: the same text really does
    // segment differently under the two grammars.
    let default_cmds = tcl_compiler::segmenter::segment_commands(snippet);
    assert_eq!(
        default_cmds[0].argv.len(),
        2, // `if` + one composite `{$x}{ set y 1 }` argument
        "default grammar must weld `}}{{` into one word: {:?}",
        default_cmds[0].texts
    );
    let irules_cmds = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        snippet,
        0,
        tcl_lexer::LexerConfig::for_dialect("f5-irules"),
    );
    assert_eq!(
        irules_cmds[0].argv.len(),
        3, // `if` + condition + body, split at `}{`
        "iRules grammar must split `}}{{` into two words: {:?}",
        irules_cmds[0].texts
    );

    // A bare `extracted_proc` call is a direct proc call, which iRules
    // reject (IRULE5005: procs are invoked with `call`), and the extraction
    // motion boundary does not cover BIG-IP, so the selection is refused.
    let src = "proc handler {} {\n    if {$x}{\n        set y 1\n        set y 2\n    }\n    puts done\n}\n";
    assert!(at_dialect(src, "set y 1\n        set y 2", "f5-irules").is_none());
}

// Unit-level helpers.

#[test]
fn variable_references_collects_bare_and_braced_names() {
    let found = variable_references("puts $a ${b} $c_1", BracedVarStyle::Tcl9Nesting);
    assert_eq!(
        found.into_iter().collect::<Vec<_>>(),
        vec!["a".to_string(), "b".to_string(), "c_1".to_string()]
    );
}
