# Special-variable registry

> **Audience:** Maintainer / Contributor
> **Type:** Design — data-structure / contract

The special-variable registry (`tcl_registry::special_vars`) is the single
source of truth for the globals an interpreter, its `init.tcl`, or the platform
inject into every Tcl program — `auto_path`, `env`, `errorInfo`, `tcl_platform`,
the command-line `argv`/`argc`/`argv0`, and the F5 iRules `static::` namespace.
It exists so the analyser, the taint / side-effect passes, and the LSP hover
provider reason about these variables from one dialect-versioned table instead
of hardcoding name lists (issue
[#831](https://github.com/bitwisecook/tcl-lsp/issues/831)).

## Why a registry

Special variables differ from user variables in ways the analysis must respect:

- A **write is observed by the runtime** even when the script never reads the
  value back. `set auto_path …` configures the package auto-loader; flagging it
  as a dead store (W220) or unused variable (W211) is a false positive.
- **Startup readability is lifecycle-specific.** `SPECIAL_VARS` describes the
  recognised runtime-sensitive vocabulary, while `initially_bound` and
  `lazily_readable` record the narrower set a default host makes readable
  before user code. W210 consumes those facts only for the initial global SSA
  version; a same-named procedure local or a value removed with `unset` still
  warns.
- The default-host qualification matters: Tcl 8.4's successful stock
  `Tcl_Init` runs `init.tcl`, which seeds `errorCode` and `errorInfo`; a bare
  `Tcl_CreateInterp`, failed/overridden initialisation, and Tcl 8.5+ do not
  provide that entry fact. `auto_index` is similarly not a startup binding:
  interactive or auto-loader activity can materialise it later, but a normal
  script's direct read remains undefined.
- A **write can carry an interpreter side effect** beyond the variable slot
  (`set tcl_precision …` changes float formatting; `set env(X) …` mutates the
  process environment).
- A **read can be a taint source**: `env`, `argv`, and `argv0` are
  attacker-influenced external input, so `exec $env(CMD)` is a tainted flow.

The set is **dialect-versioned**, exactly like `CommandSpec`: it is a
`DialectSet` membership table, because standard Tcl, iRules, Tk, Expect, and the
EDA shells expose slightly different sets.

## Data model

Each entry is a `SpecialVarSpec`:

| Field | Meaning |
|---|---|
| `name` | Bare name, no `$` / `::` (`auto_path`, `tcl_platform`, `static`). |
| `kind` | `Scalar`, `Array`, or `Namespace` (iRules `static::`). |
| `access` | `ReadOnly` vs `ReadWrite` — informational (hover). |
| `origin` | Provenance for hover grouping (`Interpreter`, `AutoLoader`, `Platform`, `Environment`, `Dialect`). |
| `dialects` | `DialectSet` in which the variable exists. |
| `initially_bound` | Dialects in which the default host has bound the global before user code. |
| `lazily_readable` | Dialects where a core read trace materialises an otherwise-unbound value. |
| `startup_binding` | Why the entry is readable (`Interpreter`, `TclInit`, `TclMain`, `AppInit`, `ReadTrace`, or `None`). |
| `keys` | Known array keys, each with its own `DialectSet` (per-key version gating). |
| `externally_read` | A write is runtime-observed → not a dead store / unused variable. |
| `cmp_unsafe` | iRules: plain access demotes the virtual server from CMP; use `static::`. |
| `write_effect` | `Option<SideEffectTarget>` — the interpreter state a write mutates. |
| `read_taint` | `Option<TaintColour>` — the taint a read produces (external input). |
| `summary` | One-line hover prose. |

Array keys carry their own `DialectSet`, so `tcl_platform(pointerSize)` is Tcl
8.5+, `tcl_platform(pathSeparator)` is Tcl 8.6+, and
`tcl_platform(tmmVersion)` is iRules-only. Build-only keys such as `threaded`
and `debug` are not advertised by a release-only profile.

## Dialect resolution

`resolve_dialect(name) -> DialectSet` maps a dialect string to the membership
flag used by the query helpers:

- An unrecognised name (empty, generic `"tcl"`, config-only `"f5-bigip"`)
  resolves to every standard Tcl version.
- The **restricted** F5 iRules runtime resolves to its own bit — it provides
  none of the command-line / auto-loader / environment globals, so `env` /
  `argv` / `auto_path` are not recognised there. iApps instead resolves with
  its documented host-Tcl base, so standard Tcl facts apply where evidenced.
- A specific Tcl version (`tcl8.6`) keeps its exact bit, so per-key version
  gating stays precise.
- Every other parseable dialect is a Tcl **superset** (Tk, Expect, EDA shells,
  BPF): its bit is widened with `ALL_TCL` so the standard globals are recognised
  there in addition to its own.

## Query API and consumers

The crate exposes name + dialect queries; consumers never hold their own list:

- `is_special_var(name, dialect)` — availability / metadata lookup; it does
  not imply an initial value.
- `is_readable_at_startup(name, dialect)` — the lifecycle-aware W210 entry
  fact, applied only to the initial global SSA version by `tcl-compiler`.
- `is_externally_read(name, dialect)` — dead-store (W220) and unused-variable
  (W211) suppression.
- `special_var_write_effect(name, dialect)` — `classify_variable_assignment`
  attaches this as an extra `SideEffect` on the write, so effect analysis and
  dead-code elimination treat `set auto_path …` as an interpreter-state
  mutation, not a removable assignment.
- `special_var_read_taint(name, dialect)` — the taint pass (`seed_entry_taints`)
  seeds the version-0 (external) read of `env` / `argv` / `argv0` as tainted, so
  a flow into a code-execution sink fires T100. A later local `set env …`
  writes a higher SSA version, so a shadowed read is unaffected.
- `special_var_in_dialect` / `special_vars_for_dialect` — the LSP hover provider
  (`tcl-lsp-core`) renders a variable's summary, dialect-gated array keys, and
  the iRules CMP-safety note.

## Extending the table

Adding a variable — or a dialect's variant of one — is an edit to
`SPECIAL_VARS`, not a new branch in a consumer. A new dialect's globals are new
rows with that dialect's `DialectSet` bit; a version-gated array key is a new
`SpecialVarKey` with the version's `DialectSet`. A new startup binding must
also state whether it comes from interpreter creation, `Tcl_Init`, `Tcl_Main`,
application initialisation, or a lazy read trace; availability alone must not
silence W210.
