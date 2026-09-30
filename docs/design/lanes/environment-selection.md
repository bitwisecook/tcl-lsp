# Lane: environment selection (#2166)

Contract: [contracts/environment-selection.md](../contracts/environment-selection.md).
Issue: bitwisecook/tcl-lsp#2166 (Jim Tcl has no selectable dialect; Jim's
own commands, `proc` statics and `class` are flagged).

## Goal

Every surface that shows or accepts a dialect name reads the environment
registry — at runtime or through a gated generator — so `jim` (and any
future environment) is selectable in every editor the day it exists, and
the six EDA shells present as what the model already says they are: a Tcl
release plus packages, explained once by a server-owned notice. Canonical
ids, directive spellings and config keys are unchanged.

## Decisions taken

| # | Decision | Why |
|---|---|---|
| 1 | Environments are the vocabulary; `dialect` stays the user-facing word | Every ingress already resolves through `resolve_environment`; renaming the setting or directive would break every stored value for no gain |
| 2 | `EnvironmentKind` is a declared field, `Language` or `Packages` | No derivation over existing fields separates `bpf` from `tk`; a judgement is a field |
| 3 | Two enumeration mechanisms only (runtime registry read, or gated generator); `catalogue-callers` allowlist gate | Fourteen hand-written dialect lists exist today; the way to stop a fifteenth is structural |
| 4 | The notice is server-owned (`showMessageRequest`, `showMessage` fallback), dismissal persisted in XDG state | The server owns the catalogue and never sends `showMessage` today; one implementation covers seven editors and needs no client code |
| 5 | Catalogue retirement (D5/C1), free composition (`EnvironmentOverlay`), D17-P deferred | Keep the change to what the issue and the sync problem need |
| 6 | Jim's additions are a second built-in pack measured from built `jimsh` 0.76–0.84 | Same evidence standard as `jim.tclspec`; the upstream tags are reachable |
| 7 | Expect is `Language`; Tk is `Packages` (user, 2026-09-30) | Expect has its own interpreter, shebang and extension; Tk is `package require Tk` |
| 8 | EDA environments take tool display names and aliases; ids unchanged (user) | The table in the contract § *Display names and aliases* |
| 9 | The notice fires for bundled-pack `Packages` environments only — the six EDA shells (user) | `tk` and third-party pack environments are not the demotion being explained |
| 10 | Pickers keep the word "dialect" and show two groups (user) | Fewest new words; the notice carries the explanation |

## Phases and owners

Phase 1 lanes run in isolated worktrees with `scripts/dev/agent-build-env.sh`
sourced (never a shared `CARGO_TARGET_DIR`). Phase 2 is independent of
Phase 1 and runs alongside it.

| Phase | Lane | Owner | Scope |
|---|---|---|---|
| 0 | plan | Fable | This document and the contract; index entries; user decisions |
| 1a | model | Sonnet | `EnvironmentKind` on `EnvironmentDefinition` (Expect `Language`, Tk `Packages`, EDA and pack-declared `Packages`); pack `kind` word (loader, export renderer, studio form — four-surface parity); `tcl-jim` in `CONTRIBUTED`; `jim` claims it with `display_name` "Jim Tcl" / `short_name` "Jim"; EDA display names, short names and aliases per the contract table as pack data, seed regenerated, catalogue rows kept equal; shebang tier reads `shebang_words`; `EnvironmentDefinition::description`; `catalogue-callers` gate with allowlist |
| 1b | runtime lists | Sonnet | CLI `--dialect` clap values and `resolve_dialect` message from the registry (drop the hand `tk`); MCP `dialect_schema`; `listDialects` returns environments with `kind` and description; `getEffectiveConfig` labels from the registry (no more nulls for `tk`/`jim`/`tcl`); `unknown_dialect_error`; the legacy language-id table becomes alias rows; session `tclLsp.dialect` validated with WARNING; test `every_runtime_enumeration_is_the_registry` |
| 1c | generators + editors | Sonnet | Re-key `gen-editor-dialects`, `gen-editor-extensions`, `gen-ai-diagnostics` on the compiled registry; `enumItemLabels`/kind-grouped `enumDescriptions`; `firstLine` from shebang words; `tcl-jim` language in VS Code (incl. `semanticTokenScopes` block, `when` regex fixed to cover every id), JetBrains, Sublime, Zed, Helix, Emacs, Neovim; new `gen-editor-configs` (Zed `extension.toml` `language_ids`, Neovim lua/README, Helix README blocks, Emacs README, Sublime `_SYNTAX_DIALECT_MAP`, `INSTALL-editors.md`); new `gen-environment-docs` (README table between markers, `docs/generated/environments.md`, AI manifest); Makefile: `make codegen` runs every generator in write mode, `_EDITOR_DIALECT_OUTPUTS` corrected; VS Code client drops `DIALECT_LABELS`/`TCL_VERSION_DIALECTS`/`LANGUAGE_ID_DIALECTS` detection for `getEffectiveConfig` + `listDialects`; status bar and quick pick grouped by kind |
| 1d | notice | Sonnet | Server notice module (kind `Packages` ∧ `Provenance::BundledPack`); client-capability switch; `showDocument` for *Learn more*; XDG state file in `tcl-platform`; `tclLsp.notifications.environmentKind` + `config.ini` key; KCS note; `config-precedence.md` exception paragraph; e2e test with a fake client |
| 2a | jim scope | Sonnet | D17-J: ordered `AuthoringScope::core` / `SurfaceQuery::core`, nearest-first `surface_admits`; `dialect_surface` differential before/after |
| 2b | jim surface | Sonnet | Clone upstream, build `jimsh` per tag 0.76–0.84, diff `info commands` against the roster; author `jim-additions.tclspec` (built-in, `Provenance::BuiltIn`); Jim `proc` with statics; `command-backing` classification for each addition |
| 2c | jim analysis | Sonnet | `class` as `DefinitionBodyGrammar` + `DefinerFamily` arm; two-word `proc` names define two-word commands; W002 says "unknown command" (hint) when the only spec is another family's (`system` is Expect's) |
| 2d | jim fixtures | Sonnet | The issue's snippets as fixtures under `# tcl-dialect: jim` with zero false diagnostics; `--dialect jim` CLI e2e; `tcl-jim` language-id e2e |
| 3 | review + docs | Fable | `/code-review`; README detection-order fix; GLOSSARY `environment` and `package` entries; KCS dialect-selection rewrite; screenshots; `make prep-pr`; PR closing #2166; subscribe |

