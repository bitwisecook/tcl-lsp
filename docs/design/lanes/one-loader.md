# Lane `one-loader` — one pack loader, not two

## Goal

`SpecTcl` shipped two live implementations of "load a pack": the CST loader
(`tcl_spectcl::loader::load_pack` / `load_pack_with`, the production path for
the LSP through `pack.rs` → `cache::load_pack_cached`) and design E's
evaluation loader (`loader::eval::evaluate_pack`, the path the Spec Studio,
the `tcl` CLI and the MCP server already used). That violated the programme's
standing constraint — no duplicate old-and-new ways of doing a thing, no
compatibility shims — in the most important place, and
`tcl-mcp/src/spectcl.rs`'s header claimed the opposite of what the tree did.

This lane makes `evaluate_pack` the single loader, deletes the CST front end,
reconciles the two caching layers into one, and converts the two-loader
byte-identity gate into a checked-in golden-snapshot gate so the proof
survives the code it was proving.

**Status: complete.** Centralisation ledger row **L1**; redesign §11.4 E9 and
§11.1 O9.

## Decisions

1. **One loader, and what is *not* a second one.** `evaluate_pack_in` is the
   only door from `.tclspec` text to a `Pack`. The row readers
   (`apply_pack_stmt`, `apply_command_stmt`, `apply_subcommand_stmt`,
   `command_from_parts`, `subcommand_from_parts`, `PackTables`, `statements` /
   `block`, and every property reader under them) are **kept**: they are the
   one `SpecTcl` vocabulary — what a word *means* once a registration has been
   captured — and every registration reaches them, whether the file spelled it
   literally or a `foreach` computed it. What was deleted is the CST *front
   end*: the statement-walking driver that duplicated the evaluation loader's
   staging.

   Deleted: `loader::load_pack`, `load_pack_with`, `load_command`,
   `expand_includes`, `report_extra_speclib_blocks`, `cache::load_pack_cached`,
   `cache::LOADER_BUILD`, `eval::evaluate_pack_cached` (moved into the cache),
   `eval::eval_snapshot_memoised` (ditto, as `cache::snapshot_memoised`), and
   `export::{Scope, record, record_command, record_subcommand}` (the CST
   canonical-record builder; `eval::record_nodes` is the live one). All are in
   `cargo xtask retired-api-gate`.

   `tcl_spec_hooks::HookHost::load_pack` was renamed `install_pack_hooks` —
   a different operation that happened to share the retired spelling, and
   renaming it is what lets the gate use the bare `load_pack` needle instead
   of a fistful of qualified ones.

2. **One cache, one key.** `crate::cache` owns both storage tiers behind one
   door, `cache::evaluate_pack_cached(source, tier)`, keyed by one identity:
   `EvalSnapshotKey` (content hash × vocabulary × loader-eval version × tier),
   stamped with the build by `stamp_build`. The in-memory tier holds the
   finished `Pack` (it cannot go to disk — a `CommandSpec` holds function
   pointers into this binary); the on-disk tier holds the segmentation the
   evaluation loader parses, which is what makes a *new process* cheap.
   `LOADER_BUILD` is gone: `LOADER_EVAL_VERSION` is the one loader-build word
   in the key, and `TCL_LSP_SPEC_CACHE_DISABLE=1` now switches off both tiers
   rather than one.

   Two facts made the reconciliation work rather than merely tidy: the
   evaluation loader reaches the CST through the same `loader::statements`
   door the memo was already installed on, and it asks for the same text more
   than once per load (verbatim index, then the static fast path), so the memo
   is now load-bearing *within* one load as well as across processes.

3. **A second *use*, not a second loader: `EvalOptions::static_fast_path`.**
   The loader has two routes to the same staging — the static fast path
   (capture straight from the CST) and the interpreter. That duality is real
   and worth gating, so the flag exists to run a pack both ways and compare.
   It is `true` in production, always, and is deliberately **not** part of the
   snapshot identity: the two routes cannot produce different answers, which
   is what the gate proves.

4. **The tier reaches the loader.** `pack.rs` passes each discovered file's
   `Tier` into the load — it is the honest provenance identity and it belongs
   in the cache key. See "Behavioural deltas" for what that did and did not
   change about E-R2.

5. **Golden snapshots replace the two-loader gate.** See below.

