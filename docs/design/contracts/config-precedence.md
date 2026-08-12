# Config precedence contract

## Summary

The Tcl Language Server merges configuration from three sources. The
project file `.tcl-lsp.ini` at the workspace root has the highest
priority and overrides editor settings, which in turn override the
global XDG `config.ini`. This document captures the rationale behind
that ordering, so future contributors can resist the temptation to
re-litigate it without new information.

The user-facing version of this contract — and the trace-it-yourself
guide — lives in
[`docs/kcs/kcs-qa-how-tcl-lsp-loads-configuration.md`](../../kcs/kcs-qa-how-tcl-lsp-loads-configuration.md).

## Precedence (lowest to highest)

| Layer | Source | Scope |
|---|---|---|
| 1 | `~/.config/tcl-lsp/config.ini` (XDG global) | Per user, all workspaces |
| 2 | Editor settings via `workspace/configuration` | Per editor scope (user / workspace / folder) |
| 3 | `<workspace-folder>/.tcl-lsp.ini` (project) | Per workspace folder, committed to source |

The merge is per-key inside each section: a higher layer that sets
`[optimiser] disabled = O109` still inherits `[optimiser] profile =
readability` from a lower layer.

Implementation: `config_ini::merge_settings`
(`rust/tcl-lsp-server/src/config_ini.rs`) deep-merges the three layers, later
winning, sections merged key by key. Each file layer is parsed by
`settings_from_ini` into the same JSON shape the editor's
`workspace/configuration` payload has, so `Backend::apply_global_config`
applies a file layer through exactly the code that applies the editor layer.

## Why project wins over editor

We surveyed how other widely-used language servers handle the same
question in May 2026:

