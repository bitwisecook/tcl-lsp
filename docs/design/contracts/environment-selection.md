# Environment selection

How a user names what they are writing, and how every surface that shows
or accepts that name stays in step. Companion to
[dialect-detection.md](dialect-detection.md) (which tier wins) and
[registry/dialect-and-package-registry-redesign.md](../registry/dialect-and-package-registry-redesign.md)
(the model).

## The rule

One vocabulary, one resolver, two enumeration mechanisms, and nothing else.

- **Vocabulary.** The selectable unit is the *environment*
  (`tcl_dialect::model::EnvironmentDefinition`, held in the
  `EnvironmentRegistry`). The user-facing spellings `tclLsp.dialect`,
  `# tcl-dialect:`, `--dialect`, `tcl-lsp.setDialect` and the picker title
  "Select Dialect" are permanent: they are how users write the name.
- **Resolver.** Every ingress resolves through
  `tcl_registry::model::ingress::resolve_environment` (validating form
  `resolve_known_environment`). There is no second path.
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
     prompt manifest, the README environment tables and
     `docs/generated/environments.md`. `make codegen` runs every generator
     in write mode.

  There is no third mechanism. `DialectProfile::all()` is the lexer's and
  the editors' *identity key*, never a user-facing list; the
  `catalogue-callers` gate holds its call sites to an allowlist so one
  cannot creep back in.

The selectable set is `EnvironmentRegistry::selectable()` (live) and
`compiled_selectable()` (generation 0): every registry entry except the
lenient `tcl` sink, `Language` before `Packages`, canonical id ascending
within a kind. A fallback is not a choice, so the sink is never listed.

## Kind

Each environment declares an `EnvironmentKind`:

| Kind | Meaning | Members |
|---|---|---|
| `Language` | The thing being written is this language: its grammar, or its core command vocabulary, is the identity | `tcl8.4`–`tcl9.1`, `f5-irules`, `f5-iapps`, `f5-tmsh`, `f5-bigip`, `jim`, `bpf`, `expect`, `spectcl`, `sslictcl` |
| `Packages` | A stock Tcl release with library packages loaded: a tool shell | `tk`, the six EDA environments, every pack-declared environment (the default) |

Kind is **declared, not derived**. No rule over the existing fields
separates `bpf` (a language whose surface is a package over a Tcl 9.0
core) from `tk` (a package over a Tcl 8.x core); the classification is a
judgement, so it is a field, and the compiler makes every environment
state one. The pack vocabulary has `kind language|packages` and
`short_name TEXT` inside an `environment` block; an omitted `kind` means
`packages`, because a pack-declared environment is by construction a base
plus packages.

Kind drives three things and nothing else: the grouping in every picker,
the description string beside each name, and whether the notice below may
fire. It never changes resolution, grammar, or availability.

Pickers keep the word **dialect** and show two groups, *Dialects* and *Tcl
+ packages*; the notice explains the second.

## Display names and aliases

The six EDA environments are named for the tool a user runs, and carry the
spellings users type. Canonical ids and language ids are the stable keys;
the names and aliases are pack data in each `specs/eda_*.tclspec`
`environment` block, and the bundled seed and catalogue row follow them.

| Id | `display_name` | `short_name` | Aliases |
|---|---|---|---|
| `xilinx-eda-tcl` | Xilinx Vivado | Vivado | `vivado` |
| `intel-quartus-eda-tcl` | Intel Quartus Prime | Quartus | `quartus` |
| `mentor-eda-tcl` | Siemens Questa / ModelSim | Questa | `questa`, `modelsim` |
| `microchip-libero-eda-tcl` | Microchip Libero SoC | Libero | `libero` |
| `synopsys-eda-tcl` | Synopsys DC / PrimeTime / ICC2 / Formality | Synopsys | `dc_shell`, `primetime` |
| `cadence-eda-tcl` | Cadence Genus / Innovus / Xcelium | Cadence | `genus`, `innovus` |

