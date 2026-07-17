# Colon names and absolute addressability (issue #934)

**Status:** implemented. This note pins the name-parsing model that every
resolution surface (analyser, LSP providers, bytecode VM, WASM runtime)
shares for names carrying colons — `proc :`, `namespace eval :`, `proc {}`,
`proc x::`, written words like `a:::b` — and the discipline that keeps
internal keys from corrupting them.

## The written-name rule (C ground truth)

`TclGetNamespaceForQualName` parses every entity's qualified name the same
way, invariant from 8.4 through 9.1:

- A run of **two or more** colons is one namespace separator; the whole run
  is consumed (`a:::b` names `a::b`;
  [8.4 `tclNamesp.c:1710-1765`](https://github.com/tcltk/tcl/blob/core-8-4-20/generic/tclNamesp.c),
  9.1 has the identical loop).
- A **lone `:`** is an ordinary name character (`proc :`, `proc a:b`,
  `rename src :2` are all real commands).
- A name **ending** in a separator run has an **empty simple name** — the
  `{}` command/variable in the qualified namespace. It is a first-class,
  addressable entity: `proc x:: {} {}` defines it (given `::x` exists), and
  `x::`, `x:::`, `::x::` all call it.
- An **all-colon word** (`::`, `:::`, `::::`, …) is the global namespace's
  `{}` command. With `proc {} args {…}` defined, `::` and `:::` both
  dispatch it; without it they raise `invalid command name`.
- `$`-substitution scans a varname as alphanumerics/underscore plus colon
  runs **started by a pair** (`tclParse.c`, 8.6 at `:1502-1505`): `$:` does
  not substitute, `$a:b` reads `$a`, `$a:::b` reads the variable `a:::b`
  (i.e. `::a::b` after run collapse).

Behaviour pinned against real tclsh 8.6.16 / 9.0.4 (identical) in
`tcl-syntax/tests/data/command_resolution_vectors.txt` (the `#934` block) —
executed by `vectors_match_real_tclsh`, the pure resolver, the analyser
settlement, the VM dispatch, and the WASM runtime dispatch conformance
tests.

## Consequences: addressability

Because a written run collapses, some legal definitions have **no absolute
spelling**:

| Definition | Reachable | Absolute form |
|---|---|---|
| `proc :` (any all-colon simple name) | bare/relative lookup only | **none** — `:::` is the `{}` proc |
| `namespace eval :` and everything inside | relative only (`namespace inscope : :`) | **none** — `:::inner` is `::inner` |
| `proc {}` / `proc x::` (empty simple name) | yes | `::` / `::x::` |
| `proc a:b` (interior lone colon) | yes | `::a:b` |

C itself trips over this: `namespace which :` renders `:::` (unresolvable),
and a namespace ensemble cannot dispatch an exported `:` (it builds
`::ns:::`, which re-parses to the empty name — `invalid command name`).

The analyser flags the unaddressable shapes as **W314** (definition-site:
all-colon simple name; `namespace eval` site: an all-colon written segment),
without cascading onto addressable definitions nested inside a flagged
namespace.

## The constructed-key discipline

Internal identities are flat strings built as `"::" + simple` /
`"{ns_key}::{simple}"` (the analyser and LSP layers use **rooted** keys;
the VM's command table uses **unrooted** keys). Two rules keep them sound:

1. **Canonicalise written words once, at intake** —
   `naming::canonical_written_command` (commands/variables; preserves a
   trailing separator as the empty simple name) and
   `normalise_qualified_name` / `qualifier_segments` (namespace names; a
   trailing run drops). `command_resolution_candidates` applies this to the
   call word and to `namespace path` entries; `qualify` /
   `Vm::qualify_name` apply it to definition names.
2. **Never re-parse a constructed key** — a key may legitimately contain a
   lone-colon segment (`":::"` is the proc — or namespace — named `:`), so
   joins concatenate with exactly one `::` and splits use the
   construction-inverse helpers: `naming::key_tail`,
   `naming::key_holder_and_tail`, `naming::key_segments` (rooted), and the
   VM's `key_holder_and_tail_unrooted`. A `rsplit("::")`, a char-pattern
   colon trim, or a re-`normalise` of a joined key is exactly how `proc :`
   used to collapse into the `{}` key (`"::"`), producing the empty
   documentSymbol name the issue reported.

For **all-colon keys** the flat encoding is ambiguous between conventions;
the helpers resolve it by the construction grammar: rooted keys are
`"::"` (2) plus 3 per `:`-named level plus 1/0 for a `:`/empty simple, so
length ≡ 0 (mod 3) ⇒ simple `:` and ≡ 2 ⇒ simple `""`; unrooted keys shift
by the missing root (the VM helper roots first). Mixed-content keys are
unambiguous up to the documented preference for the non-empty simple name
(`"::a:::"` reads as `:` in `::a`, not `{}` in a namespace named `a:`).

Written-word tail extraction via `rsplit("::")` **is** correct (a trailing
run yields the empty simple name, matching C) — the broken pattern is only
applying it to constructed keys.

## Where the pieces live

- `tcl-syntax/src/naming.rs` — the written-name canonicalisers, the
  construction-inverse key helpers, `is_absolutely_addressable`, and the
  conformance vector machinery (which builds colon-named definitions
  *relatively*, since no absolute spelling exists to write).
- `tcl-compiler/src/analyser/` — `join_namespace` /
  `namespace_from_scope_path` / `advance_command_resolution_namespace`
  join-then-never-reparse; definition handlers derive simple names via
  `key_tail` and emit W314.
- `tcl-vm/src/interp.rs` — `register_command` takes canonical unrooted keys
  verbatim (`register` strips the root from builtin literals);
  `qualify_name` canonicalises the written name before the exact-concat;
  `resolve_command_fqn` unroots candidates by a single `strip_prefix("::")`.
- `runtime/rust/src/namespace.rs` — `home_of` restores the empty simple
  name for separator-terminated words.

## Relative reachability, and the 9.0 `namespace code` intrep round-trip

An unaddressable definition (the W314 case — `proc :` inside
`namespace eval :`) is still **reachable by relative dispatch**: with the
proc defined, `namespace eval : { : }` and `namespace inscope : :` both
return `hello` (tclsh 8.6.16 **and** 9.0.4; the VM and the runtime agree,
pinned in `tricky_resolution_e2e.rs` and the runtime's
`trailing_separator_definitions_match_resolution`).  W314's message is
calibrated to exactly this: *no absolute spelling*, relative lookup only.

`namespace code :` evaluated inside `:` generates
`::namespace inscope ::: :` — and that word exposes a **value-identity**
behaviour, probed three ways (both tclshs):

| form | 8.6 | 9.0 |
|---|---|---|
| fresh literal `namespace inscope ::: :` | error (`:` at global) | error (`:` at global) |
| `eval` of the **generated** script object | error | **hello** |
| the same script after a string round-trip | error | error |

The *text* `:::` resolves identically in both versions — the
`TclGetNamespaceForQualName` walk is byte-for-byte equivalent across
8.6.16/9.0.4 (`tmp/` sources), all-colon spellings land on the global
namespace, so the dispatched `:` misses.  Tcl 9.0's generated word,
however, carries a cached `nsName` intrep — a **reference to the
namespace captured at generation time** — which `namespace inscope`
consumes directly, reaching the unaddressable namespace by identity, not
by name; 8.6 re-parses and misses, and any string shimmer (the `join`
row) restores textual semantics on 9.0 too.

For the static stack and both string-round-tripping engines the textual
semantics are the contract (they match both tclshs on every *writable*
form); the 9.0 identity round-trip is unreachable by any name a document
could contain, which is precisely W314's premise.

## Related

- [command-resolution.md](contracts/command-resolution.md) — the resolution
  order contract these names flow through.
- [name-resolution-fix-plan.md](name-resolution-fix-plan.md) — the master
  plan this work extends (issue #934 arrived during its completion pass).
- KCS: [W314](../kcs/codes/kcs-diagnostic-w314-no-absolute-name.md).
