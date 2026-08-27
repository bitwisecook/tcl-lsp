# Lane: one vocabulary — retiring the last two duplicate surfaces

Redesign §11.2 rows **D9** and **D18** (clauses **R1** and **R10**);
centralisation ledger rows **C6**, **C8**, **R1**, **R10**.

The owner's standing constraint: *no duplicate old-and-new ways of doing a
thing, and no shims*. The P8 audit left two survivors.

## Goal

1. **R1** — inline `# tcl-lsp: stub` blocks and `.tcl.stubs` sidecars stop
   being a separate overlay type consulted per consumer. They ingest as
   provenance-tagged `SurfaceDeclaration`s and are read through one door.
   `rust/tcl-registry/src/stub_overlay.rs` is deleted.
2. **C8 / D9** — the three "this command changed the command table"
   vocabularies collapse onto `state_transition.rs`'s
   `CommandBindingTransition`.
3. **R10** — the one-oracle *gate*: `pub(crate)` narrowing where that
   suffices, plus the `retired-api-gate` call-site sweep carrying the names
   this lane retires, with self-tests.

## Status

- [x] **R1 — landed.** `stub_overlay.rs` (423 lines) deleted.
- [x] **C8 / D9 — landed.** `CommandTableEffect` is no longer a consumer
      vocabulary, and the per-consumer argument destructuring is gone.
      Ledger C6's *typing* tail stays open (see "Open uncertainties").
- [x] **R10 — landed.** The one-oracle gate: visibility narrowing where
      that is enough, plus an owned-spelling call-site sweep in
      `retired-api-gate` with a `// one-oracle-ok:` escape hatch and
      self-tests.

## Decisions

### R1 — what stubs became

`rust/tcl-registry/src/model/declaration.rs` is the new home:

- `DeclaredCommand` = name + `Vec<DeclaredArgument>` (registry `ArgRole`,
  not a parallel role enum) + an ordinary `SurfaceDeclaration`.
- The declaration's **provider** is a new `Provider::Document` variant:
  active exactly in the buffer holding the row, contributing zero
  specificity breadth. Its **applicability** is the whole new
  `VersionAxisId::document()` axis (a buffer has no release train). Its
  **predicate** is `None`.
- **Provenance** is carried, not re-derived: `Provenance::Document` (a new
  lowest tier on `tcl_dialect::model::Provenance`) for an inline block,
  `Provenance::WorkspaceUntrusted` for a `.tcl.stubs` sidecar.
  `StubCommandDef::from_sidecar` chooses, at ingestion, once.
- `DeclaredSurface` is the per-document generation; `DocumentCommandSurface`
  is **the** door — catalogue generation plus that document's declarations,
  asked once. `TraitScanEnv`'s `registry` + `stub_overlay` pair collapsed
  onto one `surface` field, and `ScanCtx` with it.

**Why a per-document `ContextRegistry` generation was rejected.** A
generation is keyed by `(environment identity, keyed-versions hash, pack
overlay)` and holds `&'static CommandSpec`s. Stubs are per-buffer and
change per keystroke: admitting them to a generation would either leak a
`CommandSpec` per edit or blow the generation cache's key space. The
`Provider::Document` row records exactly that scope difference in the model
rather than hiding it in a side table.

**Why the availability check is context-free.** A `Provider::Document` row
is unconditional by construction, so running the ordinary
`ContextQueries::is_available` over it can only answer `true`.
`declared_rows_are_available_under_the_ordinary_queries` pins that against
the real context queries rather than asserting it in prose, so a future
change to the queries that would make a document row conditional fails a
test instead of silently diverging.

### C8 / D9 — what the three vocabularies collapsed onto

Everything reads `state_transition.rs`'s `CommandBindingTransition`.

**Why that one and not `CommandTableEffect`.** The three vocabularies were
not peers. `CommandBindingTransition` is a *fact* about one invocation —
which name moved where, with each operand typed `Literal` or a
provenance-carrying `Unknown`. `CommandTableEffect` is a *coarse selector*
— three words that say only "this kind of call mutates the table", after
which every consumer re-derived the fact from the argument list itself.
The analyser's `command_aliases` / `renamed_commands` / `deleted_commands`
tables are *indexes*, not a vocabulary: what made them a third vocabulary
was that they were populated by `alias.rs`'s own `detect_rename` /
`detect_interp_alias` / `detect_interp_alias_delete` destructuring, each
with its own `is_dynamic_word` rule, in parallel with the registry's
resolvers. Collapsing onto the facts is the only direction that loses
nothing: the other two are derivable from it, it is not derivable from
them.

**The shape.**

- `state_transition::command_binding` holds the three **stock descriptors**
  (`DEFINES_PROCEDURE`, `RENAMES_COMMANDS`, `CREATES_ALIASES`), lifted out
  of `proc_.rs` / `rename_.rs` / `interp.rs`. The `interp alias` shape guard
  that used to live in `alias.rs::is_interp_alias_shape` moved into the
  stock resolver, where a pack stamping the selector on an unshaped command
  hits it too.
