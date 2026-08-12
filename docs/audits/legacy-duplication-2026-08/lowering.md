# Lowering / codegen / front-end duplication audit

Scope: `rust/tcl-compiler/src/{parsing,segmenter.rs,cfg_builder,cfg*.rs,ssa.rs,ir*.rs,
executable_ir.rs,place*.rs,compilation_unit.rs,lowering*,shimmer,codegen,
representation_plan.rs,slot_allocation.rs,target_contract.rs,backend_registry.rs,
signature_scan,tcl_expr_eval.rs,text.rs}` plus `rust/tcl-syntax`, `rust/tcl-lexer`.

(Note: the red-green CST is `tcl-compiler/src/parsing/syntax/{green,red,build,segment,
descend}.rs`, not the `rust/tcl-syntax` crate — that crate holds the Tcl value grammars:
list, number, glob, naming, backslash.)

Every finding below was verified by reading both sides **and** by running the built
`tcl` CLI (`target/debug/tcl`, toolchain 1.97.0) against a reproducer. Commands and
observed output are quoted inline.

Clean subsystems (checked, nothing to report): the segmenter is fully migrated onto the
red-green CST — `segment_commands_local` (`segmenter.rs:807`) is a thin wrapper over
`parsing::syntax::build::build_document` + `segment::segments_from_document`, with the
old token loop retired to a differential-test oracle; `slot_allocation.rs` is documented
as deliberately unwired and is; `shimmer/span.rs` reads spans straight off the IR;
`executable_ir.rs` has real consumers (`world_state_ssa`, `semantic_analysis`, `gvn`,
`codegen/wasm/semantic_plan`).

---

## F1: `while` / `for` / `try` / `foreach` / `dict` lower a substituted body word as a static script, so the IR span runs past end-of-source and `tcl compwasm` panics

**Confidence:** high
**Category:** duplicated-path

**The modern path:** the family's shared "is this word a literal script?" predicates live
in `lowering/mod.rs:200` (`seg_word_is_static_braced`) and `lowering/mod.rs:210`
(`seg_word_is_static_literal`) — both gate on the *token kind* (`Str` / `Esc`), not just
on single-token-ness. `lower_if` uses it (`lowering/structured.rs:266`), and
`lower_catch` (`lowering/structured.rs:610-619`) and `lower_foreach_line`
(`lowering/structured.rs:563-565`) hand-inline the equivalent `TokenType::Str` check,
with `lower_catch`'s comment spelling out exactly why: *"Without the kind check,
`catch $cmd res` would be compiled as 'call the proc named by `$cmd`' — wrong."*

**The legacy/duplicate path:** five sibling lowerers in the same file never got the
check and gate on `arg_single` alone:

- `lower_while` — `lowering/structured.rs:383` (`if !(arg_single[0] && arg_single[1])`)
- `lower_for` — `lowering/structured.rs:332`
- `lower_foreach` / `lower_lmap` — `lowering/structured.rs:432`
- `lower_try` — `lowering/structured.rs:656`
- `lower_dict` — `lowering/structured.rs:986`

All five then call `lower_body_from_tok` (`lowering/structured.rs:1075-1086`), which
rebases the *segmenter-reconstructed* word text at `tok.span.start() + content_offset`.
For `$body` the reconstructed text is `${body}` (7 bytes) over a 5-byte source word, so
every span in the resulting `Script` lands 2 bytes too far right — past the end of the
buffer for a trailing body.

**Why it matters:** a hard crash on a production path.

```
$ printf 'while {1} $body\n' > t.tcl
$ target/debug/tcl compwasm t.tcl -o out.wasm
thread '<unnamed>' panicked at rust/tcl-compiler/src/codegen/structured.rs:284:12:
end byte index 17 is out of bounds for string of length 15
  ... tcl_compiler::codegen::structured::slice
  ... tcl_compiler::codegen::wasm::backend::WasmEmitter
```

`for {} {1} {} $body` panics identically. `catch $body` and `if {1} $body` correctly
barrier (`barrier catch with dynamic body`, `barrier if with non-literal body`) — the
divergence is visible side-by-side in one `tcl explore --show ir` run. Where it does not
crash it silently mis-compiles: `try $body` produces an `IRTry` whose body Script is the
*text* `${body}` parsed as a script (a call to a command literally named `${body}`), and
`foreach x $l $body` produces an `IRForeach` with the same fabrication.

