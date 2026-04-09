# KCS templates

Use these templates when adding a KCS note. A KCS note always fits one of
four categories. Pick the category first, then copy the matching template
and fill it in.

The rules for writing KCS notes live in [`../STYLE.md`](../STYLE.md). The
short-form checklist lives in [`AGENTS.md`](../../../AGENTS.md). Templates
here encode the structure; the style guide encodes the tone and language
rules.

## The four categories

| Type | Use for | Template |
|---|---|---|
| **Issue** | A user hits a problem, wants the fix. | [`kcs-template-issue.md`](kcs-template-issue.md) |
| **Q&A** | A single question with a short, plain answer. | [`kcs-template-qa.md`](kcs-template-qa.md) |
| **How-To** | Task-oriented steps a user or contributor follows. | [`kcs-template-how-to.md`](kcs-template-how-to.md) |
| **Functionality** | A command, feature, or tool and how to use it. | [`kcs-template-functionality.md`](kcs-template-functionality.md) |

If your content does not fit any of these four categories, you are writing
a design doc. Put it under [`../../design/`](../../design/README.md)
instead.

## Shared skeleton

Every KCS note, regardless of category, has the same top:

```markdown
# KCS: <short title>

> **Audience:** User | Contributor | Maintainer
> **Type:** Issue | Q&A | How-To | Functionality

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
