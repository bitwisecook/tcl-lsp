# Dialect detection priority chain

## Summary

The language server selects the active dialect using a layered priority chain.
Per-file hints (comment directive, shebang, file extension) always override the
global setting, so different files in the same workspace can target different
Tcl versions without manual switching.

## Priority chain (highest to lowest)

`tcl_registry::dialects::detect_dialect(source, filename, default)` is the one
detector the LSP, the editors, the CLI, and the AI integrations share. It never
fails — it returns `default` when nothing fires.

| Priority | Source | Example |
|----------|--------|---------|
| 0 | **Editor language ID / explicit `--dialect`** | Applied by the *caller*, above `detect_dialect`, and overrides everything below |
| 1 | **Comment directive** | `# tcl-dialect: tcl8.4` in the first 5 lines (`DIALECT_DIRECTIVE_SCAN_LINES`) |
| 2 | **Shebang** | `#!/usr/bin/env tclsh8.5` or `#!/usr/bin/expect` (first line only) |
| 3 | **Tcl version guard** | a tokenised `package require ?-exact? Tcl 8.5` or `package vsatisfies [package require Tcl] 8.5` |
| 4 | **Content signature** | an iRules `when EVENT {` clause, or a whole-word vendor marker (`iapp::`, `tmsh::`, `synth_design`, `compile_ultra`, `set_db`, `project_new`, `vsim`, `spawn`, …) |
| 5 | **File extension** | `.irul` / `.irule` → `f5-irules`, `.exp` → `expect`, `.xdc` → Xilinx, … |
| 6 | **Caller default** | `tclLsp.dialect` from editor config or XDG `config.ini`, ultimately `tcl8.6` |

**Content outranks the filename.** A `.tcl` file's contents are a stronger
signal than its name, so tiers 3 and 4 sit *above* the extension, not below
it. The `.tcl` extension itself maps to nothing, so a plain script is decided
entirely by its content or by the caller's default.

Tiers 3–4 are scanned streaming-style: the cheap `DETECT_SCAN_BYTES` (8192)
head is tried first, and only on a miss is the full source scanned — so a
signal a very large script reveals only near its end is still caught without
paying that cost when the signal sits near the top, which is the common case.

Tiers 1 and 2 are *not* head-limited in the same way: the directive scan reads
the first five lines and the shebang scan the first line, both of the whole
source.

The BIG-IP config-object tier — recognising an iRule wrapped in a
`bigip.conf` stanza — sits between content and extension in the project's
model but is applied by the `tcl-bigip` layer, which wraps this detector; it
is not reachable from the registry crate.

### Content-signature rules

- The iRules `when` check runs **first and on the raw head**, before comments
  are stripped: a line whose first word is `when`, followed by an identifier
  matching `[A-Z][A-Z0-9_]{2,}`, followed by `{`.
- Every other signature is matched over the head with **full-line comments
  stripped**, and each marker must match at a **word boundary**
  (`contains_token`), so `interactive` never matches the `expect` marker
  `interact`. A marker ending in a non-word byte (`iapp::`, `tmsh::`) or in
  `_` (`quartus_`) is a command-*prefix* form and imposes no right-hand
  boundary.
- The signature table is ordered most-specific first, so an EDA-tool script
  never falls through to a weaker signal.
- Vendor signatures name each vendor's **proprietary** commands only. Shared
  SDC verbs (`create_clock`, `set_input_delay`, `link_design`, `set_max_area`,
  `get_ports`) are deliberately excluded, because they appear in every
  vendor's constraint files and would misclassify a portable `.sdc`.

Language ids and dialect names are separate namespaces. The VS Code extension
contributes *undotted* version-pinned language ids (`tcl84`, `tcl85`, `tcl86`,
`tcl90`, `tcl91`) because a language id containing a `.` cannot carry a
`configurationDefaults` override — VS Code splits the key on the dot, throws,
and drops the rest of the block. The other editor integrations still send the
dotted `tcl8.4`-style id, so the server accepts both spellings and maps them to
the same dialect. Dialect *names* always keep their dots (`tcl8.4`).

## Comment directive format

Place a comment in the first 5 lines of any Tcl file:

```tcl
# tcl-dialect: tcl8.4
```

