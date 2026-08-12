# Compilation-unit scope — who can call this file's procedures?

> One contract: when may an interprocedural fact derived from the call sites
> a single file contains be trusted as a fact about *every* caller?

Source: [`rust/tcl-compiler/src/unit_scope.rs`](../../../rust/tcl-compiler/src/unit_scope.rs).

## The problem

`CompilationUnit::build_for` is single-source-text by construction, but Tcl
has no `static`: every `proc` lands in a global command table that any file
sharing the interpreter can reach. The interprocedural SCCP seed
(`params_constants_from_call_sites`) binds a parameter to a compile-time
literal only when **every** caller passes that literal — so the seed is
exactly as sound as the claim *"the call sites I found are all of them"*.

Issue #977 is that claim failing in its most ordinary form:

```tcl
# lib.tcl — no package provide
proc helper {mode} {
    if {$mode eq "prod"} { … } else { … }
}
helper prod
helper prod
```

```tcl
# main.tcl
source lib.tcl
helper dev        ;# a real caller lib.tcl's compilation unit can never see
```

Analysed on its own, `lib.tcl`'s only visible callers agree, so `mode` is
seeded as `"prod"` and the condition folds — producing an I230 "condition is
always true" that is simply wrong.

## The three layers

`unit_scope` owns the completeness claim in three layers. Each is
*monotone*: it can only ever remove a fold, never add one.

### 1. In-unit evidence

`collect_call_site_constants` walks the module's own CFGs — the top level,
every procedure, plus the `TclOO` method and `apply` / `namespace eval` body
units `build_extra_call_site_scan_contexts` supplies — and records each
resolvable call's literal arguments into a `CallSiteEvidence`.

Resolution goes through `interprocedural::resolve_internal_call` (Tcl's real
existence-checked, namespace-relative order, evaluated in the *calling*
function's namespace) with a `namespace import` fallback, and recurses into
`ArgRole::Body` arguments so a call inside `catch { … }` or a literal
`uplevel { … }` still counts.  Each of those is load-bearing: a call the walk
does not see is a call site whose literal cannot contradict the others, so the
seed would be granted on incomplete evidence.

### 2. Cross-unit evidence

`scan_source_call_sites` runs the *identical* lowering → CFG →
`record_call_site_evidence` walk over **another** file's source text,
resolving each call against the whole project's procedure names, so a host
with a workspace view can hand this unit the call sites it could never see
itself. `CallSiteEvidence::merge_from` folds them in before seeding.

| Producer | How it enumerates the project |
|---|---|
| LSP | `tcl_lsp_db::file_call_site_evidence` → `project_call_site_evidence` → `file_external_call_sites` (sliced to the file's own declarations), compare-then-set onto `SourceFile::external_call_sites` by the server's `sync_cross_file_evidence` |
| `tcl` CLI | `cross_file_call_site_evidence` across a multi-input `diag` / `validate` invocation |
| Standalone (tests, a library call) | `UnitBuildOptions::external_call_sites: None` — no view |

Slicing the merged table to the file's own declarations is what keeps
invalidation precise: a call-site edit in `main.tcl` re-sets only the file
that *defines* the callee.

`None` and `Some(empty)` are **different claims**. `None` means "no
cross-file view available", which is not the same as "no cross-file callers".
`Some` — even of an empty table — is the host asserting it enumerated the
project, so the merged evidence is the whole picture.

### 3. Registry-declared unit boundaries

`scan_unit_linkage` asks the registry whether the file itself admits to being
part of a bigger program, via `CommandRegistry::unit_linkage` (which resolves
the subcommand word, so `package provide` is a boundary and `package names`
is not). **No command name appears in the compiler** — the facts are
`Traits` bits on the specs:

| Trait | Set on | Means |
|---|---|---|
| `PROVIDES_PACKAGE` | `package provide`, `package ifneeded` | the file is a loadable package; its commands are public API |
| `EXPORTS_COMMAND` | `namespace export`, `namespace ensemble` | the file publishes command names for another unit |
| `LOADS_EXTERNAL_UNIT` | `source`, `load`, `package require`, `auto_load`, `auto_import` | another unit's script runs in this interpreter and can call back in |

`namespace import` is deliberately **not** a boundary: it is as often an
intra-file convenience over a namespace the same file defines, and layer 1
already models the import as a real caller path.

## The gate

`params_constants_from_call_sites` binds `(param, 0)` to a literal only when
all of the following hold:

1. Every recorded call site passes the same single literal at that position,
   and every one of them actually *supplied* an argument there
   (`CalleeEvidence::binds_position` — an omitted argument takes the
   parameter's default, an unknown value).
2. The parameter is not at or past a **trailing** `args`. Only a trailing
   `args` is Tcl's variadic catch-all (`TclCreateProc`,
   `generic/tclProc.c`) — in `proc f {args x}` the first word is an ordinary
   parameter.
3. `command_mutations.trusts_proc_binding(qname)` — the callee's binding was
   not perturbed by a `rename` / `interp alias` / dynamic redefinition
   anywhere in *this* module (the optimiser's own O103 trust lattice, reused
   rather than duplicated).
4. The file crosses no registry-declared boundary the evidence cannot
   cover. The two kinds differ, because a host's enumeration can only bound
   one of them:
   - `PROVIDES_PACKAGE` / `EXPORTS_COMMAND` publish this file's commands as
     an API surface, whose consumers need not be in the project at all —
     another checkout can `package require` it. **Declines
     unconditionally**, workspace view or not.
   - `LOADS_EXTERNAL_UNIT` names a caller the project normally *does*
     contain, so it declines only when the host supplied no cross-file
     view.

## Opaque callers

Some callers exist but cannot be attributed argument by argument.
`record_indirect_callers` records those as *opaque* — "a call site exists
whose arguments I do not know" — which poisons every position rather than
leaving the callee looking uncontradicted:

- an `ArgRole::CommandPrefix` argument naming a known command (`after 0
  helper`, `trace add variable v write helper`, `-command helper`): the
  runtime appends words to the prefix;
- a `CommandTableEffect::RenamesCommands` / `CreatesAliases` word naming a
  known command, which is how a cross-file `rename` reaches a callee the
  single-file trust lattice in (3) above cannot see;
- a `namespace import` in another file that binds one of this file's
  commands under a new bare name.

## Accepted residual

**The workspace is the trust boundary.** A caller outside it — a `source`
target that is not a project file, another project `package require`ing this
one — is still unenumerable. That is precisely why `PROVIDES_PACKAGE` and
`EXPORTS_COMMAND` decline the seed outright rather than trusting an
enumeration that cannot cover them.

`namespace ensemble configure -map` redirection, and `uplevel #0`'s
global-resolving body (pinned by an `#[ignore]`d regression), remain open.

## Seeing it

The compiler explorer's **Unit Scope** view (`unitScope`, rendered at the top
of the Interproc pane in both GUIs) shows the boundaries crossed, whether a
cross-file view was supplied, and the per-position seed verdict for every
callee — so a surprising, or surprisingly absent, constant fold can be traced
straight back to the evidence that produced it.

```
tcl explore --show unitScope --text lib.tcl
```

## Related docs

- [command-registry.md](command-registry.md) — `CommandSpec` field reference.
- [interprocedural-analysis.md](interprocedural-analysis.md) — `ProcSummary`
  construction (a separate interprocedural product from this seed).
