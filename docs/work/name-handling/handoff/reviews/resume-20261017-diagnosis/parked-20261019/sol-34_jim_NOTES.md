# Sol-34: Jim's commands lost with packs loaded (parked)

**Status: parked** under the user's 2026-10-19 decision. Side work that does not unblock the LSP naming merge is paused, so this is a static draft that has never been compiled. HEAD is `efdb1af7b`.

Two reviews feed into it:

- Luna-38 reviewed v1 as ADMIT-TO-BUILD (`../luna-38/REVIEW.md`).
- The user directed (`user-decisions-20261019T1230Z.json`): Jim is a core dialect of its own; the fix belongs in `tcl-registry`; Jim builds its own core store rather than borrowing plain C Tcl's.

## Deliverables

| Path (this directory) | Use |
|---|---|
| `jim-core-store-v2.patch` | **The consolidated draft, against HEAD.** It contains the fix, the tests and the doc sentence. The explorer test sits at the top of `tcl-explorer` `mod tests`. |
| `jim-core-store-v2-after-sol31.patch` | The same draft for a tree where sol-31's `explorer-pack-free.patch` has already landed. The explorer test moves to the end of `tests::bundled_pack_path` and uses `crate::` paths. |
| `v3/fix-only.patch`, `v3/tests-only.patch` | The draft split into its two halves, for a red-then-green run. |
| `v3/` | The patched files themselves. |

Superseded drafts:

- v1 is `jim-core-surface-with-packs.patch`, `fix-only.patch` and `tests-only.patch` at the top level, plus `post/`.
- The first v2 is in `v2/`. It layered Jim's surface over the plain-Tcl store.
- Do not apply any of them.

Static checks I ran, each with a temporary `GIT_INDEX_FILE`:

- `jim-core-store-v2.patch` applies to HEAD and to the dirty working tree.
- Tests-then-fix applies.
- The v2 patch followed by sol-31's patch applies.
- Sol-31's patch followed by `-after-sol31` applies.
- `rustfmt --edition 2024` is clean.
- The patches add no `#[allow]` and no check on a dialect name.

## Root cause

There are two defects, and both come from treating `jim` (which has no catalogue row) as plain Tcl.

1. **The pack-carrying store a profile-holder gets had no Jim surface.** `tcl_spectcl::install::registry_with_packs` (`install.rs:69`) built the store with `registry_for_profile_with_overlay(jim, key)` (`tcl-registry/src/cache.rs:244`). Only `command_store` (`model/assembly.rs:428`) adds a family's compiled-in surface, so this store lacked all 60 rows of `jim-own-surface.tclspec`.
   - `bundled::packs()` always carries the embedded EDA loadables, so this path is always taken.
2. **The analysis read plain Tcl's pack store.** For overlay ≠ 0, `command_store` looked up `store_profile("jim")`, which is `plain_tcl` (`assembly.rs:342`, `:419-423`).
   - The CLI (`spec_pack_key`) and MCP (`registry()`) install packs only under `jim`'s own profile.
   - So `tcl diag --dialect jim`, a `jimsh` shebang, and MCP `analyse_under`/`analyse` on `jim` met `OverlayMiss`. That produces "analysis context unavailable", and `tcl diag` exits 1.
   - The LSP avoided the miss only because it installs over every `store_profiles()` entry, and it then projected the plain-Tcl store onto jim.
   - Luna-38 statically confirmed this for jim. `tk` is unaffected: its analyser profile is `plain_tcl`.

## Fix (all assembly in `tcl-registry/src/model/assembly.rs`)

- `store_profile(&DocumentEnvironment)` now returns:
  - the catalogue profile, if the environment has a catalogue row;
  - otherwise, for an environment on a core dialect of its own (`owns_core_store`: core family ≠ `Family::Tcl`), its own execution profile;
  - otherwise `plain_tcl`.
  - So a `jim` generation reads `(jim, key)`, the store `jim` consumers are handed. With execution equal to the store profile, there is no plain-Tcl projection.
