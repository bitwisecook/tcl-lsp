# KCS: S110 — Why does the analyser say my binary data will be corrupted?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, shimmer

## Profiles

default

## Question

Why does the analyser warn that binary data passed through a string
command will be corrupted?

## Why

Tcl holds binary data and character strings in two different internal
representations. When a byte array is forced through a character-string
operation, Tcl reads each byte as a character and later writes it back
out re-encoded — so every byte from `0x80` upward changes. Case folding
pushes bytes out of the `0..255` range outright.

Unlike the `S100`–`S103` shimmer family, which is about *performance*,
S110 is a **correctness** finding: the data really does come out wrong.
In iRules this is the classic `*::payload replace` rewrite bug; in plain
Tcl it is `binary format`, then a `string` command, then a byte sink.

## Symptoms

- A yellow squiggle on the offending statement, with a message beginning
  "Byte-array corruption:" and naming the command that did the damage.
- Related markers point at where the binary data came from and where it
  was treated as a character string.

## Example that triggers it

```tcl
when CLIENT_DATA {
  set p [TCP::payload]
  set q [string map {a b} $p]
  TCP::payload replace 0 100 $q
}
```

The analyser reports **`S110`** on the `TCP::payload replace`: `$q` was
modified as a character string, so every byte at or above `0x80` will be
re-encoded on the way back into the payload.

The plain-Tcl shapes fire immediately, with no sink needed:

```tcl
proc shout {} {
  set b [binary format a* hello]
  set u [string toupper $b]
}
```

## Fix

Re-binarify the value before it reaches the byte sink:

```tcl
when CLIENT_DATA {
  set p [TCP::payload]
  set q [string map {a b} $p]
  binary scan $q a* q
  TCP::payload replace 0 100 $q
}
```

`binary scan` reinstalls the byte-array representation in place, which is
the documented fix; `binary encode` does the same to its data operand.

Better still, avoid the string round-trip. `string range`, `string
index`, and `string reverse` keep the byte-array representation intact,
so a payload sliced with one of those and written straight back is
byte-exact and never draws S110. A clean getter-to-`replace` writeback
with no string command in between is silent too.

## When it does not fire

- **Outside the iRules dialect** for the `*::payload` shapes — the
  payload byte commands are only modelled there.
- **On plain strings** that never came from a binary source.

## How to suppress

Add `# noqa: S110` on the line **above** the offending command. You can
also turn the code off for a project with `disabled = S110` under
`[diagnostics]` in `.tcl-lsp.ini`, or in your editor with
`tclLsp.diagnostics.S110` set to `false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [shimmer](../../GLOSSARY.md#shimmer)
- Related codes: `S100`, `S101`, `S103`, `W311`