- Every shipped spec that mutates the command table (`proc`, `rename`,
  `interp alias`, `tcl::OptProc`, iRules `proc`) now names its stock
  descriptor and **no longer stamps `command_table_effect` beside it** —
  declared once.
- `CommandTableEffect` survives only as the **pack-authoring selector**: a
  `SpecTcl` pack cannot supply a Rust resolver, so `CommandTableEffect::
  transitions()` resolves its one word to the very same stock descriptor.
  Pinned by `the_pack_selector_resolves_to_the_stock_transitions`.
- `CommandRegistry::command_table_effect` is **deleted**. Its replacement
  is `CommandRegistry::command_binding_transitions(words)`, which resolves
  through the ordinary `resolve_structured_invocation` under the registry's
  own profile mask — no second selection rule (C7/I4 stays intact).
- `alias.rs` keeps the alias *table* and gains the compiler's one bridge:
  `source_word` (the single `is_dynamic_word` application),
  `command_table_transitions`, `is_current_interpreter`, `subject_word`.
  `detect_rename`, `detect_interp_alias`, `detect_interp_alias_delete` and
  `is_interp_alias_shape` are deleted.
- `LegacyEffectBridge::command_table_effects: Vec<CommandTableEffect>`
  became `command_table_mutation: bool`, fed from the resolved transitions
  — that bridge was in effect a *fourth* place the same fact was said.

**Consumers ported** (all now read the facts): `realm.rs`,
`command_binding.rs`, `lowering/mod.rs`, `taint.rs`, `unit_scope.rs`,
`interprocedural.rs`, `analyser/handlers.rs` (both
`static_provenance_command_is_trusted` and `handle_interp_alias`),
`gvn.rs`, `bpf-tcl-ir::semantic_bridge`, `tcl-lsp-core::tk_preview`.

**Vocabulary extension.** `CommandBindingTransition::Alias` gained
`arguments: Vec<TransitionSubject>` — the leading arguments `interp alias
{} Cat {} Dog extra` bakes in. Without it the fact could not express what
`detect_interp_alias`'s third return value carried, and the alias table
would have lost the "prepended arguments decline" rule the indirection
walk depends on.

**What C6 still leaves open.** The analyser tables are now *populated
from* the one vocabulary, and `indirection.rs`'s link walk reads those
tables. The walk itself is a walk, not a vocabulary, so it is not a fourth
one — but the tables are still `AnalysisResult`-shaped rather than
`state_transition.rs`-shaped values, which is the residue C6's target
column names. See "Open uncertainties".

### R10 — the gate's shape, and what it prevents

Two halves, exactly as the ruling asks.

**Visibility, where that is enough.** `Analyser::builtin_command_names`
(the one oracle's registry tier — no caller outside
`tcl-compiler/src/analyser/`) and
`model::declaration::DeclaredSurface::get` (R1's raw per-document table —
the door is `DocumentCommandSurface`) are now `pub(crate)`, beside P1-G's
already-narrowed cache doors and `ProfileQueries`.

**The call-site sweep, for the doors that cannot be narrowed.** A second
pattern family in `rust/xtask/src/retired_api_gate.rs`, `OWNED`, names each
centralised *answer* and the repository-relative path prefixes whose files
may write it. Writing one elsewhere fails the gate; the escape hatch is a
`// one-oracle-ok: <reason>` waiver **plus** a row in the centralisation
ledger's §3 table — a marker deliberately distinct from `retired-api-ok:`,
so neither waiver licenses the other kind of exception.