## Site inventory

Status: `todo` / `wip` / `done`. Line numbers are as surveyed on
2026-09-30 and drift.

### Runtime enumerations (→ registry read)

| Site | Status |
|---|---|
| `rust/tcl-cli/src/cli.rs:43-59` `dialect_possible_values` (+ hand `tk`) | todo |
| `rust/tcl-cli-support/src/input.rs:218-246` `resolve_dialect`, `known_dialect_names` | todo |
| `rust/tcl-mcp/src/tools.rs:1336-1351` `dialect_schema`; `main.rs:114-125` | todo |
| `rust/tcl-lsp-server/src/lib.rs:18241-18270` `listDialects` (catalogue only, no client uses it) | todo |
| `lib.rs:17977-18089` `getEffectiveConfig` labels via `catalogue_profile()` | todo |
| `lib.rs:27392-27406` `unknown_dialect_error` | todo |
| `lib.rs:11314-11356` `dialect_from_language_id` hand remaps (`tcl-apl`, `tcl-bpf`, `tcl-libero`, `tcl-spec`) | todo |
| `lib.rs:18929-18932`, `:23494-23507` session `tclLsp.dialect` stored raw | todo |
| `rust/tcl-spectcl/src/catalogue.rs:447-457` studio labels | todo |
| `rust/tcl-dialect/src/profile.rs:1735-1755` `KNOWN_DIALECTS` duplicate | todo |

### Generated artefacts (→ re-keyed generator + gate)

| Site | Generator | Status |
|---|---|---|
| `editors/vscode/package.json` `tclLsp.dialect` enum/descriptions, AI `dialects` enum | gen-editor-dialects | todo |
| `editors/vscode/src/extension.ts:112-134` `DIALECT_LABELS`; `compilerExplorerHtml.ts:941-961` | gen-editor-dialects | todo |
| `editors/jetbrains/.../TclLspSettings.kt:715-737` `DIALECT_OPTIONS` | gen-editor-dialects | todo |
| `editors/sublime-text/sublime-package.json:18-35` enum | gen-editor-dialects | todo |
| `editors/vscode/package.json` `contributes.languages`, `onLanguage`, `configurationDefaults` | gen-editor-extensions | todo |
| `editors/vscode/src/languageIds.ts`, `extension.ts:148-170` `LANGUAGE_ID_DIALECTS` | gen-editor-extensions | todo |
| JetBrains `plugin.xml:37-44` fileTypes, `TclFileType.kt`, textmate manifest | gen-editor-extensions | todo |
| `editors/sublime-text/plugin.py:62-91` extensions | gen-editor-extensions | todo |
| Zed `languages/*/config.toml` `path_suffixes` | gen-editor-extensions | todo |
| Helix README `file-types` | gen-editor-extensions | todo |
| `editors/vscode/src/chat/dialectCatalog.ts` | gen-ai-diagnostics | todo |
| `rust/xtask/src/editor_extensions.rs:75-96` `DIALECT_SURFACES`, tcl-apl rules | (source of generator, becomes registry data) | todo |
| `rust/xtask/src/gen_zed_queries.rs:127-161` targets | (registry data) | todo |

