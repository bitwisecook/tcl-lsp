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

//! `ArgRole::CommandPrefix` deep-integration: a command-prefix callback
//! (`lsort -command myCompare`) is a first-class command reference across the
//! call graph, find-references, and unknown-command (W123) — all registry
//! driven. Default dialect tcl9.0 (the ground-truth oracle).

use tcl_lsp_core::graphs;
use tcl_registry::CommandRegistry;

/// The nested form (`return [lsort …]`) — the common real shape.
const SRC: &str = "proc myCompare {a b} { expr {$a - $b} }\nproc doSort {items} {\n    return [lsort -command myCompare $items]\n}\n";

#[test]
fn call_graph_has_callback_edge() {
    let reg = CommandRegistry::build_default();
    let g = graphs::call_graph(SRC, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("doSort")
                && e["callee"].as_str().unwrap_or("").contains("myCompare")
        }),
        "expected a doSort→myCompare callback edge; got {g}"
    );
}

#[test]
fn command_invocations_record_callback_head_with_arity() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(SRC, "tcl9.0");
    assert!(
        r.command_invocations
            .iter()
            .any(|i| i.name == "myCompare" && i.callback_arity.is_some()),
        "the nested lsort callback must record a myCompare invocation with callback_arity"
    );
}

#[test]
fn find_references_includes_callback_site() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(SRC, "tcl9.0");
    // Cursor on `myCompare` in its definition (line 0, char 6).
    let refs = tcl_lsp_core::references::references(SRC, "tcl9.0", 0, 6, &r, true);
    assert!(
        refs.iter().any(|rg| rg.start_line == 2),
        "find-references must include the lsort -command callback on line 2; got {refs:?}"
    );
}

#[test]
fn w123_fires_on_unknown_callback_but_not_a_defined_one() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    // Unknown callback head → W123.
    let bad = "lsort -command nonexistentCb {a b}\n";
    let codes: Vec<_> = a
        .analyse(bad, "tcl9.0")
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        codes.iter().any(|c| c == "W123"),
        "an unknown callback head must fire W123; got {codes:?}"
    );
    // Defined callback → no W123.
    let ok = "proc realCb {a b} {expr {$a-$b}}\nlsort -command realCb {a b}\n";
    let codes2: Vec<_> = a
        .analyse(ok, "tcl9.0")
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        !codes2.iter().any(|c| c == "W123"),
        "a defined callback must NOT fire W123; got {codes2:?}"
    );
}

#[test]
fn dynamic_callback_head_is_not_recorded() {
    // `lsort -command $cb` — a dynamic head can't resolve to a proc, so it is
    // neither recorded (no W123 false-fire) nor a call-graph edge.
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(
        "proc f {cb l} { return [lsort -command $cb $l] }\n",
        "tcl9.0",
    );
    assert!(
        !r.command_invocations
            .iter()
            .any(|i| i.callback_arity.is_some()),
        "a dynamic `$cb` callback head must not be recorded as a command-prefix invocation"
    );
}

/// A braced multi-word prefix (`{cb extra}`) — previously silently dropped
/// entirely, per the module doc's former "out of scope" note — must now be
/// recorded exactly like a bareword head, with the head's own span (not the
/// whole braced literal) and its trailing words counted as baked arguments.
#[test]
fn braced_multi_word_prefix_records_head_span_and_baked_count() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let src =
        "proc myCompare {a b extra} { expr {$a - $b} }\nlsort -command {myCompare 99} $items\n";
    let r = a.analyse(src, "tcl9.0");
    let inv = r
        .command_invocations
        .iter()
        .find(|i| i.name == "myCompare" && i.callback_arity.is_some())
        .expect("a braced multi-word prefix must record its head as an invocation");
    assert_eq!(
        inv.callback_baked_args, 1,
        "the `99` word after the head must be counted as one baked argument"
    );
    // The head span must cover only `myCompare`, not the braces or `99` —
    // confirmed by slicing the source at the recorded range.
    let head_text = &src[inv.range.start() as usize..inv.range.end() as usize];
    assert_eq!(
        head_text, "myCompare",
        "the recorded span must cover exactly the head word, got {head_text:?}"
    );
}