- `store_profiles()` also lists the store profile of every live environment that runs on its own core and has no catalogue row. The LSP's `install_pack_overlays` and the reload loop therefore install `(jim, key)`.
- `with_family_core_surface(store, &EnvironmentDefinition)` is private. It reads the profile and overlay from the store (`profile()`, `overlay_generation()`), so a mis-keyed call is impossible. `command_store` uses it.
- New `pub fn profile_store_with_overlay(profile, overlay, extend)`, re-exported from `tcl_registry::model`. It resolves the environment once, builds `(profile, overlay)` with `extend`, and returns it carrying the family's own surface. When `profile` is the environment's store profile, this is the same `Arc` the generation reads.
- `tcl-spectcl/src/install.rs`: `registry_with_packs` now just calls `profile_store_with_overlay(profile, key, install_into)`. I updated its doc comment.
- `docs/design/registry/spec-packs.md`: one sentence on Jim's own pack store.
- The `.tclspec` own-surface pack is still the data source. Jim's store is `(jim,0)`'s rows plus Jim's specs plus the pack overlay, cached under `(jim, key, Jim, core generation)`. That cache key means a late registration is not missed.

## Scope (corrected per Luna-38)

At HEAD, the pack-carrying store loses Jim's commands for:

- `tcl explore` and the TUI
- `tcl-explorer-wasm`
- the LSP explorer command when no owner document is bound
- the LSP's `registry_for_dialect` consumers:
  - semantic tokens (`lib.rs:32515`)
  - inlay hints (`:32941`)
  - signature help (`:34126`)
  - go-to-declaration (`:31495`)
  - list-subcommands (`:25344`)
  - compiler-check diagnostics (`compiler_diagnostics_for` at `:26563`, `diag_inputs.registry` at `:26048`)
- the CLI `registry_for_dialect` verbs; `standalone_findings` also swaps this store in (`diagnostic_report.rs:263`)
- the `tcl-mcp` `registry()` and the spec studio

**Hover and completion are not affected.** They ignore the registry argument and read the analysis context.

Only Jim registers its own surface today. The rule is keyed by family, so another core dialect would take the same path.

## Tests

