# KCS: How do I say a package is ambient under one of my pack's dialects and not another?

> **Audience:** User
> **Type:** How-To

## Applies to

all-editors, MCP

## Question

My pack describes two shells. One of them ships a library the author never
has to `package require`; the other does not. How do I say that, so the
version floor and the missing-require warning both follow the shell the
file is written for?

## Before you start

- You have a `.tclspec` pack. If not, start with
  [writing a SpecTcl pack](kcs-howto-write-a-tclspec-pack.md).
- Your pack declares `speclib <name> 2.0` (or later). The environment
  block is 2.0 vocabulary.

## Answer

Declare each shell as an `environment`, and put an `ambient` row in the one
that has the library. A shared version is an ordinary Tcl variable — a pack
is a Tcl program and an `environment` body is a script, so you set it once
and substitute it where it is needed.

```tcl
speclib mypack 2.0 {
    set tkver 8.6

    environment mypack-shell {
        display_name {Mypack Shell}
        core         tcl 8.6
        ambient      Tk $tkver
    }

    environment mypack-plain {
        display_name {Mypack Plain}
        core         tcl 8.6
    }
}
```

Two details are worth knowing:

1. **The body is a script, so ordinary Tcl works in it.** Variables
   substitute, and a repetitive ladder can be a `foreach` — the same as
   inside a `command` block. What each row *means* is still decided by the
   environment reader, so a misspelt row is rejected however it was
   produced.
2. **`ambient_package` is the wrong tool for this.** That row floors its
   package for *every* document the pack is active in, and it has no
   scoping flag. Writing `ambient_package Tk 8.6 -dialects {mypack-shell}`
   does not narrow it — the loader drops the whole row and tells you to
   write the environment block above, because keeping the row without the
   scoping would floor Tk in the shell you just said does not have it.

## How to tell it worked

Open a file that resolves to `mypack-shell` and call something the library
provides. The "needs a `package require`" warning (W120) is gone, and a
call gated above the declared version is reported against it — for the
example above, `entry .e -placeholder hi` reports that `-placeholder`
requires Tk 8.7.

Open the same file under `mypack-plain` and both go the other way: the
missing require is reported again, and there is no version floor to fail.

If the pack panel shows a notice mentioning `-dialects`, an
`ambient_package` row still carries the flag; replace it with an
`environment` block as above.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [How do I write a SpecTcl pack?](kcs-howto-write-a-tclspec-pack.md)
- [W120 — missing package require](codes/kcs-diagnostic-w120-missing-package-require.md)
