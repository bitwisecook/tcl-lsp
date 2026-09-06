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

//! Word-**value** resolution on the compiled path — the value half of the rule
//! `variable_name_resolution_e2e` pins for names: *a word is resolved exactly
//! once.*
//!
//! The compiled path resolved a word's value and then handed the result back to
//! the VM's runtime word substitution, which read it as source a second time.
//! For a value that happens to *look* like a braced word — `"{}"`, `"{x}"`,
//! `"\{\}"` — `subst_word`'s whole-word-braced fast path then stripped a brace
//! layer that was never in the source:
//!
//! ```text
//! % puts [string index "{}" 0]           ;# 8.6.16 / 9.0.4: {    was: (empty)
//! % set v "{}" ; puts [string length $v] ;# 8.6.16 / 9.0.4: 2    was: 0
//! % set z x ; puts "{$z}"                ;# 8.6.16 / 9.0.4: {x}  was: ${z}
//! ```
//!
//! The braced half of the same hole was closed twice before (issue #1602, then
//! `emit_cmd_subst_arg`'s braced arm). A de-quoted word is finished for exactly
//! the same reason a de-braced one is — the quotes are gone and the escapes are
//! decoded — and so is each literal *fragment* a composite word decomposes to.
//!
//! Two shapes, two fixes, because a value that still carries a marker cannot
//! simply be frozen:
//!
//! * **No marker left** (`"{}"`, a decoded `\{\}`, a `Lit` fragment): pushed
//!   unsubstituted — `CodegenCtx::push_word_value` / `push_lit_exact`. The
//!   marker test is the VM's own, so this can only remove the brace strip:
//!   `subst_word` returns a word carrying no `${` and no `[` unchanged apart
//!   from it.
//! * **A live marker inside the braces** (`"{$z}"`, `"{[pz]}"`): decomposed at
//!   compile time instead, because the `${…}` / `[…]` must still run and the
//!   surrounding braces are word content. Pushed raw, the VM stripped the
//!   braces and returned the inside *unsubstituted*.
//!
//! The same rule reaches two places that *produce* a value rather than write
//! one, and both got it wrong in both halves. A **constant fold** runs its
//! command at compile time, so its result is finished — yet the folds pushed it
//! substituting, and `dict create` did so in both emitters. A **braced `expr`
//! operand** is a literal — yet codegen decided "braced" on the necessary
//! condition alone (`{` first, `}` last) and pushed the content substituting,
//! so `{}${z}` was stripped to the unbalanced `}${z}` and `expr {{a[nope]}}`
//! ran `nope`. That first one is why `switch -- "{}$z" …` raised on a script
//! both oracles run: a switch subject reaches codegen as that operand. The
//! balance walk now has one owner, `tcl_syntax::word_rules::whole_braced_word`,
//! shared with the `subst_word` side that asks the same question.
//!
//! The negative vectors below pin both boundaries: live substitution still
//! happens, and a genuinely braced word still loses exactly one layer.
//!
//! The last group is the paths the first pass at this rule did not reach, all
//! found by the fuzzer once it could generate a word whose value is not its
//! spelling (#1897). Two are the same rule in an emitter that was missed —
//! `emit_value`'s default push, the twin of the one that was fixed, which is
//! the path a proc's `return` value takes; and a braced `switch` subject,
//! whose braced-ness the IR recorded for the *patterns* but never for the
//! subject. Three are neighbouring reads of a word that were wrong in their own
//! way: a fold's brace-depth scan that counted braces inside a quoted word as a
//! group, a word splitter that ended a word at an escaped blank, and a fold
//! that walked a quoted argument one byte at a time. The last is the mistake
//! `parse_subst_template` had fixed for #1441, surviving in its neighbour —
//! which is the argument for pinning all of them here rather than beside each
//! emitter.
//!
//! Every vector runs through the VM at all five releases and, when the matching
//! real tclsh is installed, under it too, so the table cannot drift from C Tcl.
//! A word's value is not a release axis — 8.6.16 and 9.0.4 agree on every
//! vector here — so one expectation column is enough.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_vm::{CompileError, CompileService, Vm};

/// The profile the vector selected, compiled exactly — never the ambient
/// default, or every release column would be measured against one grammar.
fn compile_exact_profile(
    src: &str,
    profile: &'static DialectProfile,
) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    if let Some(message) = tcl_compiler::lowering::first_fatal_parse_error_with_config(src, config)
    {
        return Err(CompileError(message));
    }
    let ir = tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect(
        src,
        registry,
        config,
        Some(profile),
    );
    let cfg = build_cfg_codegen(&ir, false);
    Ok(codegen_module(&cfg, &ir, registry))
}

