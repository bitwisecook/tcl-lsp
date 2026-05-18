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
2. **Editor settings.** Whatever the editor sends over
   `workspace/configuration`, typically under the `tclLsp.*` namespace
   (for example `tclLsp.dialect`, `tclLsp.optimiser.O109`). VS Code,
   Neovim, Zed, Helix, Emacs, Sublime Text, and JetBrains each have
   their own way of populating these. Use it for personal overrides on
   one machine.
3. **Project config — `.tcl-lsp.ini` at the workspace root.** A
   per-project INI file committed with the source. Every developer who
   opens the project picks the same rules up, and they survive
   switching editors. Use it for team-wide conventions — and because
   it sits at the top of the precedence chain, project config wins
   even when a developer has a conflicting setting in their editor.

The merge is per-key inside each section, so a project config that
pins `[optimiser] disabled = O109` still inherits `[optimiser] profile
= readability` from the editor or global config.

### Why project config wins over editor settings

We copied this rule from **Pyright** (the Python language server
behind Pylance), which silently overrides `python.analysis.*` editor
settings whenever `pyrightconfig.json` or `pyproject.toml` is present.
The TypeScript language server, ESLint, and clangd all follow the
same convention for the same reason: the file checked into source
control is authoritative, so what runs in CI agrees with what runs in
everyone's editor. We surveyed how other mature language servers
handle this and chose the convention with the fewest surprises — see
[`docs/design/contracts/config-precedence.md`](../design/contracts/config-precedence.md)
for the full rationale, the survey, and a list of which specific
behaviours we copied from which tool.

If you want a personal override of a setting your team has pinned in
`.tcl-lsp.ini`, the project file is the wrong layer to fight. Edit
`.tcl-lsp.ini` itself (and discuss the change with the team), or set
the override in your editor and add the same key to the project file's
local-only ignore list.

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

The two filenames are **deliberately different**. We could have used
`tcl-lsp.ini` (or `config.ini`) for both, but distinct names mean a
user who copies one file to the other location does not silently
change which precedence layer it applies to. If you `cp ~/.config/tcl-lsp/config.ini
./` thinking it will pin a project-level rule, the file will simply
not be picked up — instead of being read at a higher priority than
you expected and overriding a teammate's setting. The same applies in
the other direction: dropping `.tcl-lsp.ini` into your home config
directory is a no-op rather than a stealth promotion to global.

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

### How to figure out where a setting is coming from

When the analyser does something you did not expect for a specific
file, trace it back through the four places it could be coming from,
in this order:

**1. Ask the server for the resolved value.** The server exposes a
`tcl-lsp.getEffectiveConfig` command over `workspace/executeCommand`.
Pass the URI of the file you are looking at and it returns the
resolved `dialect`, `extra_commands`, `library_paths`, `line_length`,
and the URI of the workspace folder the file matched. This is the
single most reliable way to see what the server is actually applying:

```jsonc
// VS Code: open the Command Palette and run "Developer: Inspect
// Context Keys", or run from a script:
await vscode.commands.executeCommand(
  "tcl-lsp.getEffectiveConfig",
  vscode.window.activeTextEditor.document.uri.toString()
);
```

Other editors expose `workspace/executeCommand` through their LSP
client API — the Neovim, Emacs, and Sublime Text READMEs each show
the local invocation.

**2. Read the server log channel.** Open the **Tcl Language Server**
output channel in VS Code (or your editor's equivalent). On every
load and every config change the server logs `Loaded user config
from <path>` and `Loaded project config from <path>`. If a `.tcl-lsp.ini`
you expected to apply is missing from the log, the server did not
find it — check the workspace folder and the file name.

**3. Walk the layers manually, top-down.** If the resolved value is
still surprising, check each source from highest priority to lowest:

1. **Project file** at `<workspace-folder>/.tcl-lsp.ini`. Open it and
   look for the section and key. In a multi-root workspace, check
   the folder the file you are looking at belongs to — not any other
   folder.
2. **Editor settings.** In VS Code, run **Preferences: Open Workspace
   Settings (JSON)** and **Preferences: Open User Settings (JSON)**
   and search for `tclLsp`. The Settings UI also shows where each
   value comes from with the small **User / Workspace / Folder**
   chip next to each setting.
3. **Global file.** Open the platform-native `config.ini` (see
   "Where the global config lives" above). A typo in a section name
   or a key turns into a silent no-op.
4. **Document-level directives.** Scan the top of the file you are
   looking at for `# tcl-lsp: disable=` or `# tcl-dialect:`, and the
   line above each diagnostic for `# noqa`. These only affect
   diagnostics and dialect, but they explain unexpected suppression.

**4. If nothing matches, the value is the built-in default.** Every
setting has a default that applies when no layer sets it; those
defaults live in the source ship docs alongside each feature. Run
**Tcl: Export Settings to Config File** (or `tcl-lsp.exportConfig`)
to dump the *current* effective settings to your global config file
as an anchor for what is in play.

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
