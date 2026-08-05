# KCS: a script argument built with `list` is not analysed as a script

> **Audience:** Contributor
> **Type:** Issue

## Applies to

all-editors, analyser

## Symptom

A script argument written as a literal braced block behaves; the same script
*built* with `list` does not. Three things go missing, all in the same shape:

```tcl
proc f {disp} {
    uplevel #0 [list upvar #0 ::tk::Priv.$disp ::tk::Priv]
    variable ::tk::Priv
}
```

- `::tk::Priv` on the first line is painted as a plain `namespace` word,
  while the byte-identical text on the second line is a
  `variable [declaration]`.
- Nothing declares `::tk::Priv` in the frame `uplevel #0` names, so
  completion and go-to-definition have no cell to answer with.
- The same gap swallows reads:

```tcl
proc ::tk::SourceLibFile {file} {
    namespace eval :: [list source [file join $::tk_library $file.tcl]]
}
```

  `$file` is the proc's own parameter, yet hover, go-to-definition, and Find
  All References all answered nothing on it.

Both lines are verbatim from Tk's own `library/tk.tcl`. Reported as issue
#1138 (split out of the issue #923 differential audit, findings idx 100 and
idx 102).

## Operational context

`list` is not a dynamic construct. It packs its already-substituted arguments
into exactly one well-formed command, and Tcl then evaluates that command
deterministically. Verified on tclsh 9.0.4 and 8.6.16, byte-identical output
for all three spellings:

```tcl
set g ORIGINAL
proc direct  {} { upvar #0 g l ; set l DIRECT }
proc braced  {} { uplevel #0 {upvar #0 g l ; set l BRACED} }
proc built   {} { uplevel #0 [list upvar #0 g l] ; uplevel #0 {set l BUILT} }
direct ; puts $g      ;# DIRECT
braced ; puts $g      ;# BRACED
built  ; puts $g      ;# BUILT
```

The analyser and the semantic-token walker both keyed on "the body word is a
literal `{…}` block":

- `Analyser::analyse_body` returned early unless the body token was
  `TokenType::Str`, so nothing inside a built body was ever dispatched.
- `handle_uplevel_command` applied the same gate, so no uplevel-frame scope
  even opened.
- The semantic-token walker recursed into the `[…]` and classified `list
  upvar #0 …` as a call to `list` — which it is, textually — so the words
  after it were arguments of `list`, not of `upvar`.

## Decision rules / contracts

1. **One predicate, two consumers.**
   [`tcl_compiler::script_arg::list_quoted_script_command`](../../rust/tcl-compiler/src/script_arg.rs)
   is the only answer to "is this `[…]` argument a statically known command,
   and which one?". The analyser's body gate and the LSP's declaration
   highlighting both ask it, so a shape that navigates cannot fail to
   highlight, and neither can drift from the other. It is built on the
   existing `Traits::BUILDS_COMMAND_PREFIX` machinery
   (`signature_scan::command_prefix::list_build_is_literal`), not on a third
   parallel unwrap.
2. **Abstain on doubt.** Only a `[…]` whose sole inner command is a call to a
   `BUILDS_COMMAND_PREFIX` command with a literal, resolvable first word
   resolves. `[list $cb …]`, `[$build …]`, `[list a; list b]`,
   `[concat upvar …]`, and a widget-path head all keep the opaque-barrier
   behaviour they had.
3. **`list` invokes nothing.** The highlighting side is additionally gated on
   the enclosing argument's role being one the registry marks
   `Body` / `LambdaLiteral` / `CommandPrefix` (the `deferred_role` flag —
   see [the `apply` lambda note](kcs-issue-apply-lambda-body-not-highlighted-via-list-quoting.md),
   decision rule 3). `set x [list upvar 1 a b]` builds a four-element list
   and declares nothing; tclsh agrees (`info exists b` → 0).