/// FP guard: a dynamic head inside the braces (`{$cb extra}`) is not a
/// literal command reference and must not be recorded — mirrors the
/// bareword `$cb` guard (`dynamic_callback_head_is_not_recorded`). The
/// braced word is taken verbatim (no substitution), so `$cb` there is
/// genuinely a dynamic-looking first element, not just visually similar.
#[test]
fn braced_multi_word_prefix_dynamic_head_is_not_recorded() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(
        "proc f {cb l} { return [lsort -command {$cb 99} $l] }\n",
        "tcl9.0",
    );
    assert!(
        !r.command_invocations
            .iter()
            .any(|i| i.callback_arity.is_some()),
        "a dynamic braced-prefix head must not be recorded as a command-prefix invocation"
    );
}

/// A `[list cmd extra]` command-prefix — the idiomatic way to build a
/// callback around a dynamic value (`-command [list doSomething $x]`) rather
/// than a fixed literal prefix — is recorded exactly like the braced-list
/// shape: head span covers just the head word, and every further `list`
/// argument (whatever its own runtime value) is a baked argument.
#[test]
fn list_quoted_prefix_records_head_span_and_baked_count() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let src = "proc myCompare {a b extra} { expr {$a - $b} }\nset extra 99\nlsort -command [list myCompare $extra] $items\n";
    let r = a.analyse(src, "tcl9.0");
    let inv = r
        .command_invocations
        .iter()
        .find(|i| i.name == "myCompare" && i.callback_arity.is_some())
        .expect("a list-quoted prefix must record its head as an invocation");
    assert_eq!(
        inv.callback_baked_args, 1,
        "the `$extra` word after the head must be counted as one baked argument"
    );
    let head_text = &src[inv.range.start() as usize..inv.range.end() as usize];
    assert_eq!(
        head_text, "myCompare",
        "the recorded span must cover exactly the head word, got {head_text:?}"
    );
}

/// FP guard: `[list $cb 99]` — a dynamic head as `list`'s own first argument
/// — is not a literal command reference and must not be recorded, mirroring
/// the braced-shape guard.
#[test]
fn list_quoted_prefix_dynamic_head_is_not_recorded() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(
        "proc f {cb l} { return [lsort -command [list $cb 99] $l] }\n",
        "tcl9.0",
    );
    assert!(
        !r.command_invocations
            .iter()
            .any(|i| i.callback_arity.is_some()),
        "a dynamic list-quoted-prefix head must not be recorded as a command-prefix invocation"
    );
}

/// FP guard: `[llength $x]` — a `Cmd` substitution whose head is *not*
/// `list` (doesn't carry `Traits::BUILDS_COMMAND_PREFIX`) — must not be
/// misread as constructing a deferred command, however similarly shaped.
#[test]
fn cmd_substitution_with_non_list_head_is_not_recorded_as_prefix() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(
        "proc f {items} { return [lsort -command [llength $items] $items] }\n",
        "tcl9.0",
    );
    assert!(
        !r.command_invocations
            .iter()
            .any(|i| i.callback_arity.is_some()),
        "an [llength ...] substitution must not be recorded as a command-prefix invocation"
    );
}

/// The deferred core-Tcl callback surfaces are wired through the same generic
/// substrate — no per-command code — so a registry-declared prefix on
/// `namespace unknown` / `package unknown` / `regsub -command` /
/// `coroinject` lights up call-graph, references, and W123 automatically.
#[test]
fn namespace_unknown_handler_is_a_callback_edge() {
    let reg = CommandRegistry::build_default();
    let src = "proc onUnknown {args} { puts $args }\nproc install {} {\n    namespace unknown onUnknown\n}\n";
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("install")
                && e["callee"].as_str().unwrap_or("").contains("onUnknown")
        }),
        "expected an install→onUnknown edge from `namespace unknown`; got {g}"
    );
}

#[test]
fn regsub_command_prefix_fires_w123_only_when_unknown() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    // `regsub -command` with an unknown head → W123 on the prefix.
    let bad = "regsub -command {(\\w+)} $s missingCb out\n";
    let codes: Vec<_> = a
        .analyse(bad, "tcl9.0")
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        codes.iter().any(|c| c == "W123"),
        "an unknown regsub -command head must fire W123; got {codes:?}"
    );
    // A defined head → recorded, no W123. (Without `-command` the same word is a
    // replacement template and is never treated as a command.)
    let ok =
        "proc doSub {whole} { string toupper $whole }\nregsub -command {(\\w+)} $s doSub out\n";
    let r = a.analyse(ok, "tcl9.0");
    assert!(
        !r.diagnostics.iter().any(|d| d.code.to_string() == "W123"),
        "a defined regsub -command head must not fire W123"
    );
    assert!(
        r.command_invocations.iter().any(|i| i.name == "doSub"),
        "regsub -command must record a reference to its head"
    );
    // No `-command`: the third positional is a template, not a command.
    let template = "regsub {(\\w+)} $s missingCb out\n";
    assert!(
        !a.analyse(template, "tcl9.0")
            .diagnostics
            .iter()
            .any(|d| d.code.to_string() == "W123"),
        "a plain regsub subSpec must never be treated as a command"
    );
}