| Tool | Convention | Source |
|---|---|---|
| Pyright / Pylance | `pyrightconfig.json` / `pyproject.toml` overrides `python.analysis.*` editor settings | [pylance-release wiki](https://github.com/microsoft/pylance-release/wiki/Settings.json-overridden-by-Pyrightconfig.json-or-Pyproject.toml) |
| typescript-language-server | `tsconfig.json` overrides editor; `implicitProjectConfig` only when no tsconfig | [docs/configuration.md](https://github.com/typescript-language-server/typescript-language-server/blob/master/docs/configuration.md) |
| vscode-eslint | `.eslintrc.*` / flat config is the only source of lint rules; editor cannot override | [microsoft/vscode-eslint](https://github.com/microsoft/vscode-eslint) |
| clangd | `.clangd` is authoritative; CLI / init options being phased out | [clangd.llvm.org/config](https://clangd.llvm.org/config) |
| gopls | No on-disk file; editor settings are the only source | [go.dev/gopls/settings](https://go.dev/gopls/settings) |
| Biome | **Editor `inlineConfig` overrides `biome.json`** | [biomejs.dev/reference/vscode](https://biomejs.dev/reference/vscode/) |
| Ruff | User-selectable via `ruff.configurationPreference` (`editorFirst` default) | [docs.astral.sh/ruff/editors/settings](https://docs.astral.sh/ruff/editors/settings/) |
| rust-analyzer | Unresolved; explicit acknowledgement of the schema-default echo problem | [issue #13529](https://github.com/rust-lang/rust-analyzer/issues/13529) |

Four of the five most widely-deployed servers (pyright, tsserver,
eslint, clangd) put the committed project file above editor settings.
The shared rationale across all four:

1. The file is checked into source control.
2. CI uses the same file.
3. Teammates share the same file.
4. Therefore editor behaviour should not silently diverge from CI or
   from teammates.

Following the same convention gives users coming from those servers
the behaviour they already expect.

### What we copied from each reference implementation

Our three-layer model is not a single-source port — none of the surveyed
servers ships all three layers (project file + editor settings + global
user file). We composed the behaviour from these specific precedents:

- **Pyright** ([pylance-release wiki](https://github.com/microsoft/pylance-release/wiki/Settings.json-overridden-by-Pyrightconfig.json-or-Pyproject.toml))
  is the closest direct analogue: when `pyrightconfig.json` or
  `pyproject.toml` exists, it silently overrides `python.analysis.*`
  editor settings. We copied **the project-file-wins-over-editor rule
  and the "silent override" behaviour** (we do not warn the user when
  their editor setting is ignored — neither does Pyright).
- **typescript-language-server / VS Code's TS server**
  ([docs/configuration.md](https://github.com/typescript-language-server/typescript-language-server/blob/master/docs/configuration.md))
  applies the same rule: `tsconfig.json` overrides editor settings, and
  `implicitProjectConfig.*` only kicks in when no tsconfig is present.
  We copied **the "editor settings act as the fallback when no project
  file is present" mental model** — our XDG global config plays exactly
  that role for keys neither the project file nor the editor pins.
- **clangd** ([clangd.llvm.org/config](https://clangd.llvm.org/config))
  resolves multiple `.clangd` files by nesting (inner overrides outer,
  user overrides project). We copied **the per-workspace-folder scoping
  of the project file**: in a multi-root workspace each folder gets its
  own `.tcl-lsp.ini` and the analyser resolves the file-to-folder match
  by longest URI prefix, the same way clangd resolves nested configs.
- **ESLint** ([microsoft/vscode-eslint](https://github.com/microsoft/vscode-eslint))
  treats the project file as the only source of lint rules and lets
  editor settings only redirect *which* file is loaded. We did not copy
  this strict form — our editor layer can override individual rules
  when the project file is silent on them — but we copied **the
  "committed file is the source of truth for what runs in CI" framing**
  for the rationale section above.

The two servers we deliberately did **not** follow are Biome (editor
wins) and Ruff (user-selectable). Both are reasonable designs; both
require explaining to a user who is debugging an unexpected value.
Following the more conservative convention is worth the small loss in
flexibility, and we keep the Ruff-style escape hatch in our back
pocket (see "Escape hatch" below).

## Why we do not detect "schema default vs explicit user choice"

A tempting alternative is "editor wins, but only for non-default
values" — i.e., treat schema-default echoes from `workspace/configuration`
as if the user had not set the key. **We reject this.**

Rust-analyzer's tracking issue #13529 is the cautionary tale: VS Code
sends back the package.json `default` for every key the user has not
explicitly set, indistinguishable on the wire from a key the user has
explicitly set to that same value. Implementing "only non-default
overrides" therefore requires either:

- A defaults table in the server (must stay in sync with every editor
  schema and every change to defaults), or
- A heuristic that compares editor values to a baseline (silently
  promotes the global config to the implicit "default" for shadowing
  purposes, which is its own surprise), or
- Asking every editor client to mark settings as "explicitly set"
  (not possible with `workspace/configuration` today).

Each of these introduces subtle, hard-to-explain behaviour. A user
who explicitly sets `tclLsp.dialect = "tcl8.6"` in their editor would
see it silently ignored because it happens to equal the schema
default. The least-surprising rule is the declarative one: **higher
layer wins, full stop.**

The one place we deviate is `dialect` itself — see
[dialect-detection.md](dialect-detection.md). VS Code's per-file
workspace-folder echo of the schema-default `tcl8.6` would otherwise block the
iRules / iApps file-extension auto-switch. The deviation is scoped to one key
and one direction, and is deliberately not generalised.

## Why the two files have different names

The global file is `config.ini` and the project file is `.tcl-lsp.ini`.
We could have used the same filename and disambiguated by location,
but the distinct names are an intentional safeguard against accidental
layer-swapping:

- A user who copies `~/.config/tcl-lsp/config.ini` into a workspace
  root sees the copy ignored, rather than silently promoted to the
  highest-priority layer (where it would override a teammate's
  `.tcl-lsp.ini` if there were one, or be committed and override
  everyone else's editor settings).
- A user who copies `.tcl-lsp.ini` into their XDG config directory
  sees the copy ignored, rather than silently demoted to global and
  affecting every other workspace they open.

Different names mean copying the file does not change which precedence
layer it occupies. The cost is a small one-time learning of two
filenames; the benefit is that "I'll just copy this over" never has
silent side effects.

Implementation: the two paths are resolved by
`tcl_lsp_core::tcl_install::user_config_path` (→ `config.ini`) and
`project_config_path` (→ `.tcl-lsp.ini`). Do not unify them.

The same safeguard is applied at the **section** level for top-level
keys (`dialect`, `extraCommands`, `libraryPaths`):

- The XDG `config.ini` only honours these keys under a `[global]`
  section.
- The project `.tcl-lsp.ini` only honours them under a `[project]`
  section.
- A `[global]` block in the project file (or `[project]` in the
  global file) is logged at warning level and ignored.

The double safeguard means a user who both copies the file AND forgets to
rename the section is still caught: the wrong location ignores the file, and
the wrong section name within the file ignores the keys. The implementation is
`config_ini::Layer`, which the caller sets to `Global` or `Project` from which
file it loaded; `Layer::top_section` is the only place the two section names
appear.

## Escape hatch (not implemented)

If a future user reports that they cannot put a personal override
above the team's project file, the right fix is a Ruff-style
`tclLsp.configurationPreference` setting (`projectFirst` | `editorFirst`
| `editorOnly`) — declarative, user-controlled, and easy to explain.
Do not invert the default; add the knob.

## How users trace where a setting is coming from

Documented in
[`kcs-qa-how-tcl-lsp-loads-configuration.md`](../../kcs/kcs-qa-how-tcl-lsp-loads-configuration.md)
under "How to figure out where a setting is coming from". The
canonical mechanism is the `tcl-lsp.getEffectiveConfig` command
(`rust/tcl-lsp-server/src/lib.rs::get_effective_config_command`),
which returns the resolved per-folder values the analyser will use
for a given URI. The server also logs `Loaded user config from <path>`
and `Loaded project config from <path>` on every load.

**Contract: every value the command reports for a URI resolves through the
same chain the code acting on that URI uses.** The command is not a dump of
process-global state — it answers "what is in effect *here*", and a client is
entitled to treat it as a barrier: once it reports a value, a request issued
afterwards is answered under that value.

The `features` map is the one that makes this non-trivial. Each provider gate
(`Backend::feature_enabled` and its default-off siblings) consults the deepest
workspace folder containing the URI before falling back to the process-global
toggles. Reporting the global map alone would make a folder-scoped
`tclLsp.features.*` override invisible in the report while fully in effect in
the behaviour — and a client polling the command to learn that a toggle had
landed would be waiting on a different fact from the one the provider acts on.
Both therefore go through `Backend::resolved_feature_toggles`, the single
place the folder-over-global overlay is performed: a folder overrides exactly
the keys it sets and leaves the rest of the global set alone.

## Related

- [Dialect detection priority chain](dialect-detection.md)
- [User-facing KCS: how does tcl-lsp load configuration?](../../kcs/kcs-qa-how-tcl-lsp-loads-configuration.md)
- [User-facing KCS: what sections and keys are valid?](../../kcs/kcs-qa-what-config-sections-are-valid.md)
- [How do I turn a diagnostic off?](../../kcs/kcs-howto-suppress-diagnostics.md)
