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
- [ ] C8 / D9
- [ ] R10

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

## Open uncertainties

*(none for R1)*