6. **No consumer needed a parse-only mode.** The lane looked for one, because
   a legitimate second *use* would have justified a mode. There is none:
   `upgrade.rs` locates rows to rewrite by lexing the source directly
   (`speclib_version_span`, the byte-range discipline) and never wants a
   `Pack`; the studio's `render_spectcl` and the WASM DSL highlighter work
   from a draft or from tokens; nothing asks for "the declarations without
   running anything".

   And the question is largely dissolved rather than answered: a **wholly
   declarative pack never starts an interpreter at all** now, so an editor
   feature that must not run a sandbox already gets that for every pack
   shipped and for every pack an author writes declaratively. For a pack that
   *does* template there is no honest non-evaluating answer to give — the
   registrations only exist once the program has run — so a parse-only view
   would have had to either lie or refuse, and refusing is what the budget and
   determinism contracts already do.

## The gate's shape

`rust/tcl-spectcl/src/golden.rs` renders one loaded pack; `cargo xtask
pack-goldens` writes those renderings to `rust/tcl-spectcl/tests/golden/*.snap`
(24 files, 1,872 lines, 284 KB) and `--check` verifies them;
`rust/tcl-spectcl/tests/golden_packs.rs` compares on every `cargo test`
(24 packs, 1,515 commands, 45 notices, 1.2 s). Wired into `make xtask-check`
as `xtask-pack-goldens`. One renderer, so the file the verb writes and the
string the gate compares cannot drift; one inventory
(`golden::shipped_packs`), shared with the fast-path gate.

Pack-level facts and every notice appear in full; each command is one line
carrying its loader-level facts plus **digests** of the exhaustive
`CommandSpec` debug rendering, the `HookDecl` list and the clause grammar.
Digests because the full text is 8.6 MB in single lines of several kilobytes
— unreviewable, and a diff of it says nothing a human can read. The reader
loses nothing: the comparison recomputes the full rendering in-process, so a
failure prints the offending command's complete before/after, which a
checked-in blob would not have. Resolver function-pointer addresses are
normalised to `fn` (they move with every build); *which* resolver a pack
asked for is the stable `hooks` digest beside them.

**Honest accounting of the trade.** The two-loader gate compared two
independent readings *of the same build*: a divergence was caught with nobody
updating a file, but a bug both readings shared was invisible to it. The
golden gate compares against a reading from a *previous* build — the
direction real regressions travel, and it catches a change both halves of one
build would agree on — at the cost that a deliberate change must be recorded
by regenerating. Neither strictly dominates, so the same-build duality was
kept as well: `eval_loader.rs`'s fast-path gate holds the two routes
byte-identical (snapshot **and** `command_entry_json` through a real registry)
over the same 24 packs. Between them the two gates cover both directions the
deleted gate covered.

## Measured (principle P-B)

Dev profile (unoptimised — the acceptance profile), best of 3, on the eight
bundled `specs/*.tclspec`, `eda_xilinx` (788 commands, 20,887 lines) among
them. "cold" is a fresh on-disk cache directory; "warm" is a populated
directory with the in-memory tier dropped, i.e. what a *server restart* pays;
"memo" is a second load in the same process.

| ms (total of 8) | uncached | cold | warm | memo |
|---|---|---|---|---|
| CST loader (before) | 1096.7 | 960.8 | 165.1 | 20.2 |
| eval loader, as landed at first | 1784.8 | 1430.7 | 680.1 | 18.8 |
| + lazy verbatim index | 1237.5 | 1403.4 | 636.3 | 19.3 |
| **+ file-level fast path (shipped)** | **891.3** | **952.4** | **190.3** | **19.7** |

`eda_xilinx` alone: uncached 688.2 → 553.1 ms, cold 609.2 → 594.0 ms, warm
107.3 → 116.5 ms, memo 13.6 → 13.1 ms.

The first row of eval numbers is the finding the task anticipated: as
inherited, the evaluation loader was **1.6× slower uncached and 4.1× slower
warm** than the CST loader on the discovery path. Profiling the warm
`eda_xilinx` load (375 ms) split it as verbatim index 14 ms, staging+replay
33 ms, and **331 ms inside `run_pack_program`** — of which only ~23 ms was the
`speclib` body. The rest was the interpreter lexing and byte-compiling the
whole 1.26 MB file before the first registration word was dispatched, because
the *file's own top level* always went to the VM even when every statement in
it was static vocabulary.

Two widenings fixed it, both in `eval.rs`:

