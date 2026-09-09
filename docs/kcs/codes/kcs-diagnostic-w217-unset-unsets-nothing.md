# KCS: W217 — Why does the analyser say my `unset` unsets nothing?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, command-walk

## Profiles

default

## Question

Why does the analyser warn that my `unset` command unsets nothing?

## Why

`unset` accepts the options `-nocomplain` and `--` before its variable names. When **every** argument is one of those two words, no variable name remains, so nothing is unset. Only `-nocomplain` and `--` are options: any other word ends option processing and is used as a name, so `unset -x` really does unset a variable called `-x`. The warning therefore fires on `unset -nocomplain`, `unset --`, and `unset -nocomplain --` — usually written by an author who meant to unset a `-`-named variable.

## Symptoms

- A yellow squiggle appears under the `unset` arguments, with the message *"`unset` unsets no variable here — `-nocomplain` / `--` are consumed as options. To unset a variable whose name begins with `-`, put `--` before it (e.g. `unset -- -nocomplain`)."*

## Example that triggers it

```tcl
unset -nocomplain
```

The analyser reports **`W217`** — every argument was consumed as an option, so no variable is named.

## Fix

```tcl
unset -nocomplain -- -x
```

Add `--` to end option processing, then name the variable — a `-`-named variable must appear after `--`. The editor offers a quick fix that inserts `--` before the first word for you.

## How to suppress

Add `# noqa: W217` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
