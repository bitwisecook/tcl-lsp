# Analyser / semantic-model layer — legacy duplication audit

Branch `claude/legacy-code-duplication-audit-edm0bo`. Scope: `rust/tcl-compiler/src/analyser/**`
plus the listed semantic-model modules, audited against `rust/tcl-registry/src/`.

Findings are ranked strongest-first. Line numbers are as of the audited tree.

---

## F1: `switch` clause-list layout is hand-rolled in three analyser sites that all mis-parse value-taking options, while `CaseListSpec::invocation` already answers it

**Confidence:** high
**Category:** half-ported

**Where the knowledge lives now:**
- `rust/tcl-compiler/src/analyser/commands.rs:4655` — `let switch_list_idx = if name == "switch" { switch_list_body_index(&args) }`, gated on the literal command name.
- `rust/tcl-compiler/src/analyser/commands.rs:4856-4870` — `switch_list_body_index`: an option skip that advances **one** word per `-flag`, then assumes the next word is the subject.
- `rust/tcl-compiler/src/analyser/commands.rs:4904-4911` — `switch_arm_bodies`: the identical broken skip, consumed by `tcl_lsp_core::references::scan_my_method_region`.
- `rust/tcl-compiler/src/analyser/diagnostics/usage.rs:718-733` — `emit_w106_unbraced_switch_body`: a third copy of the same skip, plus its own `-regexp` sniff.

**Where it should live / what already exists:**
- `rust/tcl-registry/src/spec.rs:390-490` — `CaseListSpec::invocation()` returns `CaseInvocation { subject_index, clause_list_index, inline_clause_start, mode, nocase }`. It consumes value-taking options via `OptionSpec::value_word_count`, implements the Tcl 8.5+ two-argument exception, validates pair parity and the `fallthrough_body` marker.
- `rust/tcl-registry/src/registry.rs:1721-1734` — `CommandRegistry::case_invocation(name, args, dialect)`, the dialect-aware entry point.
- `rust/tcl-registry/src/commands/tcl/switch_.rs:258` — `case_list: Some(&CaseListSpec::SWITCH)`.
- The migrated twin: `rust/tcl-compiler/src/analyser/handlers.rs:4258-4262` — `handle_switch_command` already calls `registry.case_invocation(...)` and drives both forms off `invocation.clause_list_index` / `inline_clause_start`, naming no command.

**Drift evidence:**
1. `switch -matchvar m -- $s {a {puts A} b {puts B}}` (args = `["-matchvar","m","--","$s","{…}"]`).
   `switch_list_body_index` breaks its option loop on `m` (which does not start with `-`), then treats `m`
   as the subject, computes `i == 2`, finds `2 != len-1 == 4`, and returns `None`. The registry's
   `CaseListSpec::invocation` consumes `-matchvar m` as an option+value and answers `clause_list_index = 4`.
   With `None` returned, `commands.rs:4666` descends the clause list as a **plain script**, so `a` and `b`
   are read as command heads. `switch_arm_bodies` and `emit_w106_unbraced_switch_body` fail the same way.
   The registry's own `switch_arg_roles` (`rust/tcl-registry/src/commands/tcl/switch_.rs:57-99`) *does*
   handle the value options (via its private `SWITCH_VALUE_OPTIONS` at line 50), so it still reports the
   clause list as `ArgRole::Body` — the generic body descent walks it while the switch-aware path declines.
2. `switch_arm_bodies`' doc comment (`commands.rs:4884-4889`) claims it "matches
   `Analyser::handle_switch_command`'s own form-1/form-2 branching exactly … so the two can never
   disagree about where an arm's body starts and ends." That is no longer true: `handle_switch_command`
   was migrated to `case_invocation`, these were not.
3. Six other specs carry `case_list` — `rust/tcl-registry/src/commands/expect/{expect_cmd,expect_after,expect_before,expect_user,expect_tty,expect_background}.rs`
   (`CaseListSpec::EXPECT`). `tcl-lsp-core` handles them generically (folding.rs:441, semantic_tokens.rs:3376,
   references.rs:3053, minify.rs:37), but every analyser site above is `switch`-only, so Expect clause
   lists get no clause-aware analysis at all.

**Why it matters:** with a `-matchvar`/`-indexvar` switch (the standard `-regexp` capture idiom), the
W123 unresolved-command pass reads every arm *pattern* as a command head and emits a spurious
"unknown command" for each one; arm bodies are simultaneously walked in the wrong structural context.
`switch_arm_bodies`' breakage silently drops `my <method>` call sites inside such a switch from
`textDocument/references`. W106 (unbraced switch body) stops firing on the same calls.

**What cleanup looks like:** delete `switch_list_body_index` and rewrite `switch_arm_bodies` and
`emit_w106_unbraced_switch_body` on `registry.case_invocation(cmd_name, &args, mask)`, keying the
clause-list branch on `invocation.clause_list_index` and the inline branch on `inline_clause_start`,
and taking `-regexp`-ness from `invocation.mode == CaseMatchMode::Regexp` rather than a literal flag
scan. Drop the `name == "switch"` gate at `commands.rs:4655` in favour of
`registry.get(name).and_then(|s| s.case_list).is_some()`. Then stamp
`AnalyserHookId::Switch` onto the six `expect*` specs — `handle_switch_command`'s body is already
command-agnostic, so nothing else changes.

