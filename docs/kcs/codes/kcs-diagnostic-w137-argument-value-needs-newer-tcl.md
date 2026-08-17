# KCS: W137 — Why does the analyser say this argument value needs a newer Tcl?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn that a literal argument value requires a newer
Tcl version than my dialect provides?

## Why

Some commands take a fixed set of literal argument values, and individual
values in that set arrive in later Tcl releases. `string is dict`, for
example, is a real class — from Tcl 9.0. On an earlier release the same
call raises "bad class" at run time. The registry records the introducing
release for each value, and the analyser compares it against the file's
effective Tcl version — the dialect profile, raised by any
`package require Tcl`.

## Symptoms

- A yellow squiggle under the value word, with a message like
  "argument value 'dict' of 'string is' requires Tcl 9.0 but tcl8.6
  provides 8.6".

## Example that triggers it

```tcl
if {[string is dict $config]} {
  puts "config is a dictionary"
}
```

Analysed under the `tcl8.6` dialect, the analyser reports **`W137`** on
`dict`: the class needs Tcl 9.0.

## Fix

Raise the floor so the value is available:

```tcl
package require Tcl 9.0

if {[string is dict $config]} {
  puts "config is a dictionary"
}
```

Or avoid the value and use a form the older release accepts — here,
checking the value with `dict size` inside a `catch`.

A unique abbreviation of a gated value (`string is di`) is checked the
same way, because Tcl resolves abbreviations before looking the class up.
A dynamic value (`string is $class`) is not checked at all — the analyser
cannot know what it holds.

## How to suppress

Turn the code off for a project with `disabled = W137` under
`[diagnostics]` in `.tcl-lsp.ini`, for one file with a
`# tcl-lsp: disable=W137` directive at the top of the file, or in your
editor with `tclLsp.diagnostics.W137` set to `false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W135`, `W136`, `W138`, `W144`