fn profile_of(version: TclVersion) -> &'static DialectProfile {
    tcl_registry::model::ingress::resolve_environment(version.dialect_name()).analyser_profile()
}

/// Compiles at the release the VM is running, so a `[…]` the VM hands back for
/// compilation is not silently lowered under a different grammar.
struct CompilerSvc(TclVersion);

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        compile_exact_profile(src, profile_of(self.0))
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        compile_exact_profile(src, profile)
    }
}

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Run `src` in the VM at `version`, returning its trimmed `puts` output — the
/// same shape the tclsh leg produces, so the two are directly comparable.
fn vm_output(src: &str, version: TclVersion) -> String {
    let asm = compile_exact_profile(src, profile_of(version))
        .expect("test script compiles for its selected profile");
    let capture = Capture::default();
    let mut vm = Vm::with_output(Box::new(capture.clone()));
    vm.set_compiler(Box::new(CompilerSvc(version)));
    vm.set_runtime_version(version);
    let _ = vm.run_module(&asm);
    String::from_utf8_lossy(&capture.0.borrow())
        .trim()
        .to_string()
}

/// Run `src` under the real tclsh for `version`, or `None` when that binary
/// isn't installed.
fn tclsh_output(version: TclVersion, src: &str) -> Option<String> {
    let tclsh = tcl_test_support::locate_tclsh(version).ok().flatten()?;
    tcl_test_support::run_script(&tclsh.path, src.as_bytes())
        .ok()?
        .strict_text()
        .ok()
}

/// One behaviour vector: the script prints its observations, `want` is the full
/// expected stdout. Every `want` here was measured on tclsh 8.6.16 and 9.0.4,
/// which agree, and `vectors_match_real_tclsh` keeps it that way.
struct Vector {
    name: &'static str,
    script: &'static str,
    want: &'static str,
    /// Earliest release the vector runs at. Only the `dict` vectors need it:
    /// `dict` arrived in 8.5, so at 8.4 the script is an `invalid command
    /// name` on both sides and measures nothing about word values.
    since: TclVersion,
}