4. **Each word of the built command is a pre-substituted value.** `[list set
   v $x]` builds `set v <the value of x>` — the `$x` is read in the
   *building* frame, before the script runs. Two consequences:
   - The read belongs to the enclosing scope, where the command-substitution
     walk already records it. The built command is walked in the target
     scope, and `VarDef::push_reference` drops the resulting repeat.
   - A `namespace eval NS […]` scope must **not** own the substitution's
     bytes. It used to (`Scope::body_span` was set from the body token
     unconditionally), which made the scope-chain lookup stop at the
     namespace frame and answer nothing for `$file` above. `body_span` is now
     set only for a literal braced body, which is the only body that really
     runs in that frame.
5. A word that is not statically knowable inside the built command
   (`::tk::Priv.$disp`) stays a dynamic word, and the usual dynamic-word
   guards skip it — the fix widens *which* commands are seen, never how
   permissively their words are read.

## File-path anchors

- `rust/tcl-compiler/src/script_arg.rs` — `list_quoted_script_command`,
  `list_build_effective_command`, and the shape's TP/TN unit tests
- `rust/tcl-compiler/src/signature_scan/command_prefix.rs` —
  `list_build_is_literal`, the shared literal-head guard
- `rust/tcl-compiler/src/analyser/commands.rs` — `analyse_list_quoted_body`,
  the body gate's non-`Str` arm
- `rust/tcl-compiler/src/analyser/handlers.rs` — `handle_uplevel_command`'s
  gate; `handle_namespace_eval_command`'s `body_span` rule
- `rust/tcl-lsp-core/src/semantic_tokens.rs` —
  `merge_list_quoted_command_overrides`

## Failure modes

- A built body is walked as **one** command. `[list a b]` cannot express two
  commands, so this costs nothing; `[eval $script]` and friends stay opaque.
- The built command runs in the target frame, but its words were substituted
  in the building frame. A consumer that reads a built word as if it were
  source code evaluated in the target frame will be wrong; read it as a
  value.
- Renaming or aliasing `list` (`interp alias {} mylist {} list`) defeats the
  recogniser, exactly as it defeats every other registry-head lookup — see
  the "Known limitations" note in
  [the command registry design doc](../design/compiler/command-registry.md#known-limitations)
  (issue #1002).

## Triage checklist

1. Write the same command three ways — direct, braced `uplevel {…}`, and
   `uplevel [list …]` — and confirm the analyser and the semantic tokens
   agree on all three.
2. Check `list_quoted_script_command` resolves the shape at all: a dynamic
   head or a multi-command substitution is *meant* to return `None`.
3. For the highlighting half, confirm the enclosing argument's role is one of
   `Body` / `LambdaLiteral` / `CommandPrefix` — a value slot is inert by
   design.
4. For a missing *read*, check whether an enclosing scope has claimed the
   substitution's bytes via `body_span`: a `[…]` body is evaluated in the
   calling frame.

## Test anchors

- `rust/tcl-compiler/src/script_arg.rs` — `mod tests` (TP + four TN shapes)
- `rust/tcl-compiler/tests/analyser.rs` —
  `list_quoted_script_arguments::*` (uplevel frame, namespace scope, dynamic
  abstention, and the `::tk::SourceLibFile` read/`body_span` pair)
- `rust/tcl-lsp-core/tests/semantic_tokens.rs` —
  `tp_a_list_built_upvar_declares_its_local_like_the_literal_spelling`,
  `fp_a_list_in_a_value_slot_is_still_inert_data`,
  `tn_a_dynamic_list_head_keeps_todays_behaviour`
- `rust/tcl-lsp-core/tests/references_rename.rs` —
  `a_parameter_read_inside_a_list_built_body_navigates`
- `rust/tcl-lsp-server/tests/e2e/semantic_tokens.rs` —
  `test_list_built_upvar_declares_its_local_like_the_literal_spelling`
- `rust/tcl-lsp-server/tests/e2e/references.rs` —
  `references_reach_a_parameter_read_inside_a_list_built_namespace_body`

## Related

- [KCS index](README.md)
- [`apply`'s lambda body is not highlighted when reached through `[list ...]`](kcs-issue-apply-lambda-body-not-highlighted-via-list-quoting.md)
- [Find References feature](features/kcs-feature-references.md)
- [Command registry design doc](../design/compiler/command-registry.md)
- [Glossary](../GLOSSARY.md)
