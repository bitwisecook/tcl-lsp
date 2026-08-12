# Dialect detection priority chain

## Summary

The language server selects the active dialect using a layered priority chain.
Per-file hints (comment directive, shebang, file extension) always override the
global setting, so different files in the same workspace can target different
Tcl versions without manual switching.

## Priority chain (highest to lowest)

| Priority | Source | Example |
|----------|--------|---------|
| 1 | **Editor language ID** | Opening a file as `tcl-irule` or `tcl84` in the editor's language mode picker |
| 2 | **File extension** | `.irul` / `.irule` -> `f5-irules`, `.exp` -> `expect` |
| 3 | **Comment directive** | `# tcl-dialect: tcl8.4` in the first 5 lines |
| 4 | **Shebang** | `#!/usr/bin/env tclsh8.5` or `#!/usr/bin/expect` |
| 5 | **User setting** | `tclLsp.dialect` in editor config or XDG `config.ini` |
| 6 | **Hardcoded fallback** | `tcl8.6` |

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

The directive name is case-insensitive.  The dialect value must be one of the
known dialect strings (`tcl8.4`, `tcl8.5`, `tcl8.6`, `tcl9.0`, `tcl9.1`, `f5-irules`,
`f5-iapps`, `f5-bigip`, `synopsys-eda-tcl`, `cadence-eda-tcl`, `xilinx-eda-tcl`,
`intel-quartus-eda-tcl`, `mentor-eda-tcl`, `expect`).

The directive takes priority over shebang detection.  This allows a file to
have a generic `#!/usr/bin/tclsh` shebang while still targeting a specific
Tcl version for LSP analysis:

```tcl
#!/usr/bin/tclsh
# tcl-dialect: tcl8.4
set x 1
```

## Shebang detection

The first line is checked for shebang patterns:

- `#!/usr/bin/expect` or `#!/usr/bin/env expect` -> `expect`
- `#!/usr/bin/tclsh8.4` or `#!/usr/bin/env tclsh8.4` -> `tcl8.4`
- `#!/usr/bin/tclsh8.5` or `#!/usr/bin/env tclsh8.5` -> `tcl8.5`
- `#!/usr/bin/tclsh8.6` or `#!/usr/bin/env tclsh8.6` -> `tcl8.6`
- `#!/usr/bin/tclsh9.0` or `#!/usr/bin/env tclsh9.0` -> `tcl9.0`
- `#!/usr/bin/tclsh9.1` or `#!/usr/bin/env tclsh9.1` -> `tcl9.1`

A plain `#!/usr/bin/tclsh` without a version number does not select a
specific dialect and falls through to the user setting.

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
  server-side detection from source content (comment directive + shebang) for
  editors that do not perform client-side detection. The canonical detection
  itself lives in `rust/tcl-dialect`, so client and server cannot disagree.

## Re-detection triggers

Dialect is re-evaluated when:

- The active editor tab changes.
- A Tcl document is opened.
- Any of the first 5 lines of the active document are edited (covers both
  shebang and comment directive changes).
- The `tclLsp.dialect` setting changes.

## Editor language ID mapping

| Language ID | Dialect |
|-------------|---------|
| `tcl-irule` | `f5-irules` |
| `tcl-iapp` | `f5-iapps` |
| `tcl-bigip` | `f5-bigip` |
| `tcl8.4` | `tcl8.4` |
| `tcl8.5` | `tcl8.5` |
| `tcl9.0` | `tcl9.0` |
| `tcl9.1` | `tcl9.1` |
| `tcl-synopsys` | `synopsys-eda-tcl` |
| `tcl-cadence` | `cadence-eda-tcl` |
| `tcl-xilinx` | `xilinx-eda-tcl` |
| `tcl-quartus` | `intel-quartus-eda-tcl` |
| `tcl-mentor` | `mentor-eda-tcl` |
| `tcl-expect` | `expect` |

## File extension mapping

| Extension | Dialect |
|-----------|---------|
| `.irul`, `.irule` | `f5-irules` |
| `.iapp`, `.iappimpl`, `.impl` | `f5-iapps` |
| `.apl` | `f5-iapps` |
| `.exp` | `expect` |

## Command-line tools (`tcl diag` / `lint` / `validate`)

The diagnostics verbs of the `tcl` CLI apply the same chain per input
document (minus the editor language ID, which has no CLI equivalent):
an explicit `--dialect` flag overrides everything; otherwise the
in-source signals (comment directive, shebang, content), then the file
extension, then the `tcl8.6` fallback decide.  This keeps `tcl diag`
and the editor reporting the same set for the same file — an `.irul`
input gets full iRules analysis without any flag.
