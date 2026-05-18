# KCS: How does tcl-lsp load configuration, and what overrides what?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli

## Question

Where does the Tcl Language Server look for configuration, and when two
places disagree, which one wins?

## Answer

The Tcl Language Server reads configuration from three files and merges
them on every change. The layers are listed below from **least specific
to most specific** — each layer overrides any of the lower layers that
sets the same key:

1. **Global user config — XDG `config.ini`.** A single file in your
   home directory that applies to every workspace you open. Use it for
   your personal defaults.
2. **Project config — `.tcl-lsp.ini` at the workspace root.** A
   per-project INI file committed with the source. Every developer who
   opens the project picks the same rules up, and they survive
   switching editors. Use it for team-wide conventions.
3. **Editor settings (non-default values only).** Whatever the editor
   sends over `workspace/configuration`, typically under the `tclLsp.*`
   namespace (for example `tclLsp.dialect`, `tclLsp.optimiser.O109`).
   VS Code, Neovim, Zed, Helix, Emacs, Sublime Text, and JetBrains
   each have their own way of populating these. Use it for the
   override you want right now, on this machine, in this editor.

The merge is per-key inside each section, so an editor setting that
pins `[optimiser] disabled = O109` still inherits `[optimiser] profile
= readability` from the project or global config.

### Why editor settings only override for non-default values

Editors like VS Code respond to `workspace/configuration` by echoing
back the schema default for every key the user has not explicitly set.
If the server treated the schema default as if it were an explicit
override, every team setting in `.tcl-lsp.ini` would be silently
shadowed by the editor's default. To avoid that, the server only lets
an editor value override the project layer when it differs from the
schema default — an explicit user choice wins, an unset key does not.

This means a project `.tcl-lsp.ini` is the team's authoritative
default, and any developer who wants to override it adds an explicit
non-default value to their editor settings.

In addition to these three configuration layers, two **document-level
directives** apply on top of the merged config for a single file:
inline `# noqa` on the line above a command, and top-of-file
`# tcl-lsp: disable=` for the whole document. Both silence diagnostics
only; they do not change feature toggles, the formatter, or any other
setting. See
[kcs-howto-suppress-diagnostics.md](kcs-howto-suppress-diagnostics.md).

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

### Each file is only read from its own location

The two INI files share a schema, but the server only looks for each
one in its own designated place. They are not interchangeable:

- The **global** `config.ini` is only loaded from the platform-native
  config directory listed above. A `config.ini` dropped into a
  workspace root is ignored.
- The **project** `.tcl-lsp.ini` is only loaded from the workspace
  root the editor opens — the server does not walk upward through
  parent directories, and a `.tcl-lsp.ini` in your home directory is
  ignored.

If you want a setting to follow you everywhere, put it in the global
file. If you want it to travel with the project and apply to everyone
who checks it out, put it in `.tcl-lsp.ini` and commit it.

### Multi-root workspaces (VS Code and others)

VS Code's multi-root workspaces — and the equivalent in Neovim, Zed,
Helix, Emacs, Sublime Text, and JetBrains — open more than one folder
in a single editor window. The server treats each folder as its own
configuration scope:

- **Each folder gets its own project layer.** The server reads a
  `.tcl-lsp.ini` from every folder root independently. A `.tcl-lsp.ini`
  in folder A never applies to files in folder B, even if both folders
  share the same VS Code workspace file.
- **Each folder gets its own editor layer.** When the editor responds
  to `workspace/configuration`, the server pulls one payload per
  folder. VS Code's folder-scoped settings (the **Workspace** and
  **Folder** scopes in the Settings UI) override user-level settings
  on a per-folder basis.
- **The global layer is shared.** `config.ini` is loaded once and
  applies to every folder in the window.
- **Files are matched to their folder by longest URI prefix.** When
  you open a file, the server walks the list of workspace folders and
  picks the one whose URI is the longest prefix of the file's URI.
  That folder's merged settings are the ones that apply.
- **Files outside every folder fall back to the workspace level.** A
  file you open with **File ▸ Open** that does not live under any
  workspace folder uses the workspace-level fallback layers — the
  same as a single-root window with no `.tcl-lsp.ini`.

Adding or removing a workspace folder triggers a re-pull of editor
settings for that scope and a re-load of its `.tcl-lsp.ini`. You do
not need to restart the server when changing workspace folders.

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
