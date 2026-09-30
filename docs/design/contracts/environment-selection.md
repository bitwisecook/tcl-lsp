# Environment selection

How a user names what they are writing, and how every surface that shows
or accepts that name stays in step. Companion to
[dialect-detection.md](dialect-detection.md) (which tier wins) and
[registry/dialect-and-package-registry-redesign.md](../registry/dialect-and-package-registry-redesign.md)
(the model). This contract is the user-visible half the redesign left as
ledger rows D15 and D17-J.

## The rule

One vocabulary, one resolver, two enumeration mechanisms, and nothing else.

- **Vocabulary.** The selectable unit is the *environment*
  (`tcl_dialect::model::EnvironmentDefinition`, held in the
  `EnvironmentRegistry`). The user-facing spellings `tclLsp.dialect`,
  `# tcl-dialect:`, `--dialect`, `tcl-lsp.setDialect` and the picker title
  "Select Dialect" are permanent: they are how users already write the
  name, and renaming them buys nothing.
- **Resolver.** Every ingress resolves through
  `tcl_registry::model::ingress::resolve_environment` (validating form
  `resolve_known_environment`). This already holds; the contract forbids a
  second path.
- **Enumeration.** A list of names shown to a user comes from exactly one
  of two places:
  1. **Runtime lists** read the live registry at the moment they are
     built: the `tcl` CLI's `--dialect` values and its unknown-name
     message, the MCP `dialect_schema` enum, `tcl-lsp.listDialects`,
     `tcl-lsp.getEffectiveConfig` labels, the Spec Studio picker, and the
     server's `unknown_dialect_error`.
  2. **Generated artefacts** are written by `cargo xtask` from the
     compiled registry (generation 0: the built-ins plus the bundled-pack
     seed) and gated by `--check` in `make xtask-check`: every editor
     manifest, settings schema, language table and README snippet, the AI
     manifest, the README environment table and
     `docs/generated/environments.md`.

  There is no third mechanism. `DialectProfile::all()` is the lexer's and
  the editors' *identity key*, never a user-facing list; the
  `catalogue-callers` gate holds its call sites to an allowlist so one
  cannot creep back in.

## Kind

Each environment declares an `EnvironmentKind`:

| Kind | Meaning | Members |
|---|---|---|
| `Language` | The thing being written is this language: its grammar, or its core command vocabulary, is the identity | `tcl8.4`–`tcl9.1`, `f5-irules`, `f5-iapps`, `f5-tmsh`, `f5-bigip`, `jim`, `bpf`, `expect`, `spectcl`, `sslictcl` |
| `Packages` | A stock Tcl release with library packages loaded: a tool shell | `tk`, the six EDA environments, every pack-declared environment (default) |

Kind is **declared, not derived**. No rule over the existing fields
separates `bpf` (a language whose surface is a package over a Tcl 9.0
core) from `tk` (a package over a Tcl 8.x core); the classification is a
judgement, so it is a field, and the compiler makes every environment
state one. The pack vocabulary gains `kind language|packages` inside an
`environment` block; an omitted `kind` in a pack means `packages`, because
a pack-declared environment is by construction a base plus packages.

Kind drives three things and nothing else: the grouping in every picker,
the description string beside each name, and whether the notice below may
fire. It never changes resolution, grammar, or availability.

Pickers keep the word **dialect** and show two groups, *Dialects* and *Tcl
+ packages*; the notice explains the second.

## Display names and aliases

The six EDA environments are named for the tool a user runs, and gain the
spellings users type. Canonical ids and language ids are unchanged; the
names and aliases are pack data in each `specs/eda_*.tclspec`
`environment` block, and the bundled seed and catalogue row follow them.

| Id | `display_name` | `short_name` | Aliases |
|---|---|---|---|
| `xilinx-eda-tcl` | Xilinx Vivado | Vivado | `vivado` |
| `intel-quartus-eda-tcl` | Intel Quartus Prime | Quartus | `quartus` |
| `mentor-eda-tcl` | Siemens Questa / ModelSim | Questa | `questa`, `modelsim` |
| `microchip-libero-eda-tcl` | Microchip Libero SoC | Libero | `libero` |
| `synopsys-eda-tcl` | Synopsys DC / PrimeTime / ICC2 / Formality | Synopsys | `dc_shell`, `primetime` |
| `cadence-eda-tcl` | Cadence Genus / Innovus / Xcelium | Cadence | `genus`, `innovus` |

An alias never collides with a package name a pack `provides` or pins
(`synopsys` is a package, so it is not an alias).

## Description strings

Derived, never authored:

- `Language`: the environment's `display_name`.
- `Packages`: `{display_name} — Tcl {core release} + {ambient packages}`,
  e.g. `Xilinx Vivado — Tcl 8.5 + vivado, sdc, upf`.

Every generated enum description, picker line and status-bar tooltip uses
this one function (`EnvironmentDefinition::description`).

## Jim

`jim` becomes reachable from every surface by the general rule, plus four
Jim-specific changes:

1. `tcl-jim` joins the fixed contributed editor-identity set and the `jim`
   environment claims it, so the generators emit a Jim language mode in
   every editor that has language modes.