**What cleanup looks like:** replace the five ad-hoc `arg_single` gates with the existing
shared predicate (`seg_word_is_static_braced` for the brace-only bodies, matching
`lower_catch`; `seg_word_is_static_literal` where a bareword body is also legal, matching
`lower_if`), and delete `lower_catch`/`lower_foreach_line`'s hand-inlined copies so there
is one gate. Independently, `codegen/structured.rs:283`'s `slice` should clamp/`get`
rather than index, so a bad IR span degrades instead of aborting the process.

**Scale:** ~5 one-line guard changes plus one bounds-safe `slice`; the barrier reasons
already exist, so no new IR shapes.

---

## F2: the structured WASM walk re-derives clause text by slicing source and hand-stripping the closer, truncating any condition or step clause whose content ends in `}` or `"`

**Confidence:** high
**Category:** range-rederivation

**The modern path:** the IR already carries the parsed and the anchored forms of every
clause. `Statement::While` (`ir.rs:1067-1086`) and `Statement::For` (`ir.rs:1033-1060`)
each hold `condition: ExprNode`, `condition_span: Span`, **and** `condition_base:
Option<u32>` — "absolute source offset of the condition text" — computed in lowering by
`word_content_base` (`lowering/structured.rs:404`, `lowering/structured.rs:358`), the
helper documented at `lowering_hooks.rs:201` as the one place the opening-delimiter width
is recovered without guessing. `raw_args` carries the de-braced word text directly.

**The legacy/duplicate path:** `codegen/structured.rs:294-303` ignores all of it:

```rust
fn condition_text(source: &str, span: Span) -> &str {
    let s = slice(source, span).trim();
    if let Some(inner) = s.strip_prefix('{') {
        return inner.strip_suffix('}').unwrap_or(inner).trim();
    }
    ...
```

Called from the `While` arm (`codegen/structured.rs:139`), both `For` arms
(`codegen/structured.rs:153-154`) and `emit_if` (`codegen/structured.rs:218`). Its own
doc-comment states that the token span *excludes* the closing brace — so the
`strip_suffix('}')` can only ever remove a byte of real content. This is precisely the
`end.offset + 1`-class re-derivation AGENTS.md's "Word-token closing delimiters" section
forbids, in the opposite direction.

**Why it matters:** the emitted WASM evaluates a truncated expression / script.

```
$ printf 'while {${x}} {puts hi}\n' > g.tcl && target/debug/tcl compwasm g.tcl -o g.wasm
$ od -c g.wasm | tail -3
... 003   $   {   x  \0 ...      <- data segment holds "${x", not "${x}"
```

Same for `if {${x}} {puts a}`, and for a `for` step clause: `for {set i 0} {$i<2}
{set x ${i}} {puts a}` embeds `set x ${i` (9 bytes) in the module. At runtime that is a
parse error ("missing close-brace") on code the user wrote correctly. Any braced clause
whose last inner character is `}` — `${name}`, `[expr {$a}]`-free arithmetic on a braced
var, a nested dict literal — is affected.

**What cleanup looks like:** drive the emitter from `condition_base` +
`raw_args`/`ExprNode` instead of re-slicing: the lowerer already produced the exact text
it parsed, and `condition_base` maps it back to absolute source when a verbatim mapping
exists. Failing that, use `tcl_lexer::word_span` / the token's `content_offset` (the one
authoritative "whole written word" helper, `tcl-lexer/src/ranges.rs:215`) rather than
character stripping. Delete `condition_text` once the callers take the IR fact.

Note the module doc at `codegen/structured.rs:50-51` still claims *"Not currently
reachable from any wired-up production path (`structured::walk` has no caller yet)"* —
the panic backtrace in F1 shows `codegen::wasm::backend::codegen` calling it. That stale
claim is why neither `slice` nor `condition_text` was ever hardened.

**Scale:** one function deleted, four call sites re-pointed at existing IR fields; plus
one doc correction.

---

## F3: the `{*}`-expansion barrier for structured commands is keyed on a hardcoded command-name list that has drifted from the registry hook set that actually dispatches structured lowering

