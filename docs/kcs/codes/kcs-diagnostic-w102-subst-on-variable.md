# KCS: W102 — Does subst on a variable allow command execution?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn when `subst` is called on a variable?

## Why

`subst` substitutes its string a *second* time. When that string arrives from
a variable it has already been substituted once, so whatever the variable
holds — including a `[command]` — is evaluated. With `set x {[exec rm -rf /]}`,
`subst $x` runs the `exec`.

## Symptoms

- A yellow squiggle appears under the last argument of the `subst` call, with
  the message "subst with a variable argument enables code injection: any
  [cmd] and $var in the string will be evaluated. Add -nocommands -novariables
  to limit substitution scope, or use [format] / [string map] for safe
  templating."
- The message names only the substitutions this call still performs: with
  `-nocommands` it reads "any $var in the string" and suggests only
  `-novariables`.
- Where the switches cannot be narrowed — a Tcl 9.1 call already using
  `-commands` / `-variables`, whose switches may not be combined with
  `-nocommands` / `-novariables` — the message drops the switch advice and
  suggests only `[format]` / `[string map]`.

## Example that triggers it

```tcl
subst $template
```

The analyser reports **`W102`** on `$template`, the last argument — the string
`subst` substitutes.

## Fix

```tcl
subst -nocommands -novariables $template
```

Disable command and variable substitution so that only backslash substitution
is performed. In Tcl 9.1 the same call can be written `subst -backslashes
$template`, naming the one substitution that *does* run; the two switch
families cannot be combined in one call.

The warning is not reported when nothing dangerous is substituted or when the
string is not a variable at all:

```tcl
subst {hello $name}          ;# a template written here, nothing spliced in
subst $opt {hello $name}     ;# $opt is a switch — the last argument is the string
subst -backslashes $template ;# Tcl 9.1: no [cmd] and no $var runs
```

## How to suppress

Add `# noqa: W102` on the line **above** the offending command, or a
`# tcl-lsp: disable=W102` directive in the leading comment block of the file,
or `disabled = W102` under `[diagnostics]` in `.tcl-lsp.ini`, or set
`tclLsp.diagnostics.W102` to `false` in your editor. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W101`, `W308`
