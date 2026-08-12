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

Diagnostic and optimisation pages live under
[`../codes/`](../codes/README.md). The first four categories live at the
top level of [`../`](../README.md) or under
[`../features/`](../features/README.md).

If your content does not fit any of these six categories, you are writing
a design doc. Put it under [`../../design/`](../../design/README.md)
instead.

## Naming notes

Name a KCS file after the question it answers in the reader's own
words, not after the internal class, module, or mechanism behind the
answer. `kcs-issue-lsp-features-are-missing.md` is better than
`kcs-issue-vscode-lsp-startup-logs.md` — the user experiences "nothing
works", not "I should read a log channel". Functionality notes are the
exception and are named after the feature itself. See rule 10 in
[`../STYLE.md`](../STYLE.md) for worked examples.

## Shared skeleton

Every KCS note, regardless of category, has the same top:

```markdown
# KCS: <short title>

> **Audience:** User | Contributor | Maintainer
> **Type:** Issue | Q&A | How-To | Functionality | Diagnostic | Optimisation

## Question

<One sentence — the single core question this note answers.>

## Answer

<Plain-English answer. Link complex terms to ../GLOSSARY.md.>

## Related

- [KCS index](../README.md)
- [Glossary](../../GLOSSARY.md)
```

The category-specific templates add sections around this skeleton — for
example, Issue adds a `## Symptoms` section, and Functionality adds
`## Summary` and `## Surface` sections (which the help tool parses at
runtime).

## Design-doc templates

Templates for contracts, data-structure references, and ownership matrices
now live at [`../../design/templates/`](../../design/templates/). Those
are for technical documentation, not KCS notes.