| Answer | Owned by |
|---|---|
| `CommandExistenceOracle`, `command_existence_oracle`, `builtin_command_names`, `w123_registry_known_names` | `rust/tcl-compiler/src/analyser/` |
| `has_command_in_this_dialect`, `all_dialect_command_names` | `rust/tcl-registry/src/`, plus `rust/tcl-compiler/src/codegen/` (issue #1427: a constant fold skips the runtime availability gate, so it must ask) |
| `command_binding_transitions`, `command_table_transitions` | `rust/tcl-registry/`, `rust/tcl-compiler/src/`, `rust/tcl-lsp-core/src/tk_preview.rs` |
| `DeclaredSurface` | `rust/tcl-registry/src/`, `rust/tcl-compiler/src/analyser/`, `rust/tcl-lsp-db/src/` |

The retired family additionally gained this lane's deletions —
`StubOverlay`, `stub_overlay`, `StubSig`, `StubSigFlags`,
`build_stub_overlay`, `to_stub_sig`, `command_table_effect(`,
`command_table_effects`, `detect_rename`, `detect_interp_alias`,
`detect_interp_alias_delete`, `is_interp_alias_shape`.

**What it now prevents.** A consumer cannot grow a second existence oracle
by assembling its own known-name set beside `builtin_command_names`; a
second availability rule by asking `has_command_in_this_dialect` outside
the registry and the folder; a fourth command-table vocabulary by building
its own source-word bridge beside `tcl_compiler::alias`; or a second
per-document command table beside `DocumentCommandSurface` — without either
a reviewed, written-down waiver or a red gate.

**A matcher fix the new needles forced.** The boundary rule now applies
only at an end the needle itself spells with an identifier character. A
needle carrying its own punctuation (`command_table_effect(`,
`availability_for_name(`) previously matched nothing at all, because the
gate demanded an identifier boundary *outside* punctuation that already is
one. Both required properties survive: an identifier boundary is still
enforced on both sides wherever the needle spells one (so `arg_rows` still
does not match inside `project_arg_rows`), and the seeded-violation test
still asserts an exact finding count per row — the
`DialectProfile::availability_for_name(name)` row now honestly says 2,
because that line really does carry two retired spellings.

**What was not attempted, and why.** Full `pub(crate)` on
`CommandRegistry::command_names` / `get_for_dialect` — the literal reading
of "the only callers of the raw lookup layer are the two typed views" — is
not achievable as the tree stands: about 45 production call sites read them
for spec *content* (hover text, argument roles, formatter presentation),
which is not an existence answer. The sweep carries that half, keyed on the
answers rather than the doors, and the redesign's D18 row records the
divergence.

## Behavioural deltas

- **`StubSigFlags` is gone, and no flag is carried onto the declaration.**
  The registry-side copy of the `-barrier`/`-loop`/`-pure`/`-mutator`/
  `-unsafe`/`-scope_alias` bit set was *constructed and never read* — the
  only production reader of a stub flag reads the analyser-side
  `StubFlags` on `StubCommandDef`, which is untouched. Principle P-C: the
  fact comes back with the consumer that needs it. Recorded in
  `docs/design/contracts/dialect-stubs.md`.
- **`StubOverlay::fingerprint` is gone.** It had zero production callers;
  cache invalidation already rides the document's own text and lsp-db's
  `sidecar_stubs_epoch` salsa input, which is exactly what R1 says
  ("subsumed by the generation/overlay hash").
- **Role lookup is now a union everywhere.** `DocumentCommandSurface::
  arg_indices_for_role` unions the catalogue's answer with the document's.
  That is byte-identical to both former call sites (`scan_deep` already
  unioned; `resolve_arg_roles` built a per-index map in a fixed role order
  where the last role won either way) and it states §6.4's untrusted-tier
  rule directly: a declaration may add a role position, never remove one.

### C8 / D9 deltas (each with a citing comment at the site)

- **A wrong-arity `rename` now states nothing.** `rename a b c` is
  `wrong # args`, which moves nothing; the arity rule moved from
  `realm.rs`'s `record_rename` into the stock resolver, where every
  consumer sees it. `static_provenance_command_is_trusted` previously
  *distrusted* such a call (`let [old, new] = args else { return false }`)
  and now ignores it, which is what tclsh does.
- **An `interp alias` query form no longer counts as a table mutation.**
  `interp alias {} x` reads a binding back; the coarse selector said
  "creates aliases" regardless, so `command_binding.rs`'s wildcard gate,
  `unit_scope`'s opaque-caller scan and `interprocedural`'s class-clearing
  fired on it. The facts say it mutates nothing.
- **A subcommand-shaped mutator now resolves by unique prefix.** The
  retired `command_table_effect` looked the subcommand up by exact
  spelling; `resolve_structured_invocation` applies the ordinary ensemble
  rule, so `interp ali {} a {} b` is now seen as the alias creation tclsh
  actually performs. (`interp aliases` is its own subcommand and still
  states nothing.)
- **`tcl::OptProc` and iRules `proc` now state a `Define` fact.** Both
  stamped the selector but carried no transition descriptor, so every
  transition consumer saw `UnknownInvocation` for them. They now produce
  the same `Define` `proc` does.
- **The command-trace read is now keyed on the facts.** `EffectFootprint`'s
  legacy command-table bridge fired off the stamped selector; it now fires
  off `StateTransitions::touches_command_bindings()`, so it also fires for
  a dynamic operand that only widened the bindings domain, and no longer
  fires for a query form.
- **`taint.rs` handles the deletion half.** `rename OLD {}` reached it as
  `RenamesCommands` with an empty `new`, which removed the class fact; it
  now arrives as a `Delete` fact and removes it explicitly. Same outcome,
  stated rather than inferred from an empty string.

## Open uncertainties

- **C6's tail.** The analyser's `command_aliases` / `renamed_commands` /
  `deleted_commands` / offset maps are now populated *from* the one
  vocabulary, so they are indexes rather than a parallel derivation. They
  are not yet `state_transition.rs`-*shaped* values, and `indirection.rs`
  walks them in `AnalysisResult` form. Re-typing them reaches ~30 files
  across `tcl-lsp-core`'s navigation providers (definition, references,
  rename, hover, minify, workspace-index), which is a lane of its own.
