# KCS: feature — Special Variable Recognition

> **Audience:** User
> **Type:** Functionality

## Summary

Recognises interpreter-provided special variables (`auto_path`, `env`, `errorInfo`, `tcl_platform`, `argv`, the iRules `static::` namespace) so they are never mis-flagged as unused or read-before-set, documents them on hover, and treats `env`/`argv` reads as tainted external input — all aware of the file's dialect.

## Applies to

all-editors, MCP, Claude skill

## Question

What does special-variable recognition do, and how do I use it?

## How to use

It works automatically once a dialect is active — there is nothing to enable. When you write a special variable that the runtime consumes, the server suppresses the false "never read" warning; when you hover a special variable, it shows a short description and (for arrays) the keys valid in your dialect; and when an external-input variable flows into a code-execution sink, the taint analysis flags it.

The set is dialect-aware: standard Tcl files see `argv` / `env` / `auto_path`; F5 iRules files instead see the CMP-safe `static::` namespace and the BIG-IP `tcl_platform` keys, and do **not** see `env` / `argv`, which the iRules interpreter does not provide.

## Options

- `tclLsp.diagnostics.W220` — dead-store hint; special-variable writes are exempt regardless of this toggle.
- `tclLsp.diagnostics.W211` — unused-variable hint; special-variable writes are exempt.

## Example

### No false "never read" on a runtime-consumed write

```tcl
set auto_path ../          ;# no W220 / W211 — the auto-loader reads auto_path at runtime
lappend auto_path $libdir  ;# also fine
set env(TZ) UTC            ;# no warning — the write mutates the process environment
```

Before this feature, `set auto_path ../` reported `Assignment to 'auto_path' is never read [W220]`.

### Hover documentation

Hovering `$tcl_platform(os)` in a Tcl file shows a summary plus the platform keys available in that Tcl version. In an iRules file the same hover reports the BIG-IP keys (`tmmVersion`, …) and warns that a plain `$tcl_platform` access demotes the virtual server from CMP — use `static::tcl_platform`.

### Tainted external input

```tcl
exec $env(CMD)   ;# T100 — reading the environment is attacker-influenced input
eval $argv       ;# T100 — command-line input flowing into a code sink
```

## Operational context

Special variables live in one dialect-versioned registry (`tcl_registry::special_vars`), consulted generically by the analyser, the taint / side-effect passes, and the hover provider — no diagnostic hardcodes a name list. Because membership is keyed on the active dialect, the same variable can exist in one dialect and not another, and array keys can appear only in the Tcl versions that added them.

## Discoverability

- [KCS feature index](README.md)
- [Unused variable detection](kcs-feature-unused-variables.md)
- [Diagnostics](kcs-feature-diagnostics.md)
- [Hover](kcs-feature-hover.md)