const VECTORS: &[Vector] = &[
    // -- No marker left: a finished value is pushed unsubstituted. --
    Vector {
        name: "a quoted value that looks braced keeps both braces in a command arg",
        script: r#"puts [string index "{}" 0]:[string length "{}"]:[string length "{abc}"]
"#,
        want: "{:2:5",
        since: TclVersion::V8_4,
    },
    Vector {
        name: "a quoted value that looks braced keeps both braces through an assignment",
        script: r#"set v "{}"
set w "{abc}"
puts [string length $v]:[string length $w]:$w
"#,
        want: "2:5:{abc}",
        since: TclVersion::V8_4,
    },
    Vector {
        // The proc body runs through the same word emitters as the top level
        // but with the local-variable table in play, so it is pinned
        // separately: #1602's braced half regressed in exactly that direction.
        name: "the same value survives a proc-local round trip",
        script: r#"proc pv {} { set v "{q}" ; return [string length $v]:[set v] }
puts [pv]
"#,
        want: "3:{q}",
        since: TclVersion::V8_4,
    },
    Vector {
        name: "a quoted value that looks braced passes intact into a user proc",
        script: r#"proc pl {s} { return [string length $s] }
puts [pl "{}"]:[pl "{abc}"]
"#,
        want: "2:5",
        since: TclVersion::V8_4,
    },
    Vector {
        // Braces reached by *escape* rather than by quoting are the same case
        // one step later: the word is finished once its escapes are decoded, so
        // the decoded text must not be re-read as source either.
        name: "escaped braces decode to a value that is not re-stripped",
        script: r"set e \{\}
puts [string length $e]:[string length \{\}]:$e
",
        want: "2:2:{}",
        since: TclVersion::V8_4,
    },
    Vector {
        // A nested substitution builds its arguments through the *inner* word
        // emitter (`emit_cmd_subst_arg`), which had its own copy of the hole.
        name: "the inner word emitter keeps the braces too",
        script: r#"puts [string length [format %s%s "{}" "{}"]]:[format %s%s "{}" x]
"#,
        want: "4:{}x",
        since: TclVersion::V8_4,
    },
    Vector {
        // A composite word decomposes to literal *fragments*, which the
        // template parser has already decoded — so they are finished for the
        // same reason, and a fragment that looks braced was being stripped
        // too. `[string length "{}$z"]` answered 1.
        name: "a literal fragment of a composite word keeps its braces",
        script: r#"set z x
puts [string length "{}$z"]:[string length "$z{}"]
puts "{}$z"
"#,
        want: "3:3\n{}x",
        since: TclVersion::V8_4,
    },
    Vector {
        // The same fragment rule inside an `expr` operand, which has its own
        // copy of the decomposition loop.
        name: "an expr operand's literal fragment keeps its braces",
        script: r#"set z x
puts [expr {"{}$z"}]:[string length [expr {"{}$z"}]]
"#,
        want: "{}x:3",
        since: TclVersion::V8_4,
    },
    // -- A live marker: decomposed, not frozen. --
    Vector {
        // The marker test's whole purpose. Freezing every brace-shaped value
        // would print the literal `{$z}` here; pushing it raw (what the VM did)
        // stripped the braces and returned the *unsubstituted* `${z}`.
        name: "a brace-shaped value with a live variable still substitutes",
        script: r#"set z x
puts [string length "{$z}"]
puts "{$z}"
set q "{$z}"
puts $q:[string length $q]
"#,
        want: "3\n{x}\n{x}:3",
        since: TclVersion::V8_4,
    },
    Vector {
        name: "a brace-shaped value with a live command substitution still runs it",
        script: r#"proc pz {} { return x }
puts [string length "{[pz]}"]
puts "{[pz]}"
"#,
        want: "3\n{x}",
        since: TclVersion::V8_4,
    },
    // -- The negative boundary: the braced-word rule is untouched. --
    Vector {
        // The positive control for the arm this fix sits next to: a genuinely
        // braced word still loses exactly one brace layer, and its `$` / `[`
        // stay data. Freezing the quoted case must not disturb it.
        name: "a genuinely braced word still loses exactly one layer",
        script: r"puts [string length {{}}]:[string length {}]:[string length {$z[pz]}]
puts {{a}}
",
        want: "2:0:6\n{a}",
        since: TclVersion::V8_4,
    },
    // -- The same rule where the value is *produced*, not written. --
    Vector {
        // A constant fold runs the command at compile time, so its result is a
        // value with no word rule left — but the folds pushed it substituting
        // (`dict create` did so in both emitters, ignoring the `kind` its
        // siblings took), and a result that looked braced lost its braces.
        name: "a folded dict create result is a value, braces included",
        script: r"set d [dict create k \{\}]
set w [dict get [dict create k \{\}] k]
puts [string length $d]:[string length $w]:$d
",
        want: "6:2:k {{}}",
        since: TclVersion::V8_5,
    },
    Vector {
        // The marker half of the same fold bug: pushed substituting, the folded
        // result's `${b}` was read as a variable at run time.
        name: "a folded dict create result keeps a marker as data",
        script: r"set f [dict create k {a${b}}]
puts $f:[string length $f]
",
        want: "k {a${b}}:9",
        since: TclVersion::V8_5,
    },
    Vector {
        // `format`'s fold read its *bare* arguments as source spelling: it
        // stopped the word at any blank and never decoded the escapes, while
        // the quoted arm beside it did both.
        name: "a folded format result decodes its bare arguments",
        script: r"set a [format %s \{\}]
set b [format %s a\ b]
set c [format %s a\tb]
puts [string length $a]:$a:[string length $b]:$b:[string length $c]
",
        want: "2:{}:3:a b:3",
        since: TclVersion::V8_4,
    },
    Vector {
        name: "a folded list result keeps its braces",
        script: r"set l [list \{\}]
puts $l:[string length $l]
",
        want: "{{}}:4",
        since: TclVersion::V8_4,
    },
    // -- The braced *operand* arm, which decided the same question twice. --
    Vector {
        // Codegen's braced-`expr`-operand arm stripped on "first `{`, last `}`"
        // and pushed the content substituting. Both halves were wrong: the
        // content is a braced word's finished value.
        name: "a braced expr operand is a literal, and its markers are data",
        script: r"puts [expr {{a[nope]}}]:[expr {{a$b}}]:[string length [expr {{}}]]
",
        want: "a[nope]:a$b:0",
        since: TclVersion::V8_4,
    },
    Vector {
        // A `switch` subject reaches codegen as that same operand. `{}${z}`
        // closes its leading brace at byte 1, so stripping it left the
        // unbalanced `}${z}` and the VM raised on a script both oracles run.
        // The second arm proves the subject still substitutes to `{}x`.
        name: "a brace-shaped switch subject is not a braced word",
        script: r#"set z x
switch -- "{}$z" "zz" { puts A:hit } default { puts A:def }
switch -- "{}$z" "{}x" { puts B:hit } default { puts B:def }
"#,
        want: "A:def\nB:hit",
        since: TclVersion::V8_4,
    },
    Vector {
        // A brace-shaped value that is not a *whole* braced word never reached
        // the VM's strip in the first place, so the fix is measured against a
        // case it cannot have changed.
        name: "a value that is not whole-word braced is unaffected",
        script: r#"puts [string length "{} {}"]:[string length "a{}b"]
"#,
        want: "5:4",
        since: TclVersion::V8_4,
    },
    // -- The paths the first pass at this rule did not reach. --
    Vector {
        // `emit_value`'s default push — the twin of `emit_value_interpolated`'s,
        // fixed at the same time as its sibling was not. It is the emitter a
        // proc's `return` value goes through, so the value came back de-braced.
        name: "a proc's return value is a value, braces included",
        script: r#"proc pr {} { return "{abc}" }
proc pe {} { return "{}" }
puts [pr]:[string length [pr]]:[string length [pe]]
"#,
        want: "{abc}:5:2",
        since: TclVersion::V8_4,
    },
    Vector {
        // A braced `switch` subject is a literal: its `[…]` and `${…}` are
        // data. The IR recorded whether the *patterns* were braced but never
        // the subject, so codegen could not tell it from a bare word and ran
        // the command. The third arm is the boundary — an unbraced subject
        // still substitutes, which is the common form and what broke first
        // when the braced flag was read off the wrong predicate.
        name: "a braced switch subject is a literal, and an unbraced one is not",
        script: r"switch -- {a[nosuchcmd]} zz {puts A:hit} default {puts A:def}
switch -- {a${nosuchvar}} zz {puts B:hit} default {puts B:def}
set z hit
switch -- $z hit {puts C:match} default {puts C:def}
",
        want: "A:def\nB:def\nC:match",
        since: TclVersion::V8_4,
    },
    Vector {
        // The `list` fold's brace-depth scan counted a brace inside a *quoted*
        // word as a group, so a live `$x` / `[…]` looked protected and the fold
        // froze the source spelling instead of declining.
        name: "a fold declines on a quoted word that still substitutes",
        script: r#"set x 7
set l [list "{$x}"]
set m [list "{[string length ab]}"]
puts [string length $l]:$l:$m
"#,
        want: "5:{{7}}:{{2}}",
        since: TclVersion::V8_4,
    },
    Vector {
        name: "the same fold rule reaches dict create",
        script: r#"set d [dict create k "{[string length ab]}"]
puts [string length [dict get $d k]]
"#,
        want: "3",
        since: TclVersion::V8_5,
    },
    Vector {
        // An escaped blank is word *content*: `a\ b` is one word. The bracket
        // word splitter ended the word there anyway, so `string length` was
        // handed two arguments. The `\t` case is the same escape one step on —
        // the value carries a character no spelling of it contains.
        name: "an escaped blank does not end a word",
        script: r"puts [string length a\ b]:[llength [list a\ b c]]:[string length a\tb]
",
        want: "3:2:3",
        since: TclVersion::V8_4,
    },
    Vector {
        // `format`'s fold walked its quoted argument one *byte* at a time
        // through `char::from`, which maps a byte to that value's Latin-1 code
        // point — so every byte of a multi-byte character became its own
        // mojibake char. The same mistake was fixed in `parse_subst_template`
        // for #1441 and survived in its neighbour.
        name: "a folded format result counts characters, not bytes",
        script: r#"set f [format %s "café"]
puts [string length $f]:$f
"#,
        want: "4:café",
        since: TclVersion::V8_4,
    },
];

