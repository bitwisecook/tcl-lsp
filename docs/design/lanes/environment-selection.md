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
| 11 | An alias may equal a package its own environment places (`vivado`), never another environment's package, name, alias or identity | The two namespaces are separate in code; the rule guards against a rival's name, not the owner's |
| 12 | `description()` lists ambient packages in pack declaration order; the six EDA packs declare the tool package first so the string reads `Tcl 8.5 + vivado, sdc, upf` (1c reorders the rows) | One rule, no sort special-case |
| 13 | User-selectable environments are every registry entry except the lenient `tcl` sink, ordered `Language` then `Packages`, canonical name within kind — one function every list and generator reads | A fallback is not a choice |

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

All done (1b). `EnvironmentRegistry::selectable()` / `compiled_selectable()` /
`tcl_registry::model::selectable_environments()` are the one set (21 ids;
`LENIENT_ENVIRONMENT_ID` names the excluded sink). `listDialects` entries:
`{name, display_name, short_name, kind, description, aliases,
editor_language_id, extensions:[{extension, display_name}]}`;
`getEffectiveConfig` adds `dialect_id`, `dialect_kind`
(`"language"|"packages"`), `dialect_description`, `dialect_provenance`
(`"built-in"|"bundled-pack"|"user-pack"|"workspace-pack"|"studio-override"|"document"`).
Language-id remaps are `EditorLanguageIdentityId::SELECTING` +
`tcl_registry::model::resolve_language_id`. Session `tclLsp.dialect` is
validated; unknown → WARNING `logMessage` + default. `KNOWN_DIALECTS` and
`available_dialects()` removed; allowlist 35 → 28.

| Site | Status |
|---|---|
| CLI possible values / unknown-name message; MCP enum and `valid_dialects`; `listDialects`; `getEffectiveConfig`; `unknown_dialect_error`; language-id remaps; session validation; studio `DIALECTS` labels; `KNOWN_DIALECTS` | done (1b) |
| `tcl_spec_studio::browsable_dialects()` — still the 19 catalogue profiles because the studio resolves through `catalogue_dialect_or_default`, which would sink `jim`/`tk` to `tcl9.0` built-ins; stays on the allowlist | deferred, reasoned |
| f5-cli `--dialect` is a free `String` through the lenient resolver | unchanged |

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
| `rust/tcl-registry/src/dialects.rs:71-79`, `:993-1003` shebang parser → `shebang_words` | done (1a) |
| `environment.rs:243`, `:872`, `:927` `shebang_words` (only a test reads them) | done (1a): ladder rows also list `wish<release>`; bare `wish` → `tk` |
| `EnvironmentKind`, `short_name`, `description()`, pack `kind`/`short_name` words, `tcl-jim` in `CONTRIBUTED`, EDA names/aliases as pack data + seed + catalogue rows, `catalogue-callers` gate (35-file allowlist; does not yet cover `available_dialects()`) | done (1a) |
| Spec Studio has no environment form: `GAPS` entries `environment_kind`, `environment_short_name` under `GapKind::PackLevel` | done (1a) |

### Jim

| Site | Status |
|---|---|
| `rust/tcl-registry/src/model/context.rs` `surface_admits`, `AuthoringScope::core`, `SurfaceQuery::core` (D17-J) — `CorePoints`, nearest-first in `best_visible`; `ContextRegistry::resolve_command` (assembly.rs) tie-break not yet nearest-first; ledger row D17-J still reads open | done (2a) |
| `rust/tcl-spectcl/core-surfaces/jim-own-surface.tclspec` — 60 commands on measured windows, registered at `Provenance::BuiltIn` through a new `register_core_surface_specs` seam (overlays only reach catalogue profiles); `stdin`/`stdout`/`stderr` as `dynamic_surface`; `callback-inventory` walks pack commands | done |
| Jim `proc`: `ArgRole::StaticVarList`, `CommandRegistry::procedure_definition_words`; analyser, signature scan and binding replay read positions from roles; statics declared as body locals. Left out: no IR static variable, so a 4-word `proc` body gets no W210/W211 dataflow; `&g` does not mark outer `g` used | done |
| `DefinerFamily::JimClass`, `members_are_two_word_commands`; `class` row's `definition_body` (method member, `new`, built-in object methods, implicit `self`, dynamic dispatch); `ClassDef` with bases and variable dict; `CLASS method …` and `proc {CLASS M}` recorded; `[CLASS new]` types the object. Left out: IR-level W210 may fire on an instance-variable read inside a `CLASS method` body; `constructor`/`defaultconstructor`/`baseclass` via the open member set; a method written before its class is unattached; computed bases mark `inheritance_unknown` | done |
| W002 only where the providing dialect is related (`providers_in_any_dialect`, `Family::on_one_line_with`, `shares_packages_with`, `ResolvedContext::is_related_to_a_provider_of`); Expect's `system` under jim is W123. Also fixed: `resolve_spec` did not apply the inherited-surface roster (a jim document accepted `coroutine`); `both_resolution_paths_apply_the_roster` pins it; nearest-first extended to `assembly.rs` | done |
| Acceptance: `rust/tcl-spectcl/tests/jim_document.rs` on both analyser tiers | done |
| `command-backing` needs no Jim rows (reads core Tcl specs only) | n/a |
| Not done: `tcl-lsp-server` e2e for a jim document (`tcl-jim` didOpen, `jimsh` shebang), CLI `--dialect jim` e2e on the merged tree | 2d |

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