**Scale:** ~150 lines removed across three call sites, one hook stamp per Expect spec, plus tests
pinning `switch -matchvar m -- $s {…}` and one Expect clause list.

---

## F2: `upvar`'s level word is detected by sniffing the word's text in two analyser sites, contradicting the registry's `FrameLevelWord::ArityParity` rule

**Confidence:** high
**Category:** stale-consumer

**Where the knowledge lives now:**
- `rust/tcl-compiler/src/var_scoping.rs:310-319` — `looks_like_level(head)`: true for a decimal integer,
  an optionally `-`-prefixed integer, or `#<digits>`. Consumed at `var_scoping.rs:133` and `:180`.
- `rust/tcl-compiler/src/analyser/diagnostics/usage.rs:1554-1568` — `upvar_local_name_positions`: a
  *second, differently-spelled* text sniff (`head.starts_with('#') || all-digits after stripping every
  leading '-')`, reached from `Analyser::variable_name_positions` (`usage.rs:836`).

**Where it should live / what already exists:**
- `rust/tcl-registry/src/frame_effect.rs:234-253` — `FrameLevelWord::ArityParity`, with the pinned
  tclsh 9.0.4 / 8.6.14 table and the explicit warning: *"A text-sniffing consumer gets the third and
  fourth rows backwards: it drops a real binding for `upvar $lvl a b` (the commonest by-reference idiom
  of all) and invents a level for `upvar 1 b`."*
- `rust/tcl-registry/src/frame_effect.rs:344-385` — `FrameEffectSpec::level_word_len` / `resolve`.
- `rust/tcl-registry/src/commands/tcl/upvar_.rs:187` — `frame_effect: Some(UPVAR_FRAME_EFFECT)`.
- The migrated twin: `rust/tcl-compiler/src/analyser/param_traits.rs:938-961` — `upvar_level_and_pairs`
  queries `registry.frame_effect("upvar")`, and its doc says the rule "lives in the registry … queried
  here through the spec rather than re-derived, **so this stays the one description of `upvar`'s
  shape** (issue #1069)". Other migrated consumers: `cfg_builder/upvar_info.rs:501`,
  `dynamic_names.rs:658`, `tcl-lsp-core/src/semantic_tokens.rs:1669`.

**Drift evidence:** for `upvar $lvl a b` (3 words ⇒ parity says `$lvl` is the level, pair is `(a, b)`,
local is `b` at index 2):
- `upvar_local_declaration_indices("upvar", ["$lvl","a","b"])` — `looks_like_level("$lvl")` is false,
  offset 0, pair `("$lvl","a")` is skipped for its `$` source, loop ends. Returns `[]` — the `b`
  binding is lost entirely.
- `upvar_local_alias_indices` on the same input returns `[1]` — it declares **`a`** the local alias,
  which is the wrong word.
- `upvar_local_name_positions(["$lvl","a","b"])` returns `[1]` — also `a`.
For `upvar 1 b` (2 words ⇒ parity says no level, `1` is the *otherVar*, `b` is the local at index 1),
`looks_like_level("1")` is true, offset 1, no complete pair remains, so the result is `[]`.
Both rows are verbatim the two the registry doc names as the ones text sniffing gets backwards.
`frame_effect.rs:36-38` records that three copies of the level rule had already drifted before the
descriptor existed, "two of them wrong" — two are still here.

**Why it matters:** `var_scoping`'s helpers are the shared home for scope-alias declarations. Wrong
answers propagate to `textDocument/declaration` (`tcl-lsp-core/src/declaration.rs:39`, which
go-to-declarations `a` instead of `b`), the memory-SSA alias detector
(`var_escape/handlers.rs:186`, `var_escape/cfg_propagation/handlers.rs:184`), `place_bridge.rs:156`,
`cfg_builder/global_write_info.rs:388`, and dead-store elimination
(`optimiser/elimination.rs:1290`). A missed `upvar $lvl a b` alias means the optimiser does not know
`b` is observable in another frame — the same soundness shape as PR #1371. W212's name-position check
flags the wrong argument.

**What cleanup looks like:** delete `looks_like_level` and `upvar_local_name_positions`; have
`upvar_local_declaration_indices` / `upvar_local_alias_indices` take the registry (they already take
it indirectly through `scope_alias_*`) and derive the offset from
`registry.frame_effect("upvar").level_word_len(&refs)`, exactly as `param_traits::upvar_level_and_pairs`
does. `var_escape/handlers.rs:168` and `cfg_propagation/handlers.rs:172` then drop their
`looks_like_level` imports.

**Scale:** ~60 lines deleted, three call sites rewritten, plus fixing the `var_scoping.rs:548-559`
and `tests/alias_scoping.rs:444+` tests that currently pin the wrong behaviour.

---

## F3: `RepeatedArgLayout::optional_leading_word` was built for `upvar` and is set by no spec; `var_scoping` still hand-rolls all four alias layouts the registry already declares

**Confidence:** high
**Category:** half-ported

**Where the knowledge lives now:**
`rust/tcl-compiler/src/var_scoping.rs:248-262` and `:285-301` — after a registry-driven recognition
step (`is_scope_alias_call`, `var_scoping.rs:210-227`, which correctly reads `Traits::CREATES_SCOPE_ALIAS`
and `SubCommand::creates_scope_alias`), the *layout* is dispatched by a literal name match:

```rust
match canonical {
    "global"   => global_declaration_indices(args),
    "variable" => variable_declaration_indices(args),
    "upvar" | "namespace" | "namespace upvar" => upvar_local_alias_indices(canonical, args),
    _ => { /* registry ArgRole::VarWrite query */ }
}
```

The three hand-rolled parsers are `var_scoping.rs:41-47`, `:59-69`, `:119-154` / `:167-197`.

**Where it should live / what already exists:**
- `rust/tcl-registry/src/repeated.rs` — the whole module exists for exactly this family. Its header
  (lines 19-38) names `global a b c`, `variable n v n v`, `namespace upvar NS o l o l` and records
  that "every LSP consumer that needed one of these layouts therefore re-derived it by hand from the
  command's *name*" (issue #1185).
- Already declared: `global_.rs:102` + `:133` (`RepeatedArgLayout::every(VarWrite, 0)`),
  `variable_.rs:164` (`strided(VarWrite, 0, 2)`), `namespace_.rs:1056`
  (`strided(VarWrite, 2, 2)` for `namespace upvar`).
- `RepeatedArgLayout` feeds `CommandRegistry::arg_indices_for_role`, which the `_` arm above
  already uses for `my variable`.

**Drift evidence:** `rust/tcl-registry/src/repeated.rs:62-71` documents `optional_leading_word`
with `upvar`'s `?level?` as its sole motivating case, and `repeated.rs:193-206`
(`optional_leading_word_shifts_by_the_argument_count`) unit-tests it against
`upvar ?level? other local` with the parity-correct answers. **No `CommandSpec` in
`rust/tcl-registry/src/commands/` sets the flag** — `grep -rn 'optional_leading_word' commands/`
returns nothing, and `upvar_.rs` declares neither `arg_roles` nor `repeated_args`. The registry
feature was built, documented, and tested for `upvar`, then never wired to `upvar`'s spec, so
`var_scoping` had nothing to migrate to and kept its parser. `diagnostics/usage.rs:832-836` even
documents the gap as intentional ("the registry deliberately omits its roles"), which is how the
`ArityParity` bug in F2 survives.

**Why it matters:** four distinct grammars (`global`, `variable`, `upvar`, `namespace upvar`) with
five consumers each are maintained in analyser code that the registry can already describe as data.
Adding a fifth alias-creating command — a dialect's `::ns::localise`, say — requires editing
`var_scoping.rs` twice, defeating the `CREATES_SCOPE_ALIAS` recognition work that was already done.
Fixing F2 without doing this leaves the parity rule in two places (`frame_effect` and `repeated_args`).

**What cleanup looks like:** add `repeated_args: &[RepeatedArgLayout { optional_leading_word: true,
..RepeatedArgLayout::strided(ArgRole::VarWrite, 1, 2) }]` to `upvar_.rs`, then collapse both
`scope_alias_*_indices` match arms to a single `registry.arg_indices_for_role(command, &args,
ArgRole::VarWrite)` call, keeping only the two `$`-filters (the observability flavour keeps a local
whose *source* substitutes; the navigation flavour does not) as post-filters over the returned indices.
`global_declaration_indices` / `variable_declaration_indices` / `upvar_local_*_indices` and their
duplicate callers in `place_bridge.rs`, `var_escape/handlers.rs`, `cfg_propagation/handlers.rs`,
`cfg_builder/global_write_info.rs` all become one registry query.

**Scale:** one registry field addition plus ~200 lines of parser and its mirrored tests deleted
across six modules. Do it together with F2.

---

## F4: `irules_event_checks::var_write_index` is the hardcoded variable-write name table that `param_traits` already deleted as "a redundant duplicate of the registry query"

**Confidence:** high
**Category:** duplicated-table

**Where the knowledge lives now:**
`rust/tcl-compiler/src/analyser/irules_event_checks.rs:143-152`:

```rust
fn var_write_index(cmd_name: &str) -> Option<usize> {
    match cmd_name {
        "append" | "const" | "global" | "incr" | "lappend" | "ledit" | "lpop" | "lset" | "set"
        | "unset" | "variable" => Some(0),
        "array" | "gets" => Some(1),
        _ => None,
    }
}
```
Consumed at `:170`, `:182`, `:812`, `:843`.

**Where it should live / what already exists:**
`CommandRegistry::arg_indices_for_role(cmd, &args, ArgRole::VarWrite)` — the same query the sibling
module already uses. `rust/tcl-compiler/src/analyser/param_traits.rs:645-656` documents the removal
of *its* copy verbatim: *"The old hardcoded `var_write_index` name list was a redundant duplicate of
that registry query and has been removed."* Same query in `auto_path_eval.rs:676`,
`var_scoping.rs:256`, `diagnostics/usage.rs:845`.

**Drift evidence:** the hardcoded set names 13 commands. 33 spec files under
`rust/tcl-registry/src/commands/` declare `ArgRole::VarWrite`. Commands the set misses that are live
in the iRules dialect: `catch` (`catch_.rs`), `lassign` (`lassign.rs`), `scan` (`scan_.rs`),
`binary scan` (`binary_.rs`), `regexp` (`regexp_.rs`), `regsub` (`regsub_.rs`), `dict` (`dict.rs`),
`string` (`string_.rs`), `chan`, `info`, `vwait`, `trace`, `upvar`, `namespace upvar`
(`namespace_.rs:1056`), and `foreach`/`lmap` loop variables. Conversely the two-index arm (`"array" |
"gets" => Some(1)`) is unconditional, so `array names ::x` is read as a write to `::x`.

**Why it matters:** IRULE6001 (global-variable CMP pinning) and its RULE_INIT implicit-global variant
are silent for `catch {…} ::err`, `lassign $l ::a ::b`, `regexp $re $s ::m`, `scan $s $f ::v`, and
`binary scan $d $f ::v` — all of which really do pin the virtual server to one TMM. The false
positive on `array names ::x` produces a code fix that rewrites a *read* into `static::x`.

**What cleanup looks like:** replace `var_write_index` with a helper returning **all**
`ArgRole::VarWrite` indices from the registry (`Vec<usize>`, not `Option<usize>`), and let
`global_var_from_command` / `implicit_global_var_from_command` / the two fix-span lookups iterate
them. The special-cases at `:184-193` (`set` with one arg is a read; `unset` destroys; `array`
without `set`) all disappear, because `set`'s and `array`'s `arg_role_resolver`s and `unset`'s
`DESTROYS_VARIABLE` already encode them.

**Scale:** ~40 lines, one module, plus IRULE6001 test cases for `catch`/`lassign`/`regexp`.

---

## F5: `when` is matched by literal name in three places in `irules_event_checks`, including a raw-source brace scanner that is the third copy of the `when EVENT` grammar

**Confidence:** high
**Category:** stale-consumer

**Where the knowledge lives now:**
- `rust/tcl-compiler/src/analyser/irules_event_checks.rs:486` (`if cmd_name != "when"`, IRULE1002
  unknown-event check) and `:597` (the second `when` gate).
- `rust/tcl-compiler/src/analyser/irules_event_checks.rs:870-948` — `scan_when_blocks`, a byte-level
  `\bwhen\s+[A-Z_][A-Z0-9_]*` scan with its own `priority N` / `timing …` skip and its own
  balanced-brace body extractor, run over `self.source` from inside IRULE4003
  (`:749`) — i.e. re-scanning the whole document from a handler the segmenter already walked.

**Where it should live / what already exists:**
- `rust/tcl-registry/src/commands/irules/when.rs:72` — `Traits::IS_EVENT_HANDLER`, the registry's
  answer to "which command opens an event body".
- Migrated consumers, all of which explicitly say so: `rust/tcl-compiler/src/analyser/commands.rs:1900-1909`
  ("Registry facts, not command names (gap-review C3). The event handler is whatever carries
  `IS_EVENT_HANDLER` — `when` today, but a dialect that adds another gets the same treatment without
  an edit here"), `tcl-lsp-core/src/completion.rs:474`, `tcl-lsp-core/src/semantic_tokens.rs:1497`.
- The body text is already in hand structurally: `commands.rs:1909-1930` sets `self.current_event`
  and descends the registry-resolved `ArgRole::Body`, so the analyser walks each `when` body once
  already.

**Drift evidence:** `scan_when_blocks` is a *third* implementation of the same regex-free scan —
`rust/tcl-registry/src/events.rs:975-1015` (`scan_when_events`) and
`rust/tcl-registry/src/profiles.rs:339-368` (`scan_file_events`) are the other two. They already
disagree in scope: only the analyser copy handles `priority`/`timing` words, only the analyser copy
extracts bodies, and only the registry copies deduplicate. None of the three respects comments,
quoting, or `\{` inside a body, so a `when` in a `#` comment or a string literal produces a phantom
event block. `irules_event_checks.rs:445-448` in the same file *does* use the registry
(`is_irules_top_level_only`), so the file is half-migrated.

**Why it matters:** IRULE4003 (cross-event variable scoping) is computed off a text scan rather than
the event bodies the analyser already visited, so a `when` inside a comment or a braced string
invents a phantom event and a spurious hint, while a `when` written through an alias or with an
unusual `timing`/`priority` spelling is dropped. IRULE1002 (unknown event name) never fires for a
future dialect's second event-handler command even though its spec would carry `IS_EVENT_HANDLER`.

**What cleanup looks like:** replace both `cmd_name != "when"` gates with
`spec.traits.contains(Traits::IS_EVENT_HANDLER)`, exactly as `commands.rs:1909` does. Replace
`scan_when_blocks` by accumulating `(event, body_text)` during the existing structural walk — the
analyser already knows `self.current_event` and the body span — so IRULE4003 consults a map built
from the CST instead of re-scanning source. Then delete `scan_when_blocks` and its tests.

**Scale:** ~90 lines deleted, one accumulator field added to the iRules check state.

---

## F6: `inline_uplevel`'s frame-reach guard is a two-name `matches!` where `CommandSpec::frame_effect` is the typed descriptor, and it reads the surface spelling instead of the canonical command

**Confidence:** medium
**Category:** stale-consumer

**Where the knowledge lives now:**
- `rust/tcl-compiler/src/inline_uplevel.rs:192-198` — `statement_has_frame_reach`:
  `Statement::Barrier { command, .. } | Statement::Call { command, .. } if matches!(command.as_str(), "uplevel" | "upvar")`.
- `rust/tcl-compiler/src/inline_uplevel.rs:155` — `if cmd != "uplevel" { return None; }` and
  `:164` — `2 if args[0] == "1"`, a literal level-word test.

**Where it should live / what already exists:**
- `CommandSpec::frame_effect` (`rust/tcl-registry/src/spec.rs:620`) and the four frame traits
  (`ALIASES_CALLER_FRAME`, `CURRENT_FRAME_INTROSPECTION`, `EVALUATES_IN_SHIFTED_FRAME`,
  `REPLACES_FRAME`, `rust/tcl-registry/src/traits.rs`). Carriers today:
  `commands/tcl/uplevel_.rs:166`, `commands/tcl/upvar_.rs:187`, `commands/tcl/eval_.rs:82`,
  **`commands/argparse/command.rs:266`** (`FrameArgLayout::OpaqueCallerVars` — "injects variables into
  the frame of **its own caller** under names it derives from an argument mini-language this analysis
  does not interpret … a consumer must widen rather than enumerate"), plus
  `commands/tcl/tailcall_.rs` (`REPLACES_FRAME`) and `commands/tcl/info_.rs`
  (`CURRENT_FRAME_INTROSPECTION`).
- `FrameLevel::parse` / `FrameEffectSpec::resolve_for_version` (`frame_effect.rs:127`, `:395`) for the
  level word, already used by `var_escape/walker.rs:269` and `var_observability.rs:131`.
- `Statement::canonical_command_or_source()` (`rust/tcl-compiler/src/ir.rs:1225`), whose doc says
  "Use from downstream dispatch sites … so a single canonical key drives every consumer".

**Drift evidence:** `argparse` carries a `frame_effect` whose whole point is that its caller-frame
writes are unknowable, and the guard does not see it. Neither does `tailcall` nor `info level` /
`info frame`. The surface-vs-canonical split is visible within the same crate: three diagnostics
modules check both spellings — `diagnostics/helpers.rs:444`
(`canonical_command.as_deref() == Some("::unset") || command == "unset"`),
`diagnostics/var_command.rs:2675`, `diagnostics/dataflow.rs:1536-1537` — while `inline_uplevel.rs`
checks only `command`. `uri_split.rs:66-71` documents the rule the guard breaks: "the SSA / IR layer
preserves the surface text — so detection helpers must accept both". A body containing
`::uplevel 1 {…}` therefore passes the guard. `connection_scope.rs:245`
(`command == "unset"`) has the same surface-only defect.

**Why it matters:** O-code "inline an `uplevel` passthrough proc" is a *rewriting* transform, so
missing a frame reach in the body is a miscompile, not a missed optimisation — the PR #1371 shape.
`proc P {b} {uplevel 1 $b}` invoked with a body that calls `argparse`, `tailcall`, or `::uplevel`
is inlined today.

**What cleanup looks like:** thread the registry into `body_has_frame_reach` (its callers already
have one) and replace the `matches!` with
`registry.get(stmt.canonical_command_or_source()).is_some_and(|s| s.frame_effect.is_some() || s.traits.intersects(FRAME_REACH_TRAITS))`,
where `FRAME_REACH_TRAITS` is a registry-side composite constant. Replace the
`cmd != "uplevel"` / `args[0] == "1"` pair with
`registry.frame_effect(canonical).resolve(&refs)` and a `FrameLevel::Relative(1)` test. Fix
`connection_scope.rs:245` to read `canonical_command_or_source()` at the same time.

**Scale:** ~30 lines plus a registry composite-trait constant; one new inline-refusal test per
missed carrier.

---

## F7: W231 recovers a list length by byte-scanning the source for `set var {literal}` with a hardcoded scope-marker name list, next to the segmenter it could have used

**Confidence:** medium
**Category:** stale-consumer

**Where the knowledge lives now:**
`rust/tcl-compiler/src/analyser/bounds_checks.rs`:
- `:706-747` — `infer_list_length_from_recent_set`, a line-anchored raw-byte scan for
  `(?:^|\n)\s*set\s+(\w+)\s+(\{[^{}]*\})\s*(?=\n|$|;)` over the whole source before the `lset`.
- `:757-824` — `match_set_literal`, which hardcodes the byte string `"set"` and the argument layout.
- `:829-855` / `:857-895` — `scope_is_flat` / `has_scope_marker`, a second raw scan for the literal
  words `proc`, `apply`, `try`, and `namespace` followed by `eval`.

**Where it should live / what already exists:**
- The segmenter is already imported and used **in the same file**:
  `bounds_checks.rs:506-522` (`any_command_recursive`) walks `segment_commands(script)` and recurses
  into braced words structurally.
- The registry answers every fact the byte scan re-derives: `set`'s write position via
  `arg_indices_for_role(.., ArgRole::VarWrite)` (`set_.rs`), whether a command opens a definition
  scope via `Traits::DEFINES_PROCEDURE` / `CommandSpec::definition_body` — the exact pair
  `commands.rs:4843-4850` (`definition_handler_owns_body`) already uses — and body arguments via
  `ArgRole::Body`.
- Constant list values are already modelled: `crate::analyses::ConstValue` / the SCCP lattice, and
  `auto_path_eval.rs` runs a full segmenter-based constant-assignment collector
  (`collect_writes`, `auto_path_eval.rs:491`).

**Drift evidence:** `has_scope_marker`'s four names are a strict subset of what the registry marks.
It misses `oo::define` / `oo::class create` bodies, snit/itcl definer bodies (all of which carry
`definition_body`), `coroutine`, `interp eval`, and any `eval`/`uplevel` body — so a `set x {a b c}`
inside an `oo::define ... method` body followed by an `lset` outside it is treated as sharing one
flat scope. It also matches the words inside comments and string literals, since it never
segments.

**Why it matters:** W231 (`lset` index out of range) can fire on an index that is in range for the
list actually reaching the `lset`, and stays silent when the two really do share a scope but the
`set` was not written at a line start or used a non-`{}` literal. The whole mechanism is a
worse-than-SSA answer to a question SCCP already computes.

**What cleanup looks like:** drop the four byte scanners and take the list length from the SSA/SCCP
constant lattice for the `lset` target's reaching definition, the way `path_concat.rs` and
`uri_split.rs` already consume `crate::analyses::LatticeValue`. If a lattice value is not
threaded into `bounds_checks` today, the intermediate step is to reuse `segment_commands` plus
`arg_indices_for_role(ArgRole::VarWrite)` and `definition_handler_owns_body` instead of the byte
scans — which removes the hardcoded name list even before the SSA move.

**Scale:** ~190 lines removed; the SSA-backed version is a larger refactor than the
segmenter-backed intermediate.

---

## F8: `oo.rs`'s unknown-handler analysis keys on a literal `CHAIN_TARGETS` list plus `"exec"` / `"auto_load"`, while two sibling modules read the same fact from registry traits

**Confidence:** medium
**Category:** duplicated-table

**Where the knowledge lives now:**
- `rust/tcl-compiler/src/analyser/oo.rs:76-82` — `const CHAIN_TARGETS: &[&str] = &["_original_unknown",
  "_orig_unknown", "::tcl::unknown", "tcl::unknown", "original_unknown"]`.
- `rust/tcl-compiler/src/analyser/oo.rs:1996-2003` — `walk_unknown_stmt`:
  `if CHAIN_TARGETS.contains(&command.as_str()) { … } else if command == "exec" { … } else if command == "auto_load" { … }`,
  reading the bare `command` field of `Statement::Call`/`Barrier`.

**Where it should live / what already exists:**
- `Traits::UNRESOLVED_COMMAND_HANDLER` — `rust/tcl-registry/src/commands/tcl/unknown.rs:173`.
- `Traits::LOADS_EXTERNAL_UNIT` — `rust/tcl-registry/src/commands/tcl/auto_load.rs:56`.
- `exec`: `Traits::UNSAFE` + a `SideEffectTarget::Process` side effect
  (`rust/tcl-registry/src/commands/tcl/exec_.rs:48-56`).
- `Statement::canonical_command_or_source()` — `rust/tcl-compiler/src/ir.rs:1225`.
- The migrated twins, both with doc comments saying so:
  `rust/tcl-compiler/src/unit_scope.rs:884-905` ("Which command is the handler comes from
  `Traits::UNRESOLVED_COMMAND_HANDLER`, **never a literal name here**") and
  `rust/tcl-compiler/src/analyser/handlers.rs:2514-2546` (same sentence, and it admits both the
  `::unknown` and `::tcl::unknown` spellings from the one trait carrier).

**Drift evidence:** `handlers.rs:2540-2546` derives *both* accepted spellings from the single
`unknown` spec by qualifying it against `::` and `::tcl`. `CHAIN_TARGETS` lists the `::tcl` pair by
hand and omits `::unknown` / `unknown` entirely, so the two modules disagree about what "the original
handler" is named. `command == "exec"` also misses the `::exec` spelling that
`diagnostics/helpers.rs:444` and `uri_split.rs:66-71` establish as required.

**Why it matters:** `UnknownProcInfo::chains_original` / `has_exec` / `has_auto_load` gate W123
suppression for the whole file. A handler that chains via `::unknown`, or shells out via `::exec`,
reads as a self-contained dispatcher, and every unresolved command in the document gets a spurious
W123.

**What cleanup looks like:** thread the registry into `walk_unknown_stmt` (the caller at `oo.rs:1888`
already builds one) and resolve on `canonical_command_or_source()`:
`registry.commands_with_trait(UNRESOLVED_COMMAND_HANDLER)` for the chain test (matching the
`::`/`::tcl` qualification `handlers.rs:2542` uses), `LOADS_EXTERNAL_UNIT` for `auto_load`, and the
`Process` side effect for `exec`. The genuinely irreducible residue — the *user-chosen* saved names
`_original_unknown` / `_orig_unknown` / `original_unknown`, which no registry can know — shrinks to
a three-name convention list with a comment saying why it cannot move.

**Scale:** ~25 lines; the list shrinks rather than disappears.

---

## F9: `path_concat` detects `[file normalize]` by matching the source spelling, and its module doc calls the registry-declared taint path dead when it is not

**Confidence:** medium
**Category:** stale-consumer

**Where the knowledge lives now:**
`rust/tcl-compiler/src/path_concat.rs:70-86` — `is_file_normalize_of(value, var_name)`: parses the
command substitution and requires `cmd == "file" && args[0] == "normalize"` and a textual
`$var` / `${var}` match on the same variable. Driven from the same-block forward scan at `:283-295`.
`:199-202` sets `suppress_colours = TaintColour::PATH_NORMALISED` and comments "It is not currently
assigned by the taint engine".

**Where it should live / what already exists:**
- `rust/tcl-registry/src/commands/tcl/file_.rs:386` — `taint_transform: Some(TaintColour::PATH_NORMALISED)`
  on the `normalize` subcommand, with `returns_path: true`.
- `rust/tcl-compiler/src/taint.rs:390-402` — `transform_colour` resolves the *subcommand's*
  `taint_transform` ahead of the bare command's, so `[file normalize …]` really does stamp
  `PATH_NORMALISED`; `rust/tcl-registry/src/taint.rs:660-663` pins that with a test
  (`taint_transform(&registry, "file", Some("normalize")) == Some(PATH_NORMALISED)`).

**Drift evidence:** the module doc at `path_concat.rs:30-33` states "the taint engine does not yet set
`PATH_NORMALISED` on `[file normalize]` results", and `:199-202` repeats it. Both are stale — the
registry declares the transform and `taint.rs:390` applies it. The hardcoded text match is the only
live suppression path *because* the doc's premise stopped being true without the code following.

**Why it matters:** W201 (path built by string concatenation) fires despite a normalising sanitiser
whenever the spelling is not literally `[file normalize $sameVar]` — `set p [file normalize [file
join $a $b]]`, `set q [file normalize $p]; set p $q`, an `interp alias`'d `file`, or a
`file nor` abbreviation (`file`'s ensemble accepts unique prefixes, which
`is_file_normalize_of`'s `args[0] != "normalize"` rejects). Conversely the text match cannot see a
normalisation that reaches the value through a phi.

**What cleanup looks like:** delete `is_file_normalize_of` and the same-block forward scan; keep the
existing `suppress_colours` arm and let the taint lattice answer, since the fact is already in it.
Refresh the module doc. If the lattice's `PATH_NORMALISED` genuinely only rides on values that were
tainted to begin with, the correct fix is to widen the taint engine to stamp `returns_path` +
`taint_transform` colours on clean values too — a registry-driven change, not a new name match.

**Scale:** ~60 lines deleted plus a doc rewrite; possibly one taint-engine widening.

---

## F10: `auto_path_eval::collect_writes` re-implements `set` / `variable` / `namespace eval` layouts and hand-folds `file dirname|normalize|join`

**Confidence:** medium
**Category:** hardcoded-command-knowledge

**Where the knowledge lives now:**
`rust/tcl-compiler/src/auto_path_eval.rs:503-545` — `match words[0].as_str() { "set" if len == 3 => …,
"variable" if len >= 2 => …, "namespace" if len == 4 && words[1] == "eval" => … }`, with the
`variable` arm open-coding the name/value stride at `:510-537`.
`rust/tcl-compiler/src/auto_path_eval.rs:1091-1113` — a mini constant evaluator hardcoding
`info script`, `file dirname`, `file normalize`, and `file join`.

**Where it should live / what already exists:**
- `variable`'s stride is registry data: `rust/tcl-registry/src/commands/tcl/variable_.rs:164`
  (`RepeatedArgLayout::strided(ArgRole::VarWrite, 0, 2)`); `set`'s write index comes from its
  `arg_role_resolver`; `namespace eval`'s body is `ArgRole::Body` +
  `ArgRole::NamespaceName` (`namespace_.rs:785`) with `Traits::DECLARES_NAMESPACE`.
- The same file already does it the right way 130 lines later:
  `auto_path_eval.rs:660-690` (`poison_mutated_variables`) reads
  `arg_indices_for_role(head, &args, ArgRole::VarWrite)` and `Traits::CREATES_SCOPE_ALIAS`, and its
  doc comment cites AGENTS.md for doing so.
- `CommandSpec::const_fold` / `const_fold_versioned` (`rust/tcl-registry/src/spec.rs:735`, `:740`) is
  the field for the folder half. 40 specs already carry one
  (`dict create`, `lindex`, `join`, `split`, `string cat/compare/index/length`, `scan`, `subst`, …).
  `file` carries none.

**Drift evidence:** the two halves of the same module disagree about how to find a variable write —
`collect_writes` by name, `poison_mutated_variables` by role — so a write recorded by one is
invisible to the other for any command outside the three-name list (`array set`, `dict set`,
`lappend`, `append`, a stub-declared setter).

**Why it matters:** `auto_path` constant resolution decides which `source`d / `package require`d
units the workspace model can follow, so a missed `lappend dir …` or `append dir …` assignment loses
a whole file from the index (definitions, references, and completion inside it). The absent
`file` const-fold means every other consumer that wants `[file join $a $b]` folded — SCCP,
`const_subst`, the lowering fast paths — has to write its own.

**What cleanup looks like:** replace the three `match` arms with role queries: `ArgRole::VarWrite`
indices for the write targets (covering `set`, `variable`, and everything else in one arm), and
`ArgRole::Body` + `ArgRole::NamespaceName` for the namespace recursion, checking
`Traits::DECLARES_NAMESPACE` rather than `words[1] == "eval"`. Move `file dirname` / `file normalize`
/ `file join` into `const_fold` fns on the `file` subcommand specs, next to the existing
`fold_join` / `fold_split`, and have the evaluator dispatch on `spec.const_fold` like the rest of the
optimiser.

**Scale:** ~80 lines in the analyser plus three small registry `const_fold` fns.

---

## F11: `tk_checks::GEOMETRY_COMMANDS` is a hardcoded three-name set in a module that otherwise gates on registry data

**Confidence:** medium
**Category:** hardcoded-command-knowledge

**Where the knowledge lives now:**
`rust/tcl-compiler/src/analyser/tk_checks.rs:140` — `const GEOMETRY_COMMANDS: &[&str] = &["pack",
"grid", "place"]`, consumed by `is_geometry_command` at `:167`.

**Where it should live / what already exists:** no registry field carries this today — it needs one.
The specs exist (`rust/tcl-registry/src/commands/tk/{pack,grid,place}.rs`) and the surrounding module
already resolves everything else from the registry: `TK_PACKAGE` (`tk_checks.rs:148`) is documented
as "the single source of truth for both halves of the gate", and `Analyser::is_widget_command` reads
`CommandSpec::required_package`. The natural home is a new `Traits::TK_GEOMETRY_MANAGER` flag
alongside the existing widget/Tk traits, or — since TK1001 is really "two managers claim one
parent" — a small descriptor recording the manager's identity so the diagnostic can name it.

**Why it matters:** TK1001 (conflicting geometry managers on one parent) is silent for any manager
outside the three core names — notably `ttk::` megawidget managers and vendor Tk forks that ship
their own — and adding one means editing the analyser rather than the spec pack, which is exactly
what the "Command registry" section of AGENTS.md forbids.

**What cleanup looks like:** add the trait to `traits.rs`'s `declare_traits!` block, set it on the
three Tk specs, and replace `is_geometry_command` with a `spec.traits.contains(...)` check taken
from the already-resolved spec. `GEOMETRY_COMMANDS` and its helper both disappear.

**Scale:** one trait, three spec edits, ~8 lines in the analyser.

---

## Not reported (verified clean or explicitly carved out)

- `analyser/oo.rs`'s member routing (`:1605-1607`, `:3053-3140`, `:1755-1814`) — the
  `ClassDef` field / `MethodDef` kind routing and the `destructor` vs `initialise` scope question,
  carved out by AGENTS.md and documented at the call sites. Recognition and arg layout genuinely do
  come from `definition_body` / `MemberSpec::indices_for` (`oo.rs:131-134`, `:340`, `:532-535`).
- `analyser/dispatch.rs` — copies only static `arg_roles`, but every consumer of `CommandSig.arg_roles`
  is an arity/option check; the role-*resolution* path is `param_traits::resolve_arg_roles`
  (`param_traits.rs:566-595`), which honours the documented `arg_role_resolver` > `arg_roles` >
  sub-roles order via `arg_indices_for_role`. No `assigns_variable_at` read exists anywhere in the
  audited scope.
- `analyser/commands.rs` body descent, `analyser/handlers.rs` (`defines_global_unresolved_handler`,
  `handle_switch_command`, `defines_command_at` / `SymbolDef` handling), `unit_scope.rs`,
  `head_identity.rs`, `command_binding.rs`, `registry_invocation.rs`, `rendered_properties.rs`,
  `object_types.rs`, `dynamic_names.rs`, `var_refs.rs`, `var_resolve.rs`, `scan_predicate.rs`,
  `lambda_literal.rs`, `subst_nocommands.rs` — registry-driven throughout; several carry explicit
  "registry facts, not command names" comments.
- `alias.rs:201-223` (`expr_alias_names` matching the literal target `"expr"`) — real debt, but
  already documented as tracked debt at the call site with the intended replacement named
  (`Traits::EXPR_CONCATENATES_ARGS`) and the blocker stated (the registry-less
  `dispatch_lowering_hook` signature). Recorded here rather than as a finding.
- `analyser/commands.rs:154-160` (`orphaned_keyword_parent` mapping `else`/`elseif`/`then` → `if`
  and `on`/`trap`/`finally` → `try`) — the forward facts are registry data (`if`'s
  `arg_role_resolver` marks them `ArgRole::Keyword`; `try`'s `FIRST_CLAUSE_KEYWORD_VALUES`,
  `try_.rs:122-137`, lists the three), but no reverse keyword→parent index exists and the table has
  not drifted. Worth folding into any future `ArgRole::Keyword` index; not a defect today.
