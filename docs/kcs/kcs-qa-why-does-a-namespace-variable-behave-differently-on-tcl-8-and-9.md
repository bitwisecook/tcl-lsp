# KCS: Why does the same `namespace eval` script write a different variable on Tcl 8.6 and Tcl 9?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp CLI

## Question

Why does an unqualified variable name inside `namespace eval` reach a global
variable on Tcl 8.x but create a namespace variable on Tcl 9?

## Answer

Because Tcl 9 deliberately removed a fallback that Tcl 8.x had. This is a real
language change, not a bug in either interpreter — and it is easy to mistake
for one, because the same script silently does something different instead of
raising an error.

**The rule.** When Tcl resolves a *relative* (unqualified) variable name at
**namespace scope** — that is, in the body of `namespace eval`, not inside a
proc — it looks in the current namespace first. If the current namespace has no
such variable:

- **Tcl 8.4, 8.5, 8.6** then look in the **global** namespace, and use the
  global variable if it exists. Reads *and* writes both land there.
- **Tcl 9.0, 9.1** stop. A read fails with `no such variable`; a write creates
  a new variable in the current namespace.

Here is the worked example, run against real interpreters:

```tcl
set i foo
namespace eval n1 { append i baz }
puts "::i=[set ::i]  ::n1::i exists=[info exists ::n1::i]"
```

| | `::i` afterwards | `::n1::i` created? |
|---|---|---|
| tclsh 8.6.18 | `foobaz` — the global was appended to | no |
| tclsh 9.0.4 | `foo` — untouched | **yes**, set to `baz` |

Three details catch people out:

1. **It is not just `append`.** The rule governs *name resolution*, so it
   applies to everything that resolves a relative name: `set`, `append`,
   `lappend`, `incr`, `unset`, `info exists`, `$x` substitution, and array
   element access. A `dict incr b …` on a relative `b` behaves differently
   across the two releases for exactly this reason.
2. **It never walks intermediate parents.** The search is *current namespace,
   then global* — a namespace nested inside another never sees its parent's
   variables, in either release. Nesting depth does not matter: a deeply
   nested namespace under 8.x still falls back to the **global**.
3. **A `variable` declaration blocks it.** Declaring `variable v` installs a
   cell in the current namespace, so the name resolves there and never falls
   through — even when the variable is declared but unset. Likewise, an
   existing namespace variable shadows the global in both releases.

Procs are unaffected in both releases: an unqualified name in a proc body is a
frame-local, and never falls back to a global. Explicit links (`global`,
`upvar`, `namespace upvar`) are also unaffected, because they say outright
which variable they mean.

**The upstream citation.** `doc/namespace.n`, under NAME RESOLUTION:

- **8.6:** "*Variable names* are always resolved by looking first in the
  current namespace, **and then in the global namespace**." Its worked example
  reads: "Tcl looks for `traceLevel` in the namespace `Debug` **and then in the
  global namespace**."
- **9.0:** "*Variable names* are always resolved starting in the current
  namespace." The fallback clause is deleted, and the same worked example now
  ends at "Tcl looks for `traceLevel` in the namespace `Debug`."

In the C source the switch is `TCL_NAMESPACE_ONLY` (8.6 `generic/tclVar.c:757`
versus 9.0 `generic/tclVar.c:935`).

**How this project models it.** Both execution engines derive the behaviour
from the Tcl release they are emulating, rather than hardcoding one release's
answer:

- `tcl-vm` — `Vm::set_runtime_version`, which feeds `ns_var_global_fallback()`.
- `runtime/rust` — `Interp::set_runtime_version`; the `run_script` dev tool
  exposes it as `--tcl-version 8.6`.

Both key off the same ordered `tcl_dialect::TclVersion` enum, so the rule turns
on for 8.4/8.5/8.6 and off for 9.0/9.1 in one place per engine.

### Limits of this note

The output vectors are verified against tclsh **8.6.18** and **9.0.4**. The
8.4 and 8.5 ends of the range are taken from the upstream documentation and C
source rather than from a running interpreter — no 8.4/8.5 executable was
available — so they are modelled as sharing 8.6's behaviour. Tcl 9.1 is
modelled as sharing 9.0's.

This note covers *variable* name resolution only. **Command** name resolution
is a different algorithm with its own version differences (notably the 8.5
`namespace path` tier) — see the related note below.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [kcs-qa-how-are-command-names-resolved.md](kcs-qa-how-are-command-names-resolved.md)
- [Name resolution: the C algorithm and the 8.4→9.1 version matrix](../design/name-resolution-tcl-version-and-c-source.md)