- **The verbatim index is built in two layers.** Only the `speclib` body is
  needed by every load; the per-declaration bodies and the line-keyed
  statement table are needed *only when a body actually reaches the
  interpreter*, and a fast-path body never asks. Building them eagerly cost
  every declarative pack two extra walks of its own statements plus a clone of
  each into a hash table.
- **The fast path now covers the file's top level.** `drive_file_statically`
  attempts the whole file with no interpreter behind it (`Drive { ctx: None }`);
  the moment a body needs real evaluation it gives up with `NEEDS_INTERPRETER`
  and the load restarts on the interpreter path. One attempt, no partial state,
  and the segmentations the attempt did are memoised for the retry. `speclib`
  gained a shared `stage_speclib` so the static driver and the host command
  cannot drift, exactly as `stage_command` / `stage_subcommand` /
  `stage_include` already did.

Net against the CST loader the lane replaces: **uncached 19% faster, cold at
parity, warm 15% slower (165 → 190 ms for all eight packs, 107 → 117 ms for
xilinx), in-process memo at parity.** The remaining warm delta is the staging
step — the evaluation loader clones each captured statement into a staged node
and then replays it, where the CST walk consumed its statement vector in
place. That is the price of one loader that can also template, it is ~25 ms
of unoptimised startup for the whole bundled set, and it is bought back
several times over on a cold cache.

Cache hit behaviour is unchanged in kind: an unchanged pack costs a hash
check and a read; an edited pack keys differently and writes a second entry;
a corrupt, truncated, re-keyed or half-written entry falls back to a fresh
load, byte-identically (`cache.rs`'s mutation test); a target-dependent pack
(E-R1) is never stored in either tier.

## Behavioural deltas accepted

- **E-R2's untrusted class now derives from the tier→provenance map.** Before
  this lane the loader called `Tier::Workspace` untrusted while
  `PackEnvironmentTier::provenance` called the same tier
  `Provenance::WorkspaceTrusted` — a straight contradiction that never bit
  because the CST loader, which had no tier, was the discovery path. Passing
  the real tier in made it decidable, and the resolution is the one redesign
  §6.4 states: the untrusted class is keyed on the **editor's Workspace Trust
  state**, not on where a file was found, so a *trusted* workspace pack may
  still `-override` a shipped command — which is the collision policy
  `install.rs` implements, tests and reports. `untrusted()` is now
  `matches!(provenance, WorkspaceUntrusted | StudioOverride | Document)`;
  today that is reachable through the live Spec Studio override tier, and the
  day the trust state is plumbed it arrives as a tier whose provenance is
  `WorkspaceUntrusted` and the predicate already answers for it. Recorded as
  redesign §11.1 **O9**.
- **`provenance_violation(pack, tier)` is unconditional.** It answers the
  *hypothetical* ("what would an untrusted tier refuse this for"), which is
  what its two callers — the Spec Studio's `untrusted_tier_refusal` and
  `spectcl_check` — actually want; the load's own gate is in `replay`.
- **`available?` on the discovery path.** A workspace pack using `available?`
  is target-dependent (E-R1) and is not cached in either tier. No shipped pack
  uses it.
- **Budgets do not apply to a wholly static pack**, because nothing is
  evaluated. That was already true of the per-body fast path; the file-level
  one extends it to the pack. A pack that templates still runs under the full
  budget, which `a_budget_blowing_loop_fails_closed_naming_the_axis` and
  `a_wall_clock_blowing_loop_names_its_own_axis` cover.

## Gates

- `tcl-spectcl/tests/golden_packs.rs` — the golden gate (+ an orphan check).
- `cargo xtask pack-goldens [--check]`, in `make xtask-check`.
- `tcl-spectcl/tests/eval_loader.rs` — the fast-path gate over all 24 packs
  (the interpreter route needs a raised budget: `eda_xilinx` through the VM
  takes ~33 s in a dev build and blows the production wall clock **by
  design** — that is the fast path's whole reason to exist).
- `cargo xtask retired-api-gate` — `load_pack`, `expand_includes`,
  `report_extra_speclib_blocks`, `LOADER_BUILD`, `eval_snapshot_memoised` at
  zero.

## Open uncertainties

None for this lane. The one thing it *surfaced* rather than settled is O9:
nothing carries the editor's Workspace Trust state to `pack::load`, so the
untrusted-workspace half of §6.4 cannot be enforced honestly yet.
