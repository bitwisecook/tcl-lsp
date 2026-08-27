# Lane O1 / O2 — the option relation model, and the M9 dead axes

Redesign [§11.1](../dialect-and-package-registry-redesign.md) rows **O1** and
**O2**, under the standing principles of §0.05 — above all **P-B** (performance
in the general case; hooks are the exception) and **P-C** (retire unused
surface). O1 is ratified as **E-R14** in the
[deep dive](../spectcl-design-e-deep-dive.md) §14.

## Goal

1. **O2** — delete the declared-and-unpopulated model surface the M9 audit
   found, and extend the retired-api gate so it cannot return.
2. **O1 / E-R14** — generalise `OptionConstraint` into a typed relation
   covering mutual exclusion, directional *requires*, requires-one-of, and
   relations reaching a positional argument or an option *value*; make the
   declarative vocabulary the mechanism (checked natively in Rust, **zero**
   tclvm entry); keep a rare `constraints` hook as the escape hatch, following
   the `types` hook contract; wire the analyser's option-table checker; prove
   it on three real cases; and **measure** the VM-entry count.

## Design decisions taken, and why

### The relation type (`rust/tcl-registry/src/spec.rs`)

```
OptionRelation { kind, subject: Option<RelationTerm>, terms: &[RelationTerm],
                 dialects, lifecycle, message: Option<&'static str> }

RelationKind  = MutuallyExclusive | Requires | RequiresOneOf | Forbids
RelationTerm  = Option(name) | OptionValue(name, value)
              | Argument(index) | ArgumentValue(index, value)
```

* **Four terms, not two.** They are the four things a real library's option
  table talks about. `bibtex::parse` needs `Option` and `Argument`
  (`-command` requires `-channel`, and `-channel` excludes the inline `text`
  word); `struct::tree walk` needs `OptionValue` (`-order in` is illegal with
  `-type bfs`).
* **`subject` is optional.** `None` makes a relation *unconditional*, which is
  what `bibtex::parse`'s "Neither `-channel` nor text specified" needs, and is
  also how `MutuallyExclusive` — which has no subject — always reads.
* **`Forbids` exists beside `MutuallyExclusive`** because the asymmetric case
  is how libraries phrase the failure, and because the directional form names
  the word the author has to change.
* **`DEFAULT` is `MutuallyExclusive` with no subject**, so every migrated
  1.x row reads exactly as it did and the migration is a term-list rewrite,
  not a six-field one.
* **`message`** lets a spec quote the library's own error text rather than
  generate one. `struct::tree` and `bibtex` both use it.

### Evaluation, and why absence is the hard half

`OptionRelation::evaluate(&RelationFacts) -> RelationVerdict` is the whole
declarative checker: a few slice scans over facts the analyser's option walk
already produced. `TermHolds` is three-valued (`Yes`/`No`/`Unknown`), and
`RelationFacts::complete` — "the call was read to its end with every relevant
word literal" — is the **only** thing that licenses proving a term *absent*.
Presence is always provable, so an exclusion never needs `complete`; a
`Requires` over a `{*}$opts` call abstains rather than accusing the call of
omitting an option the expansion may well be supplying.

### `OptionPlacement`

New field on `CommandSpec` / `SubCommand`, default `Leading`.

Not a style preference: core Tcl's C option loops almost all `break` on the
first word that is not a declared option (`Tcl_GlobObjCmd`, `source`, `lsort`),
so a later option-shaped word there is a *positional*, and reading it as an
option would invent a relation the interpreter never raises. `Anywhere` is the
script-level shape — a parser that takes its fixed arguments and then loops
`foreach {flag value} $args`. Two shipped specs need it: `http::geturl` (URL
first) and `struct::tree`'s `walk`/`walkproc` (`node` first).

### The `constraints` hook — the escape hatch

`HookFamily::Constraints`, `CommandSpec::constraints: Option<ConstraintsHook>`.

Reached **only** when a spec declares one *and* every declarative relation
reported nothing. Contract copied from the `types` hook:

* **declared inputs** — `constraints -inputs {invocation} {call} { … }`;
* **shape/content caching** — `HookInput::Invocation` earns
  `CacheMode::Content`, keyed on the shape *plus* a hash of the call's option
  names, option values, positional words and `complete` flag, so an edit
  elsewhere in the document never re-runs the hook for an unchanged call site;
* **explicit abstention** — `abstain` cancels every report already made, and
  `ctx`'s `complete` key is what a body tests before judging;
* **error means abstain** — an erroring, budget-blown, quarantined or hostless
  hook answers silence, exactly like every other family.

Verbs (all spellings that already existed elsewhere, per P-E):

| verb | kind | meaning |
|---|---|---|
| `option-present OPTION` | reader | boolean |
| `option-value OPTION` | reader | the option's literal value, or empty |
| `literal N` | reader | positional word N, or empty when not statically known |
| `arg-count` | reader | how many positional words the call supplied |
| `invalid SLOT MESSAGE ?-conflict?` | emitter | one report; `SLOT` is `-name`, `arg N`, or `command`; `-conflict` selects W147 over the default W152 |
| `abstain` | emitter | withdraw — cannot judge this call |

Reader verbs are new machinery in `tcl-spec-hooks`: `emit::Reading` is a
per-invocation cell the host refills beside the sink, so a verb command
defined once per pack still answers about the current call.

### The SpecTcl vocabulary (2.0)

```tcl
option_conflict          {-query -querychannel}
option_requires          -command {-channel}
option_requires_one_of   {} {-channel {arg 0}}
option_forbids           {-order in} {{-type bfs}}
```

A term is an ordinary Tcl list word: `-name`, `{-name value}`, `{arg N}`,
`{arg N value}`. `option_conflict` keeps its exact 1.x shape (statement word
then term list) so an unchanged pack round-trips unchanged; the other three
take the subject between the statement and the terms, with `{}` meaning
unconditional. All four share one row parser and one flag set (`-dialects`,
`-available`, `-message`, the lifecycle trio).

### Diagnostics

* **W147** keeps its meaning and is reused for `MutuallyExclusive` and
  `Forbids` — both read "these cannot go together".
* **W152** is new, for `Requires` and `RequiresOneOf` — "this one needs that
  one". Registered in `tcl-core-types::diag_code`; all five artefact
  generators regenerated.

## What was deleted (O2), and what was kept

| Item | Disposition |
|---|---|
| `ProfileSpec::capabilities` | **deleted** — one populated row, no reader; its only intended consumer was `EventRequires::capability`, which was never `Some` |
| `ProfileSpec::conflicts` | **KEPT — live consumer.** `rust/tcl-bigip/src/validator.rs:606` iterates it to emit `DiagCode::Bigip6039` ("incompatible profile types"), a registered public code with a message-table row, an entry in `rust/tcl-mcp/diagnostics.json`, and a test. Every shipped row is `&[]` so it cannot fire today, but deleting the field means deleting a published diagnostic, which the O2 ruling does not cover |
| `EventRequires::init_only` | **deleted** — no shipped spec sets it; the `event_satisfies` / `missing_requirements_description` branches were dead |
| `EventRequires::capability` | **deleted** — never `Some` anywhere |
| `Traits::PASSWORD_OPTION` | **deleted** — no spec carries the bit |
| `Traits::IRULES_DATA_GETTER` | **deleted** — no spec carries the bit; `taint::is_irules_data_getter`'s live path is the `IRULES_TAINT_SOURCE_PREFIXES` fallback, which stands |
| `xc_operation` (command + subcommand) | **deleted** — never `Some` |
| `arg_rows` machinery (`VersionedArgRow`, `ProjectedArgs`, `ArgTables`, `project_arg_rows`, `arg_tables_at`, `arg_indices_for_role_at`, `command_prefixes_at`) | **deleted** — test-only consumers in `tcl-registry/src/registry.rs` and `tcl-lsp-core/src/completion.rs`; the `DocumentFloor` argument threaded into `expr_context::expr_arg_context_at` for it went with it |