#[test]
fn vm_matches_the_pinned_word_value_vectors() {
    for v in VECTORS {
        assert_eq!(
            vm_output(v.script, TclVersion::V8_6),
            v.want,
            "[8.6] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V9_0),
            v.want,
            "[9.0] {}",
            v.name
        );
    }
}

/// A word's value is not a release axis: every vector holds unchanged at 8.4,
/// 8.5 and 9.1 as well.
#[test]
fn the_vectors_hold_at_every_release() {
    for v in VECTORS {
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V9_1] {
            if version < v.since {
                continue;
            }
            assert_eq!(
                vm_output(v.script, version),
                v.want,
                "[{version:?}] {}",
                v.name
            );
        }
    }
}

/// The table itself is pinned to C Tcl: every `want` must match what the
/// matching real tclsh prints. Skips silently per-binary when not installed
/// (CI / dev machines with `make ensure-test-deps` have both).
#[test]
fn vectors_match_real_tclsh() {
    let mut ran = 0;
    for v in VECTORS {
        for version in [TclVersion::V8_6, TclVersion::V9_0] {
            if let Some(got) = tclsh_output(version, v.script) {
                assert_eq!(got, v.want, "[tclsh {:?}] {}", version, v.name);
                ran += 1;
            }
        }
    }
    if ran == 0 {
        eprintln!("skipping: neither tclsh8.6 nor tclsh9.0 found");
    }
}
