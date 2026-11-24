# Caching, indices and lookup layers

How analysis results, registries, indices and names are cached, interned and invalidated across the compiler, registry, SpecTcl, the LSP core, the salsa database and the server, and what one architected layer would need. Part of [tidying the architecture](architecture-tidy-plan.md); fault 15 of the [cross-cutting review](cross-cutting-review.md).

**Verdict:** salsa memoises the top layer (whole documents and some per-body results). Underneath sit about 110 caches salsa cannot see, using five staleness mechanisms that do not compose (pointer and `Weak` identity, ad hoc generation counters, salsa revisions, content fingerprints, size-triggered clear-all). Registries and spec packs are treated as static but are versioned, so they are held in process-global maps keyed by generation, several of them leaked on purpose. The server analyses the same document through several pipelines, each with its own cache. The result is slow incremental work, a large resident footprint and memory that ratchets on pack churn.

## Measured cost

An in-process probe (`Analyser::new().analyse`, `tcl9.0`, test profile at opt-level 2, on `65261ff6`) over a two-line script and a 687-line Tcl library: [`Dechcaudron/overrides`](https://github.com/Dechcaudron/overrides) at `fe18ee3dcc0ab7e034596ec0d79e3db3cb34978d`, its [`cmdOverrides.tcl`](https://github.com/Dechcaudron/overrides/blob/fe18ee3dcc0ab7e034596ec0d79e3db3cb34978d/src/overrides/cmdOverrides.tcl) followed by its [`overrides.tcl`](https://github.com/Dechcaudron/overrides/blob/fe18ee3dcc0ab7e034596ec0d79e3db3cb34978d/src/overrides/overrides.tcl) without the `source` and `package provide` lines, ending in one probe line (`set probe 1`):

| Measure | Result |
| --- | --- |
| two-line script | 63 ms cold, 17 ms per repeat |
| 687-line file | 2.16 s cold, 1.27 s per repeat |
| the 687-line file analysed 40 more times | resident memory flat |
| 40 analyses of the same file with one generated procedure appended each time | +231 MB after the first 10, then flat at about 237 MB |
| `CommandRegistry::build_default()` | about 2.4 ms per call (unoptimised build) |

There is a fixed cost of about 17 ms per analysis even for two lines, whole-file analysis is what an edit triggers (the salsa document queries are whole-file), and the capped caches fill to roughly 230 MB after ten distinct documents and stay there. This probe does not reload spec packs, so it does not exercise the generation-keyed growth below; [#2543](https://github.com/bitwisecook/tcl-lsp/issues/2543)'s pack-churn measurements do.

### Per-procedure W210 spike

The probe tracks W210 for each literal top-level procedure and rebases the warnings to their current file positions. It uses the existing analyser on each procedure declaration in the fixture's namespace. A production query would also need the dependencies that affect a procedure's warnings, such as caller constants, callee transfers and registry overlays. [Probe source](../../../rust/tcl-lsp-db/examples/w210_body_probe.rs).

On Linux x86-64 (AMD EPYC 9V74, Rust 1.100.0, opt-level 2), the original file executed 17 procedure queries. Replacing `return [llength [array get LambdaStacks $cmdName]]` with `set cmdName $missingProbeValue` in `IsOverridden` re-executed one `procedure_w210` query in each of five trials. The edits took 37.37–43.97 ms, and the same process took 1.47 s to analyse the edited file afresh. In a separate run, adding the same missing read to the `variable LambdaStacks` line and keeping the nested return took 47.72–50.14 ms, with two of five trials over budget.

Reconstruct the 687-line input from the two files pinned above. Drop the trailing blank line from `cmdOverrides.tcl` before concatenating.

```sh
fixture_dir=$(mktemp -d)
fixture_url=https://raw.githubusercontent.com/Dechcaudron/overrides/fe18ee3dcc0ab7e034596ec0d79e3db3cb34978d/src/overrides
curl -fsSLo "$fixture_dir/cmdOverrides.tcl" "$fixture_url/cmdOverrides.tcl"
curl -fsSLo "$fixture_dir/overrides.tcl" "$fixture_url/overrides.tcl"
{
    sed '$ { /^$/d; }' "$fixture_dir/cmdOverrides.tcl"
    sed -e '/^source /d' -e '/^package provide /d' "$fixture_dir/overrides.tcl"
    printf '%s\n' 'set probe 1'
} > "$fixture_dir/medium.tcl"
CARGO_PROFILE_TEST_OPT_LEVEL=2 CARGO_PROFILE_TEST_DEBUG=0 \
    cargo run --locked --profile test -p tcl-lsp-db --example w210_body_probe -- "$fixture_dir/medium.tcl"
```

Each trial primes a fresh database from the original file. The timed interval starts before updating the source and includes segmentation, the query demands and rebasing. The one-line edit in `IsOverridden` reads an unbound local. The probe compares the W210 messages and spans before and after the edit with fresh whole-file analysis, and checks Salsa's `WillExecute` events and the 50 ms budget. The fresh analysis of the edited file runs after all five timed trials.

A rerun at `508b801b` on a 4-core Intel Xeon (2.30 GHz) re-executed the same single query in each of five trials, slowest 48.58 ms, so the budget holds with little margin. Clippy with the gate's flags (`-D warnings -A clippy::assert_is_empty`) is clean on `tcl-compiler`, `tcl-lsp-db` and `tcl-lsp-core` at that commit.

## Inventory

**Memos inside memoised values** (about 15 types in `command_binding`, the realm and `analyser/types/result_caches.rs`): `Mutex<Option<…>>`, `OnceLock` and `Mutex<HashMap>` fields inside `AnalysisResult`, bindings and realms, equality-transparent and empty on clone, filling *after* the salsa query returns; capped at 65,536 entries in several places, uncapped in `ModuleRealmMemo` (inferred). Salsa's LRU sizes and `memory_usage()` do not see them.

**Process-global compiler memos** (about 20): `OnceLock<Mutex<…>>` statics and thread-locals keyed by owner address, `Weak` payloads, registry semantic keys or source images, bounded by `Weak` sweeps, small FIFOs or clear-all at 4,096 or 8,192 entries; one (`slot_presence.rs`) keyed by builtin name only, not by registry. Thread-locals multiply by the server's blocking thread pool (up to 512 threads by default; inferred), and one of them holds 16 whole `SourceCommandBindings` per thread.

**Registry, dialect and SpecTcl statics** (about 25): `REGISTRIES`, the assembly cache, ingress `CONTEXTS`, `LEAKED_GENERATIONS` and `PROFILES`, `CORE_SURFACE_REGISTRIES`, `PROJECTED_PROFILES`, the lowered-spec cache, the core-surface name sets, SpecTcl's snapshot cache, hook plans, bundled pack sets, leaked specs and the memos keyed by their addresses, and four separate `&'static str` leak interners. Several are keyed by generation with no eviction, and several leak with `Box::leak` (*read*: the core-surface name set is leaked once per generation, commented "held for the process"; SpecTcl's bundled pack set is leaked).

**Registry construction:** `CommandRegistry::project_for_profile` starts from a full `build_default()` (*read*); it is reached from ingress, assembly and the closed-native executor. Two separate static default registries exist (`tcl-registry`'s cache and `var_scoping.rs`). `tcl-lsp-db` documents the registry as "static (built once, never mutated)" and keeps it out of salsa (*read*), but its promotion key moves with pack, roster and core-surface generations, which salsa learns about only through two epoch inputs.

**Salsa** (`tcl-lsp-db`): inputs for source files, analyser configuration, the project and two epochs, plus **`RetainedModules`, a `Mutex<RetainedModuleTable>` inside an input** (*read*) acting as a hand-written 512-entry LRU. Queries are whole-file (analysis, compilation unit, compiler checks, tokens, symbols, folding) and per-body with interned keys (item bodies, lattices, lowering, optimisations, taint and summary cascades), plus light per-file and project aggregates. An item-body key carries the whole resolved analysis input.

**Server state outside salsa:** the document store, the workspace index (fed by its own off-salsa analyses during disk scans), the package resolver, a recovery-name cache, a 512-entry `FreshAnalysisCache`, workspace class analyses keyed by fingerprint, diagnostics slots and pull caches, the last semantic tokens, provider-source inventories. Per-URI eviction is centralised for five of these maps.

## Faults

1. **Salsa covers only the top layer.** Memos inside memoised values and process-global statics are invisible to it: invalidation, LRU sizing and memory accounting are all partial.
2. **The registry is modelled as static but is versioned**, so every layer that depends on it keys by generation in its own global map, and several leak by design. Every pack reload ratchets memory (the prime suspects for #2543 are the generation-keyed ingress maps, the leaked core-surface name sets, SpecTcl's snapshot cache, hook plans and leaked pack sets and specs, and the overlay-0 entries the assembly cache never prunes).
3. **Five staleness mechanisms** that do not compose, so a change visible to one is invisible to another.
4. **Several pipelines analyse the same document** (salsa, the fresh-analysis cache, the workspace scan, recovery, class analyses, provider sources), each with its own cache and key; the same text is held three times (document store, salsa input, fresh-analysis key).
5. **Coarse granularity where it costs most:** whole-file analysis on every edit (1.27 s for 687 lines), secondary indices (point extents, deferred bodies, namespace indices, the workspace index's derived sets) rebuilt from scratch when anything changes, and a 17 ms fixed cost per analysis.
6. **No interning of names:** `NameBytes` is an un-deduplicated `Arc<[u8]>` in 292 files; four separate string-leak interners exist.
7. **Determinism depends on order:** global memos carry state between analyses and between tests (the analysis-performance handover records two witnesses).

## One architected layer

- **Everything versioned is a salsa input:** the registry, pack sets, rosters, environments and dialect profiles become inputs (or interned values) with `Arc` handles; no `Box::leak`, no generation-keyed globals. The registry is built once per input revision and projected per profile as a query, not by rebuilding the default.
- **Granularity follows the [architecture plan](architecture-tidy-plan.md)'s layers:** source file → structured words → binding per file → body unit → summaries per unit → semantic queries. An edit inside one procedure re-runs that body's queries; secondary indices are tracked queries keyed by (document, body), not by pointer identity.
- **No interior-mutable memos inside memoised values:** query-scoped scratch arenas dropped when the query returns; anything worth keeping is its own query.
- **One analysis pipeline:** the fresh-analysis cache, workspace scan, recovery, class analyses and provider sources become salsa queries with open, closed and workspace tiers; the document text lives once.
- **Interning in the database:** names, namespaces, commands and places as salsa interned identities ([cross-cutting review](cross-cutting-review.md) fault 1), replacing the leak interners and un-deduplicated `NameBytes` where identity matters.
- **Memory is accounted and bounded per layer:** byte budgets and LRU at query level, a registry of every remaining static with size probes behind a debug-memory command, and a gate that forbids new `Mutex`/`OnceLock`/`thread_local!` memo types and `Box::leak` outside the database.
- **Determinism is tested:** fresh-versus-warm differentials, randomised in-process test order, and per-cache growth tests under pack churn, plus the timings above as a regression budget (per-keystroke latency on the analysis-performance corpus).

## Plan

1. **Measure and bound now:** expose sizes for every static and memo through a debug-memory command; add the probe above and a pack-churn loop to the analysis-performance suite; cap or evict the generation-keyed ingress maps and the overlay-0 assembly entries; stop leaking per generation.
2. **Registry as an input:** version registries, packs, rosters and profiles as salsa inputs; build each once and project per profile as a query; delete the duplicate default registry.
3. **Move memos into queries:** replace the memos inside analysis values and the process-global compiler memos with tracked queries or per-query scratch; retire `RetainedModules`' private LRU.
4. **One pipeline and finer granularity:** fold the server's extra analyses into salsa tiers; make secondary indices tracked queries; reach per-body re-analysis on edits once the binding walk is per body (architecture plan phases 4–5).
5. **Gates:** no new memo or leak outside the database; determinism and growth tests; latency and memory budgets in CI.