#[test]
fn coroprobe_records_reference_but_never_arity_checks() {
    // `coroprobe`'s appended arity is Unknown (depends on the yield point), so
    // the injected command is a reference/W123 surface but is never arity-checked.
    // (`coroinject`, once this test's exemplar, was moved off `Unknown` to the
    // verified `Exactly(2)` its own implementation always appends — see
    // `coroinject.rs` — so it no longer demonstrates this FP guard.)
    let reg = CommandRegistry::build_default();
    let src = "proc worker {args} { yield }\nproc kick {c} {\n    coroprobe $c worker extra\n}\n";
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("kick")
                && e["callee"].as_str().unwrap_or("").contains("worker")
        }),
        "expected a kick→worker edge from `coroprobe`; got {g}"
    );
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let inj = r
        .command_invocations
        .iter()
        .find(|i| i.name == "worker")
        .expect("coroprobe records the injected command");
    assert_eq!(
        inj.callback_arity,
        Some(tcl_registry::AppendedArity::Unknown),
        "coroprobe's callback arity is Unknown so it is never flagged too-few/too-many"
    );
}

/// tcllib callbacks flow through the same substrate.  This exercises two tcllib
/// paths at once: a sub-command-offset callback (`struct::list split`, arg after
/// the sub-command word) and a full-name math callback declared via the
/// `PREFIX_OVERRIDES` side table.
#[test]
fn tcllib_struct_list_split_is_a_callback_edge() {
    let reg = CommandRegistry::build_default();
    let src = "proc isEven {n} { expr {$n % 2 == 0} }\nproc part {items} {\n    struct::list split $items isEven evens odds\n}\n";
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("part")
                && e["callee"].as_str().unwrap_or("").contains("isEven")
        }),
        "expected a part→isEven edge from `struct::list split`; got {g}"
    );
}

#[test]
fn tcllib_calculus_func_records_reference_with_fixed_arity() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let src = "proc f {x} { expr {$x * $x} }\nmath::calculus::integral 0 1 100 f\n";
    let r = a.analyse(src, "tcl9.0");
    let inv = r
        .command_invocations
        .iter()
        .find(|i| i.name == "f")
        .expect("math::calculus::integral records its func callback");
    assert_eq!(
        inv.callback_arity,
        Some(tcl_registry::AppendedArity::Exactly(1)),
        "the calculus func callback carries its man-page-pinned Exactly(1) arity"
    );
}

/// Tk `script()→command_prefix` conversion (highest-risk change).  A bareword
/// user-proc scroll/scale callback that was previously *invisible* (script
/// recursion never recorded a single-word head) is now a first-class reference
/// + call-graph edge.
#[test]
fn tk_scale_command_bareword_head_is_a_callback_edge() {
    let reg = CommandRegistry::build_default();
    let src = "proc onScale {v} { puts $v }\nproc build {} {\n    scale .s -command onScale\n}\n";
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("build")
                && e["callee"].as_str().unwrap_or("").contains("onScale")
        }),
        "expected a build→onScale edge from `scale -command`; got {g}"
    );
}

#[test]
fn tk_scroll_callback_braced_widget_path_does_not_fire_w123() {
    // The pre-conversion `script()` recursion flagged `.sb` inside `{.sb set}`
    // as an unknown command (a widget-path FP).  As a command prefix the braced
    // head is not a literal bareword, so it is dropped — no W123.  (Regression
    // guard for the conversion's headline benefit.)
    let mut a = tcl_compiler::analyser::Analyser::new();
    let src = "listbox .lb -yscrollcommand {.sb set}\n";
    let codes: Vec<_> = a
        .analyse(src, "tcl9.0")
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        !codes.iter().any(|c| c == "W123"),
        "a braced widget-path scroll callback must not draw a widget-path W123; got {codes:?}"
    );
}

