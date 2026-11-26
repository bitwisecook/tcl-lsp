# 288: remove the tcl-spectcl → tcl-lsp-core edge

Status: designed 26 November 2026. Plan item 288; retires
[project-layout](../contracts/project-layout.md) rule 4's exception. Builds on
the [workspace overview](overview.md). [289](289-lsp-core.md) builds on it
(the source-store crate and the spec-pack notice types leave `tcl-lsp-core`
first), and so does item 270 (`tcl-lsp-core`'s tests stop needing
`tcl-spectcl`).

## Problem

`tcl-spectcl` (the `.tclspec` loader, 30k production lines) has a normal
dependency on `tcl-lsp-core` (132k). A loader sits below features in the
[tidy plan](../../work/name-handling/architecture-tidy-plan.md#crates), so the
edge is upward. It costs:

- every crate that takes `tcl-spectcl` (`tcl-explorer`, `tcl-spec-studio`,
  `tcl-cli`, `tcl-cli-support`, `tcl-mcp`, `tcl-lsp-server`, `xtask`) builds
  all of `tcl-lsp-core`, and through it `tcl-bigip`, `tcl-irules` and
  `tcl-sslictcl`;
- the dev cycle `tcl-lsp-core` → (dev) `tcl-spectcl` → `tcl-lsp-core`, which
  item 270 must break;
- in `tcl-explorer-wasm` alone, 57 third-party crates and 4 workspace crates
  (`cargo tree --target wasm32-unknown-unknown --prune tcl-lsp-core`: 159 → 102
  crates, 30 → 26 workspace crates).

What `tcl-spectcl` names (`rg tcl_lsp_core rust/tcl-spectcl`, 26 November
2026). The plan's 19 files are right for `src`; `tests/` adds 3.

| Item | Production files | Test-only files |
|---|---|---|
| `vfs::SourceStore`, `NativeStore`, `MemoryStore`, `SourcePathOrigin`, `SourceUriPathUnavailable` | 10: `bundled.rs`, `pack.rs`, `discovery.rs`, `package_sources.rs`, `package_specs.rs`, `extension_artefacts.rs`, `loader/pack_scope.rs`, and three `discovery/*_controls.rs` | `cache.rs`, `bundled.rs` and `pack.rs` test modules; `discovery/*_tests.rs`; `tests/it_3/package_specs.rs`, `tests/it_5/workspace_trust.rs` |
| `code_actions::SpecPackNoticeSubject`, `SpecPackNoticeKind` | 4: `loader.rs`, `loader/notices.rs`, `hooks.rs:225`, `pack.rs:76` | `loader/tests/command.rs` |
| `code_actions::ContextDiagnostic`, `ContextDiagnosticData`, `SPEC_PACK_DIAGNOSTIC_CODE`, `spec_pack_quick_fixes`; `definition::LspRange` | none | one test, `cache.rs:1332` (`original_loader_notice_sites_survive_cold_and_disk_warm_cache…`) |
| `wire_session::EditorWireSession` | none | `discovery/reserved_studio_path_tests.rs:47` |
| `registry_for_dialect`, `formatting::format_tcl`, `FormatterConfig` | none | `tests/it_3/pack_formatting.rs` |

So production code needs two things: the source-store seam and the two
notice types. Everything else is test code checking how the LSP renders what
the loader produces.

The source store is not standalone in `tcl-lsp-core`. `vfs.rs` (1,451 lines)
uses `file_uri` (448 lines; `tcl-platform`, `fluent-uri`) and
`source_decode` (`SourceStore::read_source`, `vfs.rs:645`). `source_decode`
(611 lines) has two halves: the decoder (`decode_source`, `DecodeReport`,
`decode_source_as`, `read_source_file`, lines 102–435) and the W107/W109
renderer (`encoding_integrity_diagnostics`, lines 466–606), which names
`definition::LspRange` and `source_style::StyleDiagnostic`. `DecodeReport`
holds `tcl_registry::value_transfer::ScriptEncoding`
(`tcl-registry/src/value_transfer/context.rs:68`), an enum over
`tcl_cmd_core::encoding::SystemEncoding`.

## Design

### `tcl-source-store` (new, layer 2)

A crate below both `tcl-spectcl` and the feature crates. It holds every path
that reads a closed file, as [lsp-source-store](../contracts/lsp-source-store.md)
requires, and both implementations undivided.

| Moves in | From |
|---|---|
| `SourceStore`, `NativeStore`, `MemoryStore`, `SourcePathOrigin`, `SourceUriPath`, `SourceUriPathUnavailable`, the inventory types | `tcl-lsp-core/src/vfs.rs` |
| `file_uri` (the URI ↔ path mapping, `VIRTUAL_URI_MOUNT`) | `tcl-lsp-core/src/file_uri.rs` |
| the decoder: `decode_source`, `decode_source_as`, `encode_source_as`, `read_source_file(_as)`, `DecodeReport`, `Utf8Error`, `Utf8Fault`, `NonTextEncoding`, `should_abstain` | `tcl-lsp-core/src/source_decode.rs:102–435, 606` |
| `ScriptEncoding` | `tcl-registry/src/value_transfer/context.rs:68`, into `tcl-cmd-core::encoding` beside `SystemEncoding`, which it wraps |

It depends on `tcl-cmd-core` (encoding conversion), `tcl-dialect`,
`tcl-platform` and `fluent-uri`. Its interface is today's, unchanged in shape:

```rust
pub trait SourceStore: std::fmt::Debug + Send + Sync {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn read_source(&self, path: &Path) -> io::Result<(String, DecodeReport)> { … }
    fn read_source_as(&self, path: &Path, encoding: ScriptEncoding)
        -> io::Result<(String, DecodeReport)> { … }
    // read_dir, metadata, retained_source_reader, uri_path, … as today
}
pub fn decode_source(bytes: &[u8]) -> (String, DecodeReport);
```

The W107/W109 renderer stays with the style diagnostics
(`source_style`, which goes to `tcl-ide-diagnostics` in 289). It reads a
`DecodeReport` and produces `StyleDiagnostic`; nothing below it needs the
renderer.

Consumers: `tcl-spectcl`, the feature crates (`package_resolver`,
`config_ini`, `tcl_install` and the workspace scan), `tcl-cli`,
`tcl-cli-support` (`source_decode`, 4 uses), `xtask`, `f5-cli` (`file_uri`),
`tcl-lsp-server` (`vfs`, 18 files on `source_decode`, 4 on `file_uri`) and
`tcl-lsp-server-wasm` (`vfs`, `file_uri`). `tcl-lsp-server/src/vfs.rs`
re-exports the trait today (`pub use tcl_lsp_core::vfs::{…}`, line 31); it
re-exports from the new crate until 290 removes the re-export.

### The notice types move below both

`SpecPackNoticeKind` and `SpecPackNoticeSubject`, with their constructors
(`from_loader`, `for_rows`, `from_original_word`) and accessors, are the
geometry of the pack-source word a notice is about. That part of
`tcl-lsp-core/src/code_actions/spec_notice.rs` (lines 21–330) names only
`tcl_lexer` (`SourceImage`, `LexerConfig`, `NativeWord`, `Span`) and
`tcl_compiler::parsing::spectcl_source::OriginalPackSourceWord`. It moves into
`spectcl_source`, the SpecTcl source grammar module, which item 251 moves to
`tcl-source`. Both `tcl-spectcl` and the feature crates already depend on that
module's crate.

What stays in `tcl-lsp-core` (and then `tcl-ide-edit`) is turning a notice
into a code action: `SpecPackNoticeData` (`spec_notice.rs:335–430`, which
names `AnalysisResult` and `CommandRegistry` and holds the process-wide
`subjects()` memo), `ContextDiagnosticData::from_spec_pack_notice`,
`SPEC_PACK_DIAGNOSTIC_CODE` and `spec_pack_quick_fixes`.

`oo_body.rs:1106` (a `tcl-lsp-core` test) names the types; it is rewritten
to the new path.

### Tests that cross the boundary

| Test | Goes to | Why |
|---|---|---|
| `cache.rs:1332` (loader subject → quick fix round trip) | split: the cold/warm subject equality stays in `tcl-spectcl`; the quick-fix half moves to `tcl-lsp-core`'s `code_actions` tests | each half needs only its own crate |
| `discovery/reserved_studio_path_tests.rs:47` | stays; the nonce comes from a local test helper | it needs a random nonce, not an editor session |
| `tests/it_3/pack_formatting.rs` | `tcl-lsp-it` (new test-only crate, layer 11) | it checks that the formatter leaves a pack loading the same, so it needs both crates |
| the `MemoryStore`/`NativeStore` tests | stay, on `tcl-source-store` | the store is below `tcl-spectcl` |

`tcl-lsp-it` is the LSP stack's integration-test crate, like
`tcl-compiler-it` (item 266). 289 and item 270 move more tests into it.

### Decisions

- **A new crate, not `tcl-platform`.** `tcl-platform` bans syscalls by
  charter and already owns the `Filesystem` capability trait. The rejection
  in lsp-source-store ("a new crate: not needed") assumed `tcl-lsp-core` was
  an acceptable home; the tidy plan's rule makes it not one.
- **The decoder moves with the trait.** Splitting the decode default off
  `read_source` was the reason the trait stayed in `tcl-lsp-core`. The
  decoder needs only `tcl-cmd-core`, so both fit below.
- **`ScriptEncoding` moves to `tcl-cmd-core`.** It is encoding vocabulary,
  and item 117 is moving `value_transfer` out of the registry anyway. Leaving
  it there would make the store depend on all of `tcl-registry` for one enum.
- **Notice types go to the source grammar, not to `tcl-spectcl`.** Putting
  them in the loader would make `tcl-lsp-core` depend on `tcl-spectcl`, and
  `tcl-spectcl` links `tcl-spec-hooks`, `tcl-engine-tclvm`, `tcl-vm` and
  `tcl-codegen` (normal edges). The subject needs only the lexer and the
  pack-source grammar, so it sits below both and neither side gains an
  edge.

### Open questions

- **Is `tcl-cmd-core` too wide a dependency for the store?** It is 51k lines.
  Settle by measuring the store's cone (`cargo tree -p tcl-source-store`)
  after the move; if `tcl-cmd-core` dominates, `encoding` becomes its own
  crate.
- **Does `SourceStore` keep its inventory types** (`WorkspaceSourceInventory*`,
  `vfs.rs:104–190`)? They serve the server's workspace scan. Settle by
  `rg` after 290: if only the server names them, they move to
  `tcl-lsp-workspace`.

## Work list

1. Move `ScriptEncoding` into `tcl-cmd-core::encoding`. Rewrite its 15
   `tcl-compiler`, 10 `tcl-lsp-server`, 6 `tcl-lsp-core` and other users
   (`rg -l ScriptEncoding`: 45 files in 10 crates).
2. Create `rust/tcl-source-store` with `vfs.rs`, `file_uri.rs` and the
   decoder half of `source_decode.rs`, with their test files
   (`vfs/*_tests.rs`, the decoder tests of `source_decode/tests.rs`).
3. Leave the renderer half in `tcl-lsp-core`'s `source_style` module.
4. Rewrite every consumer in the list above to `tcl_source_store::…`. No
   re-export stays in `tcl-lsp-core`.
5. Move the notice types to `tcl_compiler::parsing::spectcl_source` (so
   `tcl-source` once item 251 lands); rewrite `tcl-spectcl`'s five files and
   `tcl-lsp-core`'s `code_actions` and `oo_body` to import them from there.
6. Split or move the five boundary tests.
7. Create `rust/tcl-lsp-it` with `pack_formatting.rs`.
8. Drop `tcl-lsp-core` from `tcl-spectcl`'s `[dependencies]`. Add
   `tcl-source-store`.
9. Update `deny.toml` (`tcl-spectcl` leaves `tcl-lsp-core`'s wrappers), the
   lsp-source-store contract's "Where the trait lives" section, and
   project-layout rule 4 (delete the exception). Seed the rank gate's
   ledger ([overview](overview.md#the-gate)) with this edge gone.
10. Regenerate the lockfiles of `tcl-lsp-server-wasm`, `tcl-lsp-server-wasi`,
    `tcl-explorer-wasm` and `tcl-spec-studio-wasm`.

## Verification

Done when:

- `rg tcl_lsp_core rust/tcl-spectcl` finds nothing;
- `cargo tree -p tcl-spectcl -e normal` lists no `tcl-lsp-core`;
- neither crate's tree lists the other under a normal edge, and the
  only remaining edge between them is `tcl-lsp-core`'s dev-dependency, which
  item 270 removes;
- the moved tests pass in their new crates, and `make test-spectcl-compat`
  passes;
- the browser host loads packs from bytes as before (`npm run test:web`).

Record:

- `tcl-spectcl`'s cone, deduplicated (today 147 normal, 28 workspace crates);
- `tcl-explorer-wasm`'s cone (today 159 crates, 30 workspace crates;
  `--prune tcl-lsp-core` predicts about 102 and 26, plus `tcl-source-store`).
  `tcl-spec-studio-wasm` keeps `tcl-lsp-core` through `tcl-spec-studio`
  until 289;
- the rebuild count of a `tcl-lsp-core` edit for `cargo test -p tcl-spectcl`
  (today `tcl-lsp-core` and `tcl-spectcl`; after, none).

## Risks

- **The memo moves with nothing.** `SpecPackNoticeData`'s `subjects()`
  memo (`spec_notice.rs:347`) stays in the feature crate; item 90's last step
  decides whether it is deleted or moved into `tcl-db`. The subject type
  itself holds no cache.
- **`spectcl_source` grows.** It becomes the home of loader-facing geometry.
  Keep it to types with no registry or analysis dependency; anything that
  needs `AnalysisResult` stays above.
- **Browser hosts.** `MemoryStore` is how the web worker supplies packs. The
  move must keep `NativeStore` off the wasm32 target exactly as `vfs.rs`
  does today.

## Corrections to the overview

- The plan's list misses `SourceUriPathUnavailable` (`pack.rs:1666`, tests)
  and the three `tests/` files, and the formatting test's use of
  `registry_for_dialect` and `formatting`.
- Of the 19 `src` files, only 13 name `tcl-lsp-core` outside test code.
  `ContextDiagnostic`, `ContextDiagnosticData`, `SPEC_PACK_DIAGNOSTIC_CODE`,
  `spec_pack_quick_fixes` and `LspRange` are named only by one test.
- lsp-source-store's "Where the trait lives" rationale is superseded; that
  contract is updated in work list step 9.
- The plan item says the notice types "move into `tcl-spectcl`". They move
  into the pack-source grammar module instead (see Decisions), so that
  `tcl-lsp-core` does not take the VM through `tcl-spectcl`.
