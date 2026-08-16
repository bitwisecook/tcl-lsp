# KCS: IRULE6001 — Why does the analyser warn about a global variable in an iRule?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser flag `global`, or a `::`-qualified variable, and
tell me to use `static::` instead?

## Why

BIG-IP runs one TMM per CPU and spreads connections across all of them.
A variable in the global namespace has no per-TMM storage, so touching
one forces the virtual server into CMP compatibility mode: every
connection is pinned to a single TMM and the rest of the box sits idle.
On a busy virtual server that is a large, silent throughput loss.

`static::` variables are the supported alternative. They have per-TMM
storage, so the virtual server keeps running demoted across every TMM.

## Symptoms

- A yellow squiggle on the command or the variable word, with one of:
  - "'global counter' imports from the global namespace, forcing CMP
    compatibility mode and pinning the virtual server to a single TMM.
    Use 'static::counter' instead."
  - "Global namespace variable '::counter' forces CMP compatibility mode,
    pinning the virtual server to a single TMM. Use 'static::counter'
    instead."
  - "'counter' in RULE_INIT is implicitly global — RULE_INIT runs at the
    global namespace scope. …"
- A **Replace '::counter' with 'static::counter'** quick fix on the
  qualified and implicit-global forms.

## Example that triggers it

```tcl
when RULE_INIT {
  set requests 0
}

when HTTP_REQUEST {
  incr ::requests
}
```

The analyser reports **`IRULE6001`** twice: on `requests` in `RULE_INIT`,
which is implicitly global because `RULE_INIT` runs at global scope, and
on `::requests` in the event body.

## Fix

```tcl
when RULE_INIT {
  set static::requests 0
}

when HTTP_REQUEST {
  incr static::requests
}
```

The quick fix applies this rewrite, but review it before accepting:
`static::` variables live per TMM and have a different lifetime from a
global, so a counter becomes a per-TMM counter rather than a box-wide
one.

## What it detects

Three shapes:

- `global name` — an explicit import from the global namespace.
- A `::`-qualified name in the variable-writing position of `append`,
  `array set`, `const`, `gets`, `incr`, `lappend`, `ledit`, `lpop`,
  `lset`, `set`, `unset`, or `variable`.
- A plain, unqualified name written by one of those commands **inside
  `RULE_INIT`**, where every write is implicitly global. A one-argument
  `set` is a read and is not flagged; neither is `unset`, nor an `array`
  subcommand other than `set`.

A name that already starts with `static::` is never flagged.

## How to suppress

`IRULE6001` is internal: it has no per-code entry in the generated editor
settings list. Silence it for one file with a
`# tcl-lsp: disable=IRULE6001` directive at the top of the file, or for a
whole project with `disabled = IRULE6001` under `[diagnostics]` in
`.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `IRULE4001`, `IRULE4002`, `IRULE4005`