#[test]
fn tk_scroll_callback_bareword_undefined_head_fires_w123() {
    // A bareword scroll callback head that resolves to no proc is a genuine
    // typo — now caught (it was silently missed under `script()`).
    let mut a = tcl_compiler::analyser::Analyser::new();
    let bad = "listbox .lb -yscrollcommand missingScroller\n";
    let codes: Vec<_> = a
        .analyse(bad, "tcl9.0")
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        codes.iter().any(|c| c == "W123"),
        "an unknown bareword scroll callback head must fire W123; got {codes:?}"
    );
    // A defined head is silent.
    let ok = "proc myScroller {first last} { }\nlistbox .lb -yscrollcommand myScroller\n";
    assert!(
        !a.analyse(ok, "tcl9.0")
            .diagnostics
            .iter()
            .any(|d| d.code.to_string() == "W123"),
        "a defined scroll callback head must not fire W123"
    );
}

// ---------------------------------------------------------------------------
// Option-value callbacks — the prefix is the value of a named `-flag`, resolved
// through the command's `OptionSpec` array (no `command_prefixes` table entry).
// These flow through the identical generic substrate, so a bareword head lights
// up call-graph / references / W123 / arity with zero per-command code.
// ---------------------------------------------------------------------------

#[test]
fn mime_getbody_command_option_is_a_callback_edge() {
    let reg = CommandRegistry::build_default();
    let src = "proc onChunk {reason args} { puts $reason }\nproc fetchBody {tok} {\n    mime::getbody $tok -command onChunk\n}\n";
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("fetchBody")
                && e["callee"].as_str().unwrap_or("").contains("onChunk")
        }),
        "expected a fetchBody→onChunk edge from `mime::getbody -command`; got {g}"
    );
}

#[test]
fn smtp_tlspolicy_option_fires_w123_only_when_unknown() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    // Unknown -tlspolicy head → W123.
    let bad = "smtp::sendmessage $tok -tlspolicy noSuchPolicy\n";
    assert!(
        a.analyse(bad, "tcl9.0")
            .diagnostics
            .iter()
            .any(|d| d.code.to_string() == "W123"),
        "an unknown -tlspolicy callback head must fire W123"
    );
    // Defined head → silent.
    let ok = "proc tlsDecider {code diag} { return secure }\nsmtp::sendmessage $tok -tlspolicy tlsDecider\n";
    assert!(
        !a.analyse(ok, "tcl9.0")
            .diagnostics
            .iter()
            .any(|d| d.code.to_string() == "W123"),
        "a defined -tlspolicy callback head must not fire W123"
    );
}

#[test]
fn bibtex_recordcommand_option_is_a_callback_edge() {
    let reg = CommandRegistry::build_default();
    let src = "proc saveRecord {token type key data} { }\nproc parseBib {text} {\n    bibtex::parse -recordcommand saveRecord $text\n}\n";
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("parseBib")
                && e["callee"].as_str().unwrap_or("").contains("saveRecord")
        }),
        "expected a parseBib→saveRecord edge from `bibtex::parse -recordcommand`; got {g}"
    );
}

#[test]
fn halfpipe_write_command_option_records_callback() {
    let reg = CommandRegistry::build_default();
    let src = "proc onWrite {chan bytes} { }\nproc mk {} {\n    tcl::chan::halfpipe -write-command onWrite\n}\n";
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("mk")
                && e["callee"].as_str().unwrap_or("").contains("onWrite")
        }),
        "expected an mk→onWrite edge from `tcl::chan::halfpipe -write-command`; got {g}"
    );
}

// ---------------------------------------------------------------------------
// Resolver-based prefix (`hook bind`) and script-vs-prefix disambiguation
// (`processman::onexit`).
// ---------------------------------------------------------------------------

#[test]
fn hook_bind_binding_is_a_callback_edge() {
    // `hook bind subject hook observer binding` — the 4-word set form's final
    // word is a command prefix (resolver-driven, Unknown arity).
    let reg = CommandRegistry::build_default();
    let src = "proc onEvent {args} { }\nproc wire {} {\n    hook bind subj myHook obs onEvent\n}\n";
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    let edges = g["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("wire")
                && e["callee"].as_str().unwrap_or("").contains("onEvent")
        }),
        "expected a wire→onEvent edge from `hook bind`; got {g}"
    );
}

#[test]
fn hook_bind_query_form_is_not_a_callback() {
    // The 3-word query form (`hook bind subject hook observer`) names no
    // callback — a bareword there is a plain observer id, so no spurious W123.
    let mut a = tcl_compiler::analyser::Analyser::new();
    let src = "hook bind subj myHook someObserver\n";
    assert!(
        !a.analyse(src, "tcl9.0")
            .diagnostics
            .iter()
            .any(|d| d.code.to_string() == "W123"),
        "the 3-arg `hook bind` query form must not fire W123 on the observer word"
    );
}