**Confidence:** high
**Category:** stale-consumer

**The modern path:** structured lowering is dispatched purely from the registry's typed
`LoweringHookId` — `try_dispatch_structured_hook` (`lowering/mod.rs:1166-1326`) resolves
via `registry.resolve_invocation(...).semantics.lowering_hook` and covers `Proc`, `When`,
`NamespaceEval`, `If`, `Switch`, `For`, `While`, `Foreach`, `Lmap`, `ForeachLine`,
`Catch`, `Try`, `Dict`, `Eval`, `Uplevel`, `Apply`, `ArrayFor`. Its own doc-comment sells
the migration: *"Dispatching through the hook ID picks up two benefits over a bare name
match."* The non-structured hooks each ask `has_expansion(cmd)` (`lowering_hooks.rs:185`)
themselves.

**The legacy/duplicate path:** the expansion guard for the structured family is still a
command-string classifier — `structured_expand_barrier` (`lowering/mod.rs:1105-1140`):

```rust
let structured = matches!(cmd_name,
    "proc" | "when" | "namespace" | "if" | "switch" | "for" | "while"
    | "foreach" | "foreach_in_collection");
```

called at `lowering/mod.rs:1400`, immediately before the hook dispatch. Nine names
against seventeen hook IDs. This is also a direct violation of AGENTS.md's "the registry
is the source of truth — no per-command logic elsewhere".

**Why it matters:** every structured form missing from the list commits to the
*un-expanded* argv shape, which is exactly what the barrier exists to prevent. Verified
with `tcl explore --show ir`:

| source | IR |
|---|---|
| `foreach i {*}$spec {puts $i}` | `barrier foreach with argument expansion` (correct) |
| `lmap i {*}$spec {puts $i}` | `IRForeach` — one iterator over the literal word `${spec}` |
| `dict for {k v} {*}$rest {puts $k}` | `IRForeach` |
| `array for {k v} {*}$rest {puts $k}` | `IRForeach` |
| `foreachLine ln {*}$rest {puts $ln}` | `IRForeach` |
| `catch {body} {*}$rest` | `IRCatch` (result var taken from the un-expanded word) |

Two commands with identical shape and the same lowering function (`lower_foreach`) get
opposite treatment. User-visible: `set spec {{1 2} j {3 4}}` + `lmap i {*}$spec {puts
"$i $j"}` reports `W210 Variable 'j' is read before it is set`, although the expansion
makes `j` a loop variable. Downstream, SCCP/loop passes reason about an iterator count
and a loop-variable set that the program does not have.

**What cleanup looks like:** delete the name list and fold the expansion gate into
`try_dispatch_structured_hook`: after resolving `hook`, if `seg.expand_word` has any flag
set and the hook is one whose lowering commits to a positional argv shape, return the
barrier. That makes the set registry-derived and impossible to drift when a new
structured hook ID is added. `"foreach_in_collection"` in the current list is dead weight
— it resolves to `LoweringHookId::Foreach` through its spec anyway.

**Scale:** delete ~20 lines, add one guard inside the hook dispatcher; a handful of new
barrier expectations in tests.

---

## F4: the issue-#1325 body-rebase guard was adopted by the analyser only; the lowerer — the producer of the IR spans codegen slices with — never got it

**Confidence:** medium
**Category:** half-ported

