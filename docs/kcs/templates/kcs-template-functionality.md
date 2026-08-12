# KCS: feature — <Feature Name>

> **Audience:** User
> **Type:** Functionality

## Summary

<One-line description of what this command, feature, or tool does. This
is the summary column of the help database, which `rust/tcl-cli/build.rs`
builds from this directory at compile time and `tcl help` queries, so
keep it short and plain. The title line above must keep the exact
`# KCS: feature — <name>` shape, em dash included, or the note is
skipped by the build.>

## Applies to

<Comma-separated plain-text list — not bullet points. Include, in this
order:

- the editors and tools the feature runs in, for example: VS Code, Zed,
  JetBrains, Neovim, Helix, Emacs, Sublime Text, tcl-lsp CLI, MCP,
  Claude skill. Use the shorthand `all-editors` when it runs in every
  LSP editor; the build script expands it automatically;
- a content tag naming what kind of thing this is — `diagnostic`,
  `warning`, `optimisation`, `refactoring`, `analyser`, or `transform`;
- a compiler-pass tag, if the feature reads compiler facts directly.
  Name the pass whose facts it consumes, for example `ssa`, `sccp`,
  `taint`, or `lowering`.

The full tag vocabulary lives in [`../STYLE.md`](../STYLE.md) (rule 11).
Nothing validates these tags at build time — an unrecognised one is
indexed silently — so check each against those tables.>

## Question

What does <feature name> do, and how do I use it?

## How to use

<Plain, single-paragraph instructions if the feature works the same
everywhere. Only when the editors and tools in "Applies to" genuinely
differ — different menu names, keybindings, or command syntax — split
into one sub-heading per editor or tool, in the same order as the
"Applies to" line. Delete the stub sub-headings below when the steps are
identical everywhere; three near-empty sub-headings are worse than one
paragraph.>

### VS Code

<VS Code-specific instructions using the exact command palette entry
or menu label.>

### Zed

<Zed-specific instructions.>

### tcl-lsp CLI

<Command-line invocation.>

## Options

<Only if the feature has settings or flags. Give the exact key as the
user types it, and check each one against the extension's declared
settings in `editors/vscode/package.json` — or, for a CLI flag, against
the command's own argument parser. Delete this section when the feature
has no options.>

- `<setting key or flag>` — <what it controls, with the default value>.
- `<setting key or flag>` — <what it controls>.

## Example

Every Functionality note must include at least one concrete example.
Pick whichever form shows the feature best; combine them when more
than one form helps:

- Before / after code blocks, for transforms (refactor, format,
  minify, optimise, unminify).
- A short code snippet plus a plain-English pointer to what the user
  sees on which token, line, or range, for analysers (diagnostics,
  hover, completions, inlay hints, signature help, semantic tokens).
- A screenshot from [`../screenshots/`](../screenshots/) with a short
  caption, for panels, webviews, and visual features (compiler explorer,
  call hierarchy, debugger).

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