#[test]
fn processman_onexit_script_body_is_recursed_not_a_prefix() {
    // `processman::onexit id cmd` — `cmd` is a deferred *script* (Body role, run
    // later via `eval` in its own dispatch context), not a command prefix.  So
    // its braced contents are recursed as a script: a typo inside fires W123 and
    // a defined command is recorded / silent.  (Unlike a prefix, the `cmd` word
    // itself is not a callback head, and — deferred — it forms no enclosing-proc
    // call edge.)
    let mut a = tcl_compiler::analyser::Analyser::new();
    let bad = "processman::onexit $pid { noSuchCleanupProc }\n";
    assert!(
        a.analyse(bad, "tcl9.0")
            .diagnostics
            .iter()
            .any(|d| d.code.to_string() == "W123"),
        "an unknown command inside the processman::onexit script must fire W123 (body recursed)"
    );
    // A defined command inside the script is recorded as an invocation (feeding
    // references) and draws no W123.
    let ok = "proc cleanup {} { }\nprocessman::onexit $pid { cleanup }\n";
    let r = a.analyse(ok, "tcl9.0");
    assert!(
        !r.diagnostics.iter().any(|d| d.code.to_string() == "W123"),
        "a defined command inside the processman::onexit script must not fire W123"
    );
    assert!(
        r.command_invocations.iter().any(|i| i.name == "cleanup"),
        "the processman::onexit script body's inner call must be recorded as an invocation"
    );
}

// ---------------------------------------------------------------------------
// Object-instance method callbacks — the prefix is on a method of a created
// object command (`struct::graph name` / `set g [struct::graph]`), resolved
// through the class's ObjectClassSpec + the receiver's tracked type.  Foreground
// analysis (single-file diagnostics, call graph, in-file references).
// ---------------------------------------------------------------------------

#[test]
fn struct_graph_walk_command_records_callback_with_arity() {
    // `struct::graph myG` names the instance; `myG walk … -command cb` resolves
    // `cb` through the graph class's `walk` method (option-value prefix), so the
    // bareword head is recorded as an invocation carrying the Exactly(3)
    // callback arity (feeding references + the callback-arity check).
    let mut a = tcl_compiler::analyser::Analyser::new();
    let src = "proc onNode {action g node} { }\nproc build {} {\n    struct::graph myG\n    myG walk root -command onNode\n}\n";
    let r = a.analyse(src, "tcl9.0");
    assert!(
        r.command_invocations.iter().any(|i| {
            i.name == "onNode" && i.callback_arity == Some(tcl_registry::AppendedArity::Exactly(3))
        }),
        "`myG walk -command onNode` must record an onNode invocation with Exactly(3) callback arity"
    );
    // The interprocedural pass now tracks the receiver's object type, so the
    // callback is also a real call-graph edge (build → onNode).
    let reg = CommandRegistry::build_default();
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    assert!(
        g["edges"].as_array().expect("edges").iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("build")
                && e["callee"].as_str().unwrap_or("").contains("onNode")
        }),
        "`myG walk -command onNode` must form a build→onNode call-graph edge; got {g}"
    );
}

#[test]
fn struct_graph_walk_command_var_handle_fires_w123_only_when_unknown() {
    // The `set g [struct::graph]` handle form types `g`, so `$g walk … -command`
    // resolves too.  Unknown head → W123; defined 3-param head → silent.
    let mut a = tcl_compiler::analyser::Analyser::new();
    let bad = "set g [struct::graph]\n$g walk root -command noSuchNodeProc\n";
    assert!(
        a.analyse(bad, "tcl9.0")
            .diagnostics
            .iter()
            .any(|d| d.code.to_string() == "W123"),
        "an unknown `$g walk -command` head must fire W123"
    );
    let ok =
        "proc onNode {action g node} { }\nset g [struct::graph]\n$g walk root -command onNode\n";
    assert!(
        !a.analyse(ok, "tcl9.0")
            .diagnostics
            .iter()
            .any(|d| d.code.to_string() == "W123"),
        "a defined `$g walk -command` head must not fire W123"
    );
}