An alias may equal a package its own environment places (`vivado` is both
the Vivado environment's alias and the package it loads) and never equals
a package another environment or pack names, nor any other environment's
name, alias or editor identity; the registry index rejects the collision.

## Description strings

Derived, never authored, by `EnvironmentDefinition::description`:

- `Language`: the environment's `display_name`.
- `Packages`: `{display_name} — Tcl {core release} + {ambient packages}`
  in the pack's declaration order, and the packs list the tool package
  first: `Xilinx Vivado — Tcl 8.5 + vivado, sdc, upf`.

Every generated enum description, picker line and status-bar tooltip uses
this one function. `DEFAULT_ENVIRONMENT_ID` (`tcl8.6`) is the one source
of every generated and server-side default.

## Jim

`jim` is reachable from every surface by the general rule, plus four
Jim-specific facts:

1. `tcl-jim` is in the fixed contributed editor-identity set and the `jim`
   environment claims it, so the generators emit a Jim language mode in
   every editor that has language modes.
2. The shebang detection tier reads each environment's `shebang_words`
   (`jimsh` → `jim`, `wish` → `tk`, `tclsh8.5` / `wish8.5` → `tcl8.5`,
   `expect` → `expect`); editors whose manifests support a first-line
   pattern get one generated from the same words.
3. A `jim` document's authoring scope carries its **own** family ahead of
   its ancestry anchor: `AuthoringScope::core` and `SurfaceQuery::core`
   are ordered lists (`CorePoints`), `surface_admits` accepts a row from
   either, and both resolution paths rank the nearer point first. A
   `Core(Jim)` row therefore resolves for a `jim` document and shadows an
   inherited Tcl row of the same head; every other family has a
   single-point scope and is unaffected.
4. Jim's own commands are the compiled-in pack
   `rust/tcl-spectcl/core-surfaces/jim-own-surface.tclspec`, registered
   unconditionally at `Provenance::BuiltIn` beside the inherited-surface
   roster `jim.tclspec`, with `available {jim FIRST-LAST}` windows measured
   from `info commands` in a `jimsh` built from each upstream tag
   0.76–0.84. It carries Jim's `proc name arglist ?statics? body`
   (`ArgRole::StaticVarList`), `class` as `DefinerFamily::JimClass` whose
   members are two-word commands, `loop`'s two windowed forms, and the
   n-ary arithmetic commands. A `proc` whose name is a two-word list
   defines the two-word command in every dialect.

## The notice

When a document's effective environment has kind `Packages` **and**
`Provenance::BundledPack` — the six EDA shells, and any tool shell a
bundled pack declares — the server tells the user once what that means.
`tk` and workspace- or user-tier pack environments never trigger it. The
server owns it: the environment catalogue is the server's, and a
server-sent message renders in every editor with no client code.

- **Transport.** `window/showMessageRequest` when the client advertises
  `window.showMessage.messageActionItem`; plain `window/showMessage`
  otherwise. Actions: *Learn more* (a `window/showDocument` to the KCS
  note, or the URL in a `logMessage` when the client lacks
  `showDocument`) and *Don't show again*.
- **Text.** `{display_name} ({id}) is Tcl {release} plus the {packages}
  packages. Tool support is a set of library packages on a Tcl release,
  not a separate dialect; your selection keeps working as before.`
- **When.** After `didOpen`, and again when a document's dialect changes
  on re-resolution; at most once per environment per session, and never
  again once dismissed. The sending task holds no document or analyser
  lock, and waits for the first configuration pull (bounded at 30 s) before
  consulting the setting, so a document restored at start-up cannot show
  the notice to a user who has switched it off. Dismissals persist in
  `$XDG_STATE_HOME/tcl-lsp/notices.ini` (`[dismissed] environment-kind =
  xilinx-eda-tcl, …`; `~/.local/state` by default, the platform's state
  directory on macOS and Windows) so the choice follows the user across
  editors; the file is read once at start-up and rewritten atomically on
  dismissal, keeping other sections.
- **Off switch.** `tclLsp.notifications.environmentKind` (default `true`)
  in editor settings — read from the `workspace/configuration` pull,
  `initializationOptions` and `didChangeConfiguration`, applied live — and
  `[notifications] environment_kind = false` in the XDG `config.ini` or a
  project `.tcl-lsp.ini` for editors without a settings UI, layered as
  every other key in [config-precedence.md](config-precedence.md).
  `getEffectiveConfig` reports it as `notifications_environment_kind`.
- **Policy.** [config-precedence.md](config-precedence.md) keeps ignored
  settings silent. This notice is not about an ignored setting: it is a
  one-time explanation of a classification, and is the one scoped
  exception to that silence, recorded there.

## Compatibility of stored names

- **Canonical ids are stable.** `xilinx-eda-tcl` is `xilinx-eda-tcl`; the
  same holds for every language id (`tcl-xilinx`, …). A directive,
  `.tcl-lsp.ini`, `config.ini`, `settings.json` or `folderDialects` entry
  resolves by canonical id or alias. New spellings are `alias` rows only.
- **Unknown names.** A directive tier abstains. A folder setting is
  validated and dropped. A session-scope `tclLsp.dialect` is validated: an
  unknown value logs a WARNING naming the selectable set and the default
  applies. The CLI rejects an unknown value listing the selectable ids.
- **Settings schemas** enumerate canonical ids only; aliases are for
  directives and the CLI, so a stored value never fails schema validation.
- **Sidecar stubs** (`<name>.tcl.stubs`) are found by the raw configured
  name.

## Not modelled

- The `DialectProfile` catalogue remains the lexer's and the editors'
  identity key (retiring it is redesign D5 / centralisation C1). This
  contract makes it invisible, not absent.
- There is no free composition ("`tcl8.6` plus `vivado`");
  `EnvironmentOverlay` has no production caller. Kind `Packages` is the
  presentational half of that story.
- Per-package version windows are parsed and dropped (redesign D17-P);
  there is no `ToolVersion` setting.
- The vendor rows pin one release (`available {tcl 8.5}`).

## Gates

| Gate | Holds |
|---|---|
| `cargo xtask catalogue-callers --check` | `DialectProfile::all()`, `KNOWN_DIALECTS` and `available_dialects(` have no caller outside the allowlist |
| `cargo xtask gen-editor-dialects --check` | VS Code / JetBrains / Sublime enums, labels and defaults match the compiled registry |
| `cargo xtask gen-editor-extensions --check` | languages, extensions, first-line patterns, activation events, semantic-token scopes and `when` clauses match |
| `cargo xtask gen-editor-configs --check` | Zed `extension.toml`, the Zed / Helix / Emacs / Neovim / Sublime guides, Neovim Lua and `INSTALL-editors.md` regions match |
| `cargo xtask gen-environment-docs --check` | README environment tables, `docs/generated/environments.md`, the KCS lists and `ai/prompts/manifest.json` match |
| `cargo xtask gen-ai-diagnostics --check` | the VS Code chat catalogue matches |
| `every_runtime_enumeration_is_the_registry` (tests, one per crate) | each runtime list equals the selectable canonical-id set |
| `cargo run --example dialect_surface` (differential) | a Jim change moves no non-Jim command |
