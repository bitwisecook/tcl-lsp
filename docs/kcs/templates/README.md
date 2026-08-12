# KCS templates

Use these templates when adding a KCS note. A KCS note always fits one
of six categories. Pick the category first, then copy the matching
template and fill it in.

The rules for writing KCS notes live in [`../STYLE.md`](../STYLE.md). The
short-form checklist lives in [`AGENTS.md`](../../../AGENTS.md). Templates
here encode the structure; the style guide encodes the tone and language
rules.

## The six categories

| Type | Use for | Template | Filename shape |
|---|---|---|---|
| **Issue** | A user hits a problem, wants the fix. | [`kcs-template-issue.md`](kcs-template-issue.md) | `kcs-issue-<problem-in-plain-words>.md` |
| **Q&A** | A single question with a short, plain answer. | [`kcs-template-qa.md`](kcs-template-qa.md) | `kcs-qa-<the-question>.md` |
| **How-To** | Task-oriented steps a user or contributor follows. | [`kcs-template-how-to.md`](kcs-template-how-to.md) | `kcs-howto-<the-task>.md` |
| **Functionality** | A command, feature, or tool and how to use it. | [`kcs-template-functionality.md`](kcs-template-functionality.md) | `kcs-feature-<feature-name>.md` |
| **Diagnostic** | A per-code page for an E/W/S/T/IRULE diagnostic. | [`kcs-template-diagnostic.md`](kcs-template-diagnostic.md) | `kcs-diagnostic-<code>-<plain-words>.md` |
| **Optimisation** | A per-code page for an O-code optimiser rewrite. | [`kcs-template-optimisation.md`](kcs-template-optimisation.md) | `kcs-optimisation-<code>-<plain-words>.md` |

If your content does not fit any of these six categories, you are writing
a design doc. Put it under [`../../design/`](../../design/README.md)
instead.

## Where each category is filed

| Category | Directory | Index it must be linked from |
|---|---|---|
| Issue, Q&A, How-To | [`../`](../README.md) — the top level of `docs/kcs/` | [`../README.md`](../README.md) |
| Functionality | [`../features/`](../features/README.md) | [`../features/README.md`](../features/README.md) |
| Diagnostic, Optimisation | [`../codes/`](../codes/README.md) | — (per-code pages are not index-checked) |

`cargo xtask kcs-index-links` fails the build when a top-level note or a
feature note is missing from its index, and when any relative link in
`docs/` does not resolve.

## Naming notes

Name a KCS file after the question it answers in the reader's own
words, not after the internal class, module, or mechanism behind the
answer. `kcs-issue-lsp-features-are-missing.md` is better than
`kcs-issue-vscode-lsp-startup-logs.md` — the user experiences "nothing
works", not "I should read a log channel". Functionality, diagnostic,
and optimisation notes are the exception and are named after their
stable identifier (the feature name or the code), with a plain-English
tail: `kcs-diagnostic-w210-variable-read-before-set.md`. The code prefix
is always **lowercase** in the filename, and uppercase in prose and
headings. See rule 10 in [`../STYLE.md`](../STYLE.md) for worked
examples.

## Shared skeleton

Every KCS note, regardless of category, starts the same way:

```markdown
# KCS: <short title>

> **Audience:** <one of: User, Contributor, Maintainer>
> **Type:** <one of: Issue, Q&A, How-To, Functionality, Diagnostic, Optimisation>

## Applies to

<Comma-separated plain-text list of tags — not bullet points.>
```

Pick exactly one audience and exactly one type; the six type names above
are the only valid values (rule 2 and the category table in
[`../STYLE.md`](../STYLE.md)). `## Applies to` comes immediately after
the header, with one exception: a Functionality note puts its one-line
`## Summary` first, because that line is the summary column of the help
database.

Issue, Q&A, and How-To notes then carry `## Question` and `## Answer`.
The other three categories answer in sections of their own:
Functionality uses `## How to use`, Diagnostic uses `## Why` and
`## Fix`, and Optimisation uses `## Before` and `## After`. Every
category ends with `## Related`.

## What is machine-read

`rust/tcl-cli/build.rs` builds the embedded help database at compile
time from `../features/kcs-feature-*.md` only — top-level notes and
per-code pages are not indexed. From each feature note it reads the
title line (which must match `# KCS: feature — <Feature Name>` exactly,
em dash included), then the `## Summary`, `## Applies to`,
`## How to use`, and `## Example` / `## Examples` sections. Screenshots
are skipped. `tcl help` queries the result.

`parse_applies_to` in the same file lowercases each tag and replaces
internal spaces with a hyphen, so `VS Code` and `vs-code` are one tag.
It validates nothing: an unrecognised tag is indexed rather than
rejected, so keeping the vocabulary honest is a review job. The tables
in rule 11 of [`../STYLE.md`](../STYLE.md) are the canonical list.

## Links in the finished note

Spell the `## Related` links for the directory the note lands in, not
for this one:

| Note lives in | KCS index | Glossary |
|---|---|---|
| `docs/kcs/` | `README.md` | `../GLOSSARY.md` |
| `docs/kcs/features/` | `README.md` | `../../GLOSSARY.md` |
| `docs/kcs/codes/` | `README.md` | `../../GLOSSARY.md` |

The Issue, Q&A, and How-To templates sit one level deeper than the notes
written from them, so their own links are spelled `../README.md` and
`../../GLOSSARY.md`. Re-spell both when you save the note.

## Design-doc templates

Templates for contracts, data-structure references, and ownership matrices
live at [`../../design/templates/`](../../design/templates/README.md). Those
are for technical documentation, not KCS notes.
