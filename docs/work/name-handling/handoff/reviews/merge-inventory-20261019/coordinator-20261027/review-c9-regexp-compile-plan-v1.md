# Review: C9 `TclCompileRegexpCmd` inline compile route (v1)

Reviewer: independent source review (read-only; no cargo run).
Scope: commit 11ff11c73 plus the uncommitted WIP in the working tree on `work`.
Date: 2026-10-29

## Verdict: CHANGES-REQUIRED

The selection, glob reduction and emission match C 9.0.4/9.1.0 on every case I ran
(about 80 cases). Two existing registry tests still assert the old "all versions Unknown"
result. The new 9.x shape path contradicts them, so they should fail. The in-flight WIP
that moves the `$`-split rule into shape classification also needs to be finished and
pinned.

## Findings

### F1 (blocking): stale shape-path tests contradict the new 9.x selection
- `rust/tcl-registry/src/native_compilation.rs:7846-7858`
  (`regexp_capture_and_option_forms_decline_native_compilation`) asserts
  `NativeCompilationSelection::Unknown` for `["pattern","text"]`, `["-nocase","pattern","text"]`
  and `["-n","--","pattern","text"]` across `TclVersion::ALL`.
- `rust/tcl-registry/src/native_compilation.rs:7877-7880`
  (`regexp_operand_layout_is_independent_of_unresolved_operation_selection`) asserts
  `Unknown` for two `Substituted` operands across `TclVersion::ALL`.
- For V9_0 and V9_1, `regexp_grammar` (`native_compilation.rs:5614-5646`), reached through
  `select_delegated_word_grammar` (:4433), returns `Inline` in both cases. C agrees:
  tclsh 9.0.4 gives `regexp $re $s` => `loadScalar1; loadScalar1; regexp +3`. The fix is to
  split the expectation by version: Unknown for 8.4–8.6 and Inline for 9.x.

### F2 (WIP, must land together): `$`/NUL split moved into the shape classifiers
- The working tree drops the split heuristic from `simple_word_bytes`
  (`native_regexp_compilation.rs:82-91`). It adds `splits_into_text_tokens`
  (`native_compilation.rs:~5935`) and uses it in `capture_word`
  (`native_compiler_words.rs:438`, gated on `NativeStringProtocol::C`) and in the IR
  classifier (`rust/tcl-compiler/src/registry_invocation.rs:5563-5577`, **not gated on
  protocol**). The semantics are right: a TEXT+TEXT word is non-SIMPLE but compile-time
  known, just like TEXT+BS. It now affects every consumer, though. Examples:
  `compiler_head` for an 8.4/8.5 head `a$` now gives Some(false), which matches C. On
  Jim, a head `a$` changes from Literal to BackslashLiteral and so from Some(true) to
  Some(false). Either gate the IR classifier on the C protocol as `capture_word` does, or
  justify the change for Jim.
- The oracle depends on this split: tclsh 9.0.4 gives `regexp -a$ $s` =>
  `push "-a$"; regexp +3`. The pattern is non-SIMPLE, so the leading-`-` guard is skipped.
  Add this case to the registry cases and the shape-path tests.
- `registry_invocation.rs` still carries `TEMP_OPT`/`TEMP-OPT` eprintln probes (reformatted
  in the WIP). This is not regexp code, but it should not merge.

### F3 (note): theoretical 9.0/9.1 divergence
In 9.0, `strncmp(str,"-nocase",len)` accepts a braced option `-nocase\0…`. In 9.1,
`IS_TOKEN_PREFIX` rejects it (size ≤ 7). Rust follows 9.1 for both versions (`regexp_option`,
:123-131). This is unreachable from Tcl_Obj source, because NUL is stored as C0 80, so it
needs no action.

### F4 (tests): add these oracle-pinned cases
`-nocasex`/`-N`/`-` (invoke), `-- -nocase abc $s` (invoke), `-noc\x61se` (BS option →
invoke), `-n$` (invoke), `{a\<nl>b}` (BackslashLiteral → `regexp +3`), `regexp $re abc`,
`{^$}` (streq ""), `{^a\*b$}` (strmatch, not exact), `{a.+b.+c}` (OVERCOMPLEX → regexp),
`{*}{a\.b}` (non-literal expansion → invokeExpanded), `-a$`.

## Checks
1. **Branches vs C.** These all match:
   - Option loop, `--` handling (`i++; break`), the 2-operand check, and the SIMPLE/`-`
     guard (`native_regexp_compilation.rs:178-208`).
   - `re_to_glob` (`tcl_syntax/src/glob.rs:286-379`), line by line against `TclReToGlob`
     (escapes, `anchorLeft` clearing on `.`/`\B`/`\\`/`\*`, `.*`/`.+` star accounting,
     `$` placement, `***=`).
   - STR_EQ only when exact and not nocase. STR_MATCH immediate is `nocase`. REGEXP
     immediate is 3, or 11 with nocase.
   - Pattern pushed before subject.
2. **Versions.** `modelled` covers V9_0 and V9_1 only. For 8.4–8.6 the shape path returns
   Generic only for option/arity rejects, where 8.4–8.6 C rejects the same way, and
   Unknown otherwise. `diff` of the 9.0/9.1 C function shows only macro refactors;
   9.1 tclsh output is identical apart from opcode spelling.
3. **No command-name matching** in the new registry module, codegen
   (`statements/native_string.rs`), preflight (`original_preparation.rs:267`) or runtime
   decline (`native_body_artifact.rs:1272`). Routing is by `NativeCompilationGrammar::Regexp`.
4. **Tests.** The 25 registry cases and 10 codegen cases agree with my tclsh 9.0.4 runs.
   They are not vacuous: they assert the matcher, bytes, immediates and stack order.
   F1 is the gap.
5. **Fail-closed.** Opaque words and projection failure give Unknown. An Inline selection
   without a recipe gives an `OperandGeometry` unavailable. The runtime native body
   declines Regexp.

## tclsh evidence (9.0.4; 9.1.0 identical modulo `push`/`push1` names)
Script: `/tmp/claude-0/-home-user-tcl-lsp/ecf365c5-6571-597f-97b7-b7fbb7d1acfb/scratchpad/rx.tcl`, plus `rx2.tcl` and `rx3.tcl` in the same folder.
```
regexp -no -- $re $s      => loadScalar1 ;loadScalar1 ;regexp +11
regexp -nocasex abc $s    => invokeStk1 4      regexp -N abc $s => invokeStk1 4
regexp -- -nocase abc $s  => invokeStk1 5      regexp -n$ abc $s => invokeStk1 4
regexp -noc\x61se abc $s  => invokeStk1 4      regexp abc $s m   => invokeStk1 4
regexp -nocase -- -- $s   => push "*--*"; strmatch +1
regexp {^abc$} $s => push "abc"; streq         -nocase {^abc$} => strmatch +1
regexp {^$} $s => push ""; streq               regexp {^a\*b$} $s => push "a\*b"; strmatch +0
regexp {a.+.*} $s => "*a?*" strmatch +0        regexp {a.+b.+c} $s => regexp +3
regexp {\Aabc} / {(?i)abc} / {\d} / {a*} / {[a]} / {a$b} / {**} => regexp +3
regexp ^abc$ $s / "a$" / a$: / -a$ / $$ => push literal; regexp +3
regexp $ $s => "*" strmatch +0                 regexp {*}{a$} $s => "*a" strmatch +0
regexp {a\<nl>b} $s => push "a b"; regexp +3   regexp {*}{a\.b} $s => invokeExpanded
regexp abc {*}$s => invokeExpanded             regexp -- $s / -nocase => invokeStk1
```
