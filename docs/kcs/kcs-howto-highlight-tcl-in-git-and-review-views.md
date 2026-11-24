# KCS: How do I get Tcl highlighting in Git history and merge-request views?

> **Audience:** User
> **Type:** How-To

## Applies to

VS Code, semantic highlighting, navigation

## Question

How do I get full Tcl highlighting when I open a `.tcl` file from the
**Source Control** graph, an older Git revision, or a merge-request diff
from another extension?

## Before you start

- Use VS Code. This setting belongs to the VS Code extension.
- Know the URI scheme the other extension uses. Built-in Git uses `git`.
  GitLab Workflow uses `gl-review` for merge-request diffs and
  `gitlab-remote` for remote files. For other extensions, check their
  documentation.

## Answer

By default the Tcl Language Server only attaches to files in your
workspace (`file`) and new unsaved files (`untitled`), plus Git
revisions (`git`). Add any other scheme to `tclLsp.viewOnlySchemes`:

```json
"tclLsp.viewOnlySchemes": ["git", "gl-review", "gitlab-remote"]
```

Reload the window when VS Code asks; the list is read when the language
client starts.

Files opened on these schemes are treated as read-only views of code
that is not part of your workspace:

- They get semantic highlighting and hover.
- They get no diagnostics, so an old revision never fills the
  **Problems** panel.
- Go to definition, find references, rename, workspace symbols, and call
  hierarchy from your workspace files never land in them, so an open
  revision does not show up as a duplicate result. Inside a view,
  results in that same view are kept.

`file` and `untitled` are always your workspace and are ignored in this
list. Remove `git` from the list to go back to plain TextMate colouring
for Git revisions.

## How to tell it worked

Open an older revision of a `.tcl` file from the **Source Control**
graph. Procedure names and variables are coloured like in the working
file, and **Find All References** on a proc in the working file lists
only workspace locations.

## Related

- [KCS index](README.md)
- [How do I hide Tcl diagnostics in diff and compare views?](kcs-howto-hide-diagnostics-in-diff-views.md)
- [Glossary](../GLOSSARY.md)
