# Design E deep dive — executable registration against the whole surface

Status: **ADOPTED** (owner, 2026-08-26) and **IMPLEMENTED** (2026-08-27) —
design E is the SpecTcl 2.0 authoring surface, together with §1's execution
model as a package. Rulings E-R1–E-R9 in §14 are ratified; E-R10 records
the standing known limits; **E-R11, E-R12 and E-R13 were implemented as
proposed and are still marked "(proposed)" in §15 — their formal
ratification is the one thing this document leaves undone**, tracked as
O3 in the
[redesign's §11 open-questions ledger](dialect-and-package-registry-redesign.md#11-the-open-questions-ledger).
The two questions this document flags for the owner — the option
*requires* relations and the M9 dead axes — also remain open, as O1 and O2
there. Everything else in §15 has landed; §15.4 carries the tick list.

Originally written as exploration feeding the surface decision and the
Rust model redesign. **§14's verdict was tested rather than assumed**: the
evaluation loader (P2-I) proved the equivalence gate over all 24 shipped
packs, 1,515 commands, byte-identically through both loaders — and, that
proved, the CST loader was **deleted** (centralisation ledger row L1, the
`one-loader` lane), leaving `evaluate_pack` the single door every consumer
loads a pack through and the golden-snapshot gate as the standing proof. Companion to
[the six-design comparison](spectcl-syntax-alternatives.md) (which
defines design E and the shared model), the
[redesign proposal](dialect-and-package-registry-redesign.md) (§6
SpecTcl 2.0), and the
[centralisation contract](dialect-and-package-registry-centralisation.md).
*(Written before the decision:)* the owner is provisionally leaning
towards E; this document stress-tests
that lean by walking E through the trickiest real surfaces we ship or
target — `format`-class literal analysis, iRules against the profile
and event graph, TclOO/Tk object surfaces, tcllib, the EDA shells,
tcl-bpf, SpecTcl itself, and corpus-chosen oddities — and records what
each walk feeds back into the Rust model. The
[tricky-surfaces rubric](spec-dsl-examples/tricky-surfaces.md) remains
the acceptance checklist; this document is where design E answers it
item by item with worked spellings.

*(Also written before the decision:)* nothing here is a decision. Where a
walk forces a ruling candidate it
is numbered `E-R#` and collected in the final section for the owner;
where it forces a change to the Rust model it is numbered `M#` and
collected in §13. Every file:line cited below was verified against the
working tree during the survey behind this document — a snapshot of
2026-08-26, before the #1631 phases moved much of what it cites.

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

Worked, because it is the acceptance shape: the `binary scan` spec and
what it derives at a real call site from that corpus.

```tcl
namespace eval spec::binfmt {
    proc fields {fmt} { … }        ;# "cH2SS" -> {{c 1} {H 2} {S 1} {S 1}}
    proc field-type {f} { … }      ;# c -> int, H -> string, S -> int, a -> bytes
}

command {binary scan} {string:bytes formatString:string ?varName:varname ...?} {
    returns int    ;# count of conversions performed
    byte-array-effect -reads string
    roles -inputs {literal formatString} {call} {
        set i 0
        foreach f [spec::binfmt::fields [literal formatString]] {
            role [list varName $i] var-write
            incr i
        }
    }
    types -inputs {literal formatString} {call} {
        set i 0
        foreach f [spec::binfmt::fields [literal formatString]] {
            type [list varName $i] [list varname [spec::binfmt::field-type $f]]
            incr i
        }
    }
}
```

At the RADIUS-stack call site

```tcl
when CLIENT_DATA {
    binary scan [UDP::payload] cH2SS code ident length attrs
    if { $code == 1 } { … }
}
```

the hook fires (the format string is literal), `code`/`length` type as
`int` and `ident`/`attrs` as `string`, the `$code == 1` comparison
type-checks, and all four names are SSA-visible writes — while a
`binary scan $data $fmt …` site abstains to the static floor: every
trailing var is `var-write` of `varname(any)`, nothing guessed.

### 2.1 The `constraints` hook contract (E-R14)

The `types` contract above is the pattern, and the `constraints` hook —
E-R14's escape hatch for an option relation no declarative row can
express — follows it **exactly**, for the same reason: principle P-B
says a script must not run on every edit.

**It is reached last, and usually not at all.** The analyser evaluates
every declared `option_conflict` / `option_requires` /
`option_requires_one_of` / `option_forbids` relation natively —
`OptionRelation::evaluate` is a few slice scans over facts the option
walk already produced — and only asks the hook when a spec declares one
*and* every relation reported nothing. No shipped command declares one,
so no shipped command's call site ever enters the VM. Measured on a
40-call-site option-heavy corpus: **0 of 33 option-bearing call sites
entered tclvm.** When a declarative row and a hook could both say
something, the declarative row wins; a hook that restates one is a
review comment.

**Declared inputs, and content caching.** A body writes
`constraints -inputs {invocation} {words ctx} { … }`. `invocation` is a
*content* input: it earns `CacheMode::Content`, whose key is the call's
shape plus a hash of its option names, its option values, its positional
words and its `complete` flag. That is what makes "an edit elsewhere in
the document must not re-run hooks for unrelated call sites" true rather
than hoped: an unchanged call site hashes the same and is answered from
the memo. An *undeclared* hook made no claim about what it reads, so
nothing can be hashed on its behalf and it stays uncached — the
conservative direction.

**Abstention is explicit.** `ctx`'s `complete` key is false when the
call carried a `{*}` expansion or a substituted word where an option
could have been. A body tests it and calls `abstain`, which cancels
every report it had already made — a body that discovers half way
through that it cannot judge the call must be able to *withdraw*, not
merely stop. Error means abstain: a hook that raises, blows its budget,
panics, is quarantined, or has no host answers silence, like every other
family.

**The verbs are ones that already exist.** Four readers and two
emitters, no new spellings invented (principle P-E):

| verb | kind | answers / does |
|---|---|---|
| `option-present OPTION` | reader | whether the call supplied it |
| `option-value OPTION` | reader | its first literal value word, or empty |
| `literal N` | reader | positional word `N`, or empty when not statically known — the same spelling the `types` hook reaches a literal argument by |
| `arg-count` | reader | how many positional words the call supplied |
| `invalid SLOT MESSAGE ?-conflict?` | emitter | one report. `SLOT` is an option spelling, `arg N`, or `command`; `-conflict` selects W147 ("these cannot go together") over the default W152 ("this one needs that one") — the only structural choice a hook makes |
| `abstain` | emitter | withdraw: this call cannot be judged |

The analyser owns the diagnostic code and the span; the hook says only
*where* and *what*. That asymmetry is deliberate — it is what keeps a
pack from minting diagnostics.

A worked body, for a rule the vocabulary genuinely cannot state (a
*numeric* comparison between two option values):

```tcl
constraints -inputs {invocation} {words ctx} {
    if {![dict get $ctx complete]} { abstain }
    if {![option-present -min] || ![option-present -max]} { return }
    set lo [option-value -min]
    set hi [option-value -max]
    if {$lo eq {} || $hi eq {}} { abstain }
    if {$lo > $hi} { invalid -min "-min $lo exceeds -max $hi" }
}
```

Everything the four declarative statements *can* say — including
`bibtex::parse`'s three relations, `http::geturl`'s conditional pairs,
and `struct::tree walk`'s `-order in` versus `-type bfs` — is declared,
not hooked.

## 3. iRules against the profile and event graph

The iRules surface is the strongest evidence that the dialect layer is
*data all the way down* — and the strongest indictment of keeping that
data in Rust. Today the event graph lives in `events.rs` as a table
split across **21 hand-chunked functions** purely to dodge function-size
lints (`events.rs:1364-3213`); the module doc claims 247 events while
the table holds 176, and `profiles.rs` claims 57 profile types / 87
namespaces while holding 66 / 113. Nothing checks the prose against the
data because the prose *is* the only other copy. A dialect pack erases
the artefact and the drift at once: the counts become whatever the data
says, and doc-comments about the data are generated from it (M13).

### 3.1 The event graph as pack data

Everything `EventProps` carries is declarative already — the E surface
just spells it:

```tcl
family-surface f5-irules {
    event HTTP_REQUEST -side client -transport tcp \
        -profiles {HTTP FASTHTTP} -flow -hot -common \
        -multiplicity per-request
    event SERVER_DATA -side both -command-side server \
        -transport {tcp udp} -setup-event SERVER_CONNECTED \
        -collect-protocols {TCP UDP} -collect-side server
    event PERSIST_DOWN -available {bigip 11.5.0-15.0.0}

    firing-order {
        RULE_INIT
        FLOW_INIT
        CLIENT_ACCEPTED -gated-by {FASTL4 TCP}
        ...
    }

    data-protocol TCP -payload explicit-collect -release explicit \
        -bootstrap {CLIENT_ CLIENT_ACCEPTED SERVER_ SERVER_CONNECTED}
    data-protocol UDP -payload not-required -release not-applicable

    event-priority-policy -keyword priority -default 500
}
```

The walk surfaces four facts a naive port would have lost, all now
explicit in the vocabulary: `command_side` is a **third** side axis
(`SERVER_DATA` fires on both sides but side-sensitive commands take the
server side, `events.rs:2865`); an event can have properties but no
firing-order slot (176 props vs 74 order entries — the two are separate
declarations, not one row); multiplicity is currently three closed
*sets* rather than a field (`events.rs:938,3539,3559`) and becomes a
per-event field with `unknown` as the default; and the flow-chain
tables' `condition_note` is free prose that states collect-gating in
English ("Requires TCP::collect in CLIENT_ACCEPTED") — the pack form
types it as a reference to the `data-protocol` row instead, so the
quick-fix that inserts the collect call reads data, not a sentence.

Two axes come out of the walk **dead**: `ProfileSpec::capabilities` and
`::conflicts` are empty for every current profile and no command sets
`EventRequires::capability`; `EventRequires::init_only` is set by no
spec (the four commands that need exclusion use `excluded_events`).
Dead axes do not get vocabulary — each is a delete-or-populate question
for the owner (M9), because carrying a word no data uses invites packs
to guess at semantics the engine never implements.

What stays Rust: the stack algebra (`expand_profile_stack`'s transitive
`requires` closure and `stack_satisfies`' OR-over-candidates /
AND-within-expansion rule, `profiles.rs:200,219`), the execution-context
machine (`IrulesExecutionContext` and friends), and the lexer fact that
a `when` body must be one closed braced source word. Packs declare
values on axes; algorithms over those values are engine semantics —
the same closed-vocabulary boundary as the dialect axes (§6 of the
redesign).

### 3.2 Command-level facts — mostly data already, and the residue is E's home turf

Only **3 of the 984 shipped iRules specs carry any Rust hook** (`when`,
`call`, `after` — each an `arg_role_resolver`). The other 981 are pure
data, which is why the iRules walk is less about expressibility and
more about the odd corners:

- **Argument-prefix event contracts.** `MQTT::payload` carries five
  per-literal-prefix event requirement forms where the *empty prefix
  means the exact no-argument form*, not a wildcard
  (`mqtt__payload.rs:100-147`, matcher `events.rs:814`), plus a
  *separate* prefix-keyed payload-availability table where longest
  prefix wins and dynamic arguments abstain (`events.rs:148,560-583`).
  Both are rows in E; the abstention rule is the matcher's, not the
  pack's.
- **Suppression as data.** `FIX::tag map` declares `requires: None`
  *specifically to suppress* the namespace→profile fallback
  (`fix__tag.rs:58-64`). The E spelling makes the suppression visible:
  `event-requires -none` is a distinct word from omitting the clause.
- **Taint duals and abbreviation.** `HTTP::header` is source and sink
  at once, sink-gated to `{insert, replace}` — and membership is tested
  *after* ensemble-abbreviation canonicalisation (`HTTP::cookie ins` →
  `insert`, `taint.rs:594-606`). The canonicalisation is engine
  semantics; the pack writes only subcommand names.
- **Bulk trait application belongs in the pack.** Today
  `Traits::REQUIRES_HTTP_CONTEXT` reaches 32 commands through a wrapper
  function at the registration site (`commands/irules/mod.rs:1055,
  1478-1509`) — invisible from any single spec file. Under E the same
  thing is a visible `foreach` in the pack over a named command list
  (E-R5): templating makes the bulk edit *reviewable* instead of
  hidden, which is precisely the legitimate use of evaluation §1.3
  carves out.
- **The `when` tail grammar comes out of Rust.** The
  `priority N` / `timing enable|disable` keyword tail and its 0..=1000
  range are today a hardcoded match in `registry.rs:3163`; the bare
  `priority`/`timing` file-level statements parse at `registry.rs:3112`.
  Both become clause forms on `when` and value rows on the two
  statements, with only the *retroactive* semantics (a bare `priority`
  changes handlers that follow it) staying an engine rule keyed by the
  declared `irules_top_level_effect`.

Worked: the full `HTTP::header` entity in E, restyled from the 1.x
draft (`spec-dsl-examples/irules-http-header.tclspec`), plus the two
Rust-resident grammars coming out:

```tcl
family-surface f5-irules {
    command HTTP::header {subcommand ?arg ...?} {
        traits {PURE CSE_CANDIDATE DIAGRAM_ACTION}
        event-requires -transport tcp -profiles {FASTHTTP HTTP} \
            -also-in {MR_EGRESS MR_INGRESS SERVER_CONNECTED}
        taint -source tainted
        taint -output-sink IRULE3002 -subcommands {insert replace}
        side-effect HttpHeader -reads -writes -side both
        hover {
            Inspect or mutate HTTP headers in an iRule event. Use
            subcommands like `value`, `insert`, `replace`, `remove`.
        }
        subcommand value  {name}  { returns string }
        subcommand exists {name}  { returns boolean }
        subcommand insert {name value ?name value ...?} {
            credential-arg 2
            sensitive-headers {Authorization Proxy-Authorization Cookie}
        }
        subcommand sanitize {?header ...?} -available {bigip 11.6.0-}
    }

    command when {EVENT body} {
        top-level-only
        traits {IS_EVENT_HANDLER DEFERS_BODY}
        param EVENT -role event-name
        param body  -role event-body -structural
        defines-symbol EVENT -kind event
        # The keyword tail hardcoded at registry.rs:3163 becomes forms:
        form {when EVENT body}
        form {when EVENT priority N body}   { param N -type int(0-1000) }
        form {when EVENT timing MODE body}  { param MODE -values {enable disable} }
        lowering -native When
    }

    # Bulk trait application, visible instead of a Rust wrapper
    # (commands/irules/mod.rs:1055): the list is reviewable pack text.
    proc http-txn-command {name synopsis body} {
        command $name $synopsis [concat {traits REQUIRES_HTTP_CONTEXT} $body]
    }
    http-txn-command HTTP::respond  {status ?content? ?option value ...?} { … }
    http-txn-command HTTP::redirect {url} { … }
    # …30 more
}
```

Two spellings matter more than they look. `event-requires -none`
(`FIX::tag map`'s fallback suppression) is a distinct word from an
absent clause — abstention vs omission, typed (M4). And
`-also-in`/`-excluded` are separate axes on the same clause, matching
the shipped model's separation, so `TCP::payload`'s SIP escape hatch
and `HA::status`'s `RULE_INIT` exclusion read as what they are.

The **fourth BIG-IP version axis** the walk found —
`profile_defaults/generated.rs`, 2,213 lines of per-TMOS-release profile
field defaults — slots into the keyed-axis mechanism the EDA
environments already use (§10), not into `Lifecycle`; conflating them
would give a *default value change* the semantics of an availability
change. And the closed-world rule stays exactly where B12 put it: the
pack declares what exists; whether an environment treats the
virtual-server's profile set as exhaustive is overlay policy.

**Live evidence, post-scriptum.** After this walk was written the owner
landed the appliance measurements
([bigip-irule-parser-measurements.md](bigip-irule-parser-measurements.md)),
which harden several rows from modelled to measured: the
`event-priority-policy` default of 500 is confirmed on the wire (lower
priority runs first, range 0–1000 enforced at load with a misleading
diagnostic worth mirroring); the event-context validity matrix is
compile-time enforced by the real compiler, with the caveat that
`RULE_INIT` *accepts* protocol commands it cannot meaningfully run —
so our event requirements should keep warning there even where the
compiler is silent; the 31-command disabled list becomes measured
environment-policy data (including `trace` being 8.3-form-only, an
`arity_windows`-grade fact); and the two lexical divergences (the
implicit word break and the newly discovered brace-line continuation)
are `dialect`-block axis values for the F5 family packs, not
anything a `family-surface` command row ever expresses.

The measurements also restructured the F5 family tree itself (owner
rulings, 2026-08-26): the tmsh and iApp interpreters are the **same**
8.4.6-offshoot parser as iRules, so the trunk grammar (R-rules,
N-rules, inert `{*}`) belongs to a shared family `f5-tcl`, while
`f5-irules` remains a **dialect offshoot of that trunk** — a fork of a
fork — keeping its own parse-level fingerprint (the
`when`/`proc`/`priority`/`timing`-only top level and the rule
compiler's load-time strictness; the expr word operators proved to be
trunk grammar, measured byte-identical in tmsh and iApp). The
dynamic-code measurements (§4c) pin how the offshoot's load-time rules
must be modelled: they are lexical scans of braced script literals
(recursing through `eval {…}`/`uplevel #0 {…}`, escaped by
variable-held text), runtime-defined procs are persistent per-TMM
globals, `when` does not exist at runtime, and unbraced `if $var` —
the sole user-space JIT primitive — is pinned as warning-severity with
the `static::`-cached-expression idiom recognised. For this document's examples
the change is nomenclature, not structure: the `dialect` blocks split
into a trunk pack and an offshoot pack that inherits along the fork
edge, the §3 `family-surface f5-irules` block attaches its command
surface to the offshoot's environment, and §4's iApps/tmsh surfaces
ride the trunk directly (scriptd additionally as a 32-bit build
profile — the CoreProfile build axis earning its keep a second time
after Jim's `--minimal`).

## 4. iApps and tmsh — the surface E is cheapest for

The iApps walk is short because the modelling is: 41 specs, almost all
`Arity::at_least(0)` with a synopsis string — existence, not
description. The structure worth keeping is the dialect union
(`tmsh::*` loads under both the iApps environment and the standalone
tmsh shell), which in 2.0 is simply a `package-surface tmsh` block that
both environments list as ambient — no bit-set union spelled anywhere.

What the walk adds to the requirements list: `script::init` /
`script::run` / `script::help` / `script::tabc` are *template lifecycle
entry points*, not events — a distinct declaration («this proc name is
called by the host at phase X») that today has no field at all; and
`tmsh::begin_transaction` / `commit` / `cancel` bracket state the model
cannot see. Neither warrants new machinery yet: the entry points are
`defines-symbol`-grade facts plus a context gate, and transaction
bracketing joins the world-state axis question (§10, M2). The tmsh
syntax-version axis and its temporal transition are already ruled in
the redesign; iApps' `.apl` embedded-Tcl callbacks stay behind the P4
hold with the rest of the F5 evidence layer.

The whole surface in miniature — entry points, shared tmsh, and the
transaction bracket as context transitions rather than prose:

```tcl
package-surface iapp {
    entry-point script::init -phase template-load
    entry-point script::run  -phase deployment
    entry-point script::help -phase help-render
    entry-point script::tabc -phase tab-completion
    command iapp::conf {?arg ...?} {
        hover {Read or write iApp configuration state for the current
               application instance.}
    }
}

package-surface tmsh {
    # Listed as ambient by BOTH the iapps environment and the
    # standalone tmsh shell environment — the DialectSet union
    # (IAPPS ∪ TMSH) disappears into placement.
    command tmsh::begin_transaction {} \
        -context-transition {transaction open}
    command tmsh::commit_transaction {} \
        -requires-context {transaction open} \
        -context-transition {transaction closed}
    command tmsh::create {module component ?property value ...?}
}
```

`-requires-context`/`-context-transition` are the M2 context bag
speaking: `transaction` is one axis in the same typed vocabulary that
carries iRules' execution contexts and the EDA project state (§10) —
declared by the engine, referenced by packs, never invented by them.

## 5. tcl-bpf — the stress limit for "dialect"

tcl-bpf is the most instructive extreme: a surface that *parses* as Tcl
but whose semantics are eBPF. Twenty-six commands in four layers, every
one carrying a typed `BpfOpSpec` descriptor the front-end dispatches on
— never the name (`spec.rs:1477`). Seven of its words (`when`, `drop`,
`use`, `next`, `map`, `pass`, `loop`, `profile`) collide with
iRules/Tcl/Tk/tcllib commands, and the *only* disambiguator is the
dialect bit — which is exactly what the environment layer replaces: a
`bpf` environment whose contributed language identity resolves the
name to the BPF declaration, with no bit arithmetic anywhere (R-a).

What the walk establishes for the spec surface:

- **`bpf_op` stays a closed Rust vocabulary, referenced by name.** The
  descriptors carry codegen semantics (verdict kinds, effects masks,
  context struct offsets); a pack row says `-bpf-op setu32` the way a
  hook row says `-native Foreach`. A pack cannot invent an op, only
  bind a word to one — same soundness boundary as the lexer axes.
- **Two capability lattices, kept apart on purpose.** Per-command
  `BpfEffects` (drives the `allow`/`deny` lists) and per-event
  `BpfEventCaps` are overlapping-but-unequal bitsets (`bpf_op.rs:38,
  363`). The E surface declares both as named flag sets on their
  respective rows and does *not* try to unify them — the walk's lesson
  is that a spec format must be able to carry two different capability
  vocabularies on two different declaration kinds without inventing a
  common super-lattice.
- **Verdict legality is a program-type set, not an event reference**
  (`accept` = socket-filter only, `tx` = XDP only, `pass` = XDP|TC|
  cgroup, `drop` = all) — and `next` is a *non-terminal verdict* whose
  meaning ("run the next handler in priority order") is composition
  semantics, excluded from every event's verdict list yet always
  permitted. Both are data rows plus one engine rule.
- **The event schema is richer than iRules'** — ELF section names,
  typed context structs with real byte offsets and endianness, kernel
  version minima with BTF/bpf-link flags, attach parameters. All of it
  is declarative, and the kernel minimum is another **keyed version
  axis** (`KeyedAxis`-shaped, like SDC/UPF/tool versions), not a
  `Lifecycle`.
- **The gap is authoring economics, not expressibility.** No BPF spec
  today carries `hover`, `forms`, `arg_roles`, or `body_kind` — even
  the four brace-bodied commands — and the argument mini-grammars
  (`be|le|native`, `hash|array`, `shared|percpu`, `key=value`) live in
  doc comments. That is what a cheap, Tcl-native authoring surface
  fixes: the BPF pack is small enough that a complete E rewrite is an
  afternoon, and it doubles as the acceptance test that a
  *non-Tcl-semantics* dialect pack round-trips through
  `spec build --emit rust`.

The rewrite, sketched — an event with its typed context struct, and
the op-referencing command shape:

```tcl
family-surface bpf {
    event XDP -elf-section xdp -prog-type xdp -default-verdict pass \
        -caps {PKT_READ CTX_READ RINGBUF_OUTPUT} \
        -attach {interface direction} \
        -kernel {4.8 -btf} {
        context xdp_md {
            field data            -offset 0  -width 32 -order host
            field data_end        -offset 4  -width 32 -order host
            field ingress_ifindex -offset 12 -width 32 -order host
        }
    }

    command load16 {offset:int} -bpf-op load16 -effects {PKT_READ} {
        returns int
        hover {Loads a big-endian 16-bit value from the packet at the
               given byte offset — e.g. the EtherType at offset 12.}
    }
    command tx   {} -bpf-op tx   -verdict -prog-types {xdp}
    command drop {} -bpf-op drop -verdict
    command next {} -bpf-op next -verdict -non-terminal \
        { hover {Yield to the next handler in priority order; the
                 event's default verdict applies if every handler
                 yields.} }
    command map {name:name kind spec ?flag ...?} -bpf-op map {
        param kind -values {hash array}
        option -shared
        introduces -command name -class bpf-map
    }
}
```

`-bpf-op` binds a word to a closed Rust descriptor; `-effects` and the
event `-caps` are the two capability vocabularies of the walk, each on
its own declaration kind; the map's `introduces` row is M6's unified
command-creation vocabulary doing what the current BPF specs cannot
say at all.

## 6. TclOO, itcl, snit — one class model, currently split in half

The definer model is the registry's best machinery — 14-field grammars,
three member kinds, slot fold-operations pinned to C Tcl behaviour,
manufacturer tables, retraction shapes, per-member 9.0 gates — and the
walk's headline is structural: **Tk and the class systems use disjoint
halves of one object model and nothing bridges them.** Class systems
use `definition_body` + `manufacturers` + `defines_command_at` and
never `object_class`; Tk widgets use `object_class` +
`creates_instance_at` and never a member grammar. The consequence is
symmetric poverty: an `oo::class`-defined class never gets an
instance-method table, and itcl/snit declare grammars whose
construction machinery is absent (`itcl_class.rs:38-58` sets no
`creates_instance_at`, no `object_class`). M5 rules the fix: **one
`ClassSurface`** that any command may carry, holding definer grammar,
instance methods, manufacturers, superclasses, and unknown-dispatch
policy together — Tk widgets and TclOO classes instantiate the same
shape, and every command-grade fact becomes legal on a method (which
retires gap G7/G15's "command-only fields with method-shaped examples"
wholesale rather than field by field).

In E the class surface is where executable registration reads most
naturally, because a class body *is* a scope:

```tcl
class tree -prefix-matching strict -allow-unknown-methods {
    method walkproc {node ?-order order? ?-type type? cmdprefix} {
        option -order order -values {pre post in both}
        param cmdprefix -role command-prefix -timing deferred \
            -appended {action tree node}
    }
    method walk {node ?-order order? ?-type type? loopvar script} {
        param loopvar -role loop-var-list
        param script  -role body -completion-codes {5 prune}
        traits HAS_LOOP_BODY
    }
}

definer snit::type -family snit {
    member method      {name:name params:paramlist body:script}
    member typemethod  {name:name params:paramlist body:script}
    member delegate    -keyword-only -dynamic-dispatch
    member-body-command install     -binds-handle {name 0 class 2 -keyword using@1}
    member-body-command installhull -binds-handle {name -implicit hull class 1 -keyword using@0}
    bare-word-construction -values {%AUTO%} -prefixes {.}
}
```

The walk confirms the de-hooking trend the DSL ports already started:
`oo_class_arg_roles` is derivable from the manufacturer table (spelled
`arg_role_resolver from-manufacturers` in the port), and snit's
bare-word predicate reduces to `-values`/`-prefixes` data. Three things
remain genuinely native and stay `-native` references: the four class
factory state-transition resolvers (typed `CommandBinding` /
`ObjectDispatch` facts the DSL has no vocabulary for — deliberately),
and the analyser's `OoDefine`/`OoObjdefine` hooks for the **inline**
definition form. That inline form is the known generic-coverage gap the
rubric names, and M5 closes it structurally: the inline
`oo::define C method m {} {}` spelling is *derived from the same member
table* as the body form — one grammar, two projections, the same
never-author-twice rule as `tcl::mathop` (the member table is the
entity; body-form and inline-form are its projections).

Smaller corners the E surface must keep, each already proven as data:
flag-keyed members with optional named bodies (`property NAME ?-get
BODY? ?-set BODY?` — gap G12's `{name, params, body}` misfit becomes a
member layout kind, not a special case); declaration-time vs
after-the-fact visibility as distinct vocabularies; slots as fold
operations with C-pinned defaults, all six ops accepted under every
dialect by explicit modelling choice; `oo_context_facts` keyed on the
*word* (`self class` folds, the other eight words provably don't —
oracle inline at `oo_self.rs:139-150`); and `self` deliberately
avoiding a `subcommands` table because a consumer's type-inference
special-cases non-empty tables (`oo_self.rs:148-150`) — which is not a
fact about `self` at all but a consumer contract leaking into a spec,
and goes on the M14 list.

### 6.1 Resolving the split: one surface, three declaration sources

The split looks like two models of "class", but it is really two
*declaration sources* for one thing. Tk's instance surface is known
statically, so it lives in pack rows; TclOO/snit/itcl's instance
surface is declared by *user script*, so the registry ships a grammar
for reading those scripts — and then never stores what it read
anywhere a consumer can query. The fix is not to pick a winner but to
make the second source feed the first's shape:

**One `ClassSurface`** — identity, superclasses, manufacturers, an
optional definer grammar, and an *instance surface* (methods, one
option table, unknown-dispatch policy, prefix matching). **Every
`MemberSpec` gains a typed projection** stating what one member row
*contributes* to the instance surface: `method m` contributes a
method; `property p` contributes an option plus its accessor method;
`forward f target` contributes a method that resolves through
`target`; `superclass S` contributes an inheritance edge; `delegate *`
contributes the surface-level `Unknown` default. The definer grammar
stops being a sibling model and becomes a **parser producing instance-
surface increments**, which the realm state accumulates as bindings
with the same `BindingKnowledge` lattice commands use: a member seen
in a static `oo::define` body is a `Must` method binding; one behind
`delegate`/`oo::objdefine` with a computed name is `May`/`Unknown`.

```tcl
# Source 1 — pack rows (Tk, ticklecharts): the surface is closed data.
class Button -widget {
    option -background color -alias -bg
    method invoke {} -effect {invokes -command}
}

# Source 2 — the definer grammar, now carrying projections: the pack
# declares how *user scripts* populate the same surface shape.
definer oo::define -family tcloo {
    member method {name:name ?visibility? params:paramlist body:script} \
        -projects {method $name -params $params}
    member property -flag-keyed {name:name ?-get body:script? ?-set body:script?} \
        -available {tcl 9.0-} -implicit-vars {value} \
        -projects {option -$name -accessor configure}
    member forward {name:name target:commandname ?arg ...?} \
        -projects {method $name -forwards-to $target}
    member superclass -all-refs class -projects {superclass @refs}
}

# Source 3 — derivation (E-R4): manufacturers yield construction,
# projections yield the inline form, the option table yields
# configure/cget. None of these is ever authored.
```

What falls out, case by case:

- `oo::class create Account { method deposit {amount} {…} }` finally
  yields a queryable instance surface: `set a [Account new]` binds `a`
  as an `Account` handle (the existing `binds_handle` machinery),
  `$a deposit 10` resolves as a `Must` method, and `$a withdraw` is
  `Absent` — an unknown-method diagnostic with the same evidence
  discipline as W123, impossible today because the class never gets an
  `object_class`.
- The **inline** `oo::define Account method audit {} {…}` form is the
  same member row arriving through a different projection of the same
  grammar — the analyser-hook gap closes generically, not per command.
- itcl and snit gain the construction machinery their grammars already
  imply: the manufacturer table drives instance creation (B13's
  `from-manufacturers`, promoted from a DSL spelling to the model
  rule), so `itcl::class Toaster {…}; Toaster t1` binds `t1` without
  the currently missing `creates_instance_at`.
- Tk inheritance becomes real data: `class TtkButton -superclass
  TtkWidget` replaces the `ttk_widget_class!` macro's textual
  re-emission, and the registry's existing inherited-method walk —
  written for `superclasses`, used by nobody — gets its first user.
- Instance `configure`/`cget`/creation all project the one option
  table, closing the `.b configure -bogus` validation hole (§7)
  without a second table to drift.
- Honesty is preserved where the surface genuinely is open: snit's
  `delegate * to hull` and G11's computed `oo::objdefine` names
  project `Unknown`, so unknown-method diagnostics abstain exactly
  where they must — the lattice, not a boolean
  (`allow_unknown_methods` retires into it, M9).

The cost is one honest asymmetry, kept visible rather than papered
over: a pack-declared surface is complete at load time, while a
script-declared surface is only as complete as the analysis of the
scripts seen so far — which is precisely what `BindingKnowledge`
exists to say. The realm state already tracks command bindings this
way (§4.2 of the redesign); methods become the same machinery scoped
to a class, and R-c's one `exists` oracle answers both.

## 7. Tk — options as they are really used

Tk is death by a thousand well-modelled cuts, and the walk mostly found
the *asymmetries*:

- **`-bg` is a separate `OptionSpec` at 33 sites** while the `aliases`
  field whose doc-comment names exactly this case is used once
  (`hover.rs:611-617`). The E pack spells `option -background color
  -alias -bg` and the upgrade tool folds the 33 duplicates
  mechanically — a pure win with no model change.
- **Instance `configure`/`cget` carry no option table**, and subcommand
  option lookup never falls back to the class's own table
  (`registry.rs:3647-3675`), so `.b configure -bogus 1` passes
  unvalidated today despite `button`'s complete OPTIONS list. Under M5
  the class surface owns one option table and `configure`/`cget`/
  creation all project it — the trailing-option forms
  (`CONFIGURE_FORMS`) stay, the duplication goes.
- **Widget inheritance is macro-expanded, not modelled** —
  `ttk_widget_class!` textually re-emits six methods into every ttk
  class while `superclasses` sits empty everywhere. E's templating
  could replicate the macro, but the right move is the opposite: real
  `-superclass` data and the registry's existing inherited walk. Not
  everything regular should be a template (E-R5's boundary: template
  what is *bulk*, declare what is *structural*).
- **The `%`-substitution story is half a model.** What exists is taint
  data — which `%` letters inject externally-controlled text
  (`CallbackTaintInput::TkPercent`, deliberately only `%P %s %S %A
  %K`). What doesn't exist is the per-slot substitution *vocabulary*
  (which letters are legal in a `bind` script vs an entry
  `-validatecommand`), even though the DSL drafts already spell
  `-substitutions {%n node %t tree}` for `struct::tree walk`. M15
  makes the substitution table first-class per callback slot, with the
  taint subset a marked projection of it — one table, two consumers,
  and the three copy-pasted `bind`-shaped resolver pairs
  (`bind.rs:40-54`, `canvas.rs:31-45`, `wm.rs:32-36`) collapse into
  one declarative `event-bound-script` slot shape.
- **The literal mini-language problem again.** Canvas item types and
  their per-type option tables, text indices (`line.char`, `end`,
  modifier chains), `wm geometry`'s `WxH+X+Y` — all opaque strings
  today. These are the same shape as `format` (§2): a pack-local
  helper plus `types`/`roles`/`values` hooks guarded on literal
  inputs, with the static synopsis as the abstention floor. Text
  indices get an `index(text)` static type the helper narrows.
- **Commands that create commands, again.** `image create photo im1`
  and `font create` leave their created name unknown because the
  optional leading name defeats a fixed `defines_command_at` — the
  unified `introduces` vocabulary (M6) takes a *resolved position or
  pattern*, not a fixed index, and covers `zlib stream`,
  `namespace ensemble create -command`, and interp paths in the same
  stroke.

The Tk walk in E spellings — the alias fold, the `-textvariable`
tiers, and the one slot shape that replaces three copy-pasted resolver
pairs:

```tcl
package-surface Tk {
    class Button -widget -superclass Widget {
        option -background color -alias -bg
        option -borderwidth screen-distance -alias -bd
        # A button's -textvariable reads application state but is not
        # an input source; contrast Entry below (hover.rs:403-432).
        option -textvariable varname -role var-write -also-role var-read \
            -scope global
        option -command script -timing deferred \
            -taint-inputs widget-environment
        method invoke {} -effect {invokes -command}
        # cget/configure/creation derive from this table (E-R4, §6.1).
    }
    command button {pathName:widgetpath ?option value ...?} \
        -creates {pathName class Button}

    class Entry -widget -superclass Widget {
        option -textvariable varname -role var-write -also-role var-read \
            -scope global -taints-write        ;# an entry IS an input source
        option -validate values(validate-modes)
        values validate-modes {none focus focusin focusout key all}
        option -validatecommand script -timing deferred -substitutions {
            %P proposed-value:string -tainted
            %s current-value:string  -tainted
            %S edit-text:string      -tainted
            %W widget:widgetpath
            %d action:int
        }
    }

    # One declarative slot shape; bind, canvas bind, and wm protocol
    # all reference it — the three verbatim fn-pointer pairs
    # (bind.rs:40-54, canvas.rs:31-45, wm.rs:32-36) become one row.
    slot-shape event-bound-script {
        role body -timing deferred -substitutions {
            %A event-char:string -tainted
            %K keysym:string     -tainted
            %W widget:widgetpath
            %x x:int  %y y:int
        }
    }
    command bind {tag sequence ?script?} {
        param script -slot event-bound-script -when-present
    }
}
```

The substitution tables are M15 in action: each `%` letter carries a
name, a type, and an optional taint marking — semantic tokens,
completion inside the script, and the taint subset all read one table.
And the text-index mini-language gets the §2 treatment:

```tcl
namespace eval spec::tkindex {
    proc valid {s} { … }   ;# line.char / end / insert / mark ± modifiers
}
command {text index} {index:index(text)} {
    types -inputs {literal index} {call} {
        if {![spec::tkindex::valid [literal index]]} {
            invalid index {malformed text index}
        }
    }
}
```

(`invalid SLOT message` joins the hook verb vocabulary alongside
`literal`/`type`/`returns` — it is what `OptionValueOutcome::invalid`
already is in Rust, surfaced to packs.)

The geometry-manager descriptor (`TkGeometryManagerSpec` — container
policy, direct forms, release subcommands) is already exactly the kind
of typed, engine-consumed data block E just carries verbatim; the
pathname algebra stays Rust. The Tk-package-axis lifecycle on options
(`-locale` introduced Tk 9.1, orthogonal to the Tcl core axis) is §5.4
range targeting working as designed — worth a conformance vector, not
new design.

## 8. tcllib — callbacks, factories, and the scoped completion code

tcllib is where deferred callbacks live, and the walk sorts its twenty
cases into three bins:

**Already data, keep.** `struct::list`'s three callback arities with
expr-vs-body twins; `struct::graph walk`'s option-borne
`Exactly(3)` prefix; `report::report`'s non-OO namespace factory with
sub-subcommand line codes; `lambda`/`defer` bodies proven against both
oracles; `uri::register`'s registration-time body.

**Hooked today, data or E-proc tomorrow.** The arity-conditional
prefix/timing pairs (`hook bind`'s 4-arg registration vs query forms;
`control::do`'s trailing pair) become conditional rows guarded on call
shape. `bibtex::parse` — where six SAX callback options flip between
same-invocation and deferred depending on whether `-channel` appears
*elsewhere in the call* — is the cross-option dependency case, and in E
it is a five-line pack-local `timing` hook, unit-testable, replacing a
Rust fn nobody can see from the spec. The `PREFIX_OVERRIDES` post-hoc
patch loop (`clients.rs:583-603`) — a Rust mutation pass over
already-built rows because the flat `Row` table couldn't say
"command-prefix" — simply ceases to exist: E registration calls
compose, so the row says what it means at the call site.

**Not modelled at all, and cheap now.** `math::calculus`'s 20+
function-name callbacks — the largest unmodelled callback surface in
tcllib — is a `foreach` over a name list in E. `struct::tree walk`
(the loop-body twin, absent from Rust, present only in the draft) and
`struct::graph`'s third dispatch level (~15 `arc`/`node` ops with real
arities that `SubSubCommand` cannot hold — gap G16) both land once
M11 gives sub-subcommands full fidelity. `http::geturl`'s entire
option table — including the two command-prefix callbacks that are
invisible today — ports from the existing `geturl.tclspec` draft.

Three of those in E, shortest first. The `math::calculus` fleet is a
loop over data:

```tcl
foreach {name synopsis} {
    integral            {begin:double end:double nosteps:int func}
    romberg             {func begin:double end:double ?-option value?}
    newtonRaphson       {func deriv initval:double}
} {
    command math::calculus::$name $synopsis {
        foreach p {func deriv} {
            if {[param-exists $p]} {
                param $p -role command-prefix -timing deferred -appended {x}
            }
        }
    }
}
```

`bibtex::parse`'s cross-option timing flip is a five-line pack proc
instead of an invisible Rust fn:

```tcl
command bibtex::parse {?option value ...? ?text?} {
    option -command       prefix -appended {token}
    option -recordcommand prefix -appended {token type key recorddict}
    option -channel channel
    timing {call} {
        # SAX callbacks defer when reading a stream; with inline text
        # they run inside this invocation (misc_pkgs.rs:499-539).
        set mode [expr {[option-present -channel] ? "deferred"
                                                  : "same-invocation"}]
        foreach opt {-command -recordcommand -progress} {
            timing $opt $mode
        }
    }
}
```

And `http::geturl` finally declares the options that exist:

```tcl
command http::geturl {url:string ?option value ...?} {
    available {package http}
    returns token
    taint -network-sink url -code T104
    option -command prefix -timing deferred -appended {token}
    option -handler prefix -timing deferred -appended {socket token}
    option -headers dict -credential
    option -query string
    option -timeout int
    option -validate boolean
}
```

The one genuinely hard case stays hard, and gets a ruling candidate
instead of a shrug: `struct::tree::prune` is `return -code 5`,
meaningful only inside `walk`'s body — a *library-defined completion
code scoped to one command's body*. The rubric rightly refuses to open
the `completion` CFG field to packs. E-R6 proposes the narrow door:
`-completion-codes {5 prune}` on the body slot (as the class example
above spells) declares that *within this body*, code 5 is a named,
loop-adjacent completion — consumed by the existing
`BREAKS_LOOP`-family traits machinery scoped to that body, never by the
open-coded CFG vocabulary. Scoped, named, and only attachable where a
body role already is.

> **Status note (redesign P5, 2026-08-27).** The walk above was written
> before two of its three "not modelled at all" rows landed, and P5
> closed most of the rest against `tmp/tcllib-2.0` directly. Corrections
> and outcomes, so the bins above are read as history rather than as the
> current state:
>
> - **`math::calculus` is modelled**, and its measured appended arities
>   (`integral` f(x), `integral2D` f(x,y), `integral3D` f(x,y,z), the ODE
>   steppers f(t,xvec), `newtonRaphson`'s two prefixes, every root finder
>   f(x)) were re-derived from `calculus.tcl` / `rootfind.tcl` in P5 and
>   agree row for row. What is *not* modelled is
>   `integralExpr`'s fourth argument: an `expr` evaluated in the
>   **callee's** frame against a callee-supplied `x`. `ArgRole::Expr`
>   would be right about the language and wrong about the scope; the
>   missing vocabulary is a scope-and-provides qualifier on an expr slot.
> - **`struct::tree walk` and `walkproc` are modelled**, across both
>   trains: the 2.x `loopvar script` body (position-resolved, because the
>   option prefix is variable-length), `walkproc` with
>   `introduced: "2.0"`, and the 1.x `-command` option with
>   `retired: "2.0"` and the `Exactly(0)` appended arity its
>   `string map` + `uplevel` actually has — *not* a prefix arity, which
>   is the substantive difference from `struct::graph walk -command`'s
>   `Exactly(3)` (verified identical in `graph1.tcl` and `graph_tcl.tcl`,
>   so `struct::graph`'s walker has no cross-train delta at all).
> - **`http::geturl`'s option table landed**, read from `geturl`'s own
>   `set options {…}` list and callback call sites across all four
>   bundled Tcl trees, with the release deltas those trees prove
>   (8.5's http 2.7.13 adds `-keepalive -method -myaddr -protocol
>   -strict`; 9.0's 2.10.2 adds `-guesstype`).
> - **`bibtex::parse`'s cross-option timing flip** is the shipped
>   `script_timing_resolver` and was re-verified against `bibtex.tcl`;
>   P5 added the five `-command`-versus-SAX conflicts it also proves.
>   Its *other* rule — `-command` requires `-channel` — is the G2
>   directional half restated, and additionally needs a relation between
>   an option and a **positional** argument. **Both landed under E-R14**:
>   the shipped spec now carries `option_requires -command {-channel}`,
>   `option_forbids -channel {{arg 0}}`, and
>   `option_requires_one_of {} {-channel {arg 0}}` for the
>   "neither specified" case.
> - **E-R6 stands, narrowed.** `::struct::tree::prune` is now a real
>   command with `introduced: "2.0"` on the `struct::tree` axis and a
>   completion domain of exactly `{5}` — the *producer* half needed no
>   new vocabulary, because a library-defined code is
>   `CompletionCode::Other`. What E-R6 is actually for is the *consumer*
>   half, and the field is on the body slot:
>   `body_completion_codes: &[(u8, CompletionCode, &str)]`. Until it
>   exists `prune` carries no control-flow trait at all, because
>   `CONTINUES_LOOP` would be a lie the CFG builder acts on.
> - **The one new limit P5 found:** an `ObjectClassSpec` instance method
>   and its options already carry a `Lifecycle`, but the analyser has no
>   diagnostic site on the instance-method path, so `walkproc`'s and
>   `walk -command`'s lifecycles are declared and unread. That is a
>   missing *consumer*, not a missing field.

## 9. expect and argparse — grammars inside arguments

`expect`'s clause grammar is the model at its best — `CaseListSpec` is
a 22-field descriptor covering per-clause flags, outer-shape selector
flags (`-brace`/`-nobrace` with the first-arg-only remainder rule), the
poisoned `-timeout`/`-i` interaction recorded as *deliberate
abstention*, non-final keyword patterns, and the omitted-final-body
allowance. All data; E carries it as a named `case-grammar` descriptor
and the `interact` gap (a **second**, different pair grammar, currently
`case_list: None`) becomes a second instance rather than new machinery.
Two real holes: spawn ids flow through the surface untyped (`spawn`
declares no out-binding; `expect -i` takes a plain value) — E-R7's
pack-declared handle classes give `spawnid` the same treatment
`binds_handle` gives snit's `install`; and `trap`'s first argument is a
body *or* the literal `SIG_IGN`/`SIG_DFL` — a value-or-body union the
conditional-row form covers.

The clause grammar and the handle class, spelled:

```tcl
case-grammar expect-clauses {
    pair {pattern action}
    clause-flags {-exact -re -gl -notransfer -nocase -indices}
    value-flags {-timeout -i}   ;# a braced word AFTER these is a pattern
    keyword-patterns {timeout eof default full_buffer null} -non-final
    allow-omitted-final-body
    force-list-flag -brace -shape first-arg-only-remainder
    force-inline-flag -nobrace
}
command expect        {?arg ...?} -case-grammar expect-clauses
command expect_before {?arg ...?} -case-grammar expect-clauses \
    -registers-not-executes    ;# bodies run later: DEFERS_BODY derived
command expect_background {?arg ...?} -case-grammar expect-clauses \
    -registers-not-executes

# The second, different pair grammar interact needs and lacks today:
case-grammar interact-clauses {
    pair {string body} -input-interception
    clause-flags {-o -reset -echo -nobuffer}
    value-flags {-u -i -input -output -timeout}
}
command interact {?arg ...?} -case-grammar interact-clauses

handle-class spawnid
command spawn {program:path ?arg ...?} {
    returns spawnid
    introduces -variable spawn_id -class spawnid -scope global
    side-effect ProcessIo -writes
    taint -code-sink program
}
command close {?-slave? ?-i id?} -available expect {
    option -i spawnid
}

command trap {?action? ?signals:list?} {
    form {trap}                 -query
    form {trap signals}         -query
    form {trap action signals} {
        param action -role body -unless-values {SIG_IGN SIG_DFL}
    }
}
```

`-unless-values` is the value-or-body union `trap` needs (a literal
`SIG_IGN` is data, anything else is a script); `-registers-not-executes`
derives the deferred-body facts `expect_before` misses today; and
`spawnid` flowing `spawn` → `$spawn_id` → `-i` is E-R7's handle class
doing for expect what `binds_handle` did for snit.

`argparse` is the deepest case in the whole survey. The command's
*argument is a grammar*: a definition list whose 27 element switches,
suffix shorthands (`= ? ! * ^`), combinatorial legality table, and
`-forbid`/`-require`/`-imply` graphs define the *caller's* argument
surface — and today the model's whole answer is a blindness
declaration (`FrameArgLayout::OpaqueCallerVars`: widen, never
enumerate) plus a dead exported data table nothing consumes. Gap G4's
verdict was "[SOFT] — no mechanism to point the DSL at the embedded
literal". E provides exactly that mechanism, and it is the same seam as
§2 (E-R3's fact-emitting hooks, one level up):

```tcl
command argparse {?switch ...? definition:list ?arguments:list?} {
    grammar -inputs {literal definition} {call} {
        foreach element [argparse::parse-definition [literal definition]] {
            declare-local [argparse::element-key $element] \
                -type [argparse::element-type $element]
        }
    }
}
```

A `grammar` hook emits *derived declarations* — locals the call
introduces in its caller, per-call option tables, per-call arity — from
a statically-known literal, abstaining otherwise (the
`OpaqueCallerVars` widening stays as the abstention floor, unchanged).
The same hook family covers SpiceGenTcl's 164 argparse constructors
(when the definition is literal at the class site), cffi's
`{type value}` signatures with `chars[n]` cross-parameter references,
and `tls::socket -server`'s callback rewriting — each a pack helper
plus one hook, none a Rust change beyond the hook seam itself (M3).
What stays honestly unknowable stays `Unknown`: mustache's
position-dependent lambda arity and runtime-computed definitions
abstain by construction.

## 10. The EDA shells — scale, collections, and world state

EDA already lives the 2.0 life: the six vendor environments are Rust
data with **three keyed non-Tcl version axes each** (SDC, UPF, tool),
and the command surfaces are eight bundled `.tclspec` packs totalling
35,001 lines — `eda_xilinx.tclspec` alone is 20,887 lines — with the
Rust spec directories already deleted behind a field-for-field
round-trip gate. The walk's findings are therefore about what the packs
*couldn't say*:

- **Boilerplate at scale.** `option -quiet` / `option -verbose` repeat
  roughly a thousand times. In E:
  `proc vivado-command {name synopsis body} { command $name $synopsis [concat {option -quiet; option -verbose; option -return_string} $body] }`
  — a pack-local template proc, the single strongest concrete argument
  for E at this corpus scale, and per §1.3 its output is a fixed data
  set.
- **Collections are an untyped hole.** Object-query results
  (`get_cells`, `get_clocks`) are a distinct value kind consumed by
  `filter`/`foreach_in_collection` — today acknowledged only by one
  analyser hook on `foreach_in_collection`, with collection-returning
  commands saying so in hover prose. E-R7 again: the pack declares
  `type collection` as a handle class, query commands declare
  `returns collection(cell)`, and `foreach_in_collection` narrows its
  loop variable — all through the §2 typing vocabulary, no new Rust.
- **World state gates legality.** `create_clock` is meaningless with no
  open project; a run must be active for `wait_on_run`. The only
  context gate the model has is hardwired to `in_event_body`
  (`spec.rs:91-96`). M2 generalises the gate input to a typed context
  bag (event body, proc body, class body, transaction open, project
  state axis) that engines populate and packs *reference* — tmsh's
  transactions (§4) and iRules' execution contexts join the same
  vocabulary.
- **Timing-constraint value syntax** (`-waveform {2 4}`, edge lists,
  `{}`-quoted timing exceptions) is the literal mini-language pattern a
  third time — helper procs plus `types`/`values` hooks, with the SDC
  version axis gating vocabulary the same way `%b` gates on 8.6.
- The corpus census note that EDA scripts build command tables by
  `interp alias` (20 anchored hits, vs zero `bind`/`coroutine`) means
  the alias transition machinery (R-c) matters *more* here than event
  callbacks — a conformance-lattice entry, not a design change.

Assembled, the Vivado shape looks like this:

```tcl
speclib eda_xilinx 2.0

proc vivado-command {name synopsis body} {
    command $name $synopsis [concat {
        option -quiet
        option -verbose
    } $body]
}

type collection -handle -parameter of   ;# E-R7: pack-declared handle type

vivado-command get_cells {?patterns?} {
    returns collection(cell)
    option -hierarchical
    option -filter expr(collection)
    option -of_objects collection
}
vivado-command get_clocks {?patterns?} { returns collection(clock) }

command foreach_in_collection {var:varname coll:collection body:script} {
    traits {CONTROL_FLOW HAS_LOOP_BODY LOOP_LIST_HEADER}
    analyser -native Foreach
    types {call} {
        # The loop variable narrows to the collection's element type.
        type var [list varname [collection-element [type-of coll]]]
    }
}

vivado-command create_clock {?-name name? -period p:double ?-waveform edges? objects} {
    requires-context {project open}
    option -waveform list(double) -available {sdc 1.4-}
    types -inputs {literal -waveform} {call} {
        if {[llength [literal -waveform]] % 2} {
            invalid -waveform {edge list must pair rising and falling edges}
        }
    }
}
```

One template proc erases roughly a thousand boilerplate rows in the
Xilinx pack alone; `collection(cell)` gives the query→filter→foreach
pipeline real types; `requires-context {project open}` is the same M2
axis tmsh's transactions use; and the `-waveform` check is the §2
pattern on the SDC grammar, gated on the SDC version axis the
environments already carry.

## 11. Corpus oddities — the census gaps, revisited under E

The external census (G1-G16) was run against the declarative 1.x-shaped
DSL; re-reading it under E, the entries sort cleanly:

**Dissolved by E mechanics.** G4 (argparse embedded grammar — §9's
`grammar` hook). G5 (shared closed-value tables on options — `values`
registered once, referenced by any row; in E a table is just a value a
proc can splice). G10 (one method body shared across unrelated classes
— a pack-local proc invoked from N registration sites, no duplication).
G13 (scoped completion codes — E-R6). Per-flag-combination return
typing (tarray `column search`, a known-limits row) — a `types` hook
whose `-inputs` include option presence; the limit row retires.

**Landed by model changes already ruled.** G1 (`object_class` syntax —
ratified, and M5 widens it). G3 (`-introduced` on a third-party axis —
ratified; VersionSet made it general). G7/G15 (command-only fields on
methods — M5). G16 (sub-subcommand fidelity — M11). G12 (property
clause — member layout kind, §6).

**Closed since.** G2's directional half is **done**: E-R14 replaced
`OptionConstraint` with a typed `OptionRelation` and added
`option_requires` / `option_requires_one_of` / `option_forbids` beside
`option_conflict`, so all 189 of the `-require` uses the census found in
SpiceGenTcl have a spelling — and so does the cross-option *value*
legality row (`RelationTerm::OptionValue`), and the option-to-positional
relation `bibtex::parse` needs (`RelationTerm::Argument`). Every one is
checked natively with no VM entry.

**Still open, now with a shape.** G6 (foreign
code as a value — ticklecharts' `jsfunc` JS): E-R8 proposes
pack-namespaced embedded-language names (`-language ticklecharts::js`)
carrying *identity and taint colour only* — rendering and analysis
semantics only via hooks, so the closed `pattern_type` /
`format_string_type` catalogues retire into one open-named,
closed-semantics axis. G8/G9 (apave's nested tuple bodies and
closed-yet-extensible widget codes) stay the census's most speculative
rows; nothing in E makes them cheaper to get wrong, so they wait for a
second corpus witness.

The ticklecharts shapes, since they exercise four census entries at
once — shared value tables on options (G5), the literal-first-word
ensemble, the third-party version axis (G3, already landed in 2.0 as
`-available {echarts …}`), and the method-level sink (G7, legal once
§6.1 makes methods full invocation specs):

```tcl
package-surface ticklecharts -available {tcl 8.6-} {
    values line-types {solid dashed dotted}
    values series-names {barSeries lineSeries pieSeries funnelSeries …}

    class chart -manufacturers {new create} -allow-unknown-methods {
        method Add {seriesType:values(series-names) ?option value ...?} {
            option -lineStyle dict {
                option -type values(line-types)   ;# one table, many rows
                option -width double
            }
            option -colorBy values(color-by) -available {echarts 5.2.0-}
            option -alignTicks boolean -available {echarts 5.3.0-}
        }
        method Render {?option value ...?} {
            option -outfile path -taint-file-sink      ;# G7, on a method
            option -title string
        }
    }

    command ticklecharts::jsfunc {body ?-start? ?-end?} {
        # E-R8: identity is open and namespaced; semantics only via hooks.
        param body -language ticklecharts::js -taint-colour JS_CODE
    }
}
```

And tarray's per-flag-combination return typing — the known-limits row
that retires because option presence is a legal `types` input:

```tcl
command {column search} {?option ...? column value} {
    types -inputs {options} {call} {
        if {[option-present -all]} {
            returns [expr {[option-present -inline] ? "column" : "list(int)"}]
        } else {
            returns [expr {[option-present -inline] ? "any" : "int"}]
        }
    }
}
```

Two corpus workloads become acceptance tests rather than design
inputs: the KaiWilke RADIUS stacks (heavy `binary scan` on collected
payloads — §2's typing walk must light them up end to end), and
TesTcl, which *mocks* iRules commands as plain Tcl procs — the
resolution story (user `proc HTTP::header …` produces a `Must` binding
that shadows the pack declaration, softening diagnostics) exercises
R-c's realm transitions with no new mechanism, and makes a good
conformance vector for binding-vs-declaration precedence. `tclinterp`
remains the clean counter-example: a 33-line pack, fully expressible,
zero gaps — the floor E must not raise.

## 12. SpecTcl itself — self-hosting closes under E

Today the DSL's own editing experience is the strangest pack in the
tree: a compiled-in command set *plus* twelve definer grammars, with
exactly one `body_scope` (the `hover` block) — so the inner words of
eleven other block grammars sit on the same W123 footing `hover`'s keys
did before the parity test forced an environment. Two hand-kept
vocabularies (the `CommandSpec` list and the `MemberSpec` tables) have
already drifted: `available`, `environment`, `dialect`,
`file_extension` and friends exist as statements with no member rows,
and `environment`/`dialect` — 2.0's two most semantic blocks — are
modelled as flat two-argument statements whose braced bodies are
opaque strings. The `descriptor KEY NAME {…}` block's inner vocabulary
depends on a word on its own line, "which no single grammar can
express"; hook-body verbs (`fold`, `role`, `reject`, `consume`) appear
in the editor as nothing.

Under E, all four strains dissolve into the same fact: **a pack file is
a Tcl program, so the DSL surface is an ordinary command surface.**

- The vocabulary words are commands in the evaluation interp (§1.5), so
  there is exactly *one* source of truth — the evaluator's command
  table — and the editing-surface pack is **emitted from it**
  (`tcl spec build --emit self` alongside `--emit rust`), retiring the
  parity-test-per-block treadmill (E-R9). The field renames that today
  live in prose (`snippet`→`description`, `returns`→`return_value`)
  become the emitted table's data.
- Block vocabularies are evaluation scopes: `descriptor world_effects
  NAME {…}` dispatches on its key at evaluation time, so the inner
  vocabulary is exact per instance — the "no single grammar" limit was
  an artefact of describing evaluation statically.
- The old strain "`set x 1` at pack level resolves as a known command
  while the loader silently drops it" **inverts into a feature**: under
  E that line is real and meaningful. What replaces the silent drop is
  a `spec check` lint classifying top-level statements by
  registration effect, so dead code in a pack is a notice, not a
  mystery.
- The hook sandbox's 26-command whitelist and the emitter verbs get
  specs the same way — they are the `body_scope` of hook bodies,
  spelled once in the self-pack.

An excerpt of the emitted self-pack, to make the bootstrap concrete —
the DSL speccing its own `command` word and the hook-body vocabulary:

```tcl
speclib spectcl 2.0

command command {name:name ?-override? synopsis body:script} {
    introduces -declaration command -named-by name
    param body -scope command-body
    hover {
        Registers one command's surface. The body evaluates with the
        command-scope vocabulary in scope: `option`, `subcommand`,
        `hover`, `types`, `roles`, `taint`, `available`, ...
    }
}

scope command-body {
    command option {name ?value-kind? ?flag ...?} { … }
    command subcommand {name synopsis ?body?} { … }
    command available {requirement ?requirement ...?} { … }
    command hover {text} -prose
}

scope hook-body {
    # The sandbox whitelist plus the emitter verbs — today invisible
    # to the editor (strain 10), here just another scope.
    command literal {slot}        { returns string }
    command type    {slot type}   { }
    command returns {type}        { }
    command invalid {slot message} { }
    command option-present {name} { returns boolean }
}
```

Both `scope` blocks are the generalised `body_scope` of M12; the
`hover` key shadowing problem (`source` inside `hover` vs the real
`source`) is what scoping *is* in an evaluated language, no special
case left.

What survives unchanged from today's loader: vocabulary versioning and
the fail-closed classes (§1.5 — the class of an unknown *registration
command* is judged by the same Presentation/Assistance/Semantic rules,
with everything inside `dialect`/`environment` blocks Semantic by
scope), tier-based provenance (`Tier` is the §6.4 trust class, now
enforced at the registration call per E-R2), and the editor identity
(`tclspec` language id, content signature for packs saved as `.tcl`,
packs as analysis inputs never indexed documents). The self-hosting
acceptance gate: the SpecTcl 2.0 self-pack loads under its own loader,
its emitted editing surface analyses the eight EDA packs and the
`tcllib`/`tk` packs with zero unknown-word diagnostics, and
`spec build --emit rust` on the self-pack reproduces the compiled
vocabulary byte-for-byte.

## 13. What feeds back into the Rust model

The walks converge on fifteen model changes. Each cites its forcing
evidence; none is a compatibility shim — old fields are translated by
the upgrade path and deleted (P1-G extends to cover them).

- **M1 — the version axis becomes a facet, not a field.** `Lifecycle`
  is replicated onto ten structs, each with its own plumbing; `Arity`
  has no version axis at all, so specs widen to `Arity::any()`
  (`vwait`, `global`), duplicate into `subcommand_forms`
  (`package vsatisfies`), or reach for `arity_windows` — three
  mechanisms for one problem. Every declaration node (command, form,
  option, value, member, event, arity row) carries one optional
  `available: VersionSet` requirement; assembly resolves it uniformly.
- **M2 — hook and gate inputs get typed context.** `ArgRoleResolver`
  has no version parameter, so `uplevel`'s resolver abstains on the
  8.6/9.0 divergence it exists to model (`uplevel_.rs:70-76`);
  `ContextGate` is hardwired to `in_event_body`. One typed
  `InvocationContext` (resolved release, execution context, class
  scope, transaction/world-state axes) feeds every hook family —
  `const_fold_versioned` already proved the pattern.
- **M3 — one literal-driven derivation seam.** `types`, `roles`,
  `values`, `grammar`, `timing`, `forms` hooks guarded on
  `-inputs {literal N}`/`{options}` replace the nine fn-pointer
  escape hatches and the bespoke fields (`format_string_type`,
  `pattern_type`, `var_write_typing`, `taint_sink_gate`,
  `OptionArity::Hook`, `clause_shape_check` where shape is
  literal-driven). Recurring patterns A ("role depends on another
  argument's parsed content") and B ("position is variadic-relative")
  from the stdlib walk both land here; abstention floors are the
  static declarations.
- **M4 — unknowability becomes typed.** The bare `SideEffect::DEFAULT`
  sentinel (`unknown`, `eval`, `coroutine`) becomes `Effect::Opaque`;
  `AppendedArity::Unknown` splits from "not stated"; abstention is
  distinguishable from omission everywhere (the `event-requires -none`
  vs absent-clause distinction of §3 is the same rule).
- **M5 — one class surface** (mechanism in §6.1). Merge the disjoint halves
  (`object_class`+`creates_instance_at` vs
  `definition_body`+`manufacturers`+`defines_command_at`) into one
  `ClassSurface`; methods carry every command-grade fact (taint,
  sinks, forms, deprecation — G7/G15 by construction); the inline
  `oo::define` form derives from the member table (the known gap);
  itcl/snit gain the construction machinery their grammars already
  imply.
- **M6 — one `introduces` vocabulary** for commands that create
  commands: subsumes `defines_command_at`, `creates_instance_at`,
  `binds_handle`, `command_table_effect`, covers the currently
  uncovered (`zlib stream`, `namespace ensemble create -map`'s table,
  `image`/`font create`'s optional names), with position-or-pattern
  targets and typed handle classes (E-R7).
- **M7 — traits singular.** Retire the seven duplicate `SubCommand`
  booleans and the snapshot's `TRAIT_FLAGS` back-mapping; widen
  `Traits` to `[u64; N]` before the 128 ceiling (96 used) forces it
  mid-flight.
- **M8 — diagnostic identity is typed everywhere.** `taint_output_sink:
  Option<&str>` holding `"IRULE3001"` vs `SetterConstraint::code:
  DiagCode` is two conventions in one struct; every sink/code slot
  takes `DiagCode`.
- **M9 — dead surface is deleted or populated, never carried.**
  `Traits::PASSWORD_OPTION`, `IRULES_DATA_GETTER`, `xc_operation`, the
  unused `arg_rows` machinery, `EventRequires::init_only`,
  `ProfileSpec::capabilities`/`conflicts` — each gets an owner ruling:
  delete now, or a named consumer lands within the programme.
- **M10 — data tables leave Rust.** The iRules event graph (§3), the
  BPF event schema (§5), mathop/mathfunc projections (alternatives
  doc), and the remaining generated command files follow the EDA packs
  out: dialect packs compiled back via `spec build --emit rust`, with
  the 21-chunked-functions shape as the standing reminder of why.
- **M11 — uniform ensemble depth.** `SubSubCommand` gains the full
  fact set (arity first — G16's ~15 `struct::graph arc` ops have
  nowhere to put theirs); the three-valued options inheritance is kept
  but documented as the one merge rule.
- **M12 — scoped facts.** Body-scoped completion codes (E-R6),
  member-body commands, and per-body command environments unify:
  `body_scope` becomes the general "inside this body, these words/
  facts apply" mechanism every block-shaped surface uses (eleven
  SpecTcl blocks, `report::defstyle`, hook bodies).
- **M13 — no hand counts, no prose copies.** Doc-comments that restate
  data (247-vs-176) are generated from the data or deleted.
- **M14 — consumer special-cases become spec-driven.** The nine
  hardcoded command-name sites the inventory found (`sccp.rs`,
  `slot_resolution.rs`, `statements.rs`, `class_lattice.rs`,
  `uri_split.rs`, `irules_checks.rs`, `lowering/mod.rs`,
  `expr_surface.rs`) each move behind `semantic_operation`, a trait,
  or a hook id — the `spec.rs:1243` claim becomes a zero-reference
  gate (P1-G rider), including the `self`/type-inference contract leak
  (§6).
- **M15 — callback substitution vocabularies are first-class.** A
  per-slot substitution table (letters/tokens, each with type and
  taint marking) serves Tk `%`-substitutions, `struct::tree walk`'s
  `%n`/`%t`, and iRules' session tables alike; the current taint-only
  subset becomes a projection of it.

## 14. Collected ruling candidates

**Status column added 2026-08-27.** E-R1–E-R9 are ratified (owner,
2026-08-26); **E-R11, E-R12 and E-R13 are ratified (owner, 2026-08-27)**,
E-R12 with a visibility amendment (standing overrides must be surfaced —
a studio indicator plus a `spec check` warning once a patch outlives a
threshold — so an override reads as a staging area, not a home);
**E-R14 is ratified (owner, 2026-08-27)** and recorded below;
E-R10 is a standing statement of limits, not a proposal;
E-R11–E-R13 were written "(proposed)" in §15, implemented as written, and
**never formally ratified** — see O3 in the redesign's §11.

| # | ruling candidate | forced by | status |
|---|---|---|---|
| E-R1 | Conditionality is data (`-available` rows); control-flow `available?` downgrades cacheability and is flagged by `spec check` | §1.3; review B5 | ratified; implemented — `-available` rows are the shipped spelling and `available?` marks a pack target-dependent and uncacheable |
| E-R2 | The sandbox always runs; provenance (§6.4 tiers) gates what a registration call may touch, enforced at the call | §1.4; loader tier model | ratified; implemented — provenance checks fire at the registration call (compiled-name overrides, `dialect` blocks, reserved environment names). Which tiers are *untrusted* now derives from `PackEnvironmentTier::provenance`, so the loader and the environment model agree: `Tier::Workspace` is `WorkspaceTrusted` and may still `-override` a shipped command, per §6.4's "in an untrusted workspace"; the untrusted class is reachable through the Spec Studio override tier until the editor's Workspace Trust state is plumbed (O9). The tier the studio buffer and `spectcl_check` evaluate at is O4, still unratified |
| E-R3 | Hooks emit facts referencing gated vocabulary; assembly alone evaluates gates — hooks stay target-independent | §2; range targeting | ratified; implemented — hooks emit facts, assembly evaluates gates |
| E-R4 | Derived surfaces are never authored: mathop/mathfunc projections, the inline `oo::define` form, instance `configure` tables all derive from their entity | §6, §7, alternatives doc | ratified; **partly implemented** — the mathop/mathfunc projections derive (`tcl::mathop`/`tcl::mathfunc` ruling, 2026-08-26). The inline `oo::define` form and instance `configure` tables still need §6.1's one-class-surface work |
| E-R5 | Templating is for *bulk* (32× `REQUIRES_HTTP_CONTEXT`, 1000× `-quiet`); structural regularity (widget inheritance) is declared, not templated | §3.2, §7, §10 | ratified; implemented — templating is available and the canonical form is what generators emit |
| E-R6 | Body-scoped named completion codes (`-completion-codes {5 prune}`) — attachable only where a body role is, consumed by the traits machinery, never the CFG field | §8 | ratified; **not implemented, narrowed by P5** — `::struct::tree::prune` is a real command carrying `CompletionCodeDomain::Exact([Other(5)])` and deliberately **no** control-flow trait, because the scoping field does not exist. See §8's status note and D4 in the redesign's §11 |
| E-R7 | Pack-declared handle/type classes (`spawnid`, `collection(cell)`) through the existing `binds_handle`/typing vocabulary | §9, §10 | ratified; implemented for the shipped handle classes |
| E-R8 | Embedded-language identity is open and pack-namespaced (`ticklecharts::js`); embedded-language *semantics* come only from hooks | §11 (G6) | ratified; implemented |
| E-R9 | The DSL vocabulary is single-sourced from the evaluator's command table; the editing-surface pack is emitted, never hand-kept | §12 | ratified; **partly implemented** — the SpecTcl self-spec gained `available`, `environment` and `dialect` so authoring a pack gets completion and hover, but the vocabulary is not yet single-sourced from the evaluator's command table |
| E-R10 | Known limits that stand: option *requires* relations (pending owner ruling — best-value candidate), apave's nested tuples (G8/G9), position-dependent callback arity (`Unknown` is honest) | §11 | a statement of limits, not a proposal. Its first item (option *requires* relations) is O1 in the redesign's §11; P5 confirmed it independently from `bibtex::parse` |
| E-R14 | **Option relations are a typed model with a native-Tcl escape hatch.** `OptionConstraint` generalises into a typed relation covering mutual exclusion, directional requires, requires-one-of, and relations reaching positional arguments and option *values*. Authors reach it through centralised utilities for the common patterns; anything else is a `constraints` hook whose body is ordinary Tcl (`if`, `switch`, `foreach`) able to read the whole invocation. Bound by principle P-B: the declarative forms compile to a structure checked natively with **no VM entry**, the hook is the rare exception, and hook results are shape-cached on declared inputs so an edit that does not change them never re-runs the script | owner ruling 2026-08-27; P5's `bibtex::parse` evidence (`-command` requires `-channel`); census gap G2's directional half and the cross-option value-legality row | **ratified and landed.** `OptionRelation { kind, subject, terms, dialects, lifecycle, message }` over `RelationTerm::{Option, OptionValue, Argument, ArgumentValue}`, with `OptionPlacement` telling the checker where a command's options live; `OptionRelation::evaluate` is the whole declarative checker and `RelationFacts::complete` is what licenses proving a term *absent*. Four statements at both loader seams (`option_conflict`, `option_requires`, `option_requires_one_of`, `option_forbids`), an export round-trip gate over every kind and term shape, W147 for the exclusions and **W152** for the requirements. The `constraints` hook follows the `types` contract exactly — see §2.1. Measured on a 40-call-site option-heavy corpus: 33 option-bearing sites walked, 14 judged, 50 relations evaluated natively, **0 entered tclvm** |
| E-R11 | SpecTcl 2.0 has a **canonical form** — the straight-line subset of E; every generator emits it, `tcl spec export` expands any snapshot into it, contraction is never attempted | §15.1 | **proposed → implemented, unratified.** `tcl_spectcl::export` renders `Pack::registrations`; round-trip gate A holds over all 24 shipped packs (1,515 commands, 14,770 registration calls) snapshot-identical and text-idempotent; gate B proves three templated fixtures expand, reload and are fixed points. Wired as `tcl spec export`. See O3 |
| E-R12 | The studio never rewrites a **programmed** pack: its source opens read-only beside its expansion and a form edit lands as a canonical patch pack in the `StudioOverride` tier | §15.2 | **proposed → implemented, unratified.** `PackStore::programmed` is a three-tiered predicate (target-dependent, expanded, or statements the snapshot did not record), `WriteBack::Patched` routes the edit, `PackStore::standing_overrides` is the queryable report, and a guard test holds the predicate over all twelve hand-written packs. See O3 |
| E-R13 | The `spectcl_expand` MCP verb — pack source in, canonical form out — so a programmed pack is reviewable as its expansion rather than simulated in the reader's head | §15.3 | **proposed → implemented, unratified.** `spectcl_expand` ships; `spectcl_check` evaluates packs and surfaces `load_error`, `target_dependent`, per-notice classes and what an untrusted tier would refuse. See O3 |

The overall verdict the deep dive supports: design E survives every
wall the survey could find, *provided* §1's execution model is adopted
as a package — the frozen snapshot, the determinism contract, and
data-not-control-flow conditionality are not optional refinements but
the load-bearing answers to E's known costs. Where E is weakest
(static opacity to third-party tools; trust review of workspace packs)
the snapshot and the registration-time trust gate carry the weight;
where it is strongest (the EDA corpus, argparse-class embedded
grammars, self-hosting, and every place today's model hides a Rust fn)
no other surveyed design comes close.

## 15. Tooling rework: the studio, the importer, and the AI surface

Adopting E obliges three tool families that today assume
`render ∘ load = identity` on pack text. The obligation splits cleanly
once one distinction is named.

### 15.1 The canonical subset

**E-R11 (proposed 2026-08-26; implemented 2026-08-27, formal ratification
outstanding — O3): SpecTcl 2.0 has a *canonical form* — the
straight-line subset of E.** A canonical pack contains only literal
registration calls (exactly today's declarative vocabulary: every
2.0 word that just landed, no `proc`/`foreach`/`set`, no computed
arguments). Three facts make it load-bearing:

- Every 1.x pack and every 2.0 pack written so far *is already
  canonical* — the migration guarantee restated.
- Canonical source ↔ snapshot is a bijection (modulo formatting), so
  byte-stable round-tripping is a property of the subset, not of the
  loader.
- `tcl spec export` renders *any* snapshot — including one produced by
  a programmed pack — as canonical source. Expansion is total;
  contraction (recovering a program from its snapshot) is not
  attempted, ever.

Everything that *generates* packs emits canonical form: the studio's
renderer, `tcl spec import` and its MCP twin, `spec upgrade`
(`--restyle` restyles 1.x rows into canonical 2.0, never into
templates), and stub-tier conversions. Programs are for humans;
generators have nothing to gain from emitting cleverness.

### 15.2 The spec studio

The studio's architecture survives better than first appears, because
its native object was never really the text — `draft.rs`/`schema.rs`
edit D-shaped data, and `store.rs` already layers packs by `Tier` with
`StudioOverride` at the top. The rework:

- **Reading**: the store loads packs through the E evaluation loader
  and browses *snapshots*. For canonical packs nothing observable
  changes. For programmed packs, every derived command is browsable
  and carries expansion provenance ("registered from `vivado-command`
  at line 12, iteration `get_cells`") — the evaluator records the
  registration call-site the way the CST loader records line numbers
  today.
- **Editing — E-R12 (proposed; implemented, ratification outstanding — O3)**: a canonical pack is edited in place,
  round-tripped byte-stably as today. A **programmed pack is never
  rewritten by the studio**: its source opens read-only alongside its
  expanded snapshot, and a form edit lands as a canonical *patch pack*
  in the `StudioOverride` tier — the layering and collision policy
  that tier already implements. The author folds patches back into
  their program by hand (or keeps them layered); `spec check` reports
  standing overrides so they cannot rot silently.
- **Renderer**: `render_spectcl` emits canonical 2.0 and the
  `DSL_VERSION` pin lifts exactly per the condition documented at the
  constant (P2-H). `render_rs` gains a sibling: the `--emit rust`
  backend consumes snapshots, so it is loader-fed rather than
  draft-fed and works for programmed packs too.
- **Sample/Test tab**: unchanged in concept (it explains analysis of a
  sample against the loaded pack), but it runs against the snapshot,
  so it also answers the programmed-pack author's question "what did
  my loop actually register?" — the same affordance E-R13 gives the
  AI tools.
- **WASM**: the browser studio needs the evaluation loader compiled to
  WASM. The tclvm already targets WASM for the compiler explorer, and
  the deterministic sandbox (§1.2) is exactly the profile that
  compiles cleanly — no clock, no IO, no threads. This is a build
  requirement, not a design risk, but it gates shipping the studio
  rework on the eval loader's WASM CI job.
- **Form/schema machinery** (`schema`, `coverage`, `catalogue`, the
  exhaustive-destructuring witnesses): untouched by E itself; the
  M-series model changes will churn it on their own schedule, and the
  witnesses are precisely what keeps that churn honest.

The importer (`infer`/`versions`/`corpus`) is unchanged in substance:
it derives drafts from package sources and now renders them as
canonical 2.0 with the same evidence headers. Version-range
derivation, release diffing, and the corpus heuristics do not care
what surface they are printed in.

### 15.3 The AI surface (tcl-mcp) — E-R13 (proposed; implemented, ratification outstanding — O3)

The MCP tools are how an AI authors packs, and E changes their loop in
one direction only: **evaluation makes checking *safer* and richer,
not weaker.** The determinism sandbox means an AI-generated pack — a
program written by a model — is evaluated with no clock, no IO, no
network, hard budgets, and transactional registration, so the check
loop can run untrusted generated programs by construction; a runaway
`foreach` is a budget notice, not a hung tool.

- **`spectcl_check`** runs the evaluation loader and returns what it
  returns today (notices, per-word diagnostics) plus the new notice
  classes: determinism violations, budget blowouts naming the axis,
  target-dependence (`available?` use), and provenance-class
  registration errors.
- **New verb `spectcl_expand`**: pack source in, canonical form out —
  `tcl spec export` over MCP. This is the affordance that makes
  programmed packs *reviewable* by their author: generate a template,
  expand it, read the expansion as a diff against intent, iterate.
  Without it an AI (or a human reviewer) sees only the program and
  must simulate the loop in its head — exactly the B5-class opacity
  §1.1 exists to prevent.
- **`spec_import`** emits canonical 2.0 (15.1); its
  hand-to-`spectcl_check` contract is unchanged.
- **Authoring guidance** (AGENTS.md §specs, and the spec-authoring
  material in `docs/design/spec-packs.md`) gains the E rules of thumb:
  emit canonical form unless repetition is the problem being solved;
  when templating, keep the data table adjacent to the loop; run
  `expand` and read it before shipping; never branch on `available?`
  when an `-available` row will do.

The irule-generation and test tools are unaffected: they consume the
registry, not pack text.

### 15.4 Sequencing

The studio and MCP reworks are gated on the evaluation loader (P2-I),
in this order: loader lands with the equivalence gate → `spec export`
/ `spectcl_expand` (small: snapshot → canonical renderer, shared with
the studio) → studio store reads through the eval loader (canonical
packs unchanged, gate: studio round-trip suite) → `render_spectcl`
emits 2.0 and the pin lifts → StudioOverride patch-pack editing for
programmed packs → WASM job. The schema-churn from M-items proceeds
independently behind the coverage witnesses.

**Status (corrected 2026-08-27): everything above has landed.** An earlier
revision of this block listed the `render_spectcl` 2.0 emission as the one
outstanding item; it had in fact landed one wave earlier, in the same change
as `spec export`, and this block was written without that knowledge. The
renderer is version-aware, `DSL_VERSION` is `tcl_spectcl::NEWEST_VOCABULARY_VERSION`
(`"2.0"`), and the unpin condition is documented at the constant.

- ✅ **Evaluation loader + equivalence gate** — `tcl-spectcl`'s
  `loader::eval`, gated by `tests/eval_loader.rs`.
- ✅ **`spec export` / `spectcl_expand`** — `tcl_spectcl::export`,
  rendering `Pack::registrations`; gated by `tests/export.rs`.
- ✅ **Studio store reads through the eval loader** —
  `PackStore::from_source` evaluates, `declaration_site` reports
  expansion provenance, and the round-trip suite is unchanged.
- ✅ **`render_spectcl` emits 2.0 and the pin lifts** — `availability_rows`
  writes `available` / `-available` at all seven scopes the loader reads
  it, a document's own header decides its spelling so 1.x drafts keep
  `dialects` and never wear newer words (closing a latent #1627 path in
  `render_block`), and `DSL_VERSION` lifted to `2.0` per the rewritten
  condition at the constant. The round-trip gate caught the availability
  window's exclusive upper bound as 270 real differences before the fix.
- ✅ **StudioOverride patch-pack editing (E-R12)** —
  `PackStore::programmed` classifies the document,
  `WriteBack::Patched` routes a form edit into a canonical patch pack
  (`export_pack` of the edited commands plus the base's `default`
  context rows), `PackStore::pack_set` layers it after the base at
  `Tier::StudioOverride`, and `PackStore::standing_overrides` is the
  queryable report. Canonical packs keep in-place editing untouched,
  which `tests/pack_store.rs`'s canonicality guard holds over every
  hand-written pack in the tree.
- ✅ **WASM job** — the studio wasm links and *runs* the evaluation
  loader; `rust/tcl-spec-studio-wasm/test/eval-loader.mjs` loads a
  canonical and a templated fixture pack through the shipped exports
  and exercises the patch path, and `make spec-studio-wasm` runs it.
  The one target difference is documented at
  `tcl_spec_hooks::pack_eval::PACK_EVAL_WALL_CLOCK`: on
  `wasm32-unknown-unknown` the wall-clock budget is not armed (a
  page's `Date.now()` is throttled in a backgrounded tab, and with
  `tcl-vm`'s `js-clock` off it reports the epoch), so a browser
  evaluation is bounded by the command-step and value-size budgets —
  the two axes the VM measures itself, and the ones §1.2's determinism
  contract actually wants.