## Generators and editors (1c) — state

Done: `gen-editor-dialects`, `gen-editor-extensions`, `gen-ai-diagnostics`
re-keyed on `compiled_selectable()`; `tcl-apl` is the one explicit extra
language (`EXTRA_LANGUAGES`, dialect from `SELECTING`); `ZED_LANGUAGES`
replaces `DIALECT_SURFACES` and feeds `gen-zed-queries`;
`DEFAULT_ENVIRONMENT_ID = "tcl8.6"` in `tcl-dialect` feeds every generated
default; VS Code `package.json` fully regenerated (`enumItemLabels`,
`enumDescriptions`, `tcl-jim` language, `firstLine` from shebang words,
one `semanticTokenScopes` block per language, `when` regex
`/^(?:sslictcl|tcl)/` with a structural check); VS Code client picker from
`listDialects` with two separators (`dialectChoices.ts`, `dialectPicker.ts`),
status bar from `getEffectiveConfig.dialect_short_name`, client-side
detection removed; JetBrains `DIALECT_OPTIONS` + `DEFAULT_DIALECT` +
`IRULE_LANGUAGE_ID` generated (Kotlin uncompiled); Sublime enum with
descriptions and default; EDA packs list tool packages first; allowlist
28 → 25; `make codegen` runs `gen-editor-extensions`.

`gen-editor-configs` fills marker regions (`<generated: name>` …
`</generated>`, in the file's own comment syntax) in Zed `extension.toml`
(language table and `language_ids`), the Zed, Helix, Emacs, Neovim and Sublime
READMEs, `tcl_lsp.lua` and `INSTALL-editors.md`: Helix `[[language]]` blocks
(one per environment with an extension or shebang word, `language-id` = the
editor identity), Emacs derived modes / `auto-mode-alist` /
`interpreter-mode-alist` / eglot rows (`jim-tcl-mode`, `:language-id
"tcl-jim"`) / hooks, Neovim `vim.filetype.add` extensions and a shebang
`pattern`, the dialect lists and defaults. The Emacs forms evaluate in a real
Emacs; the TOML parses; the Lua is unrun (no Neovim here). Makefile:
`generate` (so `codegen`) writes it and `xtask-check` gates it.

Not done: `gen-environment-docs` (README tables,
`docs/generated/environments.md`, `ai/prompts/manifest.json` — **the old
`prompt_manifest_gaps` gate was removed, so nothing checks the manifest
until this lands**, and it still lacks `jim`/`tk`); its Makefile wiring.

For 1d (owns `lib.rs`): `DEFAULT_SESSION_DIALECT` (`lib.rs` ~:27427) and
the bare-`tcl` literal (~:11321) should read `DEFAULT_ENVIRONMENT_ID`;
`tclLsp.notifications.environmentKind` belongs in the hand-written General
settings section beside `highlightingHealth`. Observed, not changed:
`tcl-mcp/src/tools.rs:37` and `tcl-spec-studio/src/environment.rs:42` each
default to `tcl9.0`.

Unverified: VS Code test host (ENOSPC before it ran; partition counts
978→968 / 977→967 computed statically); Kotlin edits; `(?:a|b)` in real
VS Code `when` clauses.

## Gate notes

- `cargo xtask dialect-drift` fails on eight pre-existing sites in
  `tcl-compiler` and `tcl-lsp-core` (none touched here). The Makefile's
  `xtask-check:` line lists `xtask-dialect-drift` after its `##` help
  comment, so the gate has never run in CI; queued as a separate task.
  Lanes here must add no new site, and treat that gate's pre-existing
  failures as out of scope.
- Base `rust` CI at `c2860edcb` is red on `cargo-deny` and a Tank
  cache-preparation step of `rust-tests-shard`; `rust-check` is green.

## Open uncertainties

- Whether the JetBrains platform LSP client renders `showMessageRequest`
  actions; the `showMessage` fallback covers it either way, and
  `make test-jetbrains` needs the multi-GB SDK, so 1d verifies with a fake
  client and VS Code only.
- Resolved: `sleep seconds` is present in every `jimsh` 0.76–0.84 build, and
  no Jim command is package-gated (`Jim_InitStaticExtensions` runs at
  start-up). Evidence in the session scratchpad `jim-evidence/`
  (`jim-only-commands.md`, `proc-statics.md`, `oo.md`, `SUMMARY.md`).
- Which grammar codegen reaches for a runtime pack-declared environment
  (`grammar_of_dialect_name(profile.name)` may fall to the default) —
  noted by the survey, out of this lane's scope, to be filed.
