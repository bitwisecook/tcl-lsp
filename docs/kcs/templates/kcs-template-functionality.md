# KCS: feature — <Feature Name>

> **Audience:** User
> **Type:** Functionality

## Summary

<One-line description of what this command, feature, or tool does. This
line is read at runtime by the `help` tool and the feature catalogue, so
keep it short and plain.>

## Applies to

<Comma-separated plain-text list of the editors and tools this feature
is available in, for example: VS Code, Zed, JetBrains, Neovim, Helix,
Emacs, Sublime Text, tcl-lsp CLI, MCP, Claude skill. Do not use bullet
points. Use the shorthand `all-editors` when the feature runs in every
LSP editor; the build script expands it automatically. The full tag
vocabulary lives in [`../STYLE.md`](../STYLE.md) (rule 11).>

## Question

What does <feature name> do, and how do I use it?

## How to use

<Plain, single-paragraph instructions if the feature works the same
everywhere. If the editors and tools in "Applies to" have different
steps — different menu names, keybindings, or command syntax — use one
sub-heading per editor or tool as shown below, in the same order as
the "Applies to" line.>

### VS Code

<VS Code-specific instructions using the exact command palette entry
or menu label.>

### Zed

<Zed-specific instructions.>

### tcl-lsp CLI

<Command-line invocation.>

## Options

- `<option name>` — <what it controls, with the default value>.
- `<option name>` — <what it controls>.

## Example

Every Functionality note must include at least one concrete example.
Pick whichever form shows the feature best; combine them when more
than one form helps:

- Before / after code blocks, for transforms (refactor, format,
  minify, optimise, unminify).
- A short code snippet plus a plain-English pointer to what the user
  sees on which token, line, or range, for analysers (diagnostics,
  hover, completions, inlay hints, signature help, semantic tokens).
- A screenshot from `../screenshots/` with a short caption, for
  panels, webviews, and visual features (compiler explorer, call
  hierarchy, debugger).

Keep each example short enough to fit on one screen.

<!-- The stub below shows a before/after transform. Replace or extend
     with whichever form fits your feature. -->

### Before

```tcl
# Input — keep it to one screen.
```

### After

```tcl
# Rewritten output.
```

## Related

- [KCS feature index](README.md)
- [Glossary](../../GLOSSARY.md)
- <other KCS notes on the same topic>
