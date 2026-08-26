# Design E deep dive — executable registration against the whole surface

Status: exploration, feeding the SpecTcl 2.0 surface decision and the
Rust model redesign. Companion to
[the six-design comparison](spectcl-syntax-alternatives.md) (which
defines design E and the shared model), the
[redesign proposal](dialect-and-package-registry-redesign.md) (§6
SpecTcl 2.0), and the
[centralisation contract](dialect-and-package-registry-centralisation.md).
The owner is provisionally leaning towards E; this document stress-tests
that lean by walking E through the trickiest real surfaces we ship or
target — `format`-class literal analysis, iRules against the profile
and event graph, TclOO/Tk object surfaces, tcllib, the EDA shells,
tcl-bpf, SpecTcl itself, and corpus-chosen oddities — and records what
each walk feeds back into the Rust model. The
[tricky-surfaces rubric](spec-dsl-examples/tricky-surfaces.md) remains
the acceptance checklist; this document is where design E answers it
item by item with worked spellings.

Nothing here is a decision. Where a walk forces a ruling candidate it
is numbered `E-R#` and collected in §12 for the owner.

## 1. The execution model, pinned before any example

Design E's identity is that **loading a pack is evaluating a Tcl
program** whose commands register surface. Every strength (templating,
real procs as hooks, shared helper libraries inside a pack) and every
weakness (evaluation-order surface, static opacity, trust) flows from
that. The comparison document flagged four costs; E survives the deep
dive only if each is answered structurally, not by discipline.

### 1.1 Evaluation produces a frozen snapshot — nothing else does

A pack file is evaluated **once per (pack content hash, vocabulary
version, loader-library version)** in the sandboxed tclvm, and the sole
output is a **frozen registry snapshot**: the same typed declaration
set (§4 of the redesign) that a declarative surface would have parsed
out. Everything downstream — assembly into environments, hover,
completion, the compiler, `spec build --emit rust`, *other tools* —
reads the snapshot, never the source. This converts E's "harder for
other tools to statically read" weakness into a non-issue by fiat: the
interchange artefact is the snapshot, and `tcl spec export` prints it
as design-D-shaped pure data on demand. The snapshot is also the
diffable artefact for `tcl spec upgrade --verify` (U-series), which
already compares registry snapshots rather than bytes.

### 1.2 Determinism contract

Pack evaluation runs in deterministic mode: no `clock`, no `rand()`/
`srand()`, no file/socket/exec/env access, no `after`, no introspection
of the host LSP, bounded steps and memory (the same budget machinery
that runs 1.x hook bodies). `info` is restricted to the pack's own
world. Evaluation is therefore a pure function of the file bytes and
the vocabulary — which is exactly what makes §1.1's caching sound. A
pack that exhausts its budget fails the load with a diagnostic naming
the budget axis; there is no partial registration (registration is
transactional — all or nothing per pack, matching the loader's current
pack-level atomicity).

### 1.3 Target-independence: `available?` is a trap, data rows are the rule

The one thing that must **not** vary the evaluation is the analysis
target. If control flow branches on the resolved environment
(`if {[available? {tcl 8.6-}]} { option -stride count }`), the surface
depends on evaluation *per target*, the snapshot cache keys widen from
one per pack to one per (pack × environment), and review B5's warning
("surface depends on evaluation") comes back inside our own format.
The rule: **conditionality is data, not control flow.** Every
registration command takes `-available` (a §5.4 requirement expression:
`{tcl 8.6-}`, `{package Tk 8.6-}`, `{bigip 15.1-}`), and the row is
always registered, carrying its gate; assembly resolves gates per
environment exactly as for a declarative surface. `available?` exists
for the rare legitimately structural case (a whole subtree that is
meaningless off-target), evaluates against the *union* of the pack's
declared support range, and its use downgrades the pack's cacheability
class — `spec check` reports each use as a `target-dependent
registration` notice so the cost is visible in review. (E-R1)

### 1.4 Trust: sandbox always runs, provenance gates what registration may do

Evaluation itself is safe for any provenance — it is the same sandbox
that runs untrusted hook bodies today, and §1.2 removes every
observable side channel. What provenance gates (the §6.4 trust
lattice) is **what the evaluated program's registrations are allowed to
touch**: workspace-untrusted packs cannot shadow compiled family names,
cannot alter compiled dialect axes, cannot register into reserved
namespaces, and their semantic-class vocabulary (fail-closed classes,
redesign §6.1) is enforced at registration time — a forbidden
registration is a load error naming the provenance class, not a silent
drop. So E does not raise the *execution* trust bar; it moves the §6.4
enforcement point from "parse row" to "registration call", one for one.
(E-R2)

### 1.5 Registration commands are the vocabulary, versioned as data

