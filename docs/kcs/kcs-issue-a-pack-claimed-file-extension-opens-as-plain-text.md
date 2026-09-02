# KCS: A file extension my SpecTcl pack claims opens as plain text

> **Audience:** User
> **Type:** Issue

## Applies to

VS Code, JetBrains, Zed, Sublime Text, Neovim, Helix, Emacs

## Question

My pack has a `file_extension` row for `.irulex`, but opening a `.irulex`
file gives me plain text with no highlighting and no language server — how do
I get the editor to treat it as Tcl?

## Symptoms

- The file opens with no highlighting, and the editor's language indicator
  says **Plain Text** rather than Tcl or iRule.
- No diagnostics, completion, or hover appear in that file, while `.tcl`
  files in the same project are fine.
- `tcl spec check` reports the pack loaded, and the commands it declares are
  recognised in a `.tcl` file — so the pack itself is being read.

## Answer

The server routes a pack-claimed extension the moment it discovers the pack.
The editor is a step behind: it learns which extensions are Tcl from a static
manifest shipped with the extension or plugin, written long before your pack
existed. Some editors can be told at runtime and some cannot, so the fix
differs.

### VS Code

Nothing to do — the extension writes a workspace `files.associations` entry
for every extension your packs claim, and flips an already-open file onto the
Tcl language when the entry appears. If the file is still plain text:

1. Save the `.tclspec` file. Registration follows the reload the save
   triggers.
2. Check the **Tcl Language Server** output channel for a `[packs] file
   associations now:` line naming your extension.
3. If an earlier entry for the same pattern exists in
   `.vscode/settings.json` under `files.associations`, the extension leaves it
   alone — a mapping you made by hand always wins. Remove it to hand the
   extension back the pattern.

### JetBrains

Nothing to do — the plugin registers the claimed extensions with the IDE's
file types while their packs are loaded, and retires them when the packs go.
`tcl-irule` extensions become the **iRule** file type; everything else
becomes **Tcl**. If the file is still plain text:

1. Save the `.tclspec` file, and give the server a moment to reload.
2. Check **Settings → Editor → File Types** for your extension under **Tcl**
   or **iRule**.
3. If it appears under some other file type, that mapping is yours and the
   plugin will not touch it — remove it there and the plugin claims the
   extension on the next reload.

Two IDE-wide behaviours are worth knowing. Associations are shared by every
open project, so the plugin registers what all of them claim together and
drops an extension only when no open project claims it any more; a project
whose server has not started yet claims nothing, so just after IDE startup an
extension can briefly disappear and return. And nothing is removed when the
IDE exits, so if you delete a pack while the IDE is closed, its extension is
retired the first time a server reports in the next session.

### Zed, Sublime Text, Neovim, Helix, Emacs

These editors take their file-type mapping from a static configuration file,
so add the extension yourself:

1. **Zed** — add the suffix to `file_types` for the Tcl language in your
   `settings.json`.
2. **Sublime Text** — open a file of that type and use **View → Syntax →
   Open all with current extension as… → Tcl**.
3. **Neovim, Helix, Emacs** — add the extension to the filetype or
   language-configuration mapping you already use for `.tcl`.

The server does the rest: once the file arrives as Tcl, the pack's own
`-dialect` decides which dialect it is analysed as, exactly as it does for a
built-in extension.

## Related

- [How do I write a SpecTcl pack?](kcs-howto-write-a-tclspec-pack.md)
- [Dialect selection](features/kcs-feature-dialect-selection.md)
- [SpecTcl pack design](../design/spec-packs.md)
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
