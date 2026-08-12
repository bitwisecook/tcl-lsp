# KCS: How do I turn a diagnostic, optimisation, or shimmer off?

> **Audience:** User
> **Type:** How-To

## Applies to

all-editors, diagnostic, warning, optimisation

## Question

How do I silence a specific diagnostic, warning, optimisation, or
shimmer report — on one line, in one file, for one project, for one
editor, or everywhere?

## Before you start

- Know the code you want to silence (for example `W100`, `O111`,
  `S101`, `T102`, `IRULE1005`). The full catalogue lives under
  [`docs/kcs/codes/`](codes/README.md) — one page per code.
- Decide **where** the silencing should apply. Smaller scope is always
  better: turning a code off globally hides real problems in future
  projects.

## Answer

The Tcl Language Server gives you five places to turn a code off. Pick
the **smallest scope** that solves your problem. The list below runs
from most specific to least specific, which is also the precedence
order: a match found higher in the list always wins.

### 1. One command — inline `# noqa`

Put the directive on the line **before** the command you want to
silence. Bare `# noqa` silences every code on the next command;
`# noqa: CODE1,CODE2` silences only the listed codes; `# noqa: *`
is the explicit wildcard form.

```tcl
# noqa: W100
expr $x + 1
```

Use this when exactly one command needs the exception and the reason
is local — for example, a deliberate `eval` that the analyser cannot
prove safe.

### 2. One file — top-of-file `# tcl-lsp: disable=`

Put the directive at the top of the file, before any command. Blank
lines, shebangs, and copyright comments are allowed above it; the
scanner stops at the first real command.

```tcl
#!/usr/bin/env tclsh
# Copyright 2026 Example Corp.
# tcl-lsp: disable=W100,O111

package require Tcl 8.6
expr $x + 1
```

Use this for generated files, test fixtures, or legacy code where a
whole-file exception is clearer than annotating every line.

### 3. One project — `.tcl-lsp.ini` at the workspace root

Create a file named `.tcl-lsp.ini` next to the top-level folder of
your project and commit it with the source. The format matches the
[global config file](../design/contracts/xdg-config.md):

```ini
[diagnostics]
disabled = W111, IRULE1005

[optimiser]
disabled = O109

[shimmer]
enabled = false
```

Use this for team-wide conventions — every developer who opens the
project picks the same rules up automatically, and the rules survive
switching editors.

### 4. One editor — `settings.json` or equivalent

Each editor ships its own way to send settings to the server. See the
[configuration reference](../design/contracts/xdg-config.md#interaction-with-editor-settings)
for the exact key path for VS Code, Neovim, Zed, Helix, Emacs, Sublime
Text, and JetBrains.

Use this for personal preference on a single machine — for example,
disabling a hint style you find distracting while pairing with a team
project that does not take a stance on it.

### 5. Everywhere — global config file

Create or edit the [platform-native global config file](../design/contracts/xdg-config.md#file-location):

```ini
[diagnostics]
disabled = W111
```

Use this sparingly — rules turned off here follow you into every
project you open.

### Precedence, from highest priority to lowest

When two layers disagree, the more specific layer wins. From highest
to lowest:

1. Inline `# noqa` on the command.
2. Top-of-file `# tcl-lsp: disable=`.
3. Project `.tcl-lsp.ini` at the workspace root.
4. Editor settings (for example VS Code `tclLsp.diagnostics.*`).
5. Global `~/.config/tcl-lsp/config.ini` (or the platform equivalent).

Inline and file-level directives can only **turn codes off** for the
document they appear in. Project, editor, and global config can both
turn codes off and turn them back on — a project config that enables
a code overrules an editor or global config that disables it.

### Which codes can I disable?

Every code the server emits has a page under
[`docs/kcs/codes/`](codes/README.md). The six families are:

- **Errors (E)** and **warnings (W)** — general diagnostics, including
  security and style checks; see the
  [diagnostics feature page](features/kcs-feature-diagnostics.md).
- **Shimmer (S)** — type-instability warnings.
- **Taint (T)** — tainted-variable flow checks.
- **iRule checks (IRULE)** — F5 iRule–specific diagnostics.
- **Optimisations (O)** — automatic rewrite suggestions; see the
  [optimiser pages](codes/README.md).

Every one of these codes is a valid name in a `# noqa`, a file
directive, or a config file.

## How to tell it worked

- For inline and file-level directives: the squiggle or hint
  disappears as soon as you save. No server restart needed.
- For project config changes: save the `.tcl-lsp.ini` file and run
  **Tcl: Restart Language Server** (or the equivalent for your
  editor) so the server reloads it. New documents opened after the
  restart pick the rules up automatically.
- For editor or global config changes: most editors re-send settings
  to the server within a second; diagnostics refresh automatically.
  When in doubt, see
  [kcs-qa-when-to-restart-server.md](kcs-qa-when-to-restart-server.md).

## Related

- [KCS index](README.md)
- [Per-code catalogue](codes/README.md)
- [Global config file reference](../design/contracts/xdg-config.md)
- [Diagnostics feature page](features/kcs-feature-diagnostics.md)
- [Glossary](../GLOSSARY.md)