2. The shebang detection tier reads each environment's `shebang_words`
   (`jimsh` → `jim`, `wish` → `tk`, `tclsh` → the versioned Tcl release)
   instead of a hand-written `tclsh`/`wish` parser. Editors whose
   manifests support a first-line pattern get one generated from the same
   words.
3. A `jim` document's authoring scope carries its **own** family ahead of
   its ancestry anchor (`AuthoringScope::core` and `SurfaceQuery::core`
   become an ordered list; `surface_admits` accepts nearest first). This
   is redesign D17-J and is what lets Jim's own commands — `loop`,
   `range`, `lsubst`, `alias`, `local`, `upcall`, `ref`/`getref`/`setref`,
   `os.*`, `class`/`super`, … — and a Jim-specific `proc name args
   ?statics? body` spec shadow the inherited Tcl 8.6 rows.
4. Jim's additions are a second built-in pack beside `jim.tclspec`,
   measured the same way (`info commands` in a `jimsh` built from each
   upstream tag 0.76–0.84), with `available {jim 0.xx-}` windows. Jim's
   `class` is a `DefinitionBodyGrammar` plus a `DefinerFamily` arm, per
   the registry invariant; a `proc` whose name is a two-word list defines
   the two-word command.

## The notice

When a document's effective environment has kind `Packages` **and**
`Provenance::BundledPack` — the six EDA shells, and any tool shell a
future bundled pack declares — the server tells the user once what that
means. `tk` and workspace- or user-tier pack environments never trigger
it. The server owns it: the environment catalogue is the server's, and a
server-sent message renders in every editor with no client code.

- **Transport.** `window/showMessageRequest` when the client advertises
  `window.showMessage.messageActionItem`; plain `window/showMessage`
  otherwise. Actions: *Learn more* (a `window/showDocument` to the KCS
  note, or the URL in a `logMessage` when the client lacks
  `showDocument`) and *Don't show again*.
- **Text.** `{display_name} ({id}) is Tcl {release} plus the {packages}
  packages. Tool support is a set of library packages on a Tcl release,
  not a separate dialect; your selection keeps working as before.`
- **Frequency.** At most once per environment per session, and never
  again once dismissed. Dismissals persist in
  `$XDG_STATE_HOME/tcl-lsp/notices.ini` (`[dismissed] environment-kind =
  xilinx-eda-tcl, …`) so the choice follows the user across editors.
- **Off switch.** `tclLsp.notifications.environmentKind` (default `true`)
  in editor settings, mirrored as `[notifications] environment_kind` in
  the XDG `config.ini` for editors without a settings UI.
- **Policy.** [config-precedence.md](config-precedence.md) keeps ignored
  settings silent. This notice is not about an ignored setting: it is a
  one-time explanation of a classification, and is the one scoped
  exception to that silence, recorded there.

## Backwards compatibility

- **Canonical ids never change.** `xilinx-eda-tcl` stays `xilinx-eda-tcl`;
  the same is true of every language id (`tcl-xilinx`, …). A directive,
  `.tcl-lsp.ini`, `config.ini`, `settings.json` or `folderDialects` entry
  written today resolves tomorrow. New spellings are `alias` rows only.
- **Unknown names.** A directive tier abstains (unchanged). A folder
  setting is validated and dropped (unchanged). A session-scope
  `tclLsp.dialect` is now validated too: an unknown value logs a WARNING
  naming the valid set and falls back to the default, instead of
  silently resolving to the lenient `tcl` sink. The CLI rejects it with
  the full list (unchanged, list widened).
- **Settings schemas** enumerate canonical ids only; aliases are for
  directives and the CLI, so a stored value never fails schema validation.
- **Sidecar stubs** (`<name>.tcl.stubs`) are found by the raw configured
  name (unchanged).

## Deferred, deliberately

- Retiring the `DialectProfile` catalogue as the lexer key (redesign D5,
  centralisation C1, ~200 call sites). This contract makes the catalogue
  invisible, not absent.
- Free composition ("`tcl8.6` plus `vivado`") through `EnvironmentOverlay`.
  Tracked as a follow-up; kind `Packages` is the presentational half.
- Per-package version windows (D17-P) and a `ToolVersion` setting.
- The vendor rows' single-release pinning (`available {tcl 8.5}`).

## Gates

| Gate | Holds |
|---|---|
| `cargo xtask catalogue-callers --check` | `DialectProfile::all()` has no caller outside the allowlist |
| `cargo xtask gen-editor-dialects --check` | VS Code / JetBrains / Sublime enums and labels match the compiled registry |
| `cargo xtask gen-editor-extensions --check` | languages, extensions, first-line patterns, activation events match; also runs in write mode from `make codegen` |
| `cargo xtask gen-editor-configs --check` | Zed `language_ids`, Neovim, Helix, Emacs and Sublime tables and README snippets, `INSTALL-editors.md` |
| `cargo xtask gen-environment-docs --check` | README environment table, `docs/generated/environments.md`, `ai/prompts/manifest.json` |
| `every_runtime_enumeration_is_the_registry` (test) | each runtime list equals the registry's canonical set |
| `cargo run --example dialect_surface` (differential) | the Jim scope change moves no non-Jim command |