### Hand-written today (→ generated by `gen-editor-configs` / `gen-environment-docs`)

| Site | Status |
|---|---|
| `editors/zed/extension.toml:31-33`, `:49-55` languages and `language_ids` | todo |
| `editors/zed/README.md:34-53` | todo |
| `editors/neovim/tcl_lsp.lua:13`, `:19-22`; `README.md:30-43`, `:126-130` | todo |
| `editors/helix/README.md:28-173` blocks, `:195-198` list | todo |
| `editors/emacs/README.md:24-67`, `:89` | todo |
| `editors/sublime-text/LSP-Tcl.sublime-settings:17`, `README.md:39-41`, `_SYNTAX_DIALECT_MAP` | todo |
| `INSTALL-editors.md` extension lists | todo |
| `README.md:907-927` dialect table; `:1042-1078` detection order (wrong vs contract) | todo |
| `ai/prompts/manifest.json` `dialects[]` | todo |
| `docs/kcs/features/kcs-feature-dialect-selection.md:16`, `:20` | todo |
| `editors/vscode/package.json` `semanticTokenScopes` (missing `tcl-tmsh`, `tcl-microchip`, `tclspec`, `sslictcl`); 44 `editorLangId =~ /^tcl/` clauses (miss `sslictcl`) | todo |
| `editors/vscode/src/extension.ts:172-178` `TCL_VERSION_DIALECTS`, `:785-843` client-side detection | todo |
| JetBrains `PackAssociationReconciler.kt:62` `"tcl-irule"`; `TclLspSettings.kt:44` default | todo |
| `rust/tcl-dialect/src/model/environment.rs:90-111` `CONTRIBUTED` (add `tcl-jim`) | todo |

### Detection

| Site | Status |
|---|---|
| `rust/tcl-registry/src/dialects.rs:71-79`, `:993-1003` shebang parser → `shebang_words` | todo |
| `environment.rs:243`, `:872`, `:927` `shebang_words` (only a test reads them) | todo |

### Jim

| Site | Status |
|---|---|
| `rust/tcl-registry/src/model/context.rs` `surface_admits`, `AuthoringScope::core`, `SurfaceQuery::core` (D17-J) | todo |
| `rust/tcl-spectcl/core-surfaces/jim.tclspec` (roster; additions pack beside it) | todo |
| `proc` spec for Jim (statics), `class`/`super` grammar, two-word proc names | todo |
| W002 cross-family wording (`system` is `commands/expect/system.rs`) | todo |
| `rust/xtask/src/command_backing.rs` classification for Jim additions | todo |

## Behavioural deltas accepted

- `tcl --dialect` accepts every environment name and alias (was: the 19
  catalogue names, their aliases, and `tk`).
- A session-scope `tclLsp.dialect` naming an unknown environment logs a
  WARNING and uses the default (was: silently the lenient `tcl` sink).
- `#!/usr/bin/env jimsh` selects `jim`; `wish` selects `tk` (was: ignored
  / version only).
- EDA enum descriptions read as a Tcl release plus packages; display
  names may change (open decision).
- A one-time notice appears for `Packages`-kind environments.

## Landing checklist

The `rust` branch documents current state only; process history lives in
git. Before the final commit:

- Delete this file and restore `lanes/README.md` to "In flight: None".
- Rewrite `contracts/environment-selection.md` as present-tense fact: no
  ledger ids, no "deferred", no decision attributions or dates, no
  phase/lane/stage vocabulary. What the system does not do may be stated
  as fact under a neutral heading.
- Grep the whole diff against `rust` (code comments, doc comments, docs,
  generated-file headers; commit messages exempt) for `lane`, `phase`,
  `stage`, `wip`, `D17`, `D15`, `as of`, `no longer`, `previously`,
  `used to`, `legacy`, `migrat`, `2166`, and rewrite each hit to describe
  behaviour, not history.

## Open uncertainties

- Whether the JetBrains platform LSP client renders `showMessageRequest`
  actions; the `showMessage` fallback covers it either way, and
  `make test-jetbrains` needs the multi-GB SDK, so 1d verifies with a fake
  client and VS Code only.
- Whether `sleep` is a Jim core command or an extension in every measured
  build (2b measures rather than assumes).
- Which grammar codegen reaches for a runtime pack-declared environment
  (`grammar_of_dialect_name(profile.name)` may fall to the default) —
  noted by the survey, out of this lane's scope, to be filed.