| Test | Kind | Expected at HEAD | Expected with fix |
|---|---|---|---|
| `tcl-registry` `model::assembly::tests::a_core_dialect_of_its_own_reads_its_own_pack_store` | new | red (doesn't compile: `profile_store_with_overlay`, `store_profile(&env)`) | green |
| `tcl-registry` `model::assembly::tests::every_store_key_is_a_profile_a_host_can_install` | updated (adds `jim`) | — | green |
| `tcl-spectcl/tests/jim_own_surface.rs::a_pack_carrying_jim_store_keeps_jims_own_surface` | from v1, extended | red (first `+` membership) | green |
| `tcl-explorer` `jim_own_commands_resolve_with_packs_loaded` | from v1 | red (`ptr_eq` or `local` unknown) | green |
| `tcl-mcp` `policy_tests::a_jim_document_is_analysed_with_the_shipped_packs` | new | red (`analysis_context_unavailable` is `Some`) | green |
| `tcl-cli/tests/cli.rs::diag_accepts_the_jim_program_under_the_jim_dialect` | existing | red (exit 1, per Luna-38) | green |
| `tcl-cli/tests/cli.rs::diag_analyses_a_jimsh_shebang_document_as_jim` | existing | red | green |
| `tcl-cli/tests/cli.rs::the_dialect_flag_takes_the_registry_names_and_rejects_the_rest` | existing (`jim`, `jimsh`, `tcl-jim`) | red | green |

What the new and extended tests check:

- **`a_core_dialect_of_its_own_reads_its_own_pack_store`**: the jim store profile is jim's own; a plain-Tcl overlay is a miss for jim; the store installed for jim is the generation's store, by `Arc` pointer.
- **`a_pack_carrying_jim_store_keeps_jims_own_surface`**, with a probe pack:
  - every Jim row is in the store;
  - 11 Jim-only heads resolve to the jim row, both with and without packs;
  - the jim generation at the set's key is the same `Arc`;
  - a `tcl8.6` store has none of the heads.
- **`jim_own_commands_resolve_with_packs_loaded`**, with the shipped packs:
  - the explorer's registry is the jim analysis store, by `Arc` pointer;
  - 7 heads resolve to the context's rows;
  - `has_unknown_calls` agrees between the pack pipeline and the static-context pipeline, with a `::q` control.
- **`a_jim_document_is_analysed_with_the_shipped_packs`**:
  - the analysis is available, and `analyze` reports no `analysis_context_unavailable`;
  - there is no W002 or W123 for `loop`/`sleep` under jim;
  - the `tcl8.6` control does report W002.

Other expected effects:

- Non-Jim behaviour should not change. Catalogue profiles and `plain_tcl` return the same stores, and families with no registered surface return `store` itself.
- The xtask inventories iterate the catalogue, so I expect no generated-file drift.
- `store_profiles()` now resolves each live environment once per call. It is only called on a pack reload.

## Follow-ups (not in this draft)

- **Collision with a Jim command name.** `install_into`'s `installs_over` and `SecurityFloor::of` run against the store before Jim's surface is added. A pack command named like a Jim command (for example `defer`) therefore shadows it with no collision notice. This is Luna-38 note (a). The fix is to give the install closure a store that already has the family surface.
- **Stale LSP doc comment.** `install_pack_overlays` in `tcl-lsp-server/src/lib.rs:5294` still describes the list as catalogue plus the plain sink. It is a one-line edit; I left it out because another agent holds that file.
- **Stronger end-to-end assertion.** After a run, add `("::p", false)` to the explorer test, as Luna-38 suggested.

## How to resume

```sh
cd /home/user/tcl-lsp && source scripts/dev/agent-build-env.sh
P=/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/agents/sol-34
# 0. If sol-31's explorer-pack-free.patch has landed, use jim-core-store-v2-after-sol31.patch below instead.
# 1. Red: confirm the existing CLI tests fail at HEAD
cargo test -p tcl-cli --test cli jim 2>&1 | tee /tmp/sol34-red-cli.log
# 2. Apply and go green
git apply $P/jim-core-store-v2.patch
cargo test -p tcl-registry --lib model::assembly 2>&1 | tee /tmp/sol34-registry.log
cargo test -p tcl-spectcl --test jim_own_surface 2>&1 | tee /tmp/sol34-spectcl-jim.log
cargo test -p tcl-explorer --lib jim_own_commands 2>&1 | tee /tmp/sol34-explorer.log
cargo test -p tcl-mcp --lib a_jim_document_is_analysed 2>&1 | tee /tmp/sol34-mcp.log
cargo test -p tcl-cli --test cli jim 2>&1 | tee /tmp/sol34-green-cli.log
# 3. Neighbourhood
cargo test -p tcl-spectcl 2>&1 | tee /tmp/sol34-spectcl-all.log
cargo test -p tcl-explorer --lib 2>&1 | tee /tmp/sol34-explorer-all.log
grep -E "FAILED|panicked|test result" /tmp/sol34-*.log
# 4. Gate
make rust-check 2>&1 | tee /tmp/rust-check-sol34.log; grep -nE "^error|^warning|FAILED" /tmp/rust-check-sol34.log
```

## Commit text (for when it lands)

```
Build Jim's pack-carrying store as its own core store

Jim is a core dialect of its own, but with packs loaded two things treated
it as plain Tcl. The store a profile-holding consumer got from
`registry_with_packs` lacked Jim's compiled-in commands. A `jim` analysis
looked its pack overlay up in plain Tcl's store, which the CLI and MCP
never install, so `tcl diag --dialect jim` reported the analysis context
unavailable and exited 1.

`tcl_registry::model` now resolves a non-catalogue environment on a core
family other than C Tcl to its own store profile, lists those profiles in
`store_profiles`, and builds pack-carrying stores through
`profile_store_with_overlay`, which carries the family's own surface beneath
the packs. `registry_with_packs` calls it. A `jim` consumer and a `jim`
analysis now read one store.

Affected at HEAD: `tcl explore` and its TUI, the WASM explorer, the LSP's
explorer without an owner document, semantic tokens, inlay hints, signature
help, go-to-declaration, list-subcommands and compiler-check diagnostics on
`jim` documents, the CLI and MCP analysis of `jim`, and the spec studio.
Hover and completion read the analysis context and were not affected.
```
