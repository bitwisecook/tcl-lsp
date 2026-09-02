# KCS: How do I author a `.sslictcl` TLS declaration in my editor?

> **Audience:** User
> **Type:** How-To

## Applies to

all-editors, tcl-lsp CLI

## Question

I have a `.sslictcl` file describing a deployment's certificates, endpoints,
and assurance policy. How do I get the editor to help me write it, and how do
I read what it tells me?

## Before you start

- Install the tcl-lsp extension for your editor
  ([INSTALL-editors.md](../../INSTALL-editors.md)).
- Name the file with a `.sslictcl` extension, or start it with the
  `sslictcl 1` header. Either is enough — the server also recognises the
  editor's `sslictcl` language id and a `# tcl-dialect: sslictcl` comment.

## Answer

A `.sslictcl` document is written in Tcl **syntax** and is **never
evaluated**. The loader walks the syntax tree and starts no interpreter, so
nothing in the file can run. Every editor feature below is driven from the
declared vocabulary rather than from anything the file does.

1. **Open the file.** Confirm the editor reports the language as
   `sslictcl`. If it says plain Tcl, the file has neither the extension nor
   the header — add one.
2. **Write a declaration.** Completion at the top level offers exactly the
   declaration words (`certificate`, `endpoint`, `policy`, `trust-program`,
   …) and nothing from core Tcl, because nothing from core Tcl can be
   written there.
3. **Open a block.** Inside a block, completion offers exactly that block's
   members — `hsts { … }` offers `enabled`, `max-age`,
   `include-subdomains`, and `preload`, and no more. Hover and signature
   help show the vocabulary's own documentation for the word under the
   cursor.
4. **Read the diagnostics.** The loader reports its own codes, ranged over
   the exact word at fault: `SSLIC1001`–`SSLIC1012` are errors, and
   `SSLIC1101`–`SSLIC1103` are notices. Turn any one off with
   `tclLsp.diagnostics.<CODE>: false`, the same switch every other code
   uses.
5. **Navigate.** The outline lists the declarations — `endpoint
   /Common/www`, `policy corporate` — with each block's members nested
   beneath it, and every block folds.

### An unrecognised word is not an unknown command

An open block preserves a word the vocabulary does not know and reports
`SSLIC1101`; a closed block rejects one and reports `SSLIC1007`. Either
way the loader owns that verdict, so you will not also see the ordinary
`W123 Unknown command` warning on the same word, and nothing will offer to
rewrite a deliberately-preserved extension into a declaration.

### Outside the editor

`tcl diag path/to/file.sslictcl` reports the same set from the command
line, which is useful in a pre-commit hook or in continuous integration.

## How to tell it worked

Typing an unknown word at the top level draws one `SSLIC1101` notice on that
word — one hint, not two — and typing inside `hsts { }` offers four
completions rather than the whole of core Tcl.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [SslicTcl vocabulary](../design/sslictcl-vocabulary.md) — the declarations,
  the value domains, the open/closed block rule, and the diagnostic codes.
- [A pack-claimed file extension opens as plain text](kcs-issue-a-pack-claimed-file-extension-opens-as-plain-text.md)