The retired-api gate (`rust/xtask/src/retired_api_gate.rs`) grew a ledger-O2
block with every deleted spelling and a seeded-violation self-test line per
name. `ProfileSpec::conflicts` is deliberately **not** in it, with the reason
in a comment.

## The measurement (P-B)

`analyser::diagnostics::tests::an_option_heavy_document_is_checked_with_no_vm_entry`
analyses a 40-call-site option-heavy corpus (`glob`, `source`, `lsort`,
`string match`, `regsub`, `switch`, `lsearch`, `fconfigure`, `exec`,
`binary scan`, `clock`, `interp create`, `http::geturl`, `bibtex::parse`,
`struct::tree walk`) and reads `tcl_registry::spec::relation_check_stats()`
plus `pack_hooks::cache_stats()`:

```
33 option-bearing call sites walked, 14 of them judged against declared
relations, 50 relations evaluated natively, 0 entered tclvm;
cold analysis 253 ms, second pass 69 ms
```

`hook_entries == 0` and `pack-hook dispatches == 0` are asserted, not
described. `RelationCheckStats` is three thread-local `Cell<u64>` counters, so
the instrument stays on in release builds and a regression that starts
entering the VM fails a test rather than merely getting slower.

## Sites done

* `rust/tcl-registry/src/spec.rs` — the relation model, `RelationFacts`,
  `evaluate`, `OptionPlacement`, `ConstraintSlot`/`ConstraintReport`/
  `ConstraintsHook`, `RelationCheckStats`; `arg_rows`/`xc_operation` deleted.
* `rust/tcl-registry/src/pack_hooks.rs` — `HookFamily::Constraints`,
  `HookInput::Invocation`, `CacheMode` (None/Shape/Content), content hashing,
  cache ceiling, `constraints_fn`.
* `rust/tcl-spec-hooks/src/{emit,host}.rs` — the verb vocabulary, the
  `Reading` per-invocation view, the `ctx` keys (`options`, `positionals`,
  `complete`), `answer_of` for the family.
* `rust/tcl-spectcl/src/loader.rs` + `hooks.rs` — `option_relation_row` and
  the four statements at both loader seams, `relation_term` / `relation_terms`,
  the `constraints` hook statement at command and subcommand level, binding.
* `rust/tcl-spec-studio/src/{coverage,draft,help,render_rs,render_spectcl,schema,store}.rs`
  — the relation expression, the `option_placement` enum catalogue, the
  `constraints` schema field, and the **export round-trip gate**
  (`every_relation_kind_and_term_shape_round_trips_to_its_statement`).
* `rust/tcl-compiler/src/analyser/` — `CommandSig` carries
  `option_relations` / `constraints_hook` / `option_placement`;
  `scan_invocation_words` is the one walk both paths judge from;
  `option_relation_diagnostics` is the one consumer; `widget_command.rs`
  reaches instance methods (`registry.instance_methods` fallback) so
  `struct::tree walk` is checked at all.
* `rust/tcl-registry/src/commands/` — `http__geturl`, `misc_pkgs` (bibtex),
  `data_structures` (struct::tree), `spectcl/{rows,hooks}` vocabulary.
* `rust/xtask/src/retired_api_gate.rs` — the O2 block + self-test.
* Artefacts: `docs/generated/diagnostic_{codes,tables}.md`,
  `docs/references/command-spec/fields.md`, `ai/**`, `editors/**`,
  `rust/tcl-mcp/diagnostics.json`.

## Sites done — docs

* redesign §11.1 O1 and O2 rows rewritten to what landed (O2 records the
  `ProfileSpec::conflicts` exception and why).
* redesign §8's P5 known-limits list: gap (c) *option-requires relations*
  struck through and closed.
* deep dive **§2.1 The `constraints` hook contract** — new, beside the
  `types` contract in §2, with the verb table and a worked body.
* deep dive §14 E-R14 row marked landed, with the measurement.
* deep dive §11 census: G2's directional half moved from "still open" to
  "closed since"; the `bibtex::parse` note now names the three shipped rows.
* `docs/design/spec-dsl-examples/README.md`: `constraints` in the hook-family
  table, the four relation statements in both field tables, `option_placement`,
  the `xc_operation` and `arg_rows` rows removed, and the "option requirement
  relationships" known-limit closed.
