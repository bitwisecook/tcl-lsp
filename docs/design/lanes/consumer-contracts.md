# Lane: consumer contracts — step 1 landed; the plan for steps 2–10

## Goal

Step 1 of [registry-consumer-contracts.md](../compiler/registry-consumer-contracts.md)
§ *Build order*, documents only: take the four rulings (and the fifth,
narrower one on the pack-authored `state_transitions` resolver) as decided,
and repair every document whose stated rule the rulings replace. No `rust/`,
`runtime/`, `editors/`, or `art/` file is touched by this lane.

## Decisions taken

- § *Rulings still open* becomes § *Rulings*. Each subsection keeps its
  statement, its rationale, and its consequences, and gains one **Decided.**
  sentence naming the build-order steps that land it. The four
  "If the owner decides the other way" paragraphs and the fifth ruling's
  "*The other way*" clause are dropped: a decided ruling has no other way,
  and leaving them would make the page argue against its own status box.
- The fifth ruling's lead-in read "emitting alias facts only" while its own
  contract sentence allows `VariableCellAliasTransition` **and**
  `NamespaceTransition` facts. The two families are the reading carried
  through, in the page and in `spec-packs.md`.
- Every repaired rule is stated as the design has it, in the page's own
  voice, with one present-tense "today" sentence wherever the tree still
  does the thing the ruling replaces. Each such sentence was checked against
  the tree with grep before it was written; no sentence claims behaviour
  that does not exist.

## Document inventory

| Document | Section | Status |
|---|---|---|
| `compiler/registry-consumer-contracts.md` | § Rulings, § The two hook bodies that remain, § Codegen…, § C Tcl extensions, § Build order, § Related docs | done |
| `compiler/command-registry.md` | § Authoring a spec without Rust | done |
| `contracts/dialect-stubs.md` | § Flags, § Stubs are declarations | done |
| `registry/spec-packs.md` | § Workspace trust…, § What a pack still cannot say | done |
| `runtime/c-extension-shim.md` | § The implemented subset, § Out of scope | done |
| `runtime/c-extension-abi.md` | § 7 Header scope | done |
| `runtime/tclvm-opcode-status.md` | note 5, `startCommand` | done |
| `runtime/rename-alias.md` | § 3.5 Invalidation | done |
| `compiler/aot-command-priority.md` | § 5 | done |
| `docs/GLOSSARY.md` | C extension shim | done |
| `design/README.md`, `design/compiler/README.md` | the index entries | done |
| `docs/kcs/` | — | checked, no note states a replaced rule |

## Open uncertainties

- `docs/kcs/kcs-qa-what-is-the-c-extension-shim.md` and the glossary entry
  describe compiling against `tclshim.h`. That is what the tree does today,
  so both keep it; the glossary entry names the authored header as the
  contract beside it. The KCS note is left alone rather than made to promise
  a header the tree does not contain.
- `aot-command-priority.md` § 5 groups `load`/`unload` in one cell. The
  WASM runtime registers `load` on `unsupported_cmd`
  (`runtime/rust/src/cmd_misc.rs`) and does not register `unload` at all;
  the repair changes only the "fixable?" answer and leaves the grouping.

## Plan for steps 2–10

Steps 2 to 10 of [registry-consumer-contracts.md](../compiler/registry-consumer-contracts.md)
§ *Build order*, planned against the tree at `f3f9390f` on
`claude/spectcl-optimization-discussion-5qhf42`. The page is the design;
this section is the sequence an implementer follows and the reviewer checks
a landed item against. Items are numbered `CC<step>.<n>`. Each names its
files, its Rust items, what stays byte-for-byte and what changes, its tests
(with the negative case), the gates it touches, the documents it updates, a
model class (`opus` for design judgement or semantics, `sonnet` for
mechanical work from a given shape), a size (S: under a day; M: one to
three days; L: more), and the items it follows. Every item is one
`wip(consumer-contracts): …` checkpoint or a named part of one; the tree
compiles at every checkpoint (`cargo check --workspace`), the item's own
test binaries are green, and the lane stages only its own files by path.
The page's identifiers are used verbatim wherever the tree lets them be;
every adaptation is in § *Where the tree differs from the page* and in the
item that makes it.

### Where the tree differs from the page

Verified at the plan's revision. Each row is an adaptation the items use.

- `../spec-dsl-examples/if.tclspec` resolves to
  `docs/design/spec-dsl-examples/if.tclspec`; the examples directory is
  under `docs/design/`.
- `HandlerMatch`, `HandlerPlan`, `TemplateWordPlan`, `ScriptRegion`,
  `OperandId`, `AnalysisContext` and `Budget` already exist in
  `rust/tcl-registry/src/value_transfer/` (slice 1 of the value-transfers
  lane). The clause-grammar descriptor reuses
  `value_transfer::answers::HandlerMatch`; `MemberRow::body` is a
  `value_transfer::inputs::OperandId`. Neither type is declared twice.
- `OptionSpec` is declared in `rust/tcl-registry/src/hover.rs` and
  re-exported from `spec.rs`; its fields are `name`, `value`, `detail`,
  `surface`, `aliases`, `lifecycle`, `min_abbrev`. `OptionRelation` is
  `Relation<OptionTerm>` in `spec.rs`; `RelationKind::MutuallyExclusive`
  and `Relation::evaluate` exist; the loader reads `option_conflict`.
- `ResolvedInvocation<'r, 'w>` (`resolved_invocation.rs`) carries `words`,
  `canonical_command`, `subcommand`, `form` and `semantics`, and already
  answers `state_transitions()`, `effect_footprint()`, `facts()` and
  `validate_literal_arguments()`. The page's `RegistryQueries` trait is
  pseudocode by its own note; the layer is inherent methods on this type
  (decision D2.5).
- `Provenance` is `tcl_dialect::model::environment::Provenance`, not a
  registry type. The two `untrusted(…)` predicates are
  `rust/tcl-spectcl/src/loader/eval.rs:1784` (over `Tier`) and
  `rust/tcl-registry/src/model/registration.rs:287` (over `Provenance`).
- `PackEnvironmentTier::provenance` (`loader/environment_block.rs:109`)
  maps `Workspace` to `WorkspaceTrusted` unconditionally; `DiscoveryOptions`
  (`discovery.rs:182`) has no trust input; the server builds it in
  `spec_pack_discovery` (`rust/tcl-lsp-server/src/lib.rs:20837`) and reads
  `initializationOptions` in `apply_initialization_options` (`lib.rs:11179`).
- Hook bodies reach the host through `tcl_spectcl::hooks::plan_for` /
  `publish` (`rust/tcl-spectcl/src/hooks.rs`) and
  `HookHost::install_pack_hooks` (`rust/tcl-spec-hooks/src/host.rs:134`);
  `HookInstallation::declined` is the existing "not installed" record;
  `HookDecl` (`loader.rs:735`) carries no line number.
- `StubFlags` and `StubCommandDef` are in
  `rust/tcl-compiler/src/analyser/types.rs`; `parse_stub_flags` is in
  `analyser/utils.rs:1289`; `DocumentCommandSurface::arg_indices_for_role`
  (`model/declaration.rs:349`) unions the catalogue's and the document's
  answers.