#[test]
fn struct_tree_walkproc_trailing_prefix_is_recorded() {
    // `struct::tree` instance `walkproc node … cmdprefix` — a *trailing
    // positional* prefix (resolver-driven), unlike graph's option-value one.
    let mut a = tcl_compiler::analyser::Analyser::new();
    let src = "proc onN {tree node action} { }\nproc build {} {\n    struct::tree myT\n    myT walkproc root onN\n}\n";
    let r = a.analyse(src, "tcl9.0");
    assert!(
        r.command_invocations.iter().any(|i| {
            i.name == "onN" && i.callback_arity == Some(tcl_registry::AppendedArity::Exactly(3))
        }),
        "`myT walkproc … onN` must record an onN invocation with Exactly(3) callback arity"
    );
    // …and a build→onN call-graph edge via the interprocedural object typing.
    let reg = CommandRegistry::build_default();
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    assert!(
        g["edges"].as_array().expect("edges").iter().any(|e| {
            e["caller"].as_str().unwrap_or("").contains("build")
                && e["callee"].as_str().unwrap_or("").contains("onN")
        }),
        "`myT walkproc … onN` must form a build→onN call-graph edge; got {g}"
    );
}

/// Issue #923 differential audit, finding idx 47 — the `trace` idiom Tk's own
/// `etrace.tcl` writes verbatim: `trace add variable V write [list handler
/// baked…]`.  The general `[list HEAD …]` mechanism is covered above via
/// `lsort -command`; this pins the trace-specific spelling, whose head sits in
/// a `write`-op argument rather than an option value, so a future narrowing of
/// `extract_list_quoted_prefix_head` back to the option-value shape fails here.
///
/// Oracle — tclsh 8.6.16 and 9.0.4, writing the traced variable:
///
/// ```text
/// initial: 1
/// onVersionWrite fired: minlevel=1.0.0 baseline=1 args=::demo::version {} write
/// final: 42
/// ```
///
/// The handler really is invoked, solely through the trace's `[list …]`
/// command prefix, so the site is a real reference to it.
#[test]
fn trace_add_variable_write_list_prefix_is_a_reference() {
    let src = "namespace eval demo {\n\
               \x20   variable version 1\n\
               \x20   proc onVersionWrite {minlevel baseline args} { return $args }\n\
               \x20   trace add variable version write [list demo::onVersionWrite 1.0.0 $version]\n\
               }\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(src, "tcl9.0");

    let inv = r
        .command_invocations
        .iter()
        .find(|i| i.name.ends_with("onVersionWrite") && i.callback_arity.is_some())
        .unwrap_or_else(|| {
            panic!(
                "the trace's [list …] handler must be recorded as a command-prefix \
                 invocation; got {:?}",
                r.command_invocations
                    .iter()
                    .map(|i| i.name.as_str())
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(
        inv.callback_baked_args, 2,
        "`1.0.0` and `$version` are baked before Tcl appends name/index/op"
    );
    let head_text = &src[inv.range.start() as usize..inv.range.end() as usize];
    assert_eq!(
        head_text, "demo::onVersionWrite",
        "the recorded span must cover exactly the written head word"
    );

    // The reference surfaces every command_invocations-fed consumer reads:
    // find-references anchored at the declaration must reach the trace site.
    let decl_line = 2;
    let refs = tcl_lsp_core::references::references(src, "tcl9.0", decl_line, 9, &r, true);
    assert!(
        refs.iter().any(|rg| rg.start_line == 3),
        "find-references from the proc declaration must include the \
         `trace add variable … write [list …]` call site on line 3; got {refs:?}"
    );

    // …and the call graph carries the edge, attributed to the enclosing scope.
    let reg = CommandRegistry::build_default();
    let g = graphs::call_graph(src, &reg, "tcl9.0");
    assert!(
        g["edges"]
            .as_array()
            .expect("edges array")
            .iter()
            .any(|e| e["callee"]
                .as_str()
                .unwrap_or("")
                .contains("onVersionWrite")),
        "the trace handler must be a call-graph callee; got {g}"
    );
}

/// FP guard for the same shape: a *dynamic* head inside the trace's
/// `[list …]` names no static command and must stay unrecorded, so the
/// trace-specific path inherits the general mechanism's discipline rather
/// than relaxing it.
#[test]
fn trace_add_variable_write_list_prefix_with_a_dynamic_head_is_not_recorded() {
    let mut a = tcl_compiler::analyser::Analyser::new();
    let r = a.analyse(
        "proc arm {cb} {\n    variable v\n    trace add variable v write [list $cb 1]\n}\n",
        "tcl9.0",
    );
    assert!(
        !r.command_invocations
            .iter()
            .any(|i| i.callback_arity.is_some()),
        "a `[list $cb …]` trace handler is a runtime value, not a static reference"
    );
}
