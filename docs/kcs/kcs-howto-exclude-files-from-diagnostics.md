# KCS: How do I turn off all diagnostics for certain files?

> **Audience:** User
> **Type:** How-To

## Applies to

all-editors, diagnostic, warning

## Question

How do I turn off all diagnostics for certain files — for example,
documentation files full of virtual procs that the analyser would
otherwise lint?

## Before you start

- Know whether the files you want excluded share a name pattern (any
  `.ruff` file, anywhere) or a path pattern (everything under
  `docs/**` in this project).
- Have a project `.tcl-lsp.ini` or a global `config.ini` open. See
  [what config sections are valid](kcs-qa-what-config-sections-are-valid.md)
  if you have not created one yet.

## Answer

Add an `exclude` key to the `[diagnostics]` section. It takes one glob
pattern per line — not a comma-separated list, because pattern
alternation already uses commas inside braces:

```ini
[diagnostics]
exclude =
    docs/**
    generated/[a-c]*.tcl
    *.ruff
```

This flows through configuration as `tclLsp.diagnostics.exclude`, so
it follows the usual [precedence](kcs-qa-how-tcl-lsp-loads-configuration.md):
the global `config.ini` sets a baseline, editor settings replace it,
and a project `.tcl-lsp.ini` replaces both. The lists do not merge —
the highest layer that sets `exclude` wins whole. In a multi-root
workspace, each folder's own `.tcl-lsp.ini` governs its own files: a
folder that sets `exclude` replaces the inherited list for that
folder, and a folder that does not set it keeps inheriting the global
list.

### Pattern syntax

| Pattern piece | Meaning |
|---|---|
| `*` | Any run of characters within one path segment |
| `?` | Exactly one character |
| `**` (whole segment) | Any number of segments, including none |
| `[a-c]` | A character class, with ranges |
| `[!...]` or `[^...]` | A negated character class |
| `{a,b}` | Alternation, and alternatives may nest |
| `\` | Escapes the next character |

Matching is case-sensitive, and a pattern always uses `/` as the
separator, even on Windows.

### Path patterns vs. name patterns

- A pattern that contains `/` matches against the file's path
  **relative to its workspace folder root** — `docs/**` matches
  `docs/anything`, but not a `docs` folder nested somewhere else.
- A pattern with no `/` matches the file's **name at any depth**, the
  same way a `.gitignore` name pattern does — `*.ruff` excludes every
  `.ruff` file in the workspace, regardless of which folder it is in.
- A trailing `/` excludes a directory's whole tree: `vendor/` is the
  same as `vendor/**`.
- A file that is outside every workspace folder has no folder root to
  build a relative path from, so only name patterns can match it.

### What stays on

An excluded file still gets everything else the server does: it is
still indexed, and hover, completion, navigation, references, semantic
tokens, and formatting all keep working. Only diagnostics are
suppressed, and the suppression is total — every code, not a subset.

## How to tell it worked

Existing squiggles on the file clear immediately. The server logs a
line for each excluded document:

```
[timing] diagnostics excluded 0ms (uri=..., diags=0)
```

Saving `.tcl-lsp.ini` re-applies the config on its own — the server
watches the file, so no restart is needed. Removing a file from the
list, or editing its pattern so the file no longer matches, brings its
diagnostics back on the next re-analysis.

## Related

- [KCS index](README.md)
- [How do I turn a diagnostic, optimisation, or shimmer off?](kcs-howto-suppress-diagnostics.md)
- [What sections and keys are valid in tcl-lsp config files?](kcs-qa-what-config-sections-are-valid.md)
- [How does tcl-lsp load configuration, and what overrides what?](kcs-qa-how-tcl-lsp-loads-configuration.md)
- [Glossary](../GLOSSARY.md)