- The shim exports its 34 symbols with `#[unsafe(export_name = "Tcl_…")]`
  in `rust/tcl-cshim/src/ffi.rs`; `runtime/rust/src/capi.rs` has 18
  `#[no_mangle]` exports (the page's 17 plus `tcl_test_finalize`);
  `runtime/rust/include/` holds only `tcl_regex_capi.h`, so no authored
  `tcl.h` exists anywhere (the ABI's § 4.1 says the runtime ships it).
- `EvalSnapshotKey::content_hash` is a `u64` xxh3, not `[u8; 32]`
  (decision D4.3).
- `CompiledUnit` (`rust/tcl-vm/src/compiled.rs`) carries the three
  generations the page names; `ModuleAsm` (`rust/tcl-bytecode/src/lib.rs`)
  carries `profile`, `source`, `source_namespace`,
  `plain_command_dispatch`, `top_level`.
- The WASM emitter encodes modules itself
  (`rust/tcl-compiler/src/codegen/wasm/encoding.rs`); no `wasm-encoder`
  dependency exists.
- `command_backing.rs`'s four lists exist; the report
  `docs/generated/wasm-command-backing.md` classifies 389 commands (105
  handler, 177 native, 11 stdlib, 54 not-required, 42 known-gap).
- `runtime/rust` is the crate `tcl-runtime`; `xtask` depends on neither it
  nor `tcl-vm`.
- `tcl-pkg` depends on no registry crate and `tcl-spectcl` does not depend
  on `tcl-pkg`; discovery finds a pack beside a `tclpkg.tcl`
  (`Origin::BesideManifest`) without reading the manifest.
- `TclVersion::from_profile` (`rust/tcl-dialect/src/version.rs:181`)
  answers for the five plain names; `DialectProfile::runtime_base` carries
  a vendor profile's measured base (`f5-irules` is `V8_4`);
  `value_transfer::const_ops` reads the release through `from_profile`
  (`const_ops.rs:325`).
- `Engine` (`rust/tcl-engine-api/src/lib.rs:250`) has no `set_release`;
  slice 4 of the value-transfers lane adds it (boundary B3).
- The analyser consumes 43 `AnalyserHookId` variants in
  `dispatch_analyser_hook` (`analyser/commands.rs:1294`); a generic
  role-driven binding pass, `handle_var_binding_command`, already runs ahead
  of the hook dispatch (`commands.rs:1139`).
- `MethodKind::from_str_lossy` has one live call (`lowering/mod.rs:3931`),
  fed by `member_method_kind` (`lowering/mod.rs:4009`), a keyword match;
  `SwitchMode::from_str_lossy` (`ir.rs:1504`) is the case-list axis's twin
  and goes on the ledger, not into this lane.
- `CLAUSE_KEYWORDS_WITHOUT_COMMAND_SPEC` and `CLAUSE_NOISE_KEYWORDS`
  (`traits.rs:1397`, `:1412`) have three consumers outside the registry:
  `rust/xtask/src/gen_tmlanguage_keywords.rs`,
  `rust/tcl-lsp-core/src/semantic_tokens.rs`,
  `rust/tcl-lsp-core/src/minify.rs`.
- The clause-carrying commands' native resolvers are `if_arg_roles` /
  `check_if_shape` (`if_.rs`), `try_arg_roles` (`try_.rs`),
  `foreach_arg_roles` (`foreach_.rs`), `lmap_arg_roles` (`lmap_.rs`);
  `catch`, `for`, `while`, `dict for`, `dict map`, `dict update`,
  `array for` use positional `arg_roles`. E004 reads
  `spec.clause_shape_check` at `analyser/commands.rs:1708`.
- The studio's coverage witness (`rust/tcl-spec-studio/src/coverage.rs`)
  destructures `CommandSpec`, `SubCommand` and `OptionSpec` exhaustively, so
  a new registry field breaks the studio's build until the witness, the
  `Field` table and the schema entry land in the same checkpoint.

### Step 2 — the description contract

**Goal.** The derived-query layer, the clause-grammar and member-effect
descriptors, the option-effect descriptor with its four clients (`subst`,
`lsearch`, `regexp`, `CaseListSpec`), the consumer migration across every
tier onto the generic operations, the per-axis lint with its generated
ledger, and `semantic_operation`, `definition_body` and `clause_grammar`
round-tripping through the studio with no `GAPS` row.

**Exit evidence.**

- `cargo xtask registry-axes --check` is green with `docs/generated/registry-axes.md`
  committed and every pin at or below its CC2.1 baseline; the files this
  step rewrites are held clean.
- `cargo test -p tcl-registry` green, including the new
  `clause_grammar`, `option_effect` and `definer` unit rows, the
  `registry_sweep` agreement rules, and `analyser_hooks` re-baselined to
  the 31 stamps of CC2.13.
- `cargo test -p tcl-spec-studio --test spectcl_roundtrip --test spectcl_ports --test option_row_editing --test reference_doc`
  green with `GAPS` no longer naming `semantic_operation`,
  `definition_body`, `substitution_resolver` or `pattern_arg_resolver`, and
  `docs/references/command-spec/fields.md` regenerated.
- `cargo test -p tcl-compiler --test analyser --test cfg --test mro_lattice_adversarial --test codegen`
  green and byte-identical on every existing case; `cargo test -p tcl-lsp-server --test preview_tickets_e2e`
  carries the new-member-spelling witness.
- `cargo xtask pack-goldens --check`, `gen-tmlanguage-keywords --check`,
  `callback-inventory --check`, `value-transfers --check`, `owner-resolution`
  and `kcs-index-links` green; `make codegen` produces no diff except the
  files this step regenerates on purpose (none: the TextMate keyword lists
  are pinned byte-identical).

#### Work items

**CC2.1 — the per-axis lint and its ledger.**
Files: `rust/xtask/src/registry_axes.rs` (new), `rust/xtask/src/main.rs`
(`Command::RegistryAxes { check }` beside `ValueTransfers`), `Makefile`
(`xtask-registry-axes` target in `xtask-check`, after
`xtask-value-transfers`), `docs/generated/registry-axes.md` (new),
`docs/design/contracts/shared-utility-contracts-rust.md` (owner manifest
row: surface *command vocabulary outside the registry*, owner
`rust/tcl-registry/src/registry.rs`; `rust/tcl-registry/src/clause_grammar.rs`;
`rust/tcl-registry/src/definer.rs`, entry points `CommandRegistry`,
`clause_keywords`, `DefinitionBodyGrammar`, axis "per dialect surface",
gate `xtask-registry-axes`).
Rust: `pub fn run(check: bool) -> Result<ExitCode>`; `const LINT_ROOTS`
copied from `value_transfers.rs` (the ten crates; the two lists are
merged into `rust/xtask/src/util.rs` by CC2.15 once the value-transfers
lane's slice 2 lands, boundary B1); `const AXES: &[&str] = &["command",
"subcommands", "clause_grammar", "definition_body", "options",
"special_vars", "irreducible"]` (the names of the migration plan's
§ *Debt on other axes* table, so a site waived for both gates names one
axis); `fn vocabulary(reg: &CommandRegistry) -> Vocabulary` built from the
`analyser_hooks.rs` `full_registry()` shape (every loadable dialect plus
`specs/`): every command and subcommand name, every `OptionSpec::name` and
alias at command, subcommand and form level, every `MemberSpec::keyword` of
every `DefinitionBodyGrammar` the specs reference, the clause keywords
(`CLAUSE_KEYWORDS_WITHOUT_COMMAND_SPEC` ∪ `CLAUSE_NOISE_KEYWORDS` until
CC2.2 replaces them with the grammars' rows), every `SpecialVarSpec` name;
`fn scan(text) -> Vec<Site>` recognising a vocabulary word used as the
operand of `==` / `!=` / `matches!` / a `match` arm / `.eq(` / a `&[&str]`
constant table entry compared later — the same shapes `value_transfers::scan`
recognises, with the operand set widened from command heads to the
vocabulary; the waiver marker `// registry-axis-ok: <axis> — <reason>; until <step N | slice N | never>`
(the expiry the page requires; `never` only with `irreducible`), a file
waiver `// registry-axis-ok(file): …`; `RATCHET: &[(&str, usize)]` pinned
from the first write-mode run and `CLEAN_FILES` starting empty; `--check`
fails on a rising count, a stale pin, an unknown axis, a missing expiry,
or an expiry naming a step or slice that has landed (the landed set is a
`const` the lane bumps). The report renders the ledger: every waiver by
axis with its expiry, and the ratchet table.
Preserve: nothing observable changes. Tests: `rust/xtask/src/registry_axes.rs`
unit tests — `a_clause_keyword_compared_by_spelling_is_a_site`,
`a_member_keyword_in_a_match_arm_is_a_site`, `an_option_spelling_in_a_const_table_is_a_site`,
`a_waiver_without_an_expiry_fails`, `a_registry_owner_file_is_never_scanned`
(negative: `rust/tcl-registry/**` and every path under a `tests/` directory
and `#[cfg(test)]` module are excluded, exactly as `value_transfers.rs`
excludes them). Gates: the new gate; `owner-resolution` (the manifest row
must name a live gate). Docs: a `## The per-axis lint` paragraph in
`docs/design/compiler/registry-consumer-contracts.md` § *The per-axis
lint and ledger* replacing "The ledger is generated" with the marker and
the file; KCS `docs/kcs/kcs-issue-the-registry-axes-gate-reports-a-keyword.md`
(the shape of `kcs-issue-the-value-transfers-gate-reports-a-command-name.md`),
indexed in `docs/kcs/README.md` § Issues. Model: opus. Size: M. After: —.

**CC2.2 — `ClauseGrammarSpec` in the registry.**
Files: `rust/tcl-registry/src/clause_grammar.rs` (new), `spec.rs`
(`CommandSpec::clause_grammar: Option<&'static ClauseGrammarSpec>`,
`SubCommand::clause_grammar: Option<&'static ClauseGrammarSpec>`, both in
`DEFAULT` as `None`), `lib.rs` (`pub mod clause_grammar`), `registry.rs`
(`arg_indices_for_role` and `arg_indices_for_role_words` consult the
grammar first: the resolution order becomes `clause_grammar` →
`arg_role_resolver` → `arg_roles` → `assigns_variable_at`; `AGENTS.md`
§ *Command registry* gains the word), `resolved_invocation.rs`
(`ResolvedInvocation::clause_plan(&self) -> Option<ClausePlan>`),
`traits.rs` (the two clause tables become `pub fn clause_keywords_without_command_spec() -> &'static [&'static str]`
and `clause_noise_keywords()` derived once from the shipped grammars and
memoised, with a unit test pinning them equal to today's two lists),
`commands/tcl/{if_,try_,catch_,for_,while_,foreach_,lmap_,dict,array_}.rs`
(the ten grammars as `pub const … : ClauseGrammarSpec`; `if_arg_roles`,
`check_if_shape`, `walk_if`, `try_arg_roles`, `foreach_arg_roles`,
`lmap_arg_roles` deleted; `arg_role_resolver: None` on those four specs;
`arg_role_resolver_roles` stays as the closed set the walk may emit;
`if`'s `clause_shape_check: None`), `rust/tcl-spec-studio/src/coverage.rs`
(`clause_grammar: _` in `witness_command_spec` and `witness_sub_command`;
`f("clause_grammar", Surface::Key("clause_grammar"))` in `COMMAND_SPEC`
and `SUB_COMMAND`), `rust/tcl-spec-studio/src/draft.rs` (seeding writes the
grammar as a JSON value — plain data, never `UNRENDERABLE_KEY`),
`rust/tcl-spec-studio/src/schema.rs` (a `clause_grammar` field of a new
`FieldKind::ClauseGrammar`, cluster "Structure" in `relations.rs`),
`rust/tcl-spec-studio/src/help.rs` (long-form help with the `try` example
from the page), `rust/tcl-spec-studio/src/render_spectcl.rs`
(`clause_grammar_block` beside `object_class_block`, emitting the CC2.3
spelling; a transient `GapKind::LoaderGap` row for `clause_grammar` is
**not** added — CC2.2 and CC2.3 land as one checkpoint so the round trip
never sees the field without its reader).
Rust: the page's types verbatim — `ClauseGrammarSpec`, `ClauseRow`,
`ClauseRowShape`, `ClauseSlot`, `ClauseTiming`, `LoopPhase`,
`ClauseSelection`, `DefaultClause`, `ClausePlan`, `ResolvedClause`,
`ClauseRowId` — with two adaptations: `ClauseGrammarSpec::head` is a
`ClauseRow` (keyword `None`, `keyword_required: false`, shape `Once`) so
the head carries the timing the page's own DSL row `head {Body} -timing protected`
declares (D2.1); `ClauseSlot::handler: Option<value_transfer::HandlerMatch>`
(D2.2). `pub enum ClauseRowId { Head, Row(u8), Tail }`. The walk,
`ClauseGrammarSpec::walk(&self, args: &[&str], layouts: &[RepeatedArgLayout]) -> ClausePlan`,
is the loader's `ClauseGrammar::walk` (`rust/tcl-spectcl/src/loader.rs:3840`)
moved into the registry and extended: the head is filled positionally; at
each position a `Repeated` row whose keyword matches starts, else the next
unfilled keywordless `Once` row in declaration order, else the tail, else
`ExtraWords`; a `Group` row consumes `layout.stride` words at a time until
`args.len() - layout.exclude_trailing`; a `?noise?` slot consumes its word
when present; `fallthrough_body` sets `falls_through_to` on the clause
whose body word equals it (the next clause with a body); `defect` is the
existing `ClauseShapeError`. `pub fn clause_keywords(reg) -> …` is the
derivation the traits tables move onto. The ten grammars: `if` and `try`
as the page spells them; `catch` — head `Once{[Body]}@Protected`, tail
`Once{[LoopVarList optional, LoopVarList optional]}@Selected`; `for` —
head `Once{[Body]}@LoopFixture(Init)`, rows `Once{[Expr]}@Selected`,
`Once{[Body]}@LoopFixture(Next)`, tail `Once{[Body]}@PerIteration` (D2.1);
`while` — head `Once{[Expr]}@Selected`, tail `Once{[Body]}@PerIteration`;
`foreach` / `lmap` / `dict for` / `dict map` — head empty, rows
`Group{layout: 0}@PerIteration`, tail `Once{[Body]}@PerIteration`;
`dict update` — head `Once{[Value]}` (the dict variable), rows
`Group{layout: 0}@PerIteration` with `conditional_binding: true` on the
var-list slot, tail `Once{[Body]}@Selected`; `array for` — as `dict for`
with `surface: Some(TCL90_PLUS)` on the grammar. Each grammar's role
answers equal the retired resolver's on that command's whole test matrix.
Preserve: every `arg_indices_for_role` answer and every
`ClauseShapeError` for every call shape in `if_.rs`'s 24 tests and
`try_.rs`'s tests (moved into `clause_grammar.rs` as `const` corpora over
the shipped grammars); E004 positions; the semantic-token keyword set; the
TextMate keyword lists. Changes: nothing observable. Tests (new, in
`clause_grammar.rs`): `if_grammar_agrees_with_the_retired_walk_on_its_corpus`
(24 rows), `try_grammar_agrees_with_the_retired_walk` (including `on ok`,
`trap {POSIX ENOENT}`, a `-` body), `foreach_group_row_cites_layout_zero`,
`array_for_is_absent_below_9_0` (negative: under `tcl8.6` `clause_plan`
answers `None`), `a_keyword_never_matches_inside_a_slot` (`if else {a}`);
`rust/tcl-registry/tests/registry_sweep.rs` gains
`clause_grammar_group_rows_cite_a_real_layout` and
`conditional_binding_clause_slots_never_carry_var_write` (the sibling of
`repeated_arg_layouts_never_pair_conditional_binding_with_an_ssa_def_role`)
and `clause_slots_use_only_the_six_roles`; `traits.rs` gains
`derived_clause_keyword_tables_equal_the_pinned_lists`. Gates:
`gen-tmlanguage-keywords --check` (byte-identical), `registry-axes`
(CC2.1's pins unchanged), `pack-goldens --check`, `reference_doc`
regenerated. Docs: `docs/design/compiler/command-registry.md` § *CommandSpec
field reference* gains `clause_grammar`; `registry-consumer-contracts.md`
§ *The clause-grammar descriptor* "today" sentences flipped. Model: opus.
Size: L. After: CC2.1.

**CC2.3 — `clause_grammar` in the loader, renderer and studio.**
Files: `rust/tcl-spectcl/src/loader.rs` (`ClauseGrammar` and `ClauseWalk`
deleted; `clause_grammar_block` builds a `ClauseGrammarSpec` with `leak_slice`
/ `leak_str`; `PackCommand::clause_grammar: Option<&'static ClauseGrammarSpec>`;
the derivation hooks recorded as today with `HookSource::Derived`;
`abstain_arg_roles` / `accept_clause_shape` no longer installed for a
grammar-carrying command — the registry walk answers, so the two placeholders
stay only for `-native` escapes), the subcommand block (`clause_grammar`
on a `subcommand` body), `docs/design/spec-dsl-examples/README.md`
(§ *Clause grammars, declaratively* and the coverage matrix row: `head {slots} ?-timing T?`,
`repeated KEYWORD {slots} ?-timing T? ?-pattern completion-code|error-code-prefix? ?-optional-keyword? ?-available V?`,
`once ?KEYWORD? {slots} ?-timing T?`, `group N ?-timing T?`,
`tail ?KEYWORD? {slots} ?-timing T?`, `fallthrough_body WORD`,
`default_clause ROW|tail ?-final-only?`, `selection first-match|all`;
timings `selected|always|per-iteration|init|next|protected`; a slot word
`?noise?` as today, `Pattern` carries the row's `-pattern`, a slot
`-conditional` suffix spelt `LoopVarList?` is **not** introduced: the
`-conditional` flag sits on the row and applies to its var-list slot),
`docs/design/spec-dsl-examples/{if,foreach}.tclspec` (`if` gains
`-timing`; `foreach` gains `clause_grammar { group 0 -timing per-iteration; tail {Body} -timing per-iteration }`),
`rust/tcl-spec-studio/src/render_spectcl.rs` (the block writer),
`rust/tcl-spec-studio/src/store.rs` (the `clause_grammar` presence flag
becomes the field itself), `rust/tcl-spec-studio/src/examples/fields_behaviour.rs`
(the example), `rust/tcl-spec-studio/web/src/editors.ts` (a read-only
structured view of the rows is enough for this step; editing rows is an
open question Q2).
Preserve: every existing port loads to the same spec; `spectcl_roundtrip`'s
`every_command_in_every_dialect_round_trips_through_spectcl` passes with
no `clause_grammar` gap. Tests: `rust/tcl-spec-studio/tests/spectcl_ports.rs`
— `the_clause_grammar_derivation_agrees_with_the_shipped_walk` widened to
`if`, `foreach` (both from the examples) and, over the shipped grammars,
every grammar-carrying command (the corpora of CC2.2), asserting roles and
defect per call; `derivations_are_recorded_as_derivations` unchanged;
`rust/tcl-spectcl/tests/eval_loader.rs` gains `a_clause_grammar_row_with_an_unknown_timing_is_dropped_with_a_notice`
(negative). Gates: `pack-goldens --check` (the EDA packs declare no
grammar; no snapshot moves), `reference_doc`. Docs: the memo rows above;
`docs/design/contracts/command-spec-studio.md` needs no change (no `GAPS`
row). Model: opus. Size: M. After: CC2.2 (same checkpoint).

**CC2.4 — `MemberEffect` in the registry.**
Files: `rust/tcl-registry/src/definer.rs` (`MemberSpec::effect: MemberEffect`,
required; the page's `MemberEffect`, `MemberReceiver`, `CallableRole`,
`StateScope`, `RelationSlot`, `InitTiming` verbatim; `MemberRow`,
`MemberArity`; `DefinitionBodyGrammar::member_row(&self, keyword_index: usize, words: InvocationArguments<'_>, dialect: Option<SurfaceQuery<'_>>) -> Option<MemberRow>`
— one member statement at a time, the shape the analyser already asks
(D2.6); every constructor (`flat`, `all_refs`, `all_vars`, `keyword_only`,
`wrapper`, `wrapper_or_body`, `flag_keyed`) takes the effect or a builder
`.effect(…)` sets it; the TclOO, snit and itcl rows get the effects the page
lists — `method`/`classmethod`/`typemethod`/`proc`/`constructor`/`destructor`/
`onconfigure`/`oncget` `Callable` with `name_slot`, `params_slot`,
`body_slot` positioned by their `arg_roles`' `Name`/`ParamList`/`Body`;
`forward` `Forward{name_slot: 0, prefix_slot: 1}`; `variable`/`typevariable`/
`common`/`component`/`typecomponent` `StateDeclaration`; `option`
`StateDeclaration{scope: Option}`; `superclass`/`mixin`/`filter`/`inherit`
`Relation`; `export`/`unexport` `Visibility`; `deletemethod`/`renamemethod`
`Retraction`; `typeconstructor` `InitScript{body_slot: 0, timing: AtDefinition}`;
`initialise`/`initialize` `InitScript{…, AtDefinition}`; `self`/`private`/
`public`/`protected` (wrappers) and `definitionnamespace`/`delegate`/`expose`/
`property` `Configuration`; every SpecTcl and SslicTcl document-grammar
member `Configuration` (D2.7)), `rust/tcl-spec-studio/src/coverage.rs`
(a new `witness_member_spec` and `MEMBER_SPEC` table, wired into the
`definition_body` draft once CC2.5 seeds it).
Preserve: `MemberSpec::indices_for_call*`, `option_for*`,
`declared_visibility_for*` unchanged. Tests: `rust/tcl-registry/tests/registry_sweep.rs`
gains `every_member_carries_an_effect_that_agrees_with_its_roles` (the
page's five rules: a `Callable` row's three slots index `Name`,
`ParamList`, `Body` in `arg_roles`; a `Relation` row carries `slot`; a
`Retraction` row carries `retraction`; a `Visibility` row carries
`visibility_effect`; a `Forward` row's prefix slot is `CommandPrefix`),
and `no_member_effect_names_a_family` (negative: the enum's `Debug`
strings contain none of `TclOo`, `Snit`, `Itcl`); `definer.rs` unit rows
for `member_row` on `method m {a b} {…}`, `self method`, `superclass -append B`
(`slot_op`), `forward f ::x`, a dynamic name word (`name: None`).
Gates: none beyond the crate tests. Docs: `command-registry.md`
§ *CommandSpec field reference* (`definition_body` paragraph gains the
effect); `docs/design/contracts/tcloo-implementation.md` if it lists
member rows (the implementer checks and updates the row table). Model:
opus. Size: M. After: CC2.1.

**CC2.5 — `definition_body` and `semantic_operation` leave `GAPS`;
`-effect` on the member row.**
Files: `rust/tcl-spectcl/src/loader.rs` (`member_row` reads `-effect {callable -receiver R -role K ?-name N? ?-params N? ?-body N?}` / `{forward -name N -prefix N}` / `{state-declaration per-instance|per-type|option}` / `{relation superclass|mixin|filter}` / `visibility` / `retraction` / `{init-script -body N -timing at-definition|at-construction}` / `configuration`,
the page's spelling; a row without `-effect` is a notice and the member is
dropped), `rust/tcl-spec-studio/src/draft.rs` (seeding writes a shipped
grammar by name when the pointer is one of `TCLOO_GRAMMAR`,
`TCLOO_CONFIGURABLE_GRAMMAR`, `SNIT_GRAMMAR`, `SNIT_WIDGET_GRAMMAR`,
`ITCL_GRAMMAR` (`std::ptr::eq`) and the full block otherwise; seeding
writes `semantic_operation` as `{kind, detail}` from `kind_str` /
`detail_str`; both keys leave `UNRENDERABLE_KEY`),
`rust/tcl-spec-studio/src/render_spectcl.rs` (`definition_body NAME` or
the inline block with `-effect` on every member; `semantic_operation Invoke|{Intrinsic ID}|{StructuredLowering ID}`;
the two `GAPS` rows deleted), `schema.rs` / `help.rs` / `examples/` (the
member-row form gains the effect control; `semantic_operation` becomes a
closed-vocabulary field), `rust/tcl-spec-studio/web/src/editors.ts`
(member-row effect select), `docs/design/spec-dsl-examples/README.md`
(matrix rows; `snit-type.tclspec` and `oo-class.tclspec` gain `-effect`).
Preserve: every port loads to the same grammar. Tests:
`spectcl_roundtrip.rs` — `every_command_in_every_dialect_round_trips_through_spectcl`
with two fewer gap keys (the report's gap register shrinks);
`the_eleven_port_fixtures_render_and_reload_as_themselves` unchanged;
`spectcl_ports.rs` gains `a_member_row_without_an_effect_is_dropped_with_a_notice`
(negative); `rust/tcl-spectcl/tests/eval_loader.rs` gains
`semantic_operation_round_trips_through_the_renderer`. Gates:
`pack-goldens --check` (snapshots unchanged unless an EDA pack declares a
`definition_body`; the implementer regenerates and stages if one does),
`reference_doc`. Docs: `command-spec-studio.md` § *Fields that cannot
round-trip* loses the two rows; `registry-consumer-contracts.md` § *The
studio round-trip for `semantic_operation`* flips to past tense. Model:
opus. Size: M. After: CC2.4.

**CC2.6 — `OptionEffect` in the registry, and the four clients.**
Files: `rust/tcl-registry/src/option_effect.rs` (new: `OptionEffect`,
`OptionEffectKind`, `EffectAxis`, `SubstitutionKind`, `OptionEffectFamily`,
`FamilyBase`, `FamilyCombine`, `OptionEffects`, verbatim from the page;
`pub fn option_effects(spec_options: &[OptionSpec], families: &[OptionEffectFamily], args: InvocationArguments<'_>, reserved_trailing_words: usize, dialect: Option<SurfaceQuery<'_>>) -> OptionEffects`
— the walk of `lsearch_pattern_args` made generic: the scan ends at
`reserved_trailing_words` before the end, spellings resolve through
`resolve_available_option_prefix` (made `pub(crate)` for the module), an
unresolvable hyphenated word or a non-literal word makes `complete: false`
and every axis value `true`; `Accumulate` applies each effect,
`LastWins` keeps the last accepted; `EndsOptions` stops the scan;
`SuppressesRole` and `ReservesTrailingWords` are reported as `shifts`),
`hover.rs` (`OptionSpec::effect: Option<OptionEffect>`), `spec.rs`
(`CommandSpec::option_effect_families: &'static [OptionEffectFamily]`,
`SubCommand::option_effect_families`; `CaseListSpec` loses `regex_option`,
`exact_option`, `glob_option`, `nocase_option`, `end_options_option`;
`CaseListSpec::invocation` reads the command's `OptionEffects` — axes
`Selection(mode)` and `CaseSensitivity`, and `EndsOptions` — through a
new parameter `effects: &OptionEffects`; `substitution_resolver` deleted
from `CommandSpec`; `pattern_arg_resolver` stays), `substitution.rs`
(`SubstitutionResolver` and `subst_substitutions` deleted;
`SubstitutionKinds` stays; the three `tp_*` unit rows move to
`option_effect.rs` as rows of the generic derivation), `patterns.rs`
(`pattern_args(spec, inv) -> Vec<PatternArg>`: the resolver escape hatch
first, else the projection of `EffectAxis::PatternLanguage` onto the
pattern operand at `option_end + 1`), `registry.rs`
(`substitutions_performed` becomes the projection of `option_effects`
onto `SubstitutionKinds`; signature unchanged; the mixed-family call no
longer answers `ALL` silently — see the relation below),
`resolved_invocation.rs` (`ResolvedInvocation::option_effects(&self) -> OptionEffects`,
`pattern_args(&self)`), `commands/tcl/subst_.rs` (six rows gain `effect`,
two families `negated {AllOn, Accumulate}` and `positive {AllOff, Accumulate, surface: TCL91}`,
`reserved_trailing_words: 1`, and one `OptionRelation` of
`RelationKind::MutuallyExclusive` over the two term sets — W147 at the
call site, produced by the existing relation check in
`analyser/diagnostics/validity.rs:2127` with no new consumer code),
`commands/tcl/lsearch_.rs` (`lsearch_pattern_args` deleted;
`pattern_arg_resolver: None`; `-glob`/`-regexp` `Selects(PatternLanguage(…))`,
`-exact`/`-sorted` `Disables(PatternLanguage(Glob))`, family `match {Only(PatternLanguage(Glob)), LastWins}`;
`reserved_trailing_words: 2` stays), `commands/tcl/regexp_.rs`
(`-inline` `SuppressesRole(VarWrite)`, `-about` `ReservesTrailingWords(1)`;
`regexp_arg_roles` stays as the argument-role hook body of § *The two
hook bodies that remain* and reads `option_effects().shifts` for the
suppression and the reservation instead of assigning `VarWrite` to every
trailing word and assuming two reserved operands), `commands/tcl/switch_.rs`
(`-exact`/`-glob`/`-regexp` `Selects(Selection(…))`, `-integer`
`Selects(Selection(Other))`, `-nocase` `Selects(CaseSensitivity)`, `--`
`EndsOptions`, family `match {Only(Selection(Exact)), LastWins}`),
`commands/tcl/case_.rs` and the expect specs (their `CaseListSpec`s lose
the five fields; `case` has no options; expect's clause flags are not
options and stay on the descriptor), `rust/tcl-spec-studio/src/coverage.rs`
(`effect: _` in `witness_option_spec`; `f("effect", Surface::Key("effect"))`;
`option_effect_families: _` in the two spec witnesses; the deleted
`substitution_resolver` leaves `COMMAND_SPEC`, `schema.rs`, `help.rs`,
`draft.rs`, `relations.rs`, and its `GAPS` row; `pattern_arg_resolver`
keeps its schema entry and loses its `GAPS` row because no shipped spec
sets it — the round trip therefore never sees it set).
Preserve: every `substitutions_performed` answer for a single-family call
(the rows moved from `substitution.rs`), every `lsearch_pattern_args` case
including `lsearch -regexp -glob` searching the list `-regexp` for the glob
pattern `-glob`, every `regexp` role answer for calls without `-inline` or
`-about`, every `CaseListSpec::invocation` answer. Changes: a mixed-family
`subst` call answers `complete: false` and every kind on (W102 advice
unchanged) **and** draws W147; `regexp -inline … var` no longer assigns
`VarWrite` to the trailing words (they are an error the relation reports:
add `OptionRelation` `-inline` forbids positional 2+, message from
`tclsh`); `regexp -about exp` resolves its expression with one reserved
operand. Tests: `option_effect.rs` unit rows (`no_switches_runs_every_substitution`,
`a_negated_switch_turns_off_only_its_own_kind`, `a_positive_switch_turns_on_only_its_own_kind`,
`the_two_families_mixed_are_unreadable_and_every_kind_is_on`,
`an_abbreviation_the_table_cannot_resolve_is_unreadable`,
`a_computed_switch_word_is_unreadable`, `lsearch_last_style_wins`,
`lsearch_regexp_glob_scans_no_option_past_the_reserved_words`,
`switch_dash_dash_ends_options`); `registry.rs`'s
`substitutions_performed_answers_per_call_and_only_for_substituting_commands`
unchanged; `registry_sweep.rs` gains `every_option_effect_names_a_declared_family`
and `a_family_base_mentions_only_axes_its_options_mention`;
`rust/tcl-registry/tests/tcl91_dialect.rs` gains
`subst_positive_family_is_91_only` and
`subst_mixed_families_error_matches_tclsh91` (the differential row: runs
`subst -nocommands -variables x` on the `tclsh9.1` the value-transfers
lane's `differential_fold.rs` locates through `PATH`, asserts the error
message the relation's `-message` carries; skips with a note when no
`tclsh9.1` is on `PATH`, as that file does); `regexp_.rs` gains
`inline_suppresses_the_capture_var_writes` and `about_reserves_one_operand`.
Gates: `audit-option-dialects --check` (the option rows keep their
surfaces), `callback-inventory --check`, `pack-goldens --check`,
`reference_doc`. Docs: `command-registry.md` § *OptionSpec and option
terminators* gains the effect and the family; `registry-consumer-contracts.md`
§ *Options with semantic effects* "today" sentences flipped. Model: opus.
Size: L. After: CC2.1.

**CC2.7 — `option -effect` and `option_effect_family` in the loader,
renderer and studio.**
Files: `rust/tcl-spectcl/src/loader.rs` (`option_row` reads
`-effect {disables|selects AXIS VALUE} | {suppresses-role ROLE} | {reserves-trailing-words N} | ends-options`
and `-family NAME`; the pack-level and subcommand-level
`option_effect_family NAME { base all-on|all-off|{only AXIS VALUE} combine accumulate|last-wins ?-introduced V? }`
statement; axes spelt `substitution backslashes|commands|variables`,
`pattern-language glob|regex|…` (the `PatternType` names), `case-sensitivity`,
`selection exact|glob|regexp|other`), `render_spectcl.rs` (`option_row`
emits `-effect` / `-family`; a `family` writer), `schema.rs` / `help.rs` /
`examples/` / `relations.rs` (the option-row form gains the two controls;
the families a list field under "Options"), `rust/tcl-spec-studio/web/src/editors.ts`
(the controls patch non-structurally: `patch({effect: …}, false)`),
`docs/design/spec-dsl-examples/README.md` (§ *Option rows* and the matrix),
`docs/design/spec-dsl-examples/string.tclspec` or a new
`subst.tclspec` port carrying the page's `command subst { … }` example
verbatim (the port is the round-trip fixture).
Preserve: every existing pack loads unchanged. Tests:
`rust/tcl-spec-studio/tests/option_row_editing.rs` gains
`option_effect_edits_are_marked_non_structural` and
`switching_the_effect_kind_still_rebuilds`; `spectcl_ports.rs` gains
`the_subst_port_answers_the_same_kinds_as_the_shipped_spec` over the
`option_effect.rs` corpus; `eval_loader.rs` gains
`an_effect_naming_an_undeclared_family_is_a_notice` (negative). Gates:
`pack-goldens --check`, `reference_doc`. Docs: `command-spec-studio.md`
§ *Fields that cannot round-trip* loses `substitution_resolver` and
`pattern_arg_resolver`. Model: sonnet (the spelling is given by CC2.6 and
the page). Size: M. After: CC2.6.

**CC2.8 — the derived-query layer.**
Files: `rust/tcl-registry/src/resolved_invocation.rs` (inherent methods on
`ResolvedInvocation`: `clause_plan` (CC2.2), `option_effects` and
`pattern_args` (CC2.6), `case_invocation(&self) -> Option<(CaseInvocation, Vec<InlineCaseClause>)>`
re-keying `CaseListSpec::invocation` and `inline_clauses`,
`frame_effect(&self) -> Option<(FrameLevel, Vec<OperandId>)>` re-keying
`FrameEffectSpec` over the call's words, `arg_roles(&self) -> Vec<(usize, ArgRole)>`
re-keying `arg_indices_for_role_words` with the CC2.2 order,
`return_type(&self) -> Option<TclType>`, `effects(&self) -> EffectFootprint`
(the existing `effect_footprint`, renamed with a deprecated alias for one
checkpoint); `member_rows` is `DefinitionBodyGrammar::member_row` (D2.6);
`template_plan` is not added — slice 5's (D2.4)), `value_transfer/context.rs`
(`AnalysisContext::surface_query(&self) -> Option<SurfaceQuery<'_>>` from
`profile`), `registry.rs` (`CommandRegistry::invocation(&self, words: InvocationWords<'w>, ctx: &AnalysisContext) -> StructuredInvocationResolution`,
the page's `invocation(words, ctx)`: `resolve_structured_invocation(words, ctx.surface_query())`).
The three rules hold by construction: every answer is a value; every
answer carries its abstention (`None`, `complete: false`, `name: None`);
every answer is computed under the invocation's `SurfaceQuery`, which the
context fixes.
Preserve: every existing answer of the re-keyed functions; the old entry
points stay for one checkpoint and are removed by CC2.9–CC2.11 as their
callers move. Tests: `resolved_invocation.rs` unit rows — one per query on
a literal call, a computed-head call (`None`), and a call under a release
that lacks the command (`None`); `registry_sweep.rs`'s
`sweep_every_command_every_accessor` extended to call every query on every
command with a representative shape (no panic, no `unwrap`). Gates: none.
Docs: `command-registry.md` § *How registry feeds the compiler* gains a
paragraph naming the queries; the page's § *The derived-query layer*
"proposed" note flips. Model: opus. Size: M. After: CC2.2, CC2.6.

**CC2.9 — clause consumers in the compiler.**
Files: `rust/tcl-compiler/src/ir.rs` (`TryHandler::kind: HandlerMatch`
replaces the `String`; `IfClause` unchanged), `lowering/structured.rs`
(`lower_if`, `lower_try` build from `ResolvedInvocation::clause_plan`:
clauses with `timing == Selected` become `IfClause` / `TryHandler`,
`Always` becomes `finally_body`, `falls_through_to` the fall-through
marker; the `"elseif"` / `"else"` / `"then"` / `"finally"` / `"on"` /
`"trap"` / `"-"` literals are gone), `executable_ir.rs:3452`
(`handler.kind == HandlerMatch::ErrorCodePrefix`),
`cfg_builder/cfg_lower.rs:955` (`is_on_ok` becomes
`handler.kind == HandlerMatch::CompletionCode && tcl_registry::completion::CompletionCode::from_word(&handler.match_arg) == Some(CompletionCode::Ok)`
— the registry's own completion-code word parse at
`rust/tcl-registry/src/completion.rs:56`, made `pub` if it is not; the
page's `is_default` does not apply because `try` declares no
`default_clause` — see D2.8), `analyser/handlers.rs`
(`handle_try_command` walks the plan: `Protected` and `Selected` bodies
through `analyse_selected_body`, `Always` through `analyse_body`, the
var-list slot through `define_vars_from_list`, a `falls_through_to` clause
skipped; `handle_for_command` deleted — the generic body walk in
`dispatch_body_arguments` bumps `control_flow_body_depth` for a
`PerIteration` or `LoopFixture` body and `conditional_depth` for a
`Selected` one, reading the plan), `analyser/commands.rs`
(`orphaned_keyword_parent` becomes a lookup of the word in
`clause_keywords()`'s owner map — `clause_grammar::owner_of_keyword(word) -> Option<&'static str>`
added to CC2.2's module and used here; `:1708` reads
`clause_plan().and_then(|p| p.defect)` first, then the `clause_shape_check`
escape hatch), `signature_scan/walker.rs` (the two `if` walks and the two
`try` walks at lines 428–440, 487–498, 584–594, 622–626 read the plan
through a `SignatureScan::clause_plan_for(seg)` helper).
Preserve: every `rust/tcl-compiler/tests/{cfg,analyser}.rs` `if` / `try` /
loop case byte-identical; every `signature_scan.rs` test; E004 positions.
Changes: none observable. Tests: `lowering/structured.rs`'s existing
`try` tests updated to `HandlerMatch` (the `("on", "ok", true)` triples
become `(HandlerMatch::CompletionCode, "ok", true)`);
`rust/tcl-compiler/tests/cfg.rs` gains
`a_try_handler_walk_reads_timing_not_keywords` (a pack command declaring a
`try`-shaped grammar under a different keyword set lowers the same CFG —
the negative control: the same body without the grammar stays one opaque
call). Gates: `registry-axes` pins for `lowering/structured.rs`,
`executable_ir.rs`, `cfg_lower.rs`, `analyser/handlers.rs`,
`analyser/commands.rs`, `signature_scan/walker.rs` lowered to zero and
the files added to `CLEAN_FILES`; `value-transfers --check` (the
`lowering/structured.rs` and `analyser/commands.rs` value-transfer pins
are the other lane's — untouched, see B1). Docs: `docs/design/compiler/lowering-dispatch.md`
(the `if`/`try` lowering paragraph names the plan). Model: opus. Size: L.
After: CC2.8.

**CC2.10 — clause consumers in the editor and tool tiers.**
Files: `rust/tcl-lsp-core/src/refactor/if_to_switch.rs:179–199`,
`rust/tcl-lsp-core/src/refactor/datagroup.rs:445` (`parse_if_chain`),
`rust/tcl-mcp/src/datagroup.rs:422`, `rust/tcl-lsp-core/src/semantic_tokens.rs`
and `minify.rs` (the two tables' functions from CC2.2),
`rust/xtask/src/gen_tmlanguage_keywords.rs` (same), `rust/tcl-lsp-core/src/formatting/`
and `rust/tcl-compiler/src/analyser/recovery.rs` (the `then` distinction
read from the slot's `noise` through `clause_noise_keywords()` where a
literal `"then"` remains; the implementer greps `"then"` in both).
Preserve: every refactor, formatter, minifier and semantic-token test;
the generated TextMate lists byte-identical. Tests:
`rust/tcl-lsp-core/tests/lsp_providers.rs` or the refactor modules' own
tests gain `if_to_switch_reads_the_clause_plan` with a pack-declared
`if`-shaped command (negative: a command without a grammar is not
converted). Gates: `gen-tmlanguage-keywords --check`, `registry-axes`
pins lowered to zero for the six files. Model: sonnet. Size: M. After:
CC2.9.

**CC2.11 — member consumers.**
Files: `rust/tcl-compiler/src/analyser/oo.rs` (`apply_oo_subcommand_in`'s
eleven arms become one `match member_row.effect`: `Relation` folds through
`apply_slot_member` on the graph `RelationSlot` names; `Callable` keyed on
`CallableRole` and the resolved `receiver` — `Instance`+`Method` →
`methods`, `TypeObject`+`Method` → `class_methods`, `Constructor` →
`constructors`, `Destructor` → `destructor`; `StateDeclaration` →
`variables` through `apply_slot_member`; `Forward` → `apply_oo_forward`;
`Configuration` on a wrapper → `apply_oo_self` / `apply_oo_private`
selected by the wrapper's own shift (the side is the row's resolved
`receiver`); `property`'s flag-keyed accessors stay `extract_property_defs`
until they are `Callable` rows of their own — recorded on the ledger with
expiry "the 9.0 `property` accessor rows"); `dispatch_snit_member` and
`parse_itcl_definition_body` route through the same `match`, deleting the
snit and itcl prefix conventions (33 keyword literals);
`member_method_kind` in `lowering/mod.rs` is replaced by
`MethodKind::from_effect(role: CallableRole, receiver: MemberReceiver) -> Option<Self>`
in `ir.rs`, `from_str_lossy` deleted with its tests; `extract_one_member`
reads the row), `rust/tcl-lsp-core/src/{references,folding,oo_body,document_symbols,definition,hover,workspace_index,workspace_symbols}.rs`
(the `"constructor"` / `"destructor"` literals read `MethodDef::kind`).
Preserve: `rust/tcl-compiler/tests/mro_lattice_adversarial.rs`,
`analyser.rs`, `rust/tcl-lsp-core/tests/oo_mro_parity.rs`, every OO
provider test byte-identical. Changes: none observable for the shipped
grammars. Tests: `rust/tcl-lsp-server/tests/preview_tickets_e2e.rs` gains
`a_pack_declared_member_spelling_reaches_every_provider` — a workspace
pack declaring `definition_body { family Snit member mymethod -roles {0 Name 1 ParamList 2 Body} -effect {callable -receiver instance -role method} … }`
on a private definer; document symbols, go-to-definition and hover see
`mymethod` with no consumer edit; the file's comment that "a new definer
spelling needs an `apply_oo_subcommand` arm" is deleted; negative: the
same pack with `-effect configuration` yields no method. Gates:
`registry-axes` pins for `oo.rs`, `lowering/mod.rs` and the eight
provider files lowered; `value-transfers --check` untouched (the `oo.rs`
value-transfer pin is the other lane's, B1). Docs: `docs/design/contracts/tcloo-implementation.md`
(the arm table becomes the effect table); `AGENTS.md` § *The registry is
the source of truth* keeps its sentence. Model: opus. Size: L. After:
CC2.4, CC2.8.

**CC2.12 — scope, namespace and interpreter transitions; loop and bind
by role; the special-variable pack statement; the package layout.**
Files: `rust/tcl-compiler/src/analyser/handlers.rs` (one generic
`apply_state_transitions(&mut self, inv: &ResolvedInvocation, arg_tokens, scope_path)`
consuming `VariableCellAliasTransition` (the alias the handler recorded by
hand: `define_var(local, …)` plus the `link_target_span` unification the
`global` / `variable` / `upvar` / `namespace upvar` handlers perform today)
and `NamespaceTransition` (namespace variable declarations); the four
handlers `handle_global_command`, `handle_variable_command`,
`handle_upvar_command`, `handle_namespace_upvar_command` are deleted once
their unit tests pass through the generic consumer — `handle_global_defines_each_name`,
`handle_variable_defines_only_names_skipping_values`,
`handle_variable_single_name_no_value` and the `upvar` tests are retargeted;
`handle_dict_for_command`, `handle_dict_update_command`,
`handle_incr_command`, `handle_append_lappend_command` deleted — the
generic `handle_var_binding_command` binds every `LoopVarList` and
`VarWrite` position the roles name, with `warn_if_unused` false for a
command whose `semantics` resolves to a cell read-modify-write (the
value-transfers lane's derived descriptor on `append` / `lappend` /
`incr`; `incr` today passes `true`, a delta recorded below);
`handle_foreach_command` keeps only the literal-iteration simulation and
binds through roles; `handle_package_require` reads `-exact` through
`option_effects` (`package_.rs` gains `EndsOptions`-free rows as today
plus a `Selects(…)`-free read: the presence check is
`OptionFacts::options`), the index arithmetic replaced by the roles the
spec declares), `rust/tcl-registry/src/commands/tcl/package_.rs`
(`arg_roles` for `require` / `provide` / `ifneeded` where missing),
`rust/tcl-registry/src/special_vars.rs` and `rust/tcl-spectcl/src/loader.rs`
(a pack-level `special_var NAME -kind K -access A -origin O ?-dialects {…}? ?-startup B?`
statement building a `SpecialVarSpec`; `special_vars_for_dialect` reads
pack-declared rows from the overlaid registry — a
`CommandRegistry::special_vars()` door the loader fills; the `lappend auto_path`
arm and `set auto_path` arm become one read of `special_var("auto_path")`'s
`VarAccess`), `docs/design/spec-dsl-examples/README.md` (the row).
Preserve: every `handlers.rs` and `analyser.rs` test except the delta
below. Changes: `incr x` with `x` never read afterwards no longer draws
W211 (the target is read before it is written; `append` already reads so);
a `namespace upvar` with a dynamic local still skips it (the transition
abstains — the witness the page requires before the isolated per-item
pass consumes transitions: `alias_and_binding_resolvers_abstain_on_a_dynamic_word`
in `rust/tcl-registry/src/state_transition.rs`). Tests: the retargeted
handler tests; `rust/tcl-compiler/tests/analyser.rs` gains
`a_pack_declared_scope_alias_binds_its_local` (a pack command with
`frame_effect -level-word … -layout AliasPairs` and a `state_transitions`
resolver emitting alias facts binds like `upvar`; negative: the same
command with a dynamic level word binds nothing) and
`a_pack_declared_special_variable_is_readable_at_startup`. Gates:
`registry-axes` pins for `handlers.rs` lowered; `analyser_hooks`
re-baselined by CC2.13. Docs: `docs/design/registry/special-variable-registry.md`
gains the pack statement; `spec-packs.md` § *What a pack still cannot say*
loses "the `state_transitions` resolver … today" sentence once CC2.14
lands the resolver family. Model: opus. Size: L. After: CC2.8, CC2.14.

**CC2.13 — hook retirement and the re-baseline.**
Files: `rust/tcl-registry/src/hooks.rs`, the specs that carried the
variants, `rust/tcl-compiler/src/analyser/commands.rs` (`dispatch_analyser_hook`),
`rust/tcl-registry/tests/analyser_hooks.rs`.
Retire (the handler's only command-specific knowledge was a position or a
keyword a descriptor now states): `Try` (CC2.9), `For` (CC2.9), `DictFor`,
`DictUpdate`, `Incr`, `Append`, `Lappend`, `Upvar`, `NamespaceUpvar`,
`Global`, `Variable` (CC2.12) — eleven variants, and the twelve stamps that
carried them (`::tcl::dict::for` and `::tcl::dict::update` included).
Keep, documented at the call site as the page's residue: `Proc`, `OptProc`,
`Apply` (procedure definition and the `all_procs` table), `Uplevel` and
`NamespaceEval` (dynamic-target synthetic domains), `NamespaceEnsemble`,
`NamespaceImport`, `NamespaceExport`, `NamespaceForget`, `NamespacePath`,
`NamespaceUnknown` (export tombstone ordering and the namespace domain),
`Foreach` (literal-iteration simulation), `Switch` and `Catch` (case-list
walk and completion protocol), `InterpAlias`, `InterpEval`, `InterpCreate`,
`InterpDelete`, `InterpHide`, `InterpExpose` (the interpreter-domain
stack and value binding of a created interpreter), `Rename` (rename
epochs), `OoDefine`, `OoObjdefine` (member routing to `ClassDef` fields),
`PackageRequire`, `PackageProvide`, `PackageIfneeded`, `PackagePrefer`
(package-index bookkeeping), `Source`, `Load`. On the ledger (command-
specific, retired by a named slice): `Set` and `DictWith` (the value axis,
slices 5 and 8), `RegexPatternCapture` (slice 5). The pinned set in
`analyser_hook_stamps_match_the_former_guard_list` shrinks from 43
variants to 32 and the comment on each kept row names its residue class;
`analyser_hook_stamps_are_disjoint_from_definer_families` unchanged.
Tests: the re-baselined pinned set; `hooks.rs`'s own count assertion if
one exists. Gates: `registry-axes` (the ledger rows for `Set`, `DictWith`,
`RegexPatternCapture` carry `until slice 5|8`). Docs: `docs/design/compiler/command-registry.md`
§ *How registry feeds the compiler* (the residue list). Model: sonnet
(the table above is the design). Size: S. After: CC2.9, CC2.12.

**CC2.14 — the `state_transitions` resolver family in the loader.**
Files: `rust/tcl-registry/src/pack_hooks.rs` (`HookFamily::StateTransitionResolver`,
verbs `alias IDX-LOCAL IDX-TARGET ?-level W?` and `namespace-variable IDX`,
silence "no transitions", `field` `state_transitions.resolver`;
`HOOK_FAMILIES` becomes `[HookFamily; 12]`; the dispatch thunk returns a
`StateTransitions` holding only `VariableCellAliasTransition` and
`NamespaceTransition` facts — any other verb is not injected, so the
family cannot emit the four forbidden families; a dynamic target word
abstains and the abstention widens through `StateTransitionWidening`),
`rust/tcl-spec-hooks/src/emit.rs` (the two verbs), `rust/tcl-spectcl/src/loader.rs`
(`state_transitions_value` reads `resolver {words ctx} {…} | -native ID | none | from-frame-effect`,
`argument_shape`, `widen`, `covers`, `commit` — the drop list loses the
five rows; the comment "reference-only by design" deleted),
`rust/tcl-spec-studio/src/render_spectcl.rs` (`state_transitions`'s `GAPS`
row shrinks to the resolver's `-native` form: `DraftOpaque` stays for a
descriptor naming a native resolver, and a pack-authored body renders),
`docs/design/spec-dsl-examples/README.md` (the rows).
Tests: `rust/tcl-spectcl/src/loader.rs`'s `native_hook_tables_cover_their_catalogues`
covers the new family; `rust/tcl-spec-hooks/tests/families_e2e.rs` gains
`a_state_transition_body_emits_alias_facts_only` (negative: a body calling
`command-binding …` fails to compile because the verb is not defined in
its sandbox); `rust/tcl-spectcl/tests/spec_corpus.rs`'s baseline loses the
"`state_transitions` row … is not yet loadable" notices (regenerate
`spec_corpus_baseline.txt`). Gates: `callback-inventory --check` (a new
executable position tier: the resolver body), `pack-goldens --check`.
Docs: `spec-packs.md` § *What a pack still cannot say* (the "today"
sentence goes). Model: opus. Size: M. After: CC2.1.

**CC2.15 — the lint's roots shared, the ledger's expiries closed, and the
step's documents.**
Files: `rust/xtask/src/util.rs` (`pub const ANALYSIS_TIER_ROOTS` used by
both gates — lands only after the value-transfers lane's slice 2 commit,
B1), `rust/xtask/src/registry_axes.rs` (the landed-set constant gains
step 2; every `until step 2` waiver must be gone), KCS notes:
`docs/kcs/compiler/kcs-qa-where-does-a-clause-shape-come-from.md`
(Contributor; the grammar, the plan, the escape hatch),
`docs/kcs/compiler/kcs-qa-what-does-a-member-effect-say.md`,
`docs/kcs/kcs-howto-declare-an-option-effect-in-a-tclspec-pack.md`
(User/Contributor; the `subst` example), each indexed in
`docs/kcs/README.md` and `docs/kcs/compiler/README.md`; `docs/GLOSSARY.md`
entries for *clause grammar*, *member effect*, *option effect*;
`docs/design/README.md` and `docs/design/compiler/README.md` index lines
for `registry-consumer-contracts.md` drop "proposal" for the description
contract. Gates: `kcs-index-links`, `registry-axes --check`. Model:
sonnet. Size: S. After: every other CC2 item.

#### Ordering and checkpoints

1. CC2.1 → checkpoint `wip(consumer-contracts): the registry-axes gate and its baseline`.
   Green: `cargo check -p xtask`, `cargo test -p xtask registry_axes`,
   `cargo xtask registry-axes --check`, `owner-resolution`.
2. CC2.2 + CC2.3 → one checkpoint `… clause grammars as data`. Green:
   `cargo check --workspace`; `cargo test -p tcl-registry` (lib,
   `registry_sweep`, `analyser_hooks`); `cargo test -p tcl-spectcl --test eval_loader --test spec_corpus`;
   `cargo test -p tcl-spec-studio` (lib, `spectcl_roundtrip`, `spectcl_ports`,
   `schema_coverage`, `reference_doc`); `cargo test -p tcl-compiler --test cfg --test analyser`
   (unchanged behaviour); `gen-tmlanguage-keywords --check`; `pack-goldens --check`.
3. CC2.4 + CC2.5 → `… member effects and the studio round trip`. Green as
   in 2 plus `cargo test -p tcl-compiler --test mro_lattice_adversarial`.
4. CC2.6 + CC2.7 → `… option effects`. Green as in 2 plus
   `cargo test -p tcl-registry --test tcl91_dialect`, `audit-option-dialects --check`,
   `callback-inventory --check`.
5. CC2.8 → `… the derived-query layer`. Green: `cargo test -p tcl-registry`.
6. CC2.14 → `… the state-transition resolver family`. Green:
   `cargo test -p tcl-spec-hooks`, `cargo test -p tcl-spectcl`,
   `callback-inventory --check`.
7. CC2.9 → `… lowering and the analyser read the clause plan`. Green:
   `cargo test -p tcl-compiler` in full; `registry-axes --check`.
8. CC2.10 → `… the editor tiers read the clause plan`. Green:
   `cargo test -p tcl-lsp-core`, `cargo test -p tcl-mcp`,
   `gen-tmlanguage-keywords --check`.
9. CC2.11 → `… members by effect`. Green: `cargo test -p tcl-compiler`,
   `cargo test -p tcl-lsp-core`, `cargo test -p tcl-lsp-server --test preview_tickets_e2e`.
10. CC2.12 → `… transitions, roles and special variables`. Green:
    `cargo test -p tcl-compiler`, `cargo test -p tcl-registry`.
11. CC2.13 → `… retire eleven analyser hooks`. Green:
    `cargo test -p tcl-registry --test analyser_hooks`, `cargo test -p tcl-compiler --test analyser`.
12. CC2.15 → `… step 2 documents and the shared lint roots`. Green:
    `make rust-check`.

#### Review checklist

- The registry rule: no new `match cmd_name { "…" => … }` anywhere outside
  `rust/tcl-registry`; `registry-axes --check` and `value-transfers --check`
  both green with no pin raised.
- No new `#[allow]`; UK spelling in every new identifier and comment
  (`serialise`, `initialise`, `behaviour`, `catalogue`); the AGPL header on
  every new `.rs` (`clause_grammar.rs`, `option_effect.rs`,
  `registry_axes.rs`) and on no generated or fixture file.
- The page's identifiers verbatim: `ClauseGrammarSpec`, `ClauseRow`,
  `ClauseRowShape`, `ClauseSlot`, `ClauseTiming`, `LoopPhase`,
  `ClauseSelection`, `DefaultClause`, `ClausePlan`, `ResolvedClause`,
  `ClauseRowId`, `MemberEffect`, `MemberReceiver`, `CallableRole`,
  `StateScope`, `RelationSlot`, `InitTiming`, `MemberRow`, `MemberArity`,
  `OptionEffect`, `OptionEffectKind`, `EffectAxis`, `SubstitutionKind`,
  `OptionEffectFamily`, `FamilyBase`, `FamilyCombine`, `OptionEffects`; the
  two adaptations D2.1 and D2.2 and nothing else.
- Every descriptor moved all four surfaces in one checkpoint (registry,
  loader, renderer, studio form) or the `GAPS` table names it; the
  `spectcl_roundtrip` gap register lists exactly the rows the page leaves
  (`frame_effect`, `world_effects`, `state_transitions` (resolver only),
  `event_*`, `case_list`, `body_scope`, `bpf_op`, `data_collection`,
  `side_switch_target`, `irules_top_level_effect`, `result_stability`,
  `remote_method`, `variable_write_min_args`, `body_interpreter`,
  `completion`, `dispatch_dependencies`, `native_lowering`, `semantics`).
- The parity gates named per item are byte-identical, not "mostly":
  `cfg.rs`, `analyser.rs`, `mro_lattice_adversarial.rs`, `signature_scan.rs`,
  `lsp_providers.rs`, `oo_mro_parity.rs`, the TextMate lists.
- Risks: the walk's keyword rule (a keyword never matches inside a slot —
  `if else {a}` is the witness); `foreach`'s `Group` row and
  `exclude_trailing` off by one on a two-word call; `for`'s keywordless
  rows filled in the wrong order; the `regexp -inline` role change reaching
  W210/W211 through the SSA def roles (the `stub_arg_roles.rs` and
  `analyser.rs` regexp cases are the gate); `incr`'s W211 delta accepted
  and recorded; a retired hook whose handler had a second effect (the
  retargeted unit tests are the proof, never the dispatch table).

#### Behavioural deltas expected

- `subst -nocommands -variables …` draws W147 (mutually exclusive options)
  beside W102; no other output changes.
- `regexp -inline … a b` draws the new option relation's finding instead of
  binding `a` and `b`; `regexp -about exp` no longer reports a missing
  operand.
- `incr x` with `x` unread afterwards draws no W211.
- A workspace pack may declare a `state_transitions` resolver body,
  `clause_grammar` timings, `member -effect`, `option -effect`,
  `option_effect_family`, `special_var`, and `semantic_operation`, and the
  Spec Studio renders all of them; `spectcl_check` reports the resolver as
  a hook.
- Nothing else: every parity gate above is byte-identical.

### Step 3 — trust gates execution; stub flags reach their fields

**Goal.** `WorkspaceTrust` plumbed from the LSP client to discovery; the
two `untrusted` predicates collapsed into one exported from `tcl-registry`;
hook-body execution gated on trust with the dormant-hook abstention
reported on the pack file; `spectcl_check`'s tier and trust parameters;
the six `StubFlags` consumed on their catalogue fields with nearest-wins
role resolution.

**Exit evidence.** `cargo test -p tcl-spectcl` (in full) green including
the new `workspace_trust.rs` harness; `cargo test -p tcl-lsp-server`'s
spec-pack e2e subset green with the dormant notice published;
`cargo test -p tcl-lsp-core --test stub_arg_roles` green with one
positive and one negative witness per flag; `spectcl_check` reports
`dormant_hooks`; `cargo xtask callback-inventory --check` and
`kcs-index-links` green.

#### Work items

**CC3.1 — `WorkspaceTrust` through discovery, provenance and the cache key.**
Files: `rust/tcl-dialect/src/model/environment.rs` (`pub enum WorkspaceTrust { Trusted, Untrusted }`,
`Default` `Trusted`, beside `Provenance` — D3.1), `rust/tcl-spectcl/src/discovery.rs`
(`DiscoveryOptions::workspace_trust: WorkspaceTrust`; `PackFile::trust: WorkspaceTrust`,
set from the options for `Tier::Workspace` and `Trusted` for every other
tier), `rust/tcl-spectcl/src/pack.rs` (`MergedPack::trust`;
`MergedPack::provenance(&self) -> Provenance`), `loader/environment_block.rs`
(`PackEnvironmentTier::provenance(self, trust: WorkspaceTrust)`: `Workspace`
maps to `WorkspaceTrusted` or `WorkspaceUntrusted`; every caller passes
the pack's trust), `loader/eval.rs` (`EvalOptions::trust`,
`EvalSnapshotKey::trust`, the `tier` doc comment corrected to name the
trust state), `cache.rs` (`key_for(source, tier, trust)`; `entry_key`
hashes the trust), `install.rs` and `registration.rs` (the provenance
passed through), `rust/tcl-mcp/src/spectcl.rs` and `rust/tcl-cli`
callers (pass `Trusted` explicitly).
Preserve: with `Trusted` (the default) every answer is byte-identical;
every existing cache key for a trusted workspace pack is unchanged only if
the hash mixes `Trusted` as zero bytes — it does not: the entry key
changes for every pack and the on-disk cache rebuilds once (a
non-observable delta; `cache::VERSION` bumps). Tests:
`rust/tcl-spectcl/tests/workspace_trust.rs` (new harness, added to
`scripts/dev/rust-test-binary-shards.tsv` as
`5	tcl-spectcl::workspace_trust	test	tcl-spectcl	workspace_trust`):
`an_untrusted_workspace_pack_is_workspace_untrusted_provenance`,
`an_untrusted_workspace_pack_still_declares_its_facts` (arity, roles and
the `semantics` declaration reach the registry — the authority ruling),
`an_untrusted_workspace_pack_cannot_override_a_compiled_name` (E-R2, the
negative), `a_client_that_reports_nothing_is_trusted`,
`the_snapshot_key_distinguishes_trust`. Gates: none. Docs:
`spec-packs.md` § *Workspace trust* "today" sentence goes;
`docs/design/registry/dialect-and-package-registry-redesign.md` § 11.1
O9 closed. Model: opus. Size: M. After: —.

**CC3.2 — one `untrusted` predicate.**
Files: `rust/tcl-registry/src/model/registration.rs` (`pub fn untrusted(provenance: Provenance) -> bool`,
re-exported as `tcl_registry::model::untrusted`), `rust/tcl-spectcl/src/loader/eval.rs`
(`untrusted(tier)` deleted; `provenance_violation_in` takes the
`Provenance` and calls the registry's predicate; `provenance_violation(pack, tier)`
becomes `provenance_violation(pack, provenance)`), `rust/tcl-mcp/src/spectcl.rs`
(the caller). Tests: `registration.rs` unit row
`the_one_untrusted_predicate_names_the_three_classes`;
`workspace_trust.rs` asserts the loader and the registration layer agree
for every `Provenance` (the negative: `WorkspaceTrusted` and `User` are
not untrusted). Model: sonnet. Size: S. After: CC3.1.

**CC3.3 — hook bodies gated; the dormant notice.**
Files: `rust/tcl-spectcl/src/loader.rs` (`HookDecl::line: u32`),
`rust/tcl-spectcl/src/pack.rs` (`load_sources` pushes, for every hook of
a pack whose `provenance()` is `WorkspaceUntrusted`, a `PackNotice` at the
hook's line, context `command NAME`, message
"`FIELD` is dormant: the workspace is not trusted, so this hook body does
not run and the command keeps its declarative facts", `Severity::Information`),
`rust/tcl-spectcl/src/hooks.rs` (`plan_for` allocates no slot for a
dormant pack's programs, so `specialise` leaves the loader's abstaining
placeholder — the abstention "exactly as a declared-but-unbound hook
does"; `HookPlan::dormant: Vec<(pack, command, field)>` for `spectcl_check`),
`rust/tcl-lsp-server/src/lib.rs` (`apply_initialization_options` reads
`workspaceTrust: "trusted" | "untrusted"`; `did_change_configuration`
reads `tclLsp.workspaceTrust`; a change triggers `reload_spec_packs(ReloadTrigger::Trust)`;
`spec_pack_discovery` sets `workspace_trust`), `editors/vscode/src/`
(the client sends `workspace.isTrusted` in `initializationOptions` and
re-sends the setting on `onDidGrantWorkspaceTrust` — D3.2; the other
editors send nothing and are trusted).
Preserve: a trusted workspace installs every hook as today
(`hooks.rs`'s `a_declared_body_replaces_the_abstaining_placeholder`).
Tests: `workspace_trust.rs` — `an_untrusted_pack_installs_no_hook_body_and_reports_each_as_dormant`
(a pack with a `const_fold` body: `plan_for` has zero programs for it, one
notice per hook at its line; the negative: the same pack trusted installs
the body and folds), `granting_trust_reloads_and_installs_the_bodies`
(server unit test beside the `reload_spec_packs` tests);
`rust/tcl-spec-hooks/tests/containment_e2e.rs` unchanged (containment is
not trust). Gates: `callback-inventory --check` unchanged (no new tier).
Docs: `spec-packs.md` § *Workspace trust*; KCS
`docs/kcs/kcs-qa-why-is-my-pack-hook-dormant.md` (User; all-editors,
VS Code sub-heading for the trust prompt), indexed. Model: opus. Size: M.
After: CC3.1, CC3.2.

**CC3.4 — `spectcl_check`'s tier and trust.**
Files: `rust/tcl-mcp/src/spectcl.rs` (`tier` argument
`bundled|user|workspace|studio-override`, default `workspace`; `trust`
argument `trusted|untrusted`, default `trusted`; the provenance verdict
computed for that pair; a `dormant_hooks` array), `rust/tcl-mcp/src/tools.rs`
(the tool schema's two properties). Tests: `rust/tcl-mcp/src/spectcl.rs`
unit rows — the default equals today's output; `trust: untrusted` lists
every hook as dormant and refuses an `-override`. Gates: `gen-ai-diagnostics --check`
if the tool catalogue is generated from `tools.rs` (the implementer runs
`make codegen` and stages what moves). Docs: the redesign's § 11.1 O4
closed; `docs/kcs/features/` MCP note updated if it lists the tool's
arguments. Model: sonnet. Size: S. After: CC3.3, and after the
diagnostic-policy lane's MCP commit (B2).

**CC3.5 — the six `StubFlags` on their catalogue fields; nearest-wins.**
Files: `rust/tcl-registry/src/model/declaration.rs` (`DeclaredCommand::traits: Traits`,
`DeclaredCommand::side_effects: Vec<SideEffect>`; `DeclaredCommand::new`
keeps its signature and a builder `with_traits` / `with_side_effects`;
`DocumentCommandSurface::traits(&self, name) -> Option<Traits>` and
`side_effects(&self, name) -> Option<&[SideEffect]>` answering the
document's declaration where it speaks, else the catalogue's;
`arg_indices_for_role` and `command_prefixes` answer nearest-wins: the
declaration alone when it declares the name, the catalogue otherwise),
`rust/tcl-compiler/src/analyser/types.rs` (`to_declared_command` maps
`BARRIER` → `Traits::CREATES_DYNAMIC_BARRIER`, `LOOP` → `HAS_LOOP_BODY`,
`PURE` → `PURE`, `UNSAFE` → `UNSAFE | SAFE_INTERP_HIDDEN`, `SCOPE_ALIAS` →
`CREATES_SCOPE_ALIAS`, `MUTATOR` → `SideEffect { target: Variable, writes: true, .. }`;
the doc comment's "has never had a consumer" goes),
`rust/tcl-compiler/src/ssa.rs:625` and `:1693`, `unit_scope.rs:521`,
`memory_ssa.rs:517` (read `surface.traits(name)` — the surface is already
built in `unit_scope.rs`; `ssa.rs` and `memory_ssa.rs` receive it through
`UnitBuildOptions::declared_commands`, which reaches lowering already),
`rust/tcl-compiler/src/analyser/utils.rs` (`parse_stub_flags` unchanged),
`docs/design/contracts/dialect-stubs.md` § *Flags* and § *Stubs are
declarations* ("today" sentences go).
Preserve: a document with no stub is byte-identical; a stub that names a
command the catalogue lacks is unchanged. Changes: a stub re-declaring a
catalogued command narrows its roles to the declaration (nearest-wins
replaces the union); each flag has an effect. Tests:
`rust/tcl-lsp-core/tests/stub_arg_roles.rs` gains one pair per flag —
`a_pure_stub_in_statement_position_is_reported_unused` (O108 / the pure
statement finding; negative: without `-pure` it stays),
`a_mutator_stub_keeps_the_store_it_reads` (O109),
`a_barrier_stub_blinds_the_reads_after_it` (W210 silence; negative: no
flag, W210 fires), `a_loop_stub_bumps_the_loop_body_depth` (W240 family),
`a_scope_alias_stub_aliases_its_local` (the alias walk sees the local as
the target's cell), `an_unsafe_stub_is_hidden_in_a_safe_interpreter`
(the safe-interp diagnostic the catalogue's `UNSAFE` commands draw), and
`a_stub_that_redeclares_a_catalogued_command_answers_nearest_wins`
(negative: the catalogue role the stub omits is not assigned). Gates:
`registry-axes` (no change). Docs: `dialect-stubs.md`; KCS
`kcs-howto-annotate-commands-with-stubs.md` gains the flags' effects.
Model: opus. Size: M. After: —.

#### Ordering and checkpoints

CC3.5 first (independent; `wip(consumer-contracts): stub flags reach their fields`;
green: `cargo test -p tcl-registry`, `cargo test -p tcl-compiler --test analyser`,
`cargo test -p tcl-lsp-core --test stub_arg_roles`), then CC3.1 + CC3.2
(`… workspace trust reaches discovery`; green: `cargo test -p tcl-spectcl`,
`cargo test -p tcl-registry`, the shard manifest check
`bash scripts/dev/test-nextest-binary-shards.sh`), then CC3.3
(`… dormant hook bodies`; green: `cargo test -p tcl-spectcl`,
`cargo test -p tcl-spec-hooks`, `cargo test -p tcl-lsp-server` spec-pack
subset, `npm test` in `editors/vscode` for the client change), then CC3.4
(`… spectcl_check tier and trust`; green: `cargo test -p tcl-mcp`).

#### Review checklist

- Authority is never gated: under `Untrusted` every declarative fact of the
  pack reaches the registry and the analyser (the
  `an_untrusted_workspace_pack_still_declares_its_facts` witness).
- Exactly one `untrusted` predicate exists in the tree
  (`grep -rn "fn untrusted" rust/` returns one hit).
- A dormant hook is reported once per hook at its own line, never once per
  call; `plan_for` allocates it no slot.
- The trust wire is one input; the server treats an absent value as
  trusted; the cache key includes it.
- Nearest-wins changed a union to a replacement: the negative witness
  exists and the delta is recorded.
- Risks: the cache-key change invalidating every pack snapshot once (accepted);
  `SAFE_INTERP_HIDDEN` reaching a diagnostic the stub author did not expect;
  the VS Code client's trust event racing `initialized`.

#### Behavioural deltas expected

- In an untrusted VS Code workspace, pack hook bodies do not run and each
  is reported as an Information notice on the `.tclspec`; declarative facts
  are unchanged; an `-override` of a compiled command is refused with the
  provenance named.
- A stub's `-pure`, `-mutator`, `-barrier`, `-loop`, `-scope_alias` and
  `-unsafe` now change analysis as the table in `dialect-stubs.md` says.
- A stub re-declaring a catalogued command narrows its roles to the
  declaration.

### Step 4 — identity: `alias_of`, the alias target's identity, the stamp rule, `SiteClaim`

**Goal.** `alias_of` as the one admissible source of a pack command's
builtin identity; codegen recording the alias target's identity; the
loader's stamp rejection rule (target's own, tier gate, drop the stamp
only); `SiteClaim` and `PackFactStamp` recorded in the artefact and checked
at admission for rungs 1 and 2.

**Exit evidence.** The two witnesses the page names, in
`rust/tcl-spectcl/tests/codegen_stamps.rs`: a refused stamp under the tier
gate, and an accepted one whose recorded identity the VM's alias hop
resolves; `cargo test -p tcl-vm --test command_mutation_deopt_e2e` green
with the rung-1 stamp check; `spectcl_roundtrip` green with `alias_of`
round-tripping.

#### Work items

**CC4.1 — `alias_of`.**
Files: `rust/tcl-registry/src/spec.rs` (`CommandSpec::alias_of: Option<&'static str>`,
doc: "the shipped builtin this pack command is; the only admissible
source of a builtin identity for a pack command; never inferred from a
realm alias"), `rust/tcl-spectcl/src/loader.rs` (`alias_of NAME`),
`render_spectcl.rs` / `schema.rs` / `help.rs` / `draft.rs` / `coverage.rs`
(the four surfaces; a plain string field, no `GAPS` row),
`docs/design/spec-dsl-examples/README.md`. Tests: `registry_sweep.rs` —
`alias_of_names_a_shipped_command_of_the_same_family` (negative: a spec
naming an unknown target fails the sweep); `spectcl_roundtrip.rs` sees the
field round-trip. Model: sonnet. Size: S. After: —.

**CC4.2 — the stamp rejection rule.**
Files: `rust/tcl-spectcl/src/stamps.rs` (new: `pub fn admit_codegen_stamps(command: &mut PackCommand, provenance: Provenance, shipped: &CommandRegistry) -> Vec<StampRefusal>`;
rule 1: a `codegen_hook`, `inline_codegen_hook` or
`semantic_operation Intrinsic(…)` on a pack command is admitted only when
`alias_of` names a shipped spec carrying that same hook identity; rule 2:
only `BuiltIn` and `BundledPack` may carry one at all; rule 3: a refusal
clones the leaked spec, clears the stamp, re-leaks it, and returns the
refusal with the provenance's label and the target the stamp would have
had to name), `rust/tcl-spectcl/src/pack.rs` (`load_sources` calls it per
command after the merge and pushes one `PackNotice` per refusal at the
command's line, `Severity::Warning`: "`codegen_hook Lassign` refused for
`vendor::unpack`: a trusted workspace pack may not name a codegen
catalogue member; the stamp would have to sit on `alias_of lassign`"),
`rust/tcl-spectcl/src/loader.rs` (the existing "names a codegen hook"
notice is replaced by the rule's notice), `rust/tcl-spectcl/src/install.rs`
(`debug_assert!` that no stamp survives on a non-admitted provenance).
Preserve: a bundled pack's stamps (none of the shipped `specs/*.tclspec`
carries one; `jim.tclspec` is a core surface) load as today. Tests:
`rust/tcl-spectcl/tests/workspace_packs.rs` gains
`a_workspace_stamp_without_alias_of_is_refused_and_names_the_target`,
`a_workspace_stamp_with_alias_of_is_refused_by_the_tier_gate`,
`a_refused_stamp_costs_no_analysis_fact` (arity and roles survive), and
`a_bundled_stamp_on_an_alias_of_target_is_admitted` (loaded through
`bundled::load_from` on a temp `specs/` dir; negative: the same pack with
`alias_of lsort` is refused by rule 1). Gates: `pack-goldens --check`,
`spec_corpus` baseline. Docs: `spec-packs.md` § *What a pack still cannot
say* and `registry-consumer-contracts.md` § *The loader's stamp rejection
rule* ("today" sentences go). Model: opus. Size: M. After: CC4.1.

**CC4.3 — codegen records the alias target's identity.**
Files: `rust/tcl-compiler/src/codegen/emitter/bytecoded.rs`
(`registry_codegen_hook` builds the identity from
`resolved.spec.alias_of.unwrap_or(resolved.spec.name)`; the inline path
in `codegen/cmd_subst.rs` likewise), `rust/tcl-vm/src/interp.rs`
(`command_binding_matches` unchanged — the alias hop already resolves).
Tests: `rust/tcl-spectcl/tests/codegen_stamps.rs` (new harness; shard row
`5	tcl-spectcl::codegen_stamps	test	tcl-spectcl	codegen_stamps`):
`an_admitted_alias_stamp_records_the_targets_identity` (compile
`vendor::unpack {a b} $l` against a registry with the bundled pack; the
module's `command_bindings` names `lassign`'s identity), then
`the_vm_admits_it_through_the_alias_hop` (`interp alias {} vendor::unpack {} lassign`
in a `tcl_vm::Vm` running the module: admitted, result correct) and
`a_proc_at_the_pack_name_recompiles_plain` (negative). Gates: none.
Model: opus. Size: M. After: CC4.2.

**CC4.4 — `SiteClaim` and `PackFactStamp`.**
Files: `rust/tcl-runtime-api/src/site_claim.rs` (new: `SiteClaim`,
`PackFactStamp` with `content_hash: u64` (D4.3), `IdentityKind`;
`RuntimeBacking` and `BodySource` are step 7's — the `ReferenceBody` and
`ShippedImplementation` variants are added by CC7.1 and CC8.2, so this
item lands `Generic`, `PackFacts`, `BuiltinAlias`), `rust/tcl-bytecode/src/lib.rs`
(`FunctionAsm::site_claims: Vec<SiteClaim>`), `rust/tcl-registry/src/registry.rs`
(`CommandRegistry::pack_origin(&self, spec: &CommandSpec) -> Option<&PackOrigin>`
— a side table keyed by spec pointer; `PackOrigin { pack: String, content_hash: u64, vocabulary_version: String }`
in `rust/tcl-registry/src/pack_origin.rs`, filled by
`tcl-spectcl::install::install_into` — D4.4), `rust/tcl-compiler/src/codegen/emitter/bytecoded.rs`
(a site whose hook came through `alias_of` records `BuiltinAlias { binding, facts }`;
a constant fold emitted from a pack `const_fold` hook records
`PackFacts(stamp)`; `overlay_generation` and `evaluator_revision` from the
`AnalysisContext` the codegen context carries), `rust/tcl-vm/src/interp.rs`
(`Vm::set_pack_facts(&mut self, stamps: Vec<PackFactStamp>)`; admission
in `function_command_bindings_match` gains the rung-1 check: every
`PackFacts` / `BuiltinAlias.facts` stamp equals a stamp the VM holds on
`pack`, `content_hash`, `vocabulary_version`, `overlay_generation`,
`evaluator_revision`; a mismatch is plain dispatch or the admission error
`run_module` already produces), `rust/tcl-lsp-server` (the optimise path
sets the facts from the published pack set).
Tests: `codegen_stamps.rs` gains `a_changed_pack_invalidates_the_site`
(same module, VM holding a stamp with a different `content_hash`:
recompiled plain — negative control: the same stamp admits);
`rust/tcl-vm/tests/command_mutation_deopt_e2e.rs` gains
`a_rung_zero_module_is_admitted_under_a_changed_pack_set`. Docs:
`docs/design/contracts/vm-compiled-artifact-provenance.md` gains the
claim; the page's § *What the artefact records per rung* "proposed" note
flips for rungs 1 and 2. Model: opus. Size: L. After: CC4.3.

#### Ordering and checkpoints

CC4.1 (`wip(consumer-contracts): alias_of`), CC4.2 (`… the stamp rejection rule`),
CC4.3 (`… codegen records the alias target`), CC4.4 (`… site claims`).
Green at each: `cargo check --workspace`; `cargo test -p tcl-spectcl`;
from CC4.3 `cargo test -p tcl-compiler --test codegen --test codegen_integration`
and `cargo test -p tcl-vm --test command_mutation_deopt_e2e`; the shard
manifest check after CC4.3.

#### Review checklist

- `alias_of` is the only source of a pack command's builtin identity: no
  code path reads `realm.rs`'s aliases to admit a site (grep
  `alias_of` and `CommandBindingIdentity` construction sites).
- A refusal drops the stamp and nothing else (the
  `a_refused_stamp_costs_no_analysis_fact` witness).
- The identity recorded is the target's, never the pack command's own name
  (the module's `command_bindings` assertion).
- `content_hash` is the snapshot key's `u64` (D4.3), named as such in
  `PackFactStamp`'s doc comment.
- Risks: the spec clone-and-releak on refusal leaking per reload (bounded
  by the pack-set cache; note it); the VM's rung-1 check refusing a module
  compiled with no pack at all (a module with no claims must admit).

#### Behavioural deltas expected

- A pack stamp at any non-bundled tier is refused with a Warning notice
  naming the provenance and the target; today it loads with a note.
- A specialised site from a bundled `alias_of` command is admitted through
  the alias hop and runs specialised; today it recompiles plain.
- A changed pack turns its sites plain on the next admission.

### Step 5 — persisted guard identities, per-member semantics keys, the Explorer record

**Goal.** Intrinsic guard identities persisted in both runtimes across
command-environment mutations and the profile pin, keyed by command-token
generation and following rename and hide; `guard_semantics_key` one key
per `IntrinsicId` member; the AOT observability record complete for every
rejected native premise.

**Exit evidence.** `command_mutation_invalidates_guard_and_identity_attestation`
re-stated as `an_unrelated_mutation_keeps_the_guard_and_a_rebinding_drops_it`
(runtime) and its VM twin green; `guarded_boxed_intrinsic_runs_and_falls_back_against_the_real_runtime`
gains the "unrelated proc keeps the fast path" row; one
`tcl-fuzz` campaign over the `tclvm`/`runtime-rust` pair with no new
finding (manual, recorded in this document).

#### Work items

**CC5.1 — one semantics key per member.**
Files: `rust/tcl-registry/src/intrinsic.rs` (`guard_semantics_key` reads a
per-member table `const SEMANTICS_REVISION: [(IntrinsicId, u32); 28]`
combined with the release variant for the versioned members; the two
`RUNTIME_INVARIANT_SEMANTICS` / `VERSIONED_STRING_SEMANTICS` constants
become per-member; `guard_semantics_variants` likewise). Tests:
`intrinsic.rs` — `every_member_has_a_distinct_semantics_key`,
`bumping_one_members_revision_moves_no_other_key`; the runtime and VM
tests that compute the key by calling the function are unchanged; the one
literal `0x0306`-style assertion is updated. Model: sonnet. Size: S.
After: —.

**CC5.2 — guard identities keyed by token generation, in both runtimes.**
Files: `rust/tcl-vm/src/interp.rs` (`guarded_commands: RefCell<HashMap<String, (u64, BTreeSet<GuardIdentity>)>>`
keyed with the command's `command_identity.generations` entry;
`bump_cmd_epoch` no longer clears it; `GuardDomain::CommandEnvironment`
invalidation becomes per token: a mutation of token T drops T's guards
and leaves the rest; rename and hide move the entry with the builtin
identity as `builtin_identities` moves; `set_dialect_profile` keeps the
entries whose spec is in the pinned generation), `runtime/rust/src/interp.rs`
(`invalidate_command_environment` the same way), `rust/tcl-runtime-api/src/guard.rs`
(a `GuardDomain::CommandToken(generation)` if the domain lattice needs the
finer key — the implementer decides whether the existing domain set
suffices and records it), `docs/design/runtime/rename-alias.md` § 3.5,
`docs/design/runtime/tclvm-opcode-status.md`.
Preserve: a renamed or traced intrinsic still falls back (the two rows in
`wasm_real_link.rs`); the interpreter and object-dispatch domains stay
poisoned. Changes: a `proc foo` after registration no longer disables
`string length`'s fast path. Tests: `runtime/rust/src/interp.rs`'s
`command_mutation_invalidates_guard_and_identity_attestation` re-stated
(three rows: unrelated proc keeps; `rename string x` drops; the profile pin
keeps); `rust/tcl-vm/tests/command_mutation_deopt_e2e.rs` twin;
`wasm_real_link.rs` gains the "unrelated proc" row. Gates: `tcl-fuzz`
campaign (`cargo run -p tcl-fuzz -- run` over the two-backend pair per
`docs/kcs/kcs-howto-work-on-fuzz-findings.md`), recorded here. Docs: the
two runtime pages; the page's § *Codegen and the registry today* diagram
"never repopulated" row flips. Model: opus. Size: L. After: CC5.1.

**CC5.3 — the Explorer observability record.**
Files: `rust/tcl-compiler/src/mixed_region_plan.rs`, `common_aot_plan.rs`
(a failed native-add composition serialises every rejected premise as a
`NativeDecline { premise, reason }` list on the region's plan),
`rust/tcl-explorer/src/serialise.rs` and `view_tree.rs` (the `aot` view
renders them; `tcl explore --show aot --text` prints them). Tests:
`rust/tcl-compiler/tests/wasm_tiers.rs` gains
`a_failed_native_add_names_every_rejected_premise`; an explorer text
snapshot. Docs: `semantic-aot-optimisation.md`'s "That observability gap
must close" sentence goes. Model: opus. Size: M. After: —.

#### Ordering and checkpoints

CC5.1 (`wip(consumer-contracts): one semantics key per intrinsic`), CC5.3
(`… the AOT decline record`), CC5.2 last (`… guard identities persist`).
Green: `cargo test -p tcl-registry`; `cargo test -p tcl-runtime`;
`cargo test -p tcl-vm`; `cargo test -p tcl-compiler --test wasm_real_link --test wasm_tiers`;
`cargo test -p tcl-explorer`.

#### Review checklist

- Both runtimes change together (the page's "a design change to a tested
  contract in both runtimes"); the fuzz campaign is recorded with its seed
  count and release.
- A persisted identity is invalidated when its dependency changes: the
  `rename` and `trace` rows still fall back.
- No identity is attached by name after registration (the sweep of CC7.2
  is the only attach path; step 5 adds none).
- Risks: a guard surviving a `proc string` in a child namespace that
  shadows the builtin for calls made from that namespace (the token
  generation is per fully-qualified key; the binding check re-resolves the
  namespace — assert it in a test row).

#### Behavioural deltas expected

- After any `proc`, `rename` or `interp alias` of an unrelated command, a
  guarded intrinsic fast path stays live in both runtimes; today it is
  disabled for the rest of the interpreter's life.
- The Explorer's `aot` view lists rejected native premises.

### Step 6 — the take-shipped floor, the capability matrix, the overlay to the compile service

**Goal.** The take-shipped floor over the whole codegen and runtime axis;
`CodegenCapability` applied by `DependencyTier`; the overlay generation
delivered to the compile service; an overlay miss failing closed.

**Exit evidence.** `cargo test -p tcl-spectcl --test i6_security_floor`
green with the six new rows; `cargo test -p tcl-registry` green with the
ingress test inverted; `cargo test -p tcl-lsp-db --test dialect_seam`
green; the server's optimise path compiles under the workspace overlay.

#### Work items

**CC6.1 — the floor widens.**
Files: `rust/tcl-registry/src/security_floor.rs` (`apply` takes
`lowering_hook`, `analyser_hook`, `semantic_operation`, `state_transitions`,
`native_lowering`, `bpf_op` shipped; `MERGED_FIELDS` lists them;
`every_security_bearing_field_is_in_the_floor` names the six explicitly
beside its `taint` / `codegen` filter). Tests: `i6_security_floor.rs` —
one row per field: `a_workspace_override_cannot_swap_the_lowering_hook`
etc. (negative: `the_floor_does_not_invent_facts_for_a_new_command`
unchanged). Docs: `spec-packs.md` § *Workspace trust* floor sentence;
the page's rule 4 "today" goes. Model: sonnet. Size: S. After: —.

**CC6.2 — `DependencyTier` and `CodegenCapability`.**
Files: `rust/tcl-registry/src/model/capability.rs` (new: `DependencyTier`,
`CodegenCapability`, `CodegenCapability::for_tier(tier) -> Self` — the
page's matrix: `Root` everything; `Direct` `runtime_backing` and
`builtin_alias`, no stamps, no body; `Transitive` and `Development`
nothing), `rust/tcl-spectcl/Cargo.toml` (`tcl-pkg` dependency, data model
only — D6.2), `rust/tcl-spectcl/src/discovery.rs` (`PackFile::dependency_tier: Option<DependencyTier>`:
a pack beside a manifest that the nearest `tclpkg.lock` lists as a
dependency gets the tier the lockfile's graph gives it — in the root
manifest's `requires` `Direct`, reached only transitively `Transitive`,
`dev` `Development`; the workspace's own manifest `Root`; no lockfile or
not listed `None`), `rust/tcl-spectcl/src/stamps.rs` (a second gate after
CC4.2's: `alias_of`, `runtime_backing` (step 7) and a reference body
(step 8) are dropped with a notice when the capability forbids them;
both gates must pass), `docs/design/registry/spec-packs.md`.
Tests: `workspace_packs.rs` gains `a_transitive_dependencys_alias_of_is_dropped`
and `a_direct_dependency_keeps_alias_of_but_not_a_stamp` (negative:
`Root` keeps both). Model: opus. Size: M. After: CC4.2.

**CC6.3 — the overlay reaches the compile service; a miss fails closed.**
Files: `rust/tcl-registry/src/model/ingress.rs` (`context_registry` and
`registry_for_environment_if_built` return `Result<Arc<ContextRegistry>, OverlayMiss>`;
the test at `ingress.rs:796` asserting the fallback is inverted),
`rust/tcl-compiler/src/compile_service.rs` (`RegistryTarget::Overlay { profile, overlay: u64 }`;
`BytecodeCompileService::for_profile_with_overlay`), `rust/tcl-lsp-db/src/lib.rs`
(`compilation_unit` and `registry_with_overlay` propagate the error as a
diagnostic-free abstention: the unit is not built and the reason is
logged once), `rust/tcl-lsp-server/src/lib.rs` (`optimise_document_command`
builds the service with `spec_pack_key`), `rust/tcl-vm` (unchanged: the
service is injected). Preserve: every compile with an installed overlay
or no overlay. Changes: an uninstalled overlay is an error, not a silent
un-overlaid compile. Tests: `ingress.rs` unit `an_uninstalled_overlay_is_an_error_not_the_plain_generation`;
`rust/tcl-lsp-db/tests/dialect_seam.rs` gains
`the_compilation_unit_sees_the_packs_commands` (a pack command resolves in
the unit; negative: after the pack is retired the unit is rebuilt without
it, never with a stale overlay). Boundary: slice 4 of the value-transfers
lane lands `spec_pack_key` reaching `compilation_unit` first (B3); this
item lands after it. Model: opus. Size: M. After: —.

#### Ordering and checkpoints

CC6.1 (`wip(consumer-contracts): the take-shipped floor covers the codegen axis`),
CC6.2 (`… the dependency-tier capability matrix`), CC6.3 (`… overlay misses fail closed`).
Green: `cargo test -p tcl-registry`; `cargo test -p tcl-spectcl`;
`cargo test -p tcl-lsp-db`; `cargo test -p tcl-compiler --test codegen`.

#### Review checklist

- The floor is a codegen-axis contract, not a trust gate on analysis
  facts: an override still changes arity, roles, hover.
- Both gates (provenance, capability) apply and a declaration passes both;
  the notices name which one refused.
- No silent fallback remains in `ingress.rs` (grep `unwrap_or` around the
  overlay lookup).
- Risks: `tcl-pkg` as a `tcl-spectcl` dependency pulling `tcl-sandbox` into
  every server build (check the build graph; if it hurts, the lockfile
  parser moves to a `tcl-pkg-model` crate — record the decision).

#### Behavioural deltas expected

- A workspace override that swapped `lowering_hook`, `analyser_hook`,
  `semantic_operation` or `state_transitions` keeps the shipped value.
- An uninstalled overlay yields no compilation unit instead of an
  un-overlaid one.

### Step 7 — identities from the pinned generation, `runtime_backing`, the intrinsic families, the manifest, the runtime context

**Goal.** Identities attached after registration and after the pin from
the pinned shipped generation only; `runtime_backing` a registry fact
with `command_backing`'s lists as its rows and a query both runtimes
answer; the intrinsic table split by family; `ArtefactIdentityManifest`
on `CompiledUnit` and as a WASM custom section; the runtime pin a context;
a fuzz campaign as the exit.

**Exit evidence.** `cargo xtask command-backing --check` green with the
report generated from the query and `KNOWN_UNBACKED` the only list left;
`cargo test -p tcl-runtime` and `cargo test -p tcl-vm` green; the
manifest witnesses in `rust/tcl-vm/tests/command_mutation_deopt_e2e.rs`
and `rust/tcl-compiler/tests/wasm_real_link.rs`; one fuzz campaign over
the two-backend pair recorded here.

#### Work items

**CC7.1 — `RuntimeBacking` on the spec.**
Files: `rust/tcl-registry/src/runtime_backing.rs` (new: `RuntimeBacking`,
`BodySource` verbatim; `CommandSpec::runtime_backing: RuntimeBacking`,
default `None`), every core spec (`ShippedBuiltin { identity }` for the
282 handler and native rows, `TclBody { source: PackageSource { relative_path: "init.tcl" | "package.tcl" | "parray.tcl" } }`
for the 11 stdlib rows, `None` for the 54 not-required rows; the 42
known-gap rows declare `ShippedBuiltin` — the target state — and stay on
`KNOWN_UNBACKED` as the drift waiver, D7.1), `security_floor.rs`
(`runtime_backing` joins the take-shipped list), `rust/tcl-spectcl/src/loader.rs`
(`runtime_backing shipped-builtin ID | tcl-body {-package-source PATH} | tcl-body {-pack-text {…}} | host-native | none`;
`PackText` emits an Information notice at load), the four studio
surfaces, `rust/xtask/src/gen_irule_test_data.rs` (emits a mock only for
`None` or `HostNative`; every iRules command is `None`, so the output is
byte-identical). Tests: `registry_sweep.rs` — `every_core_command_declares_a_backing`;
`gen_irule_test_data.rs` — `committed_files_match_registry_generation`
unchanged. Gates: `gen-irule-test-data --check`, `pack-goldens --check`,
`reference_doc`. Model: sonnet (the rows are the report's rows). Size: M.
After: —.

**CC7.2 — the backing query in both runtimes; the gate asks it.**
Files: `runtime/rust/src/interp.rs` (`Interp::backing_report(&self) -> Vec<(String, RegisteredBacking)>`
over the handler table: `Builtin` for a registered fn, `Object` for a
TclOO bootstrapped object, `Stdlib` for a name the embedded library
defines, `Absent`; `register_spec_builtin` stops reading `build_default()`
— `cmd_string.rs:47` — and the identity sweep `attach_identities(&mut self, registry: &CommandRegistry)`
runs at the end of `register_builtins` and again in the profile pin,
from the pinned generation only), `rust/tcl-vm/src/interp.rs` (the same
two functions), `rust/xtask/Cargo.toml` (`tcl-runtime` and `tcl-vm`
dependencies — D7.1), `rust/xtask/src/command_backing.rs` (the source
scan, `HANDLER_EXTRA`, `STDLIB` and `NOT_REQUIRED` deleted; the report is
rendered from `spec.runtime_backing` against both runtimes'
`backing_report()`; a spec declaring `ShippedBuiltin` whose runtime report
is `Absent` fails unless it is on `KNOWN_UNBACKED`; a spec declaring
`None` that the runtime registers is drift), `docs/generated/wasm-command-backing.md`
(regenerated: the same 389 rows, the `backing` column now the declared
fact and a new `vm` column for `tcl-vm`). Tests: `command_backing.rs` unit
rows re-stated over the query (`a_declared_builtin_the_runtime_lacks_is_drift_unless_waived`,
`a_declared_none_the_runtime_registers_is_drift`); `runtime/rust/src/interp.rs`
gains `identities_come_from_the_pinned_generation_never_an_overlay`
(negative: an overlay-only spec attaches nothing). Gates:
`command-backing --check` (the report regenerated and staged). Docs:
`AGENTS.md` § *WASM command parity* (the gate's description), the page's
§ *Consequences for the runtimes* second bullet flips. Model: opus. Size:
L. After: CC7.1.

**CC7.3 — the intrinsic table by family.**
Files: `rust/tcl-registry/src/intrinsic.rs` (`IntrinsicId::family(self) -> IntrinsicFamily`
— `Value` for the shared-core functions, `FamilyB { domain: GuardDomain, fires_traces: bool }`
for the variable-store and channel operations; `info exists` and the
array operations `fires_traces: true`), `runtime/rust/src/codegen_abi.rs`
and `rust/tcl-vm` (the guard domain a registration derives comes from the
family, not from a per-call table). Tests: `intrinsic.rs` —
`every_member_names_a_family`, `trace_firing_members_are_family_b`.
Docs: `docs/design/runtime/family-b-routing.md` § 4 (the gap row closes).
Model: opus. Size: S. After: CC5.1.

**CC7.4 — `ArtefactIdentityManifest` and the runtime context pin.**
Files: `rust/tcl-runtime-api/src/manifest.rs` (new: `ArtefactIdentityManifest`
verbatim; `BuildProfileId` is `tcl_dialect::build_info`'s id),
`rust/tcl-vm/src/compiled.rs` (`CompiledUnit::manifest: Option<ArtefactIdentityManifest>`),
`rust/tcl-bytecode/src/lib.rs` (`ModuleAsm::manifest`), the compiler's
emitters (fill it from the `AnalysisContext` and the pack set),
`rust/tcl-compiler/src/codegen/wasm/encoding.rs` (a custom section
`tcl.manifest`, a length-prefixed field list in declaration order),
`rust/tcl-vm/src/environment.rs` and `runtime/rust/src/environment.rs`
(`RuntimeContext { environment, release, build, packages, overlay_generation }`
resolved through `tcl_registry::model::ingress` — the same ingress the
compiler uses — replacing the profile-only pin; `set_dialect_profile`
becomes `pin_context(&RuntimeContext)` with the profile form kept as a
constructor; the `namespace` and `trace` subcommand gates read the pinned
profile, not the release name), `rust/tcl-vm/src/exec.rs`
(`validate_module_profile` becomes the manifest check: per rung, a
disagreeing field refuses only the sites that rest on it), the WASM link
harness (`wasm_real_link.rs` helpers read the section and refuse on an
`abi_version` or `intrinsic_table_hash` mismatch). Tests:
`command_mutation_deopt_e2e.rs` — `a_manifest_disagreeing_on_packs_refuses_only_rung_one_sites`,
`a_rung_zero_unit_is_admitted_under_a_changed_pack_set`;
`wasm_real_link.rs` — `a_module_with_a_foreign_intrinsic_table_is_refused`
(negative: the matching hash links). Docs:
`vm-compiled-artifact-provenance.md`; the page's manifest "proposed" note
flips. Model: opus. Size: L. After: CC4.4, CC7.2.

**CC7.5 — the fuzz exit.** One `tcl-fuzz` campaign over the
`tclvm`/`runtime-rust` pair after CC7.4 (the skill `fuzz-findings`); seed
count, release and outcome recorded in this document's status section.
Model: sonnet. Size: S. After: CC7.4.

#### Ordering and checkpoints

CC7.1 (`wip(consumer-contracts): runtime_backing rows`), CC7.3
(`… intrinsic families`), CC7.2 (`… the backing query replaces the scan`),
CC7.4 (`… the artefact manifest and the runtime context`), CC7.5. Green:
`cargo test -p tcl-registry`; `cargo test -p tcl-runtime`;
`cargo test -p tcl-vm`; `cargo test -p xtask`; `command-backing --check`;
`gen-irule-test-data --check`; `cargo test -p tcl-compiler --test wasm_real_link --test wasm_codegen --test codegen_integration`.

#### Review checklist

- Registration stays a runtime-owned handler table in its documented order
  (TclOO overrides `variable`, the event loop replaces `update`, `string`
  last); the sweep attaches identities, it never registers.
- The gate's residue is one list (`KNOWN_UNBACKED`); the report is a
  rendering of the query, and `--check` fails on a changed row.
- The manifest is checked per rung, never as a global refusal.
- The `xtask` build time with the two runtime crates linked is measured
  and recorded (if it exceeds a minute, the query moves behind a
  `cargo xtask command-backing --from-runtime` build step — record the
  decision).
- Risks: the identity sweep running before `package` is registered (the
  first pin happens inside `register_builtins`); the custom section's
  byte layout drifting between emitter and reader without a shared
  encoder (one function pair in `tcl-runtime-api`, tested round-trip).

#### Behavioural deltas expected

- `docs/generated/wasm-command-backing.md` gains a `vm` column and reads
  its `backing` column from the spec.
- A bytecode module compiled under one pack set is refused for its rung-1
  sites under another; a WASM module with a different intrinsic-table hash
  is refused at link.

### Step 8 — reference bodies, `tcl spec test`, the manifest `spec` directive

**Goal.** A reference Tcl body as a declared implementation and as code;
`tcl spec test`; the manifest `spec` directive with its `DependencyTier`
and lockfile hash. `Engine::set_release` is slice 4's and is consumed,
not added (B3).

**Exit evidence.** `rust/tcl-spectcl/tests/codegen_stamps.rs` rung-3
rows green; `cargo test -p tcl-cli` with the `spec test` rows;
`cargo test -p tcl-pkg` with the directive and lockfile rows;
`cargo xtask pack-goldens --check`.

#### Work items

**CC8.1 — the reference body as code (rung 3).**
Files: `rust/tcl-runtime-api/src/site_claim.rs` (`SiteClaim::ReferenceBody`),
`rust/tcl-compiler/src/inlining/` and `inline_uplevel.rs` (a body inlined
for a command whose `runtime_backing` is `TclBody` records the claim with
`ProcedureBindingIdentity` built from the body source; `PackageSource`
bodies are read through the host filesystem seam — the value-transfers
lane's pinned provisioning path — never `std::fs`), `rust/tcl-vm/src/interp.rs`
(`procedure_binding_matches` gains the conjunct: the claim's backing is
`TclBody`; activation defines the proc only then), `rust/tcl-vm/src/command.rs`
(the `source` command's direct `std::fs::read_to_string` at `:307` routed
through `tcl_platform`'s host filesystem seam and honouring `-encoding`
— the page's § *Consequences for the runtimes* fifth bullet, landed here
because rung 3 is its first consumer). Tests: `codegen_stamps.rs` —
`a_tcl_body_backed_command_is_inlined_and_admitted`,
`a_host_native_backing_never_defines_a_proc` (negative: the same body with
`host-native` backing is neither inlined nor activated),
`a_pack_text_body_that_diverges_turns_the_site_plain`. Model: opus. Size:
L. After: CC7.1, CC7.4.

**CC8.2 — the reference body as a declared implementation and as a
derivation source.**
Files: `rust/tcl-registry/src/value_transfer/declaration.rs` (a
`TclBody`-backed command whose body the sandbox can express — no
`upvar`, `uplevel`, `global`, `variable`, channel or `exec` word, decided
by a static scan the registry owns — derives a `Declared` semantics with
`EvalRoute::Implementation`; the value-transfers lane's route machinery
runs it under `Engine::set_release`), `rust/tcl-spec-studio/src/infer.rs`
(`infer_from_body(body) -> InferredFacts { pure, side_effects, return_type, callback_slots }`
through `tcl_compiler`'s interprocedural summary; `tcl spec import` and
the `spec-author` skill's draft carry them as proposals), `ai/claude/skills/spec-author/SKILL.md`
(the "questions only the author can answer" list shrinks). Tests:
`rust/tcl-registry/tests/value_transfers.rs` gains
`a_reference_body_is_a_declared_implementation_when_the_sandbox_can_express_it`
(negative: a body with `upvar` derives nothing); `rust/tcl-spec-studio`'s
`infer` tests gain a body row. Gates: `value-transfers --check` (the
inventory gains the route; the pinned route set in `value_transfers.rs`
grows — coordinate with the lane, B1). Model: opus. Size: L. After:
CC8.1, and slice 4 of the value-transfers lane.

**CC8.3 — `tcl spec test`.**
Files: `rust/tcl-cli/src/cli.rs` (`SpecCommand::Test(SpecTestArgs { pack, tclsh, package })`),
`rust/tcl-cli/src/commands/spec.rs` (`run_test`: loads the pack, requires
the package in the named shell under `tcl-pkg`'s policy
(`rust/tcl-pkg/src/policy.rs`, opt-in), then for every command diffs the
declared facts against the implementation — arity windows against
`wrong # args`, `return_type` and each `hover.examples` line against the
shell's result, a `TclBody` reference body against the real command on
the same inputs, `pure` against a second run with traced globals —
printing one row per divergence; exit 1 on any), `docs/kcs/features/kcs-feature-tcl-verb-cli.md`.
Tests: `rust/tcl-cli/tests/` gains `spec_test_reports_an_arity_divergence`
over a fixture pack and a fixture Tcl package (skipped without a
`tclsh` on `PATH`). Model: opus. Size: M. After: CC8.1.

**CC8.4 — the manifest `spec` directive, the lockfile hash, the container generator.**
Files: `rust/tcl-pkg/src/manifest.rs` (`ManifestAst::spec: Option<SpecDirective>`;
`SpecDirective { packs: Vec<String>, requested_tier: DependencyTier }`
with `DependencyTier` defined in `tcl-pkg` (the page's enum; `tcl-registry`'s
`model::capability::DependencyTier` becomes a re-export of it once the
dependency exists — D6.2), parsed from `spec { packs {a.tclspec} tier direct }`;
data-only, never executed), `rust/tcl-pkg/src/lockfile.rs`
(`LockedPackage::spec_integrity: Option<String>` — the xxh3 of each pack
as hex, joined; `PackFactStamp::content_hash` is the same value),
`rust/tcl-pkg/src/resolver.rs` (the tier clamp: the declared tier is never
nearer than the resolution's), `rust/tcl-spectcl/src/discovery.rs`
(reads the directive's paths as the pack list beside a manifest instead of
globbing), `rust/tcl-cli/src/commands/docker.rs` (`tcl docker create`
computes the `HostNative` commands' extensions from the packs'
`runtime_backing` and passes `DockerfileSpec::native_extensions: Vec<String>`;
`rust/tcl-pkg/src/docker.rs` renders the install lines and stays
registry-free — D8.2), `docs/design/tclpkg/architecture.md` § Manifest and
§ Lockfile. Tests: `rust/tcl-pkg` unit rows — `the_spec_directive_is_data_only`,
`a_changed_pack_changes_the_lockfile` (negative: an unchanged pack keeps
the hash), `a_manifest_cannot_claim_a_nearer_tier`;
`rust/tcl-pkg/tests/manifest_env_drift.rs` unchanged. Model: opus. Size:
M. After: CC6.2, CC7.1.

#### Ordering and checkpoints

CC8.4 first (independent of the runtimes;
`wip(consumer-contracts): the manifest spec directive`), CC8.1
(`… reference bodies as code`), CC8.3 (`… tcl spec test`), CC8.2 last
(`… reference bodies as declared implementations`, after slice 4). Green:
`cargo test -p tcl-pkg`; `cargo test -p tcl-spectcl`; `cargo test -p tcl-vm`;
`cargo test -p tcl-cli`; `cargo test -p tcl-registry --test value_transfers`;
`value-transfers --check`.

#### Review checklist

- Rung 3's extra conjunct: no proc is ever defined for a `HostNative` or
  `None` backing (the negative witness).
- A `PackText` body is reported at load and turns its sites plain on the
  first mismatch.
- `tcl spec test` never runs at editor load; it is a CLI verb under the
  package manager's opt-in policy.
- The lockfile hash and the artefact stamp are one value.
- Risks: the sandbox-expressibility scan admitting a body that reads
  world state through a command the scan does not know (the scan is a
  whitelist of commands, never a blacklist — assert it).

#### Behavioural deltas expected

- A pack with `runtime_backing tcl-body {-pack-text …}` loads with an
  Information notice.
- `tcl docker create` lists a `HostNative` command's extension.
- A `tclpkg.lock` gains a `spec_integrity` field for packages that ship
  packs.

### Step 9 — the codegen axis versioned; evaluation points through the evidence gate

**Goal.** Codegen-axis facts versioned as arity is (ordered windows, first
covering row wins, selected at the primary, plain dispatch when a
declared target range disagrees with the primary); a vendor
environment's evaluation point fed through the evidence gate rather than
answered by name; package version windows carried on `SurfaceQuery`.

**Exit evidence.** `cargo test -p tcl-registry` with the window sweep;
`cargo test -p tcl-compiler --test codegen --test dialect_threading`
with the straddle row; the value-transfers lane's
`kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md` witnesses
updated for iRules.

#### Work items

**CC9.1 — versioned stamps.**
Files: `rust/tcl-registry/src/spec.rs` (`codegen_hook_windows: &'static [StampWindow<CodegenHookId>]`,
`inline_codegen_hook_windows`, `semantic_operation_windows`,
`native_lowering_windows` beside the unversioned fields, each a
`StampWindow<T> { lifecycle: Lifecycle, value: T }` mirroring `ArityWindow`
in `arity.rs`), `resolved_invocation.rs` (selection at the primary
release; a query whose release range straddles two windows or a window
edge answers `None` — plain dispatch), the loader (`codegen_hook -native ID -introduced V ?-deprecated V? ?-retired V?`
repeatable, as `arity` is), the four studio surfaces,
`docs/design/spec-dsl-examples/README.md`. Tests: `registry_sweep.rs` —
`stamp_windows_never_overlap` (the arity-window gate's twin);
`rust/tcl-compiler/tests/codegen.rs` — `a_stamp_declared_from_9_0_declines_under_a_profile_spanning_8_6`
(negative: pinned at 9.0 it specialises). Model: opus. Size: M. After:
CC4.2.

**CC9.2 — the evidence gate.**
Files: `rust/tcl-dialect/src/profile.rs` (`DialectProfile::evaluation_point(&self) -> Option<TclVersion>`:
`runtime_base` when the profile's row in `data/reference-toolchains.tsv`
or the profile's own measured-fork note marks it measured, else `None`),
`rust/tcl-dialect/src/version.rs` (`TclVersion::from_profile` delegates to
it), `rust/tcl-registry/src/value_transfer/const_ops.rs:325` (unchanged
call, new answer — the value-transfers lane owns the file; the change is
one line in `version.rs`, B1), `rust/tcl-spec-hooks/src/host.rs` (the
`tcl-version` context key for a vendor profile). Changes: under
`f5-irules` a versioned fold answers as Tcl 8.4 (`incr` over `010`
folds to 9; a bignum overflow declines) where today it answers the
unanimous subset. Tests: `profile.rs` — `a_measured_vendor_profile_has_an_evaluation_point`,
`an_unmeasured_profile_has_none` (negative); the value-transfers lane's
`differential_fold.rs` rows for iRules updated with it (B1). Docs: the
lane's KCS note; `docs/design/registry/dialect-profile-model.md` (the
gate paragraph). Model: opus. Size: M. After: —.

**CC9.3 — package version windows.**
Files: `rust/tcl-dialect/src/model/authored_surface.rs`
(`SurfaceQuery::packages: &'a [PackageFloor<'a>]`, `PackageFloor { name, version: Option<&str> }`;
`surface_admits` consults a package row's `Lifecycle` against the floor),
every `SurfaceQuery` constructor (`core`, the profile's `surface_query`),
`rust/tcl-registry/src/registry.rs` (`ambient_package_floor` feeds the
floor). Tests: `rust/tcl-registry/tests/library_axis.rs` gains
`a_package_row_introduced_after_the_floor_is_not_admitted` (negative: at
or above the floor it is). Model: opus. Size: M. After: —.

#### Ordering and checkpoints

CC9.2 (`wip(consumer-contracts): evaluation points through the evidence gate`),
CC9.3 (`… package version windows`), CC9.1 (`… versioned codegen stamps`).
Green: `cargo test -p tcl-dialect`; `cargo test -p tcl-registry`;
`cargo test -p tcl-compiler`; `cargo test -p tcl-spec-hooks`.

#### Review checklist

- "Per measured row, never by name": no `match profile.name` decides a
  release.
- A straddle is a decline to plain dispatch, never a silent choice of one
  row.
- Risks: the iRules fold delta reaching `tcl-irule-test` fixtures
  (`gen-irule-test-data --check`); `SurfaceQuery` is `Copy`-shaped and
  widely constructed — the compile fan-out is large but mechanical.

#### Behavioural deltas expected

- Under `f5-irules` and `f5-iapps`, versioned folds answer as Tcl 8.4.
- A codegen stamp with a window outside the profile's range specialises
  nothing.

### Step 10 — the extension legs

**Goal.** In the page's order: the conservative default fact; the C scan,
the probe and the bridge; the host opt-in `load`; the authored header
with its CI gate, `Tcl_CreateObjCommand` and the shared-table `Command`
variant; the engine interface's completion and variable doors;
WASM-hosted evaluation last.

**Exit evidence.** `rust/tcl-cshim/tests/pkga_e2e.rs` green against the
authored header on both legs (the same vectors); the CI gate compiling
`pkga.c` for `wasm32-wasi`; `sandbox_isolation.rs` unchanged; one
extension evaluated under WASM through the engine interface with fuel.

#### Work items

**CC10.1 — the conservative default fact.**
Files: `rust/tcl-registry/src/spec.rs` (`CommandSpec::extension_default(name: &'static str) -> Self`:
`Arity::any()`, `Traits::EVALUATES_CODE | CREATES_BARRIER | CREATES_DYNAMIC_BARRIER | UNSAFE | SAFE_INTERP_HIDDEN | TAINT_SINK | TAINT_SOURCE | ESTABLISHES_VARIABLE_TRACE`
(the implementer verifies each name in `traits.rs`), `side_effects`
Unknown reads and writes, `command_table_effect: Some(Unknown)`,
`completion` any code with a normal successor, `pure: false`,
`runtime_backing: HostNative`), `rust/tcl-registry/src/model/declaration.rs`
(`DeclaredCommand::extension(name)` seeded from it and narrowed by the
CC3.5 flags axis by axis), `rust/tcl-compiler/src/analyser/types.rs` (a
stub for a name a `load`-provided package declares starts from the
default: the `provides` directive / extra-commands path). Tests:
`registry_sweep.rs` — `the_extension_default_is_at_the_top_of_every_axis`
(negative: narrowing `-pure` clears `TAINT_SOURCE`? No — it clears only
purity; assert the other axes stay). Model: opus. Size: S. After: CC7.1,
CC3.5.

**CC10.2 — the C scan, the probe and the bridge.**
Files: `rust/tcl-spec-studio/src/infer.rs` (`scan_c_source(text) -> Vec<CDeclaredCommand>`
over `Tcl_CreateObjCommand` / `Tcl_CreateCommand` / `Tcl_NRCreateCommand`
name literals, `Tcl_PkgProvide(Ex)`, `Tcl_WrongNumArgs` usage strings,
`Tcl_GetIndexFromObj(Struct)` tables, variable and eval calls, factory
patterns; blind to the OO C API and C-built ensembles by declaration),
`rust/tcl-cli/src/commands/spec.rs` (`tcl spec import --c-source DIR`
and `--probe PACKAGE` (the sandboxed `package require` under `tcl-pkg`'s
policy, listing the `info commands` delta), each row carrying its
provenance `c-scan` / `probe`), `rust/tcl-cshim/src/lib.rs`
(`Loaded::declared_surface(&self) -> Vec<DeclaredCommand>` — the bridge:
every registered command at the default fact), `ai/claude/skills/spec-author/SKILL.md`.
Tests: `infer.rs` — `the_scan_finds_pkga_s_commands_and_provide`
(over `rust/tcl-cshim/tests/c/pkga.c`; negative: a `Tcl_CreateObjCommand`
with a computed name yields a row marked dynamic); `pkga_e2e.rs` —
`the_loaded_report_bridges_to_the_default_fact`. Model: opus (the scan),
sonnet (the bridge). Size: L. After: CC10.1.

**CC10.3 — the host opt-in `load` bridge.**
Files: `rust/tcl-engine-tclvm/src/lib.rs` (`TclVmEngine::enable_static_extensions(&mut self, table: &'static [(&'static str, tcl_cshim::InitProc)])`:
a host command `load` that resolves its name in the table and calls
`tcl_cshim::Interp::load_static` on a shim interpreter sharing this
engine — the shim's `Interp` gains a constructor over `&mut E` for the
duration of one load, `Interp::load_static_into(engine, init)`, so the
engine is not moved), `rust/tcl-cshim/src/lib.rs`, `rust/tcl-vm-cli`
(the opt-in flag). Constraints: trusted host code only; no pack word
reaches it; `sandbox_isolation.rs` stays green. Tests: `pkga_e2e.rs` —
`load_through_the_host_bridge_defines_the_commands` (negative: a name not
in the table is `couldn't load`). Model: opus. Size: M. After: CC10.1.
Open question Q6.

**CC10.4 — the authored header, the CI gate, `Tcl_CreateObjCommand`, the
shared-table variant.**
Files: `runtime/rust/include/tcl.h` (new, the ABI § 7 scope, with
`TCL_MAJOR_VERSION` switching `Tcl_Size` as `tclshim.h`'s
`TCL_SHIM_TCL_MAJOR` does; every declaration a host does not implement is
absent from that host's build through `TCL_HOST_NATIVE` /
`TCL_HOST_WASM` guards), `rust/tcl-cshim/include/tclshim.h` (deleted),
`rust/tcl-cshim/build.rs` (compiles `tests/c/pkga.c` against the authored
header), `rust/tcl-cshim/src/obj.rs` (`#[repr(C)] pub struct Obj` publishes
the § 4.2 layout), `runtime/rust/src/capi.rs` (`Tcl_CreateObjCommand`,
`Tcl_DeleteCommand`), `runtime/rust/src/interp.rs` (`Command::ObjCmd { proc: Tcl_ObjCmdProc, client_data: *mut c_void, delete_proc: Option<Tcl_CmdDeleteProc> }`
— a shared-table function index under WASM; `dispatch` calls it with the
prebuilt argv `tcl_invoke_argv` already routes), `.github/workflows/`
and `Makefile` (`check-c-extension-wasm`: wasi-sdk clang compiles
`pkga.c` for `wasm32-wasi` against the header and links it with the
runtime; in `xtask-check`'s CI mirror), `docs/design/runtime/c-extension-shim.md`
§ *The implemented subset* and `c-extension-abi.md` § 7 (the "today"
sentences go), `docs/GLOSSARY.md`, `docs/kcs/kcs-qa-what-is-the-c-extension-shim.md`.
Preserve: every `pkga_e2e.rs` expectation byte-for-byte. Tests:
`pkga_e2e.rs` unchanged; `rust/tcl-compiler/tests/wasm_real_link.rs`
gains `a_compiled_script_calls_an_extension_registered_command` (the § 12
seam: `Foo_Init` registers `foo`, a compiled script calls it; negative:
before registration the call is `invalid command name`). Model: opus.
Size: L. After: CC10.2.

**CC10.5 — the engine interface's completion and variable doors.**
Files: `rust/tcl-engine-api/src/lib.rs` (`HostCommand::invoke` returns
`Result<HostOutcome, EngineError>` with `HostOutcome { value, code: CompletionCode }`;
`Engine::variable(&self, name) -> Result<Option<Value>, EngineError>`,
`set_variable`, `unset_variable`; `Engine::eval_in_invocation(&mut self, script) -> Result<HostOutcome, EngineError>`),
`rust/tcl-engine-tclvm/src/lib.rs`, `rust/tcl-cshim/src/lib.rs:207–219`
(the `TCL_BREAK` / `TCL_CONTINUE` / `TCL_RETURN` narrowing goes;
`Tcl_ObjSetVar2`, `Tcl_GetVar2Ex`, `Tcl_EvalObjEx` exported through the
doors), `rust/tcl-spec-hooks/src/host.rs` (unchanged: a hook body's
completion is still an abstention). Tests: `rust/tcl-cshim/src/lib.rs`
unit rows — `a_c_command_returning_break_is_a_break_completion`;
`pkga_e2e.rs` gains a `Tcl_EvalObjEx` row; `rust/tcl-engine-tclvm`'s
tests gain the variable door. Model: opus. Size: M. After: CC10.4.

**CC10.6 — the WASM runtime implements `Engine`; WASM-hosted evaluation.**
Files: `runtime/rust/src/engine.rs` (new: `impl Engine for Interp` under a
`engine` feature — the hook host and the shim can target it),
`rust/tcl-vm-wasm` (the second WASM engine with the browser host, the
fallback profile, no filesystem — the same statements), `rust/tcl-spec-hooks/src/host.rs`
(a body tested on two engines), the extension evaluation route
(`EvalRoute::Implementation` with `EvaluatorCapability` naming the
extension artefact hash in the memo key; WASI imports for the clock,
filesystem, randomness, environment and arguments stubbed per side module
in the link harness; eligibility from a declared route on the command;
a per-extension differential vector against the real shell in
`rust/tcl-compiler/tests/wasm_real_link.rs` gating shipping). Tests:
`rust/tcl-spec-hooks/tests/families_e2e.rs` runs every family on both
engines; `wasm_real_link.rs` — `pkga_evaluates_under_wasm_with_fuel`
(negative: a body over budget is a `BudgetExceeded`, never a hang). Model:
opus. Size: L. After: CC10.4, CC10.5.

#### Ordering and checkpoints

CC10.1, CC10.2, CC10.3, CC10.4, CC10.5, CC10.6 in that order, one
checkpoint each (`wip(consumer-contracts): the extension default`,
`… describe an extension from three sources`, `… the host load bridge`,
`… one header, two hosts`, `… the engine's completion and variable doors`,
`… evaluation under WASM`). Green: `cargo test -p tcl-cshim` (native and,
where the toolchain exists, the wasm check), `cargo test -p tcl-runtime`,
`cargo test -p tcl-engine-tclvm`, `cargo test -p tcl-spec-hooks`,
`cargo test -p tcl-compiler --test wasm_real_link`; `make ensure-rust-deps`
before any WASM build on macOS.

#### Review checklist

- Extensions are recompiled, never binary-loaded; no pack word loads native
  code; a shimmed command is not a `-native` hook (`sandbox_isolation.rs`
  is the proof and stays untouched).
- A declaration outside a host's subset is absent from that host's leg of
  the header, never present and failing.
- C is never evaluated natively; the WASM route is gated by fuel, memory,
  a declared route, per-module WASI stubs and the differential vector.
- The engine interface carries the completion code before any hosted
  extension exercises the default fact.
- Risks: `Tcl_Obj` layout changes breaking `pkga_e2e.rs` byte-for-byte
  (the vectors are the gate); the `load` bridge's engine ownership
  (Q6); the WASI stubs leaking host time or randomness into an
  evaluation (assert determinism across two runs).

#### Behavioural deltas expected

- The shim compiles extensions against `runtime/rust/include/tcl.h`;
  `tclshim.h` is gone.
- `Command::ObjCmd` commands dispatch from compiled WASM code.
- `tcl spec import --c-source` and `--probe` produce draft packs with
  provenance rows.

### Boundaries with the other lanes

**B1 — value-transfers (slices 2–13).** Its slice 2 is at checkpoint
`f3f9390f` (not complete); its working tree holds uncommitted edits to
`rust/tcl-registry/src/value_transfer/*`, `rust/tcl-compiler/src/{sccp,value_transfer,static_loops,intervals,type_infer,taint}.rs`,
the optimiser, `rust/tcl-registry/tests/{value_transfers,differential_fold}.rs`,
`rust/xtask/src/value_transfers.rs`, and `docs/generated/value-transfers.md`.
This lane never edits any of those files. Shared files and their owners:

- `rust/tcl-registry/src/spec.rs`: the lane owns `semantics` on the three
  scopes; this lane adds `clause_grammar` (CC2.2), `option_effect_families`
  (CC2.6), `alias_of` (CC4.1), `runtime_backing` (CC7.1), the stamp
  windows (CC9.1) and deletes `substitution_resolver` and the five
  `CaseListSpec` option fields (CC2.6). Disjoint fields; whichever lands
  second rebases.
- `rust/tcl-registry/src/resolved_invocation.rs`: the lane owns
  `InvocationSemantics::value`; this lane adds the query methods (CC2.8).
  Disjoint impl blocks.
- `rust/tcl-registry/src/value_transfer/`: the lane's. This lane only
  *reads* `HandlerMatch`, `OperandId`, `AnalysisContext`; CC2.8's
  `AnalysisContext::surface_query` is the one addition, in `context.rs`,
  landed as a two-line commit the lane is told of.
- `rust/tcl-spectcl/src/loader.rs`, `render_spectcl.rs`, `schema.rs`,
  `draft.rs`, `help.rs`, `coverage.rs`: slice 4 adds the `semantics` /
  `evaluate` / `facts` spellings; this lane adds `clause_grammar`
  (CC2.3), `-effect` (CC2.5), `option -effect` (CC2.7), the resolver
  family (CC2.14), `alias_of` (CC4.1), `runtime_backing` (CC7.1), the
  windows (CC9.1). Each is its own statement or row flag; the `GAPS`
  table is edited by both (the lane removes `semantics`; this lane removes
  `semantic_operation`, `definition_body`, `substitution_resolver`,
  `pattern_arg_resolver`). Order: this lane's step 2 before slice 4 where
  possible; on conflict the second rebases and re-runs `spectcl_roundtrip`.
- `rust/tcl-spectcl/src/cache.rs`, `loader/eval.rs` (`EvalOptions`,
  `EvalSnapshotKey`): slice 4 adds target values; CC3.1 adds `trust`.
  Disjoint fields; the second rebases.
- `Engine::set_release` (`rust/tcl-engine-api`): slice 4's. CC8.2 waits
  for it.
- `rust/tcl-lsp-db/src/lib.rs` `spec_pack_key` → `compilation_unit`:
  slice 4's analysis half; CC6.3's compile-service half follows it.
- `subst`: slice 5 owns `template_plan` and the four template-word
  consumers (`dynamic_names.rs`, `push_substituted_commands`,
  `lowering/mod.rs`'s subst fold, `specialise_factories.rs`, extract-proc);
  this lane owns `subst_.rs`'s option rows and families, the
  `option_effects` derivation and the `substitutions_performed` projection
  (CC2.6). Slice 5 builds `template_plan` over `option_effects`.
- `rust/xtask/src/value_transfers.rs`, `docs/generated/value-transfers.md`,
  the migration plan's ledger: the lane's. CC2.1's gate is a sibling
  module; CC2.15 merges the roots after slice 2 lands. The ratchet pins in
  `value_transfers.rs` for files this lane rewrites (`lowering/structured.rs`,
  `analyser/commands.rs`, `analyser/oo.rs`, `analyser/handlers.rs`) are
  *not* lowered by this lane; the lane lowers them when it reviews those
  files, or this lane files a one-line request.
- `rust/tcl-dialect/src/version.rs` (CC9.2) changes what
  `const_ops.rs:325` answers for vendor profiles; the lane's
  `differential_fold.rs` rows for iRules move with it — CC9.2 lands only
  with the lane's agreement and its rows updated in the same checkpoint.

**B2 — diagnostic-policy (slices 4–7 at checkpoint `5bc40e95`).** It owns
`rust/tcl-lsp-core/src/diagnostic_policy.rs`, `diagnostic_report.rs`,
`config_ini*`, `code_actions.rs`, the diagnostic-code table, the xtask
catalogue generators, and the adapter edits in `rust/tcl-lsp-server/src/lib.rs`,
`rust/tcl-cli/src/commands/diag.rs`, `policy.rs`, `rust/tcl-mcp/src/tools.rs`.
This lane's server edits (CC3.3: `apply_initialization_options`,
`did_change_configuration`, `spec_pack_discovery`, `reload_spec_packs`)
and MCP edits (CC3.4: `spectcl.rs`, the `spectcl_check` entry in
`tools.rs`) are in different functions; they land after that lane's
adapter commits and rebase over them. This lane adds no diagnostic code
(W147 already exists); the option-relation finding CC2.6 adds is a
registry relation the existing producer reports. No new `DiagCode`, so no
catalogue regenerates.

**B3 — ordering across lanes.** Slice 4 (value-transfers) before CC6.3
and CC8.2; the diagnostic-policy adapter commits before CC3.3 and CC3.4;
everything else in this lane is independent of both.

### Decisions taken

- **D2.1** `ClauseGrammarSpec::head` is a `ClauseRow` (keyword `None`,
  `Once`), and keywordless `Once` rows are legal and entered in
  declaration order. Reason: the page's own DSL row carries
  `head {Body} -timing protected`, and `for`'s three fixture bodies need
  three timings that `&[ClauseSlot]` cannot carry.
- **D2.2** `ClauseSlot::handler` is `value_transfer::answers::HandlerMatch`;
  no second enum. Reason: the page says `HandlerPlan` carries "this page's
  `HandlerMatch`", and the type already exists.
- **D2.3** `clause_grammar` sits on `CommandSpec` and `SubCommand`.
  Reason: `dict for`, `dict map`, `dict update` and `array for` are
  subcommands.
- **D2.4** `template_plan` is not added in step 2; it is slice 5's.
  Reason: an answer with empty `script_regions` and `reads` would read as
  a fact; `substitutions_performed` as a projection of `option_effects` is
  what the page's four callers need until then.
- **D2.5** The derived-query layer is inherent methods on
  `ResolvedInvocation` plus `CommandRegistry::invocation(words, ctx)`;
  `CallWords` is `InvocationWords`; `ResolvedEffects` is `EffectFootprint`.
  Reason: the page calls `RegistryQueries` pseudocode and the type already
  answers three of the queries.
- **D2.6** `member_rows` is `DefinitionBodyGrammar::member_row` over one
  member statement; the analyser folds the body. Reason: a definer call's
  body is a script the compiler segments; the registry never re-segments.
- **D2.7** The SpecTcl and SslicTcl document grammars' members carry
  `MemberEffect::Configuration`. Reason: a DSL row declares data and opens
  no method frame; the vocabulary stays family-neutral.
- **D2.8** `cfg_lower.rs`'s `on ok` test reads `HandlerMatch::CompletionCode`
  plus the registry's completion-code word parse, not `is_default`.
  Reason: `try` has no default clause; `ok` is a value of the pattern
  word.
- **D2.9** The per-axis lint is a sibling gate (`registry-axes`) with its
  own marker carrying an expiry, the migration plan's axis names, and a
  ratchet. Reason: `value_transfers.rs` is the other lane's in-flight
  file; the page requires an expiry the existing waiver lacks.
- **D2.10** The two clause-keyword tables become derivations pinned equal
  to today's lists. Reason: the page's "read the slot's `noise` word"
  without changing a generated keyword list.
- **D2.11** The hook retirement table of CC2.13 is the step's residue
  list; `Set`, `DictWith` and `RegexPatternCapture` go on the ledger with
  their slices. Reason: the page's residue classes, applied to each
  handler's verified body.
- **D3.1** `WorkspaceTrust` lives in `tcl_dialect::model::environment`
  beside `Provenance`; `Tier` is unchanged and the trust rides `PackFile`,
  `MergedPack`, `EvalOptions`, `EvalSnapshotKey` and the cache key.
  Reason: `Tier` is the discovery location and its ordering is
  precedence; trust is a second axis, and a snapshot evaluated under a
  different provenance registers differently (E-R2).
- **D3.2** The wire is `initializationOptions.workspaceTrust` and the
  setting `tclLsp.workspaceTrust`; VS Code sends `workspace.isTrusted`;
  an absent value is trusted. Reason: the page's "a client that does not
  report it is treated as trusted".
- **D3.3** Dormancy is decided from `MergedPack::provenance()` in two
  places that read one fact: `pack::load_sources` (the notice) and
  `hooks::plan_for` (no slot). Reason: the host never learns trust; an
  unallocated slot is exactly the declared-but-unbound abstention.
- **D3.4** `DeclaredCommand` gains `traits` and `side_effects`;
  `DocumentCommandSurface` gains `traits` and `side_effects` doors;
  `SubCommand::pure` is moot for a stub. Reason: the page's field map.
- **D4.1** `alias_of` is a `CommandSpec` field only. **D4.2** The stamp
  rule runs in `pack::load_sources` on the loaded command. **D4.3**
  `PackFactStamp::content_hash` is the `u64` xxh3 the snapshot key
  interns (the page's `[u8; 32]` would be a second hashing rule, which the
  page forbids for the lockfile's sake). **D4.4** A spec's pack origin is
  a registry side table, not a `CommandSpec` field (no studio surface, no
  `GAPS` row). **D4.5** `SiteClaim`, `PackFactStamp`, `IdentityKind` and
  the manifest live in `tcl-runtime-api`; `FunctionAsm` carries
  `site_claims`.
- **D5.1** Guard identities are keyed by command-token generation and the
  `CommandEnvironment` domain invalidates per token; the interpreter and
  object-dispatch domains stay whole-domain.
- **D6.1** CC6.1 covers the six existing fields; `runtime_backing` joins
  the floor in CC7.1. **D6.2** `DependencyTier` is defined in `tcl-pkg`
  and re-exported by `tcl-registry`'s `model::capability`; `tcl-spectcl`
  depends on `tcl-pkg` for the manifest and lockfile data model only.
  **D6.3** An overlay miss is an error at the ingress.
- **D7.1** `HANDLER_EXTRA`, `STDLIB`, `NOT_REQUIRED` become
  `runtime_backing` rows; `KNOWN_UNBACKED` stays as the drift waiver;
  `xtask` links `tcl-runtime` and `tcl-vm` to ask `backing_report()`.
  **D7.2** The WASM manifest is a custom section `tcl.manifest` encoded
  by one function pair in `tcl-runtime-api`.
- **D8.1** `Engine::set_release` is slice 4's. **D8.2** `docker.rs` stays
  registry-free; `tcl docker create` computes native extensions.
- **D9.1** Versioned stamps are `StampWindow<T>` slices mirroring
  `ArityWindow`. **D9.2** `DialectProfile::evaluation_point` is the
  evidence gate; `TclVersion::from_profile` delegates.
- **D10.1** The authored header is `runtime/rust/include/tcl.h` (the ABI
  § 4.1 places it with the runtime); the shim includes it by path.

### Open questions for the owner

The plan does not block on these; each states what it assumes.

- **Q1** Whether `clause_grammar` should also cover `switch`'s inline
  `pattern body …` pairs. Assumed no: the page keeps `case_list` as a
  value grammar, and the two share vocabulary only.
- **Q2** Whether the studio's clause-grammar form is editable in step 2
  or read-only until a pack needs to author one. Assumed read-only
  (CC2.3); the round trip does not depend on the editor.
- **Q3** The exact set of hooks retired in step 2 (CC2.13's table). The
  page says "most"; the table is the plan's reading of each handler.
- **Q4** `content_hash` as `u64` (D4.3) versus the page's `[u8; 32]`.
- **Q5** `tcl-spectcl` depending on `tcl-pkg` (D6.2), or a data-model-only
  crate split out of `tcl-pkg`.
- **Q6** The `load` bridge's engine ownership (CC10.3): a shim `Interp`
  over `&mut E` for one load, or the bridge living in `tcl-vm-cli` as the
  embedder. Assumed the former.
- **Q7** Whether the studio should seed an `alias_of` suggestion from the
  realm's alias facts (the page: "may seed a suggestion"). Assumed not in
  step 4.
- **Q8** Which new `IntrinsicId` members "grow the intrinsic table by
  family" (CC7.3). Assumed none in step 7; the family split and the
  trace-firing flag land, members follow their own changes.
- **Q9** The `-inline` / capture-variable option relation's message
  (CC2.6): taken from `tclsh9.0`'s error text at implementation time and
  pinned by the differential row.
- **Q10** The `xtask` build cost of linking both runtimes (CC7.2).
