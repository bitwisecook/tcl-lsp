# Lane `one-loader` — one pack loader, not two

## Goal

`SpecTcl` shipped two live implementations of "load a pack": the CST loader
(`tcl_spectcl::loader::load_pack` / `load_pack_with`, the production path for
the LSP through `pack.rs` → `cache::load_pack_cached`) and design E's
evaluation loader (`loader::eval::evaluate_pack`, the path the studio, the
`tcl` CLI and the MCP server already used). That violates the programme's
standing constraint — no duplicate old-and-new ways of doing a thing, no
compatibility shims — in the most important place.

This lane makes `evaluate_pack` the single loader, deletes the CST loader,
reconciles the two caching layers into one, and converts the two-loader
byte-identity gate into a checked-in golden-snapshot gate so the proof
survives the code it was proving.

## Decisions

1. **One loader.** `evaluate_pack_in` is the only door from `.tclspec` text
   to a `Pack`. The CST *readers* (`apply_pack_stmt`, `apply_command_stmt`,
   `apply_subcommand_stmt`, `command_from_parts`, `subcommand_from_parts`,
   `PackTables`, every row reader, `statements`/`block`) are **not** a second
   loader — they are the one vocabulary, which the evaluation loader replays
   through. What is deleted is the CST *front end*: the statement-walking
   driver that duplicated the evaluation loader's staging.
2. **One cache, one key.** `crate::cache` owns both storage tiers behind one
   door, `cache::evaluate_pack_cached(source, tier)`, keyed by one identity:
   `EvalSnapshotKey` (content hash × vocabulary × loader-eval version × tier).
   The in-memory tier holds the finished `Pack` (it cannot go to disk — a
   `CommandSpec` holds function pointers into this binary); the on-disk tier
   holds the segmentation the evaluation loader parses, which is what makes
   a *new process* cheap. `cache::LOADER_BUILD` is gone: `LOADER_EVAL_VERSION`
   is the one loader-build word in the key.
3. **The tier reaches the loader.** `pack.rs` now passes each discovered
   file's `Tier` into the load, so design E-R2 provenance gating is live on
   the discovery path instead of only in the studio. This is a deliberate
   behavioural delta (below).
4. **Golden snapshots replace the two-loader gate.** See "The gate's shape".

## Status

- [x] `cache.rs` rewritten as the single cache over the evaluation loader.
- [x] `pack.rs` discovery/merge on `cache::evaluate_pack_cached` /
      `loader::evaluate_pack_in`.
- [x] CST front end deleted (`load_pack`, `load_pack_with`, `load_command`,
      `expand_includes`, `report_extra_speclib_blocks`, the unreachable
      `subcommand` arm of `apply_command_stmt`).
- [x] Every caller and test ported.
- [x] Golden-snapshot gate + `cargo xtask pack-goldens` regeneration verb.
- [x] Retired-api gate entries.
- [x] `tcl-mcp/src/spectcl.rs` doc comment corrected.
- [x] Load times measured before and after.
- [x] Redesign §11 + centralisation ledger updated.

## Behavioural deltas accepted

- **E-R2 on the discovery path.** A workspace-tier pack that claims a
  reserved compiled name with `-override`, declares a `dialect` block, or
  claims a reserved compiled environment name now fails to load with a
  provenance error, where before it loaded and was rejected later at
  registration. This is what E-R2 says; the earlier behaviour was an
  artefact of the CST loader not knowing its own tier.
- **`available?` on the discovery path.** A workspace pack using `available?`
  is target-dependent (E-R1) and is not memoised. Shipped packs do not use it.

## Open uncertainties

None outstanding; see "Measured" for the performance finding.