**The modern path:** `segmenter::contiguous_prefix` (`segmenter.rs:376-394`) and
`segmenter::body_text_in_region` (`segmenter.rs:411-421`) exist specifically because
rebasing a body word's spans by `tok.span.start() + content_offset` is only truthful
while the word's *value* is a verbatim slice of the source. Their doc-comment names the
real incident: a compound `{body}x` word slid every token one byte left, and 40
zero-width spaces in a real iRule produced an offset inside a UTF-8 sequence that
panicked the first consumer to slice the source (issue #1325). The analyser's
`analyse_body` uses it before segmenting a body (`analyser/commands.rs:220-239`).

**The legacy/duplicate path:** `Lowerer::lower_body_from_tok`
(`lowering/structured.rs:1075-1086`) performs the same rebase with no guard —

```rust
let offset = tok.span.start() + u32::from(tok.content_offset);
self.lower_body(text, offset, namespace)
```

— and `lower_body_inner` (`lowering/mod.rs:945-952`) feeds that straight to
`segment_commands_with_offset_and_config`. `body_text_in_region` / `contiguous_prefix`
have exactly one production caller in the workspace, the analyser one above (everything
else is `tests/span_char_boundaries.rs`).

**Why it matters:** it is the second missing line of defence behind F1 — with the guard
in place, `while {1} $body`'s 7-byte `${body}` against a 5-byte source region would be
clamped instead of producing spans past end-of-source, and the WASM emitter would not
abort. More generally, IR spans are the contract every downstream consumer trusts
(AGENTS.md: "Trust it rather than re-deriving"), and the analyser side of the pipeline
currently produces trustworthy ones while the lowering side does not.

**What cleanup looks like:** give `Lowerer` the document source (it already receives it
in `lower`/`lower_into_script` but does not retain it) and route `lower_body_from_tok`
through `body_text_in_region(source, base, tok.span.end(), text)` the way
`analyser::commands` does. Add a `debug_assert` in `Script::from_statements` (or in
`lower_segmented`) that no emitted span exceeds the document length, so a future producer
cannot regress this silently.

**Scale:** one field on `Lowerer`, one call-site change, one assertion; the helper is
already written and tested.

---

## F5: `ir_helpers::tokenise_command_words` is a second, dialect-blind command/word tokenisation loop feeding the SCCP dynamic-name barrier

**Confidence:** medium
**Category:** duplicated-path

**The modern path:** `segment_commands_with_offset_and_config`
(`segmenter.rs:328-341`) is the single command/word splitter, CST-backed since the
migration (`segmenter.rs:807-823`) and *dialect-aware*: it threads
`LexerConfig::for_dialect`, which controls `{*}` expansion (off for Tcl 8.4 and iRules)
and the iRules `}{` ghost separator. Every body re-segmentation in the lowerer threads
`self.config` for exactly this reason (`lowering/mod.rs:918`, `lowering/mod.rs:946`; the
field at `lowering/mod.rs:760` and its doc spell out the contract). `SegmentedCommand` already
exposes `texts`, `argv` kinds, `single_token_word` and `expand_word`.

**The legacy/duplicate path:** `ir_helpers::tokenise_command_words`
(`ir_helpers.rs:587-630`) re-implements the loop by hand — `Lexer::new(source)` with
`LexerConfig::default()`, its own `prev_is_sep` accumulator, its own word-concatenation
rule, and its own `CommandWord { text, raw, substituted, braced_literal }` record whose
doc (`ir_helpers.rs:571-577`) concedes that `raw` reads back as `{s` for a braced word.
Its one caller is `dynamic_names::scan_script_text` (`dynamic_names.rs:542`), which feeds
`scan_command` and therefore `DynamicNameBarrier` — consumed by SCCP
(`sccp.rs:1061`) and stored on the CFG (`cfg.rs:240`) and compilation unit
(`compilation_unit.rs:630`).

**Why it matters:** the barrier is an *optimisation-soundness* fact ("does anything in
here write through a computed variable name?"). Deriving it from a differently-configured
tokenisation than the one that built the IR means the two disagree on word boundaries for
any non-default dialect: under `f5-irules` the segmenter splits `cmd {a}{b}` into three
words and this loop into two; under `tcl8.4` the segmenter treats `{*}` as a literal
word and this loop as an expansion marker. A word-boundary disagreement changes which
argument index `scan_command` inspects, so a dynamic write can fail to raise the barrier
and SCCP can then propagate a constant across it.

**What cleanup looks like:** replace the body of `tokenise_command_words` with a call to
`segment_commands_with_offset_and_config(text, 0, config)` and map each
`SegmentedCommand` to the four facts `dynamic_names` wants — `text` from
`SegmentedCommand::texts`, `braced_literal` from `argv[i].kind == Str && single_token_word[i]`,
`substituted` from the word's `WordExpr` (`ir.rs:437`, which already distinguishes
`Variable` / `CommandSubstitution` / `Template`), and the raw spelling from the word
span. Thread the document's `LexerConfig` into `dynamic_name_barrier` so the barrier is
computed under the same dialect as the IR.

**Scale:** ~40 lines deleted, one config parameter threaded from `compilation_unit`
through `dynamic_names`.