The directive name is case-insensitive. The value is an **environment
name**, resolved through the one environment resolver
(`tcl_registry::model::resolve_known_environment`) exactly as a
`tclLsp.dialect` setting or `tcl-lsp.setDialect` is: a canonical id
(`tcl8.4`, `f5-irules`, `xilinx-eda-tcl`, `tk`, `jim`, `tcl`), an alias
(`wish` → `tk`, `irules` → `f5-irules`, `jimsh` → `jim`), a contributed
editor identity (`tcl-irule`, `tcl90`), or an environment a loaded SpecTcl
pack declares — matched case-sensitively, and answered as the resolved
**canonical id**. An unrecognised value makes the directive tier abstain,
and detection falls through to the next tier rather than erroring.

`tk` therefore resolves (#1631 E8): it is a package plus an environment,
never a dialect (§2), and the directive selects environments. The
`KNOWN_DIALECTS` list still feeds the CLI's `--dialect` choices and the MCP
`dialect_schema` enum — the payload rows tracked as D15 in
[the #1631 open-questions ledger](../dialect-and-package-registry-redesign.md#11-the-open-questions-ledger)
— but it no longer gates the directive.

The directive takes priority over shebang detection.  This allows a file to
have a generic `#!/usr/bin/tclsh` shebang while still targeting a specific
Tcl version for LSP analysis:

```tcl
#!/usr/bin/tclsh
# tcl-dialect: tcl8.4
set x 1
```

## Shebang detection

The first line only is checked, and only when it starts with `#!`. The line is
lower-cased first, so matching is case-insensitive.

- The word `expect` anywhere on the line (at word boundaries) → `expect`.
  `#!/usr/bin/expect` and `#!/usr/bin/env expect` both match.
- Otherwise, `tclsh<major>.<minor>` — `tclsh` at a left word boundary,
  followed by digits, a `.`, digits, and a right word boundary. The version
  must then be exactly one of `8.4`, `8.5`, `8.6`, `9.0`, or `9.1` to name a
  dialect.

A plain `#!/usr/bin/tclsh` without a version number does not select a
specific dialect and falls through to the next tier. So does a version this
project does not model (`tclsh8.3`, `tclsh9.2`) — an unmodelled version is an
abstention, not an error.

## User setting (`tclLsp.dialect`)

This setting acts as the default dialect for files that have no per-file
hint.  Set it in your editor configuration:

**VS Code** (`.vscode/settings.json`):
```json
{
  "tclLsp.dialect": "tcl8.4"
}
```

**XDG config** (`~/.config/tcl-lsp/config.ini`):
```ini
[dialect]
dialect = tcl8.4
```

## Where detection runs

- **VS Code extension** (`detectDialectFromDocument` in `extension.ts`):
  runs when a document is opened, focused, or when the first few lines change.
  Sends the detected dialect to the server via `workspace/didChangeConfiguration`.
- **LSP server** (`did_open` in `rust/tcl-lsp-server/src/lib.rs`): runs
  server-side detection for editors that do not perform client-side
  detection, delegating to `tcl_registry::dialects::detect_dialect`. That
  function is the canonical detector; the profile *catalogue* it resolves
  names against lives one crate lower, in `rust/tcl-dialect`.
- `dialect_hint_markers()` is the cheap "could this edit change the detected
  dialect" filter: a conservative superset of every substring a hint could
  contain (`tcl-dialect:`, `#!`, `package`, `vsatisfies`, `when`, and every
  content-signature marker). `did_change` uses it to skip a full re-detect on
  an edit that plainly cannot touch the hint. It is deliberately allowed to
  false-positive — it does not replicate the word-boundary or tokenisation
  rules — but nothing that changes the detected dialect can appear in a diff
  without containing one of its substrings.

## Re-detection triggers

Dialect is re-evaluated when:

- The active editor tab changes.
- A Tcl document is opened.
- Any of the first 5 lines of the active document are edited (covers both
  shebang and comment directive changes).
- The `tclLsp.dialect` setting changes.

## Editor language ID mapping

`Backend::dialect_from_language_id` maps every accepted spelling; each row's
alternatives are all accepted and land on the same dialect.

| Language ID | Dialect |
|-------------|---------|
| `tcl`, `tcl8.6`, `tcl86` | `tcl8.6` |
| `tcl8.4`, `tcl84` | `tcl8.4` |
| `tcl8.5`, `tcl85` | `tcl8.5` |
| `tcl9.0`, `tcl90` | `tcl9.0` |
| `tcl9.1`, `tcl91` | `tcl9.1` |
| `tcl-irule`, `f5-irules` | `f5-irules` |
| `tcl-iapp`, `tcl-apl`, `f5-iapps` | `f5-iapps` |
| `tcl-tmsh`, `f5-tmsh` | `f5-tmsh` |
| `tcl-bpf`, `bpf` | `bpf` |
| `tcl-expect`, `expect` | `expect` |
| `tcl-synopsys`, `synopsys-eda-tcl` | `synopsys-eda-tcl` |
| `tcl-cadence`, `cadence-eda-tcl` | `cadence-eda-tcl` |
| `tcl-xilinx`, `xilinx-eda-tcl` | `xilinx-eda-tcl` |
| `tcl-quartus`, `intel-quartus-eda-tcl` | `intel-quartus-eda-tcl` |
| `tcl-mentor`, `mentor-eda-tcl` | `mentor-eda-tcl` |
| `tk` | `tk` |

`tcl-apl` is the APL (iApp presentation language) editor id — an iApp
sublanguage, so it analyses as `f5-iapps` rather than falling through to the
default. `tk` is not a catalog profile (it is a library pin, see
[dialect-profile-model.md](../dialect-profile-model.md) §7.2) but it parses to
a `DialectSet` bit, which the table's own debug assertion accepts. There is no
`tcl-bigip` language-id row: `f5-bigip` is reached through the file's content
and the BIG-IP layer, not an editor language mode.

## File extension mapping

`dialect_from_extension` first checks two vendor filename *conventions* that
are not a single trailing extension, then falls back to the extension itself.
The whole basename is lower-cased first, so matching is case-insensitive.

| Filename shape | Dialect |
|-----------|---------|
| `*.synopsys_dc.setup`, `*.synopsys_pt.setup` | `synopsys-eda-tcl` |
| `*.invs_setup.tcl`, `*.genus_setup.tcl` | `cadence-eda-tcl` |
| `.irul`, `.irule`, `.irules` | `f5-irules` |
| `.iapp` | `f5-iapps` |
| `.tmsh` | `f5-tmsh` |
| `.exp`, `.expect` | `expect` |
| `.xdc` | `xilinx-eda-tcl` |
| `.sdc` | `synopsys-eda-tcl` |
| `.do` | `mentor-eda-tcl` |
| `.qsf`, `.qpf`, `.qip` | `intel-quartus-eda-tcl` |
| `.globals` | `cadence-eda-tcl` |

`.svrf` is **deliberately not mapped**: Calibre DRC/LVS rule decks are a
declarative DSL, not Tcl, so the extension falls through to content detection
and the caller's default rather than being forced to an EDA Tcl dialect.

This is a different question from `TCL_SOURCE_EXTENSIONS`, the set of
extensions the toolchain treats as indexable project *source* (`tcl`, `tk`,
`itcl`, `tm`, `irul`, `irule`, `iapp`, `iappimpl`, `impl`, `exp`, `apl`,
`test`). Several vendor extensions above (`.sdc`, `.do`, `.xdc`) are Tcl but
are not indexed as project source; several source extensions (`.tcl`, `.tm`,
`.apl`, `.iappimpl`) imply no dialect of their own. `TCL_SOURCE_EXTENSIONS` is
the single source of truth for the workspace scan, the watched-file filter,
the rename filter, the CLI's directory discovery, and the generated VS Code
`workspaceContains` activation glob.

## Command-line tools (`tcl diag` / `lint` / `validate`)

The diagnostics verbs of the `tcl` CLI apply the same chain per input
document (minus the editor language ID, which has no CLI equivalent):
an explicit `--dialect` flag overrides everything; otherwise the
in-source signals (comment directive, shebang, content), then the file
extension, then the `tcl8.6` fallback decide.  This keeps `tcl diag`
and the editor reporting the same set for the same file — an `.irul`
input gets full iRules analysis without any flag.
