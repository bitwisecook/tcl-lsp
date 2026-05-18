# KCS: How does tcl-lsp load configuration, and what overrides what?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli

## Question

Where does the Tcl Language Server look for configuration, and when two
places disagree, which one wins?

## Answer

The Tcl Language Server reads configuration from up to five places. They
are merged on every change, with more specific layers overriding less
specific ones. From **highest priority to lowest**:

1. **Inline `# noqa` on the command above the line.** Silences one or
   more diagnostic codes on the very next command. See
   [kcs-howto-suppress-diagnostics.md](kcs-howto-suppress-diagnostics.md).
2. **Top-of-file `# tcl-lsp: disable=` directive.** Silences codes for
   the whole document. The `# tcl-dialect: tcl8.4` directive lives in
   the same header block and pins the dialect for that one file.
3. **Project config — `.tcl-lsp.ini` at the workspace root.** A
   per-project INI file committed with the source. Same schema as the
   global config. Every developer who opens the project picks the same
   rules up, and they survive switching editors.
4. **Editor settings.** Whatever the editor sends over
   `workspace/configuration`, typically under the `tclLsp.*` namespace
   (for example `tclLsp.dialect`, `tclLsp.optimiser.O109`). VS Code,
   Neovim, Zed, Helix, Emacs, Sublime Text, and JetBrains each have
   their own way of populating these.
5. **Global user config — XDG `config.ini`.** A single file in your
   home directory that applies to every workspace you open.

When two layers set the same key, the higher-numbered layer wins. The
merge is per-key inside each section, so a project config that sets
`[optimiser] disabled = O109` still inherits `[optimiser] profile =
readability` from the global config.

### Where the global config lives

The path follows platform conventions, and `$XDG_CONFIG_HOME` always
takes precedence when set:

- **Linux, BSD, and WSL2:** `~/.config/tcl-lsp/config.ini`
- **macOS:** `~/Library/Application Support/tcl-lsp/config.ini`
- **Windows (native):** `%APPDATA%\tcl-lsp\config.ini`
- **MSYS2 and Cygwin:** `~/.config/tcl-lsp/config.ini`

Both the global file and the project `.tcl-lsp.ini` use the same INI
schema. The recognised sections today are `[diagnostics]`,
`[optimiser]`, `[shimmer]`, `[xcDiagnostics]`, `[features]`,
`[formatting]`, and `[style]`. Keys outside a known section are
ignored.

### Dialect is special

For a single document, the dialect is chosen by its own priority chain
documented in
[dialect-detection.md](../design/contracts/dialect-detection.md):

1. `# tcl-dialect:` directive in the first five lines of the file.
2. File-name and extension auto-detection (for example `.irul` files
   pick `f5-irules`).
3. The user setting — `tclLsp.dialect` from editor settings or the
   global `config.ini`.

So a `# tcl-dialect:` comment overrides any config file, and a recognised
file extension overrides the user setting. Use the config file to choose
the **default** dialect for files that have neither.

### When changes take effect

Inline `# noqa` and the top-of-file directive apply as soon as you save
the document. Editor-settings changes are picked up within a second on
every editor that supports `workspace/configuration`. Project and global
config files are re-read on server start; after editing one, run **Tcl:
Restart Language Server** (or the equivalent for your editor) to pick
the new values up. See
[kcs-qa-when-to-restart-server.md](kcs-qa-when-to-restart-server.md) for
the full list of changes that need a restart.

## Related

- [KCS index](README.md)
- [How do I turn a diagnostic, optimisation, or shimmer off?](kcs-howto-suppress-diagnostics.md)
- [When should I restart the server?](kcs-qa-when-to-restart-server.md)
- [Dialect detection contract](../design/contracts/dialect-detection.md)
- [Glossary](../GLOSSARY.md)