The words a pack may call (`command`, `option`, `class`, `method`,
`event`, `mathfunc`, `values`, `descriptor`, `package-surface`,
`family-surface`, …) are the SpecTcl vocabulary, and the existing
versioning ruling carries over unchanged: `speclib NAME 2.0 { … }`
declares the vocabulary major/minor; unsupported major fails closed;
legacy (1.x) files are *translated* into the same registration calls by
the upgrade path, never dispatched by a second loader. Because the
vocabulary words are ordinary commands in the evaluation interp, the
self-hosting story (§9) gets them for free: SpecTcl's own pack specs
them like any other command set.

### 1.6 What E buys, restated precisely

- **Templating** where the surface is genuinely regular (Tk's themed
  widgets, ticklecharts' 124 value tables, an EDA vendor's hundreds of
  near-identical commands) — with §1.3 keeping the template's output a
  fixed data set.
- **Hooks are procs**, defined next to the commands they serve, shared
  through pack-local helper namespaces, unit-testable by running the
  pack file under plain tclsh with a stub vocabulary.
- **One migration story**: every other design's files are valid E input
  once their forms are accepted as data by the registration commands —
  which is also what `tcl spec upgrade --restyle` emits.

## 2. `format`, `scan`, `binary` — literal-driven analysis as the type test

The `format` family is the cleanest test of the typing contract
(alternatives doc, "Typing arguments and options") because the
interesting types live *inside a literal argument*, and because three
commands share the analysis: `format` reads a format string and types
its trailing arguments; `scan` reads one and **types the variables it
writes**; `binary format`/`binary scan` do both over a different spec
grammar with byte-layout facts.

In E the shared analysis is a pack-local helper library plus three
`types` hooks that are ordinary procs:

```tcl
namespace eval spec::fmt {
    # Pure Tcl, runs in the same sandbox as every hook; unit-tested by
    # sourcing this file under tclsh with the stub vocabulary.
    proc specs {fmt} { … }              ;# -> list of {verb type} pairs
    proc spec-type {spec} { … }         ;# %d -> int, %s -> string, %f -> double
    proc scan-writes {fmt} { … }        ;# -> list of written-var types
}

command format {formatString:string ?arg:any ...?} {
    returns string
    types -inputs {literal formatString} {call} {
        set i 0
        foreach spec [spec::fmt::specs [literal formatString]] {
            type [list arg $i] [spec::fmt::spec-type $spec]
            incr i
        }
    }
}

command scan {string:string format:string ?varName:varname ...?} {
    returns int   ;# conversions performed (or scanned values under -inline, 8.5-)
    roles {call} {
        set i 0
        foreach t [spec::fmt::scan-writes [literal format]] {
            role [list varName $i] var-write
            incr i
        }
    }
    types -inputs {literal format} {call} {
        set i 0
        foreach t [spec::fmt::scan-writes [literal format]] {
            type [list varName $i] [list varname $t]   ;# varname(int) etc.
            incr i
        }
    }
}
```

The points the walk establishes:

- **The bespoke fields die.** 1.x's `format_string_type`,
  `pattern_type` and `var_write_typing` were single-purpose escapes for
  exactly this shape; under E they are a helper namespace plus general
  `types`/`roles` hooks, and the Rust model keeps only the general
  hook seams. (Feeds §11.)
- **Abstention is the common path.** A non-literal format string means
  the `-inputs` guard never fires, the static synopsis types stand
  (`?arg:any ...?`, every scan var `varname(any)` but still
  `var-write`), and nothing is guessed. Same discipline as the
  alternatives doc's narrow-never-widen rule.
- **Version deltas stay data.** `%b` (8.6-), `%ld` width modifiers,
  `scan -inline` (8.5-) are `-available` rows on *values inside the
  helper's table*, expressed as the helper consulting a
  `values format-specs` table registered with per-value gates — so the
  range-targeting engine (§5.4) sees `format "%b" $x` under a
  `tcl 8.5-9.0` target and flags the 8.5 side without any hook running
  per target: the hook emits the *value use*, the gate check is
  assembly's. (E-R3: hooks emit facts referencing gated vocabulary;
  they never evaluate gates themselves.)
- **`binary` adds layout, not new machinery** — its helper returns
  byte-layout facts (`element_structure`, storage kinds) through the
  same `types` verb extended with the existing layout vocabulary, and
  the KaiWilke RADIUS corpus (heavy `binary scan` on packet payloads)
  is the acceptance workload: every `binary scan [UDP::payload] …` in
  those iRules must type its written variables when the template
  literal is present.

<!-- §3-§10 are completed from the domain harvests; §11-§12 collect
     the Rust-model feedback and ruling candidates. -->
