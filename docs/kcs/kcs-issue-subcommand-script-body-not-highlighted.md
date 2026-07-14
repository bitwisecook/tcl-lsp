# KCS: A subcommand's script argument is not highlighted as a body

> **Audience:** Contributor
> **Type:** Issue

## Applies to

all-editors

## Symptom

A script argument to a subcommand-dispatch command renders as one opaque,
unhighlighted string — no keyword, comment, variable, or nested-command
colouring inside the braces — even though the identical script pasted at the
top level highlights normally.

Reported for `console eval { ... }` (issue #925): `console` and `eval`
highlight as a command, but everything inside the `{...}` stays plain text.
The same shape existed for `consoleinterp eval script` / `consoleinterp
record script` (used from inside a `console eval` body to reach back into the
attached interpreter) and for `ttk::treeview`/`ttk::notebook`'s `instate
statespec ?script?`.

## Operational context

Whether an argument *is* a script body isn't a text-shape question — the
[command registry](../design/compiler/command-registry.md) answers it via
`ArgRole::Body`. `collect_script`'s override pass calls
`CommandRegistry::arg_indices_for_role` for every command head; for a
subcommand-dispatch command it only consults the matched `SubCommand`'s
`arg_roles` / `arg_role_resolver` when the parent `CommandSpec` actually has a
non-empty `subcommands` table — `spec.subcommands.is_empty()` short-circuits
straight to the (irrelevant) top-level `arg_roles` first. A command modelled
as a flat `CommandSpec` — no `SubCommand` table at all, just a generic
`"console subcommand ?arg ...?"` synopsis string and empty `arg_roles` — can
never contribute a `Body` role for any of its subcommands, no matter how
their `detail`/`synopsis` text reads. `console` was exactly this: registered
via the shared flat-command builder in `tk_extra_cmds.rs` alongside genuinely
subcommand-less commands like `bindtags`. `consoleinterp` was not registered
at all. `ttk::treeview`/`ttk::notebook` *did* have a `SubCommand` table, but
their `instate` entry declared `arity: Arity::at_least(1)` (no upper bound)
and no `arg_roles`, even though its own `detail` text says "optionally
running a script".

## Decision rules / contracts

1. A subcommand whose manual page shows a `script` (or `body`) argument needs
   an explicit `arg_roles: &[(N, ArgRole::Body)]` (or `arg_role_resolver`) on
   that `SubCommand` entry — never inferred from arity or synopsis text.
   Modelled in
   [`rust/tcl-registry/src/commands/tk/console.rs`](../../rust/tcl-registry/src/commands/tk/console.rs),
   mirroring `interp eval`
   ([`rust/tcl-registry/src/commands/tcl/interp.rs`](../../rust/tcl-registry/src/commands/tcl/interp.rs)).
2. A command that *looks* flat (a generic `"cmd subcommand ?arg ...?"`
   synopsis, registered through a shared simple-command builder) may still
   dispatch subcommands with materially different argument shapes. Skimming
   the synopsis string is not a substitute for checking the real per-version
   manual page (or C source, for an internal command like `consoleinterp`)
   before deciding a command needs a `SubCommand` table — see
   `docs/design/compiler/command-registry.md` for the `arg_roles` /
   `arg_role_resolver` / `assigns_variable_at` priority order.
3. A subcommand that evaluates code in a *different* interpreter from the
   caller's (the Tk console has its own interpreter; `console eval` runs in
   it, `consoleinterp eval`/`record` cross back into the interpreter the
   console is attached to) is a cross-interpreter sink — `T105`, declared via
   `taint_interp_eval_subcommands` on the parent `CommandSpec` — not the
   same-interpreter `T100` sink `eval`/`uplevel` are (`Traits::TAINT_SINK`).
   Get the sink category right, not just the highlighting: both were wired up
   together in the `console`/`consoleinterp` fix.
4. Declaring a role correctly in the registry does not guarantee the LSP can
   reach it at every real call site. `ttk::treeview instate`/`ttk::notebook
   instate` are now modelled correctly, but this codebase has no
   widget-path-to-widget-type tracking, so a realistic `$w instate ... {...}`
   (where `$w` is a variable) never resolves to the `ttk::treeview`/
   `ttk::notebook` `SubCommand` table today — only a literal `ttk::treeview
   instate ...` head would. Fixing the registry data ahead of the resolution
   that would make it observable is still correct (the contract the registry
   makes to every consumer must be accurate regardless of which consumers
   currently exercise it), but do not claim a behavioural fix you cannot
   demonstrate — see the Test anchors below for how the two cases are
   verified differently.
5. This class of bug lives entirely in registry data. The recursion
   mechanism itself (`ArgOverride::BodyScript` inside `collect_script`,
   [`rust/tcl-lsp-core/src/semantic_tokens.rs`](../../rust/tcl-lsp-core/src/semantic_tokens.rs))
   is generic and already correct for every command that declares the role —
   do not special-case a command name in the walker to work around a missing
   registry entry.

## File-path anchors

- `rust/tcl-registry/src/commands/tk/console.rs` — `console` (`eval` / `hide`
  / `show` / `title`) and `consoleinterp` (`eval` / `record`) specs
- `rust/tcl-registry/src/commands/tk/ttk__treeview.rs`,
  `rust/tcl-registry/src/commands/tk/ttk__notebook.rs` — `instate` arity +
  `ArgRole::Body` fix
- `rust/tcl-registry/src/registry.rs` — `CommandRegistry::arg_indices_for_role`
  (the subcommand-table short-circuit)
- `rust/tcl-lsp-core/src/semantic_tokens.rs` — `collect_script`,
  `insert_role_overrides`, `ArgOverride::BodyScript`
- `rust/tcl-compiler/src/taint.rs` — `classify_network_interp_sinks` (T105,
  reads `taint_interp_eval_subcommands`)

## Failure modes

- A command registered through a shared "simple flat command" builder (see
  `tk_extra_cmds.rs`'s `cmd()` helper) silently drops any subcommand
  structure the real command has — the builder has no `subcommands` field to
  fill in, so nothing fails loudly; the command just never highlights or
  taint-classifies its body correctly.
- Setting `ArgRole::Body` without also checking whether the eval crosses an
  interpreter boundary misclassifies the taint sink category (`T100` vs
  `T105`).
- Declaring the role correctly but assuming that alone makes it observable at
  every call site overclaims the fix — check whether the LSP can actually
  resolve the call site's head to that spec (see rule 4 above).

## Triage checklist

1. Decode the semantic tokens for a minimal repro (`cmd sub {body}`) and
   confirm the body's inner tokens (a `$var`, a nested `[cmd]`) appear as
   their own token kinds rather than the whole `{...}` staying one `String`.
2. Check whether the command has a non-empty `subcommands` table at all — a
   flat `CommandSpec` can never contribute a subcommand-level `Body` role.
3. If it does, confirm the matched `SubCommand`'s `arg_roles` (or
   `arg_role_resolver`) actually marks the script argument's index.
4. If the script crosses into a different interpreter, confirm
   `taint_interp_eval_subcommands` lists the subcommand for `T105`, instead
   of (or in addition to) any `T100` classification.
5. Confirm the LSP can actually resolve a realistic call site's head to the
   spec you edited — a corrected `SubCommand` on a widget class is invisible
   until (if) widget-path → widget-type resolution exists.

## Test anchors

- `rust/tcl-lsp-core/src/semantic_tokens.rs` —
  `console_eval_body_recurses_into_script`,
  `consoleinterp_eval_and_record_bodies_recurse_into_script`
- `rust/tcl-registry/src/commands/tk/mod.rs` —
  `console_and_consoleinterp_eval_bodies_are_registered_correctly`,
  `ttk_instate_script_arg_is_a_body_with_tight_arity`
- `rust/tcl-compiler/src/taint.rs` —
  `t105_cross_interp_for_tainted_console_eval`,
  `t105_cross_interp_for_tainted_consoleinterp_eval_and_record`

## Related

- [KCS index](README.md)
- [array element not highlighted as variable](kcs-issue-array-element-not-highlighted-as-variable.md)
- [Semantic Tokens feature](features/kcs-feature-semantic-tokens.md)
- [Command registry design doc](../design/compiler/command-registry.md)
- [Glossary](../GLOSSARY.md)
