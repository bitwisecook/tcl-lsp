# KCS: My spec studio edit does not appear in the Pack DSL pane

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors

## Question

I edited a command in the spec studio — or imported a package and pressed
**Add all N to the pack** — the studio said it wrote, and the Pack DSL pane
still shows the document exactly as it was. Where did the edit go?

## Symptoms

- The status line says the command was written, and the document text does
  not change.
- The command is missing from the Pack DSL pane and from the file you
  download or save.
- The status line adds "as a patch over this document, which is a program and
  is not rewritten".

## Answer

The studio never rewrites a **programmed** pack — one that computes its
declarations rather than stating them. Your edit is not lost: it stands as a
patch over the document, which is what the studio and the language server
resolve the command through. The document itself is left exactly as you
wrote it, because rewriting it would replace your program with the commands
it happened to produce this time.

A document is a program when any of these is true:

1. It asks `available?` while registering, so its surface depends on the
   analysis target.
2. A command it declares has no `command NAME { … }` statement of its own —
   a `foreach`, a `proc`, or a computed name registered it.
3. A statement in it runs rather than registers — a `set`, a `proc`, an `if`
   around the declarations.

To get the edit into the document, edit the program that produces the
declaration, in the Pack DSL pane. There is no way to fold a patch back into
a program: only you know which line of the program should have produced it.

A patch stands for the session it was made in. Downloading or saving the
document saves the document — the patch is not part of it — so treat a patch
as a staging area, not a home.

If the document is *not* a program and the edit still does not appear, look
for text left outside the `speclib NAME VERSION { … }` block — a half-typed
`command`, a stray word. The loader reads and drops it, so it is harmless to
the pack, but tidy it up: it is the only text in the file the studio's own
report cannot account for.

## Related

- [How do I write a SpecTcl pack?](kcs-howto-write-a-tclspec-pack.md)
- [How do I create command specs without Rust?](kcs-howto-create-command-specs-without-rust.md)
- [SpecTcl pack design](../design/spec-packs.md)
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