* `docs/design/contracts/shared-utility-contracts-rust.md` point 6 rewritten:
  the projection point stands, the per-argument *lifecycle* machinery is
  recorded as retired.
* `docs/design/compiler/command-registry.md`: `option_relations`,
  `option_placement`, `constraints`; `PASSWORD_OPTION` and `xc_operation` rows
  removed.
* `docs/references/command-spec/{fields,diagnostics}.md`; **new KCS article**
  `docs/kcs/codes/kcs-diagnostic-w152-option-relation-unmet.md` with its index
  entry and its `command-walk` stage row in the xtask gate; the W147 article
  updated for directional and value-reaching exclusions.
* `docs/design/README.md` gained the in-flight-lanes section the KCS index
  gate needs, and `docs/design/lanes/README.md` lists this lane.

## Gates run

* `cargo check --workspace` — clean (run before every commit).
* `cargo clippy --workspace --all-targets -- -D warnings` — clean, with **no
  new `#[allow]`**: the eight-argument relation queue became a
  `ScannedInvocation` struct, `Option<Option<&str>>` became an
  `OptionPresence` enum (which also collapsed the duplicate match arms), and
  three functions that crossed the line budget were split along real seams.
* `cargo fmt --all --check` — clean.
* `cargo test` — green on `tcl-registry`, `tcl-spectcl`, `tcl-spec-hooks`,
  `tcl-spec-studio`, `tcl-core-types`, `xtask` (26 binaries), on
  `tcl-compiler`, `tcl-lsp-core`, `tcl-mcp`, `tcl-bigip` (100 binaries), and
  on `tcl-lsp-server`, `tcl-irules`.
* `make xtask-check` — **exit 0**, with every generated artefact regenerated
  and back in sync (diag tables, editor settings, the VS Code package, the
  JetBrains catalogue, the AI/MCP diagnostics, the callback-surface inventory,
  the KCS index, `owner-resolution`, and `retired-api-gate`).

## Known unrelated red

`tcl-cli`'s `explorer_gui::gui_renders_the_wasm_tab_and_settles_the_spinner`
fails with a Playwright `page.fill` timeout on the compiler-explorer WASM GUI.
Nothing in this lane touches the explorer, its WASM bundle, or its driver; the
skip heuristic in that test only covers a *missing* browser, not a slow one.
Recorded here so it is not mistaken for an O1/O2 regression.

## Remaining

Nothing for this lane. The orchestrator folds this file into the final commit
message and removes it.

## Behavioural deltas accepted

* **W147 span and message for a `Forbids` relation** name the subject *and*
  the offending term, not two options from a flat set.
* **`struct::tree walk` / `walkproc` are now option-checked at all** — they
  were previously invisible to the option-table checker because the
  instance-dispatch path only consulted the creator command's `subcommands`.
  The same change removes a class of W001 false positives on tcllib factory
  methods (a method declared on an `ObjectClassSpec` was reported unknown).
* **`http::geturl` gains two `RequiresOneOf` relations** (`-queryprogress`
  and `-queryblocksize` each need `-query` or `-querychannel`), so a call
  supplying one alone now warns where it was silent.
* **`bibtex::parse` gains five more relations**, including the two
  `-commentcommand` / `-progresscommand` exclusions P5 had listed but not
  declared.
* `expr_arg_context_at` lost its `DocumentFloor` parameter (it existed only to
  reach `arg_indices_for_role_at`).

## Uncertainties

* A **bareword** instance receiver (`::struct::tree t` then `t walk …`) is
  still not option-checked: `record_widget_dispatch_candidate`'s pre-filter
  only buffers `$var` and `.`-prefixed widget paths. Widening it is a
  separate, riskier change (it would buffer every unknown command word), so
  the lane left it. `$t walk …` — the shape tcllib documents — works.
* `struct::tree walk`'s `OptionPlacement::Anywhere` models `_walk {name node
  args}` faithfully for the shapes in the corpus, but a `loopvar` literally
  spelled like a declared option would be misread. No such call exists.
