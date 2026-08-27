# SpecTcl authoring-surface alternatives: six designs

> **DECIDED (owner, 2026-08-26): design E — executable registration —
> is the SpecTcl 2.0 authoring surface**, adopted together with the
> execution model pinned in
> [the design-E deep dive](spectcl-design-e-deep-dive.md) §1 (frozen
> snapshots, the determinism contract, data-not-control-flow
> conditionality, registration-time trust) and its ratified rulings
> E-R1–E-R9. The recommendation section below is retained as the
> comparison record it was; the deep dive supersedes it as the basis
> of the decision. Migration guarantees stand and are now **proved**: the
> evaluation loader landed with an equivalence gate showing all 24 shipped
> packs — 1,515 commands — load byte-identically through the CST loader
> and the evaluator alike, hooks, clause grammars, degraded flags and
> declaration lines included, so 1.x packs load forever and the 2.0
> declarative rows are valid input verbatim (E's registration commands
> accept them as data).
>
> One promise here is still outstanding: **`tcl spec upgrade` did not grow
> a `--restyle` step.** `tcl spec upgrade` implements U0–U10 and already
> rewrites 1.x rows into 2.0 spellings, so `--restyle` is a formatting
> affordance on top of a landed translation rather than a missing
> capability — tracked as D13 in the
> [redesign's §11 open-questions ledger](dialect-and-package-registry-redesign.md#11-the-open-questions-ledger),
> which is the single list of what this programme left open.
>
> Original status: **EXPLORATION for an owner decision.** A user's complaint —
> SpecTcl is valid Tcl but does not *read* like Tcl — is solid: the
> current surface is flag-soup rows (`arg 0 -role VarWrite -closed`),
> index-addressed arguments, and pseudo-keyword tables that could as
> easily be INI. This document offers six deliberately different
> authoring surfaces over the **same internal model** (the loader's
> word→draft→registry pipeline makes the surface independent of the
> semantics), each shown on one identical worked example so they compare
> directly. Constraints held by every design: published `speclib 1.x`
> packs load forever, untouched; the studio round-trips whatever we
> choose; the 2.0 semantic vocabulary now landing (`available`,
> `environment`, `dialect` blocks, fail-closed classes) is
> surface-neutral and survives any of these skins.

## 0. The rubric, and the baseline

"Tcl-like" unpacked into testable properties:

- **R1 — reads as idiom**: a Tcl programmer recognises the shapes
  (synopsis lines, `proc` signatures, ensembles-as-namespaces, dicts)
  without learning a schema.
- **R2 — names over indices**: arguments referred to by their names, as
  man pages and `proc` do — never `arg 2`.
- **R3 — low ceremony**: the common case (a command, its signature, a
  sentence of hover) is 2–5 lines; annotations only where inference
  cannot reach.
- **R4 — one obvious spelling**: minimal synonym surface; diffable,
  greppable, and mechanically upgradable.
- **R5 — degrades to documentation**: a reader with no tooling still
  learns the command's usage from the spec.

**The worked example** used by all six designs (chosen to exercise the
hard features): `lsort` — positional arg with a role, seven options
including a value-taking `-command` whose value is a script, mutually
exclusive comparison modes, the 8.6-gated `-stride`, hover text — plus a
small ensemble `counter` with subcommands `create`/`incr` (name-creating,
versioned arity: `incr` grew an increment argument in 2.0 of its
package), and one `available` gate.

**Baseline — the current 1.x surface** (for comparison):

```tcl
speclib mylib 1.2 {
    command lsort {
        hover {Sorts the elements of a list, returning a new list.}
        arity 1..
        arg 0 -role list
        available {tcl 8.4-}
        option -ascii
        option -dictionary
        option -integer
        option -real
        option -increasing
        option -decreasing
        option -command -takes script -role CommandPrefix -script-timing deferred
        option -stride -takes count -introduced 8.6
        option_conflict {-ascii -dictionary -integer -real}
    }
    command counter::create {
        hover {Creates a counter instance command.}
        arity 1
        arg 0 -role name
        defines_command_at 0
        required_package counter
    }
    command counter::incr {
        hover {Increments a counter.}
        arity 1..2 -introduced 1.0 -retired 2.0
        arity 2..3 -introduced 2.0
        arg 0 -role name
    }
}
```

Legible, but R1/R2 fail: nothing about `arg 0 -role list` is Tcl idiom,
and the reader assembles the synopsis in their head.

---

## Design A — synopsis-first (the man-page grammar)

**Philosophy.** Tcl already has a universal signature language every
programmer reads daily: the man-page SYNOPSIS. Make that string the
primary declaration; infer arity, optionality, repetition, option
placement, and names from it; attach semantics by *name* only where
inference cannot reach.

```tcl
speclib mylib 2.0 {
    command lsort {?options? list} {
        hover {Sorts the elements of a list, returning a new list.}
        available {tcl 8.4-}
        option -ascii|-dictionary|-integer|-real  ;# | = mutually exclusive
        option -increasing
        option -decreasing
        option {-command cmdPrefix} {role command-prefix; timing deferred}
        option {-stride count} {available {tcl 8.6-}}
        arg list {role list}
    }
    command {counter create counterName} {
        hover {Creates a counter instance command.}
        provides-command counterName
        available {package counter}
    }
    command {counter incr counterName ?increment?} {
        available {package counter 1.0-2.0}
    }
    command {counter incr counterName increment ?step?} {
        available {package counter 2.0-}
    }
}
```

Synopsis grammar: `?word?` optional, `word ...` repetition, literal words
are subcommand path elements, everything else is a named argument. A
versioned arity change is **two synopses**, each carrying its window —
the way a man page across releases would actually differ.

*Strengths*: highest R1/R5 of any design; R2 by construction; versioned
signatures read naturally; the importer already produces synopsis-shaped
output from `Tcl_WrongNumArgs` strings. *Weaknesses*: needs a rigorous
grammar for the synopsis mini-language (edge cases: literal `?`,
alternation, greedy tails); two synopses for one command must merge into
one spec deterministically; deep annotation still needs the block.
*Cost*: a synopsis parser + name-keyed annotation resolution; renderer
straightforward. *Migration*: `tcl spec upgrade` can synthesise synopses
from arity + arg rows mechanically.

---

## Design B — proc-mirror (specs shaped like definitions)

**Philosophy.** The most Tcl-like way to describe a command is the shape
used to *create* one. `spec proc` mirrors `proc`; `spec ensemble`
mirrors `namespace ensemble`; attributes ride as dash-flags after the
signature, like `snit`/`tcltest` options.

```tcl
speclib mylib 2.0 {
    spec proc lsort {args list} -returns list -available {tcl 8.4-} {
        doc {Sorts the elements of a list, returning a new list.}
        flag -ascii -conflicts {-dictionary -integer -real}
        flag -dictionary; flag -integer; flag -real
        flag -increasing; flag -decreasing
        flag -command cmdPrefix -role command-prefix -timing deferred
        flag -stride count -available {tcl 8.6-}
        param list -role list
    }
    spec ensemble counter -available {package counter} {
        spec proc create {counterName} -defines-command counterName {
            doc {Creates a counter instance command.}
        }
        spec proc incr {counterName ?increment?} -until {counter 2.0}
        spec proc incr {counterName increment ?step?} -since {counter 2.0}
    }
}
```

*Strengths*: strong R1 (it looks like writing the library); ensembles
nest as they dispatch; optional params in the `proc`-familiar `?name?`
form; R2 by construction. *Weaknesses*: `proc`'s argument-list syntax
must be extended (optionality, repetition) — a *near*-proc syntax risks
uncanny-valley confusion with real `proc` defaults (`{a {b 1}}` means a
default value in proc, optionality here?); flag-heavy commands still
produce long attribute runs. *Cost*: moderate; the signature parser is
small. *Migration*: mechanical.

---

## Design C — namespace-native (structure mirrors dispatch)

**Philosophy.** Tcl's own organising idiom is the namespace tree.
Command structure *is* namespace/ensemble structure: the pack is a tree
of `in` scopes, and nesting replaces every parent/child pseudo-word
(`subcommand`, `sub_subcommand`, `method`).

```tcl
speclib mylib 2.0 {
    in :: {
        command lsort {
            doc {Sorts the elements of a list, returning a new list.}
            args {?options? list}
            option -ascii -conflicts {-dictionary -integer -real}
            option -increasing
            option -decreasing
            option -command {takes cmdPrefix; role command-prefix; timing deferred}
            option -stride {takes count; available {tcl 8.6-}}
        }
    }
    in ::counter -available {package counter} {
        ensemble
        command create {
            doc {Creates a counter instance command.}
            args {counterName}
            defines-command counterName
        }
        command incr {
            args {counterName ?increment?}          -until {counter 2.0}
            args {counterName increment ?step?}     -since {counter 2.0}
        }
    }
}
```

*Strengths*: kills the three-level `subcommand`/`sub_subcommand`
asymmetry (G16) structurally — depth is just nesting; object classes and
methods fall out (`in ::mytree { class; method walk {…} }`); reads like
a namespace listing. *Weaknesses*: an ensemble's *arm* is not really a
namespace child in Tcl semantics (ensembles map subcommand words), so
the metaphor over-promises; flat commands pay an `in ::` wrapper.
*Cost*: moderate; internal model already close. *Migration*: mechanical.

---

## Design D — pure data (specs are dicts)

**Philosophy.** Tcl's *other* native idiom is the dict. A spec is one
literal nested dict per command — no DSL words at all, maximum
regularity, `dict get`-able by any tool, trivially diffable and
round-trippable.

```tcl
speclib mylib 2.0 {
    command lsort {
        doc     {Sorts the elements of a list, returning a new list.}
        args    {options {optional 1 options 1} list {role list}}
        options {
            -ascii      {conflicts {-dictionary -integer -real}}
            -dictionary {} -integer {} -real {}
            -increasing {} -decreasing {}
            -command    {takes cmdPrefix role command-prefix timing deferred}
            -stride     {takes count available {tcl 8.6-}}
        }
        available {{tcl 8.4-}}
    }
    command {counter create} {
        doc {Creates a counter instance command.}
        args {counterName {defines-command 1}}
        available {{package counter}}
    }
    command {counter incr} {
        forms {
            {args {counterName {} increment {optional 1}}  until {counter 2.0}}
            {args {counterName {} increment {} step {optional 1}} since {counter 2.0}}
        }
    }
}
```

*Strengths*: perfect R4 (one shape: key value), machine-friendliest of
all six; the draft model *is* this shape already, so the loader nearly
disappears; excellent for generated packs (importer output). *Weaknesses*:
weakest R1/R5 — braces-of-braces read like JSON with the commas removed;
empty-dict placeholders (`-integer {}`) are ugly; humans hand-editing
deep dicts misplace braces. *Cost*: lowest. *Migration*: trivial.

---

## Design E — executable registration (a real Tcl DSL)

**Philosophy.** The most Tcl-like thing a Tcl file can be is a *program*.
The pack runs in a locked-down interpreter exposing only registration
commands (the sandboxed engine already exists for hook bodies). Authors
get real Tcl power: loops, conditionals, shared helpers.

```tcl
speclib mylib 2.0
command lsort {?options? list} {
    doc {Sorts the elements of a list, returning a new list.}
    foreach mode {-ascii -dictionary -integer -real} {
        option $mode -conflicts [lremove {-ascii -dictionary -integer -real} $mode]
    }
    option -increasing; option -decreasing
    option -command cmdPrefix -role command-prefix -timing deferred
    if {[available? {tcl 8.6-}]} { option -stride count }   ;# discouraged: prefer the data form
    option -stride count -available {tcl 8.6-}
}
namespace eval-spec counter -available {package counter} {
    command create {counterName} { doc {Creates a counter instance command.}; defines-command counterName }
    command incr {counterName ?increment?} -until {counter 2.0}
    command incr {counterName increment ?step?} -since {counter 2.0}
}
```

*Strengths*: templating kills repetition (the 788-command xilinx pack
shrinks dramatically; `foreach op {+ - * /} {command ::tcl::mathop::$op …}`);
ultimate R1 — it *is* Tcl. *Weaknesses*: inverts the load-time
**read-from-CST-never-execute** principle — loading becomes evaluation
(sandboxed, budgeted, deterministic-mode: no clock/rand/IO), which
raises the trust bar for workspace packs (§6.4 classes must gate
execution by provenance), complicates caching (cache keys become
evaluation-closure hashes), and makes specs harder for *other tools* to
statically read; conditional registration reintroduces "surface depends
on evaluation" — exactly review B5's warning, now inside our own format.
*Cost*: highest. *Migration*: any other design's files are valid input
(the registration commands can accept the declarative forms verbatim).

---

## Design F — annotated stubs (specs are Tcl source)

**Philosophy.** The spec *is* a Tcl source file: real (empty) `proc`
definitions carrying structured annotations — the inline-stub mechanism
(`# tcl-lsp: stub …`) grown into the full format. A pack is loadable by
`tclsh` (harmlessly), doubles as an interface stub for humans, and
`spec-author` output is already nearly this.

```tcl
# speclib mylib 2.0
#@ available {tcl 8.4-}
#@ doc Sorts the elements of a list, returning a new list.
#@ option -ascii -conflicts {-dictionary -integer -real}
#@ option -dictionary ; #@ option -integer ; #@ option -real
#@ option -increasing ; #@ option -decreasing
#@ option -command cmdPrefix -role command-prefix -timing deferred
#@ option -stride count -available {tcl 8.6-}
#@ arg list -role list
proc lsort {args} {}

namespace eval counter {
    #@ available {package counter}
    #@ doc Creates a counter instance command.
    #@ defines-command counterName
    proc create {counterName} {}
    #@ until {counter 2.0}
    proc incr {counterName {increment {}}} {}
    #@ since {counter 2.0}
    proc incr2 {counterName increment {step {}}} {}   ;# name clash forces aliasing — a real weakness
}
```

*Strengths*: R5 is maximal (the file is its own documentation and a
loadable stub library for editors/tooling that know nothing of SpecTcl);
unifies the stub sidecar tier with packs (ruling R1 of the
centralisation doc) into one mechanism. *Weaknesses*: comments carrying
semantics is fragile idiom (reformatters, comment-strippers); versioned
signatures collide with proc-name uniqueness (see `incr2` — needs an
awkward escape); annotations don't nest well for deep structures
(object classes, clause grammars); R4 suffers (`#@` rows are the old
flag-soup again, displaced into comments). *Cost*: low-moderate.
*Migration*: mechanical for simple packs, lossy pressure on complex ones.

---

## Comparison

| | A synopsis | B proc-mirror | C namespace | D dict | E executable | F stubs |
|---|---|---|---|---|---|---|
| R1 reads as idiom | ★★★ | ★★★ | ★★☆ | ★☆☆ | ★★★ | ★★☆ |
| R2 names not indices | ★★★ | ★★★ | ★★★ | ★★★ | ★★★ | ★★☆ |
| R3 low ceremony | ★★★ | ★★☆ | ★★☆ | ★☆☆ | ★★★ | ★★☆ |
| R4 one spelling / tooling | ★★☆ | ★★☆ | ★★★ | ★★★ | ★☆☆ | ★☆☆ |
| R5 degrades to docs | ★★★ | ★★☆ | ★★☆ | ★☆☆ | ★☆☆ | ★★★ |
| deep-structure coverage (G7/G15/G16) | ★★☆ | ★★☆ | ★★★ | ★★★ | ★★★ | ★☆☆ |
| static readability / trust | ★★★ | ★★★ | ★★★ | ★★★ | ★☆☆ | ★★★ |
| implementation cost | med | med | med | low | high | low-med |
| generated-pack fit (importer) | ★★★ | ★★☆ | ★★☆ | ★★★ | ★☆☆ | ★★☆ |

## Recommendation (for comparison, not a decision)

The axes are separable, which invites a hybrid rather than a pure
winner: **A's synopsis line as the signature** (it is the single biggest
Tcl-likeness win and the importer already speaks it), **C's nesting for
structure** (killing the subcommand-depth pseudo-words), **name-keyed
annotation blocks** shared by both, and **D as the canonical *generated*
form** (importer/studio output, byte-stable) with A/C as the *authoring*
form — one loader, one draft model, two sanctioned surfaces with the
renderer able to emit either. E is best deferred: its templating win is
real for mega-packs but it re-imports review B5's evaluation problem
into our own format; if wanted later, it can *emit* A/C/D rather than
being the storage format. F's unification instinct is right but belongs
to the stub tier (ruling R1), not the pack format.

Whichever surface wins becomes the 2.0 **authoring** syntax; the 2.0
semantic vocabulary now landing is unaffected (it attaches at the
annotation layer of every design), 1.x packs load forever, and
`tcl spec upgrade` grows a `--restyle` step translating 1.x rows into
the chosen surface.

---

# Hooks and the five-domain examples

Hooks are where "Tcl-like" is hardest, because a hook is *behaviour*:
either a **native reference** (a stable id naming a compiled resolver —
the hot-path form, `28 µs` Tcl-body vs `410 ns` native) or a **Tcl
body** with declared inputs so the result is shape-cacheable. Every
design must say where both forms live. The canonical semantic content,
rendered once per design below:

| # | Domain | Facts to express |
|---|---|---|
| T | Tcl `set` (+ `string repeat`) | synopsis `set varName ?value?`; a **role resolver** — one arg ⇒ `varName` is a variable *read*, two ⇒ a *write* (native id `set-roles`, also shown as a Tcl body); `string repeat` carries `const_fold -native string-repeat` |
| K | Tk `button` | `button pathName ?options?`; **creates the instance command** at `pathName` (widget class `Button` with methods `invoke`/`cget`/`configure` — the invocation-refinement case); `-text` takes text; `-command` takes a script, **timing deferred**, callback-taint inputs from the widget environment; provider `package Tk` on Tk's own axis |
| L | tcllib `struct::tree` | `struct::tree ?treeName?` creates an object command whose method set is **runtime-extensible** (`dynamic_surface`); method `walk`: `tree walk node ?-order order? -command cmd`, `cmd` deferred with `%n`/`%t` substitutions and **completion code 5 scoped to that callback** (census G13); `package struct::tree 2.1-` |
| S | SpiceGenTcl | `Resistor create name value ?-model model? ?-tc1 tc1?` with the **directional constraint `-tc1 requires -model`** (census G2); class `Simulator` method `runAndRead` carrying a **method-scoped process/file taint sink** (G7/G15 — the shared-`InvocationSpec` case); `available {tcl 9.0-} {package SpiceGenTcl 0.70-}` |
| R | iRules `when` / `HTTP::header` | `when EVENT { body }` top-level declaration; `HTTP::header value name` **taints its result** from the client request; `event_requires HTTP`; family `f5-irules` (closed world comes from the environment, not the spec) |

## A — synopsis-first

Hooks live in the name-keyed annotation block; a resolver's Tcl body
receives arguments **by parameter name**, not index — the synopsis names
become the hook's vocabulary.

```tcl
command {set varName ?value?} {
    hover {Reads or writes a variable.}
    roles -native set-roles
    ;# the same resolver as an inline body, for a private pack:
    ;# roles {call} { if {[llength $call] == 2} {role varName var-read} \
    ;#                else {role varName var-write; role value value} }
}
command {string repeat string count} { fold -native string-repeat; pure }

command {button pathName ?options?} {
    available {package Tk}
    creates-command pathName -class Button
    option {-text text}
    option {-command script} {timing deferred; callback-taint widget-environment}
    class Button {
        method {invoke}                  {effect {invokes -command}}
        method {cget option}             {pure}
        method {configure ?option value ...?}
    }
}

command {struct::tree ?treeName?} {
    available {package struct::tree 2.1-}
    creates-command treeName -class struct.tree -dynamic-surface
    class struct.tree {
        method {walk node ?-order order? -command cmd} {
            option {-order order} {values {pre post in both}}
            role cmd command-prefix {timing deferred
                substitutions {%n node %t tree}
                completion-codes {5 {continue walking}}}
        }
    }
}

command {Resistor create name value ?-model model? ?-tc1 tc1?} {
    available {tcl 9.0-} {package SpiceGenTcl 0.70-}
    option {-model model}
    option {-tc1 tc1} {requires -model}          ;# directional (G2)
}
command {Simulator} { class Simulator {
    method {runAndRead ?-nodelete?} {
        sink process {runs the simulator binary}   ;# method-scoped (G7)
        sink file-read {reads the raw output}
    }
}}

command {when EVENT body} {
    available {f5-irules}
    top-level-only
    role EVENT event-name
    role body event-body
}
command {HTTP::header value name} {
    available {f5-irules}
    event-requires HTTP
    taint result client-request
}
```

## B — proc-mirror

Hooks are declared like the thing they are — small named definitions —
either referencing a native id or carrying a body; attachment is a
trailing attribute.

```tcl
spec proc set {varName ?value?} -roles @set-roles {
    doc {Reads or writes a variable.}
}
spec resolver @set-roles -native set-roles
;# Tcl-body form a private pack would write:
spec resolver @my-roles {call} {
    if {[llength $call] == 2} { role varName var-read } \
    else { role varName var-write; role value value }
}
spec proc {string repeat} {string count} -pure -fold [native string-repeat]

spec proc button {pathName ?options?} -available {package Tk} \
        -creates {pathName class Button} {
    flag -text text
    flag -command script -timing deferred -callback-taint widget-environment
}
spec class Button {
    spec method invoke {} -effect {invokes -command}
    spec method cget {option} -pure
    spec method configure {?option value ...?}
}

spec proc struct::tree {?treeName?} -available {package struct::tree 2.1-} \
        -creates {treeName class struct.tree dynamic}
spec class struct.tree {
    spec method walk {node ?-order order? -command cmd} {
        flag -order order -values {pre post in both}
        param cmd -role command-prefix -timing deferred \
            -substitutions {%n node %t tree} -completion-codes {5 continue}
    }
}

spec proc {Resistor create} {name value ?-model model? ?-tc1 tc1?} \
        -available {{tcl 9.0-} {package SpiceGenTcl 0.70-}} {
    flag -model model
    flag -tc1 tc1 -requires -model
}
spec class Simulator {
    spec method runAndRead {?-nodelete?} \
        -sink {process {runs the simulator binary}} \
        -sink {file-read {reads the raw output}}
}

spec proc when {EVENT body} -available f5-irules -top-level-only {
    param EVENT -role event-name
    param body  -role event-body
}
spec proc {HTTP::header value} {name} -available f5-irules \
    -event-requires HTTP -taints-result client-request
```

## C — namespace-native

Hooks are first-class members of a scope; sharing is lexical (a hook
defined at an outer `in` is visible below) — the most natural *reuse*
story of the declarative designs.

```tcl
in :: {
    hook set-roles -native set-roles
    command set {
        args {varName ?value?}
        roles set-roles
    }
    command string { ensemble
        command repeat { args {string count}; pure; fold -native string-repeat }
    }
}

in ::tk -available {package Tk} {
    command button {
        args {pathName ?options?}
        creates-command pathName -class ::tk::Button
        option -text {takes text}
        option -command {takes script; timing deferred; callback-taint widget-environment}
    }
    class Button {
        method invoke  {effect {invokes -command}}
        method cget    {args {option}; pure}
        method configure {args {?option value ...?}}
    }
}

in ::struct -available {package struct::tree 2.1-} {
    command tree {
        args {?treeName?}
        creates-command treeName -class tree -dynamic-surface
    }
    class tree {
        method walk {
            args {node ?-order order? -command cmd}
            option -order {takes order; values {pre post in both}}
            role cmd command-prefix {timing deferred
                substitutions {%n node %t tree}
                completion-codes {5 {continue walking}}}
        }
    }
}

in ::SpiceGenTcl -available {{tcl 9.0-} {package SpiceGenTcl 0.70-}} {
    class Resistor {
        method create {
            args {name value ?-model model? ?-tc1 tc1?}
            option -tc1 {takes tc1; requires -model}
        }
    }
    class Simulator {
        method runAndRead {
            args {?-nodelete?}
            sink process {runs the simulator binary}
            sink file-read {reads the raw output}
        }
    }
}

in :: -available f5-irules {
    command when { args {EVENT body}; top-level-only
                   role EVENT event-name; role body event-body }
    command HTTP::header { ensemble
        command value { args {name}; event-requires HTTP; taint result client-request }
    }
}
```

## D — pure dict

Hooks are entries whose value is either `{native ID}` or
`{params … body …}` — a body is data (a string), which is exactly how
the loader stores it today.

```tcl
command set {
    args  {varName {} value {optional 1}}
    hooks {roles {native set-roles}}
}
command {string repeat} { args {string {} count {}} pure 1 hooks {fold {native string-repeat}} }

command button {
    available {{package Tk}}
    args {pathName {creates-command {class Button}} options {optional 1 options 1}}
    options {
        -text    {takes text}
        -command {takes script timing deferred callback-taint widget-environment}
    }
    class {Button {
        invoke    {effects {{invokes -command}}}
        cget      {args {option {}} pure 1}
        configure {args {option {optional 1 repeat 1} value {optional 1 repeat 1}}}
    }}
}

command struct::tree {
    available {{package struct::tree 2.1-}}
    args {treeName {optional 1 creates-command {class tree dynamic 1}}}
    class {tree {
        walk {
            args {node {} order {option -order values {pre post in both}}
                  cmd {option -command role command-prefix timing deferred
                       substitutions {%n node %t tree} completion-codes {5 continue}}}
        }
    }}
}

command {Resistor create} {
    available {{tcl 9.0-} {package SpiceGenTcl 0.70-}}
    args {name {} value {} model {option -model} tc1 {option -tc1 requires -model}}
}
command Simulator { class {Simulator {
    runAndRead {args {nodelete {option -nodelete}}
                sinks {{process {runs the simulator binary}}
                       {file-read {reads the raw output}}}}
}}}

command when {
    available {f5-irules} top-level-only 1
    args {EVENT {role event-name} body {role event-body}}
}
command {HTTP::header value} {
    available {f5-irules} event-requires HTTP
    args {name {}} taint {result client-request}
}
```

## E — executable registration

Hooks are **real procs** — defined, named, shared, generated. This is
the design where hook authoring is genuinely native; the same
sandboxing/budgeting that runs 1.x hook bodies runs the whole file.

```tcl
proc roles::set {call} {
    if {[llength $call] == 2} { role varName var-read } \
    else { role varName var-write; role value value }
}
command set {varName ?value?} -roles roles::set          ;# or -roles [native set-roles]
command {string repeat} {string count} -pure -fold [native string-repeat]

package-surface Tk {
    command button {pathName ?options?} -creates {pathName class Button} {
        option -text text
        option -command script -timing deferred -callback-taint widget-environment
    }
    class Button {
        method invoke {} -effect {invokes -command}
        method cget {option} -pure
        method configure {?option value ...?}
    }
    ;# templating: every themed widget shares the callback shape
    foreach w {checkbutton radiobutton menubutton} {
        command $w {pathName ?options?} -creates [list pathName class [string totitle $w]] {
            option -command script -timing deferred -callback-taint widget-environment
        }
    }
}

package-surface struct::tree -min 2.1 {
    command struct::tree {?treeName?} -creates {treeName class tree dynamic}
    class tree { method walk {node ?-order order? -command cmd} {
        option -order order -values {pre post in both}
        param cmd -role command-prefix -timing deferred \
            -substitutions {%n node %t tree} -completion-codes {5 continue}
    }}
}

package-surface SpiceGenTcl -min 0.70 -core {tcl 9.0-} {
    class Resistor { method create {name value ?-model model? ?-tc1 tc1?} {
        option -tc1 tc1 -requires -model
    }}
    class Simulator { method runAndRead {?-nodelete?} \
        -sink {process {runs the simulator binary}} \
        -sink {file-read {reads the raw output}} }
}

family-surface f5-irules {
    command when {EVENT body} -top-level-only \
        -role {EVENT event-name} -role {body event-body}
    command {HTTP::header value} {name} -event-requires HTTP \
        -taints-result client-request
}
```

## F — annotated stubs

Hook bodies get F's one redemption: since the file is real Tcl, hooks
are **real procs in the stub file**, referenced by annotations — no
semantics squeezed into comments. Structure (classes, scoped completion
codes) is where F strains hardest.

```tcl
# speclib examples 2.0
proc @roles::set {call} {
    if {[llength $call] == 2} { role varName var-read } \
    else { role varName var-write; role value value }
}
#@ roles @roles::set
proc set {varName {value {}}} {}
#@ pure; fold -native string-repeat
proc ::string::repeat {string count} {}      ;# ensemble arm as namespaced stub

#@ available {package Tk}
#@ creates-command pathName -class Button
#@ option -command script -timing deferred -callback-taint widget-environment
#@ option -text text
proc button {pathName args} {}
#@ class Button method
proc @Button::invoke {} {}                    ;# @-procs carry class members
#@ class Button method -pure
proc @Button::cget {option} {}

#@ available {package struct::tree 2.1-}
#@ creates-command treeName -class tree -dynamic-surface
proc struct::tree {{treeName {}}} {}
#@ class tree method
#@ option -order order -values {pre post in both}
#@ param cmd -role command-prefix -timing deferred \
#@     -substitutions {%n node %t tree} -completion-codes {5 continue}
proc @tree::walk {node args} {}

#@ available {tcl 9.0-} {package SpiceGenTcl 0.70-}
#@ option -tc1 tc1 -requires -model
proc @Resistor::create {name value args} {}
#@ class Simulator method
#@ sink process {runs the simulator binary}
#@ sink file-read {reads the raw output}
proc @Simulator::runAndRead {args} {}

#@ available f5-irules
#@ top-level-only
#@ role EVENT event-name ; #@ role body event-body
proc when {EVENT body} {}
#@ available f5-irules -event-requires HTTP -taints-result client-request
proc @HTTP::header::value {name} {}
```

## The compilation target (owner goals, restated as constraints)

The goal set: the pack must **compile into a performant form the LSP
consumes**, with **hooks executing on the tclvm** where deeper analysis
needs them (option shapes, dynamic roles), **versions expressed well**,
and first-class understanding of **TclOO, iRules, Tk, and the EDA
libraries**. Consequences for this comparison:

- **Performance is surface-neutral.** Every design parses to the same
  draft and compiles to the same loaded form: specs interned once per
  generation; **Tcl-body hooks compile once to tclvm bytecode at pack
  load** (replacing today's 28 µs re-interpretation), execute under the
  sandboxed budgeted VM, and shape-cache their results exactly as
  `-inputs` declarations allow today (24.5 ns cached). Native-ID hooks
  stay function pointers. So no surface buys speed — the AOT pipeline
  (`tcl spec build`) equalises them, and the choice is purely
  ergonomics, safety, and tooling.
- **Hooks-on-tclvm favours designs where a hook is a real proc** (E, F)
  or a named block (A/B/C) — all five compile identically; D's
  body-as-dict-value compiles the same but authors worst. The
  hook-compilation step also gives every design load-time hook
  *validation* for free (a hook that doesn't compile is a load error,
  not a runtime abstention).
- **Versions**: the `available {tcl 8.6-} {package X 2.1-}` VersionSet
  rows attach uniformly; A and B additionally version the *signature
  itself* (one synopsis per window), which is the most honest rendering
  of versioned arity — D's `forms` list is its generated twin.
- **TclOO / Tk / EDA**: the class/method examples above are the test —
  and they show the requirement lands on the **model** (shared
  `InvocationSpec`: methods carry sinks, timing, effects, forms), not
  the surface. C renders it most naturally, F worst.
- **iRules**: family availability + event rows suffice at the surface;
  grammar and closed-world policy stay in the dialect/environment
  layers where the redesign put them.

Net effect on the recommendation: unchanged in direction, stronger in
reasoning — since compilation equalises performance, E's only unique
advantage (authoring-time templating) must be weighed against its unique
cost (a surface whose content depends on evaluation, which is exactly
what the compiled form must be able to cache deterministically), and the
A+C authoring / D generated hybrid keeps every goal: fast compiled
packs, tclvm hooks as named compiled bodies, per-window synopses for
versions, and class members as first-class invocation specs.

## Prose blocks: multi-line text without indentation damage

Braced Tcl strings are byte-verbatim, so today a long `hover` either
hugs column 0 (breaking the file's visual nesting) or ships its leading
spaces into every tooltip. The fix is a **prose value class** shared by
every surface: values declared prose-typed in the schema (`hover`/`doc`,
sink and effect descriptions, deprecation advice, values `-detail`) are
**margin-stripped at load** by one deterministic rule:

1. a newline immediately after the opening brace is dropped;
2. the *margin* is the longest common byte-prefix of whitespace across
   all non-blank lines (no tab expansion — bytes, so mixed tab/space
   files stay deterministic);
3. the margin is stripped from every line; trailing blank lines and
   per-line trailing whitespace are trimmed;
4. a blank line is a paragraph break; a single newline is *soft*
   (renderers may reflow) — **except** inside a fenced block
   (```` ``` ````), which is preserved verbatim relative to the margin,
   so worked examples in hovers keep their own indentation;
5. `doc -verbatim {…}` is the escape hatch for byte-exact needs, and
   prose values must be braced words (which also makes `#` inside prose
   a non-issue — the "`arity 2 # not a comment`" trap cannot bite a
   braced value).

So this indents with the code and renders clean:

```tcl
command {lsort ?options? list} {
    hover {
        Sorts the elements of a list, returning a new list in sorted
        order. The default comparison is ASCII.

        -integer, -real, and -dictionary select other comparisons:

        ```
        % lsort -integer {10 9 8}
        8 9 10
        ```
    }
}
```

Consequences, per design and for the pipeline:

- **Round-trip becomes canonical rather than byte-exact for prose**: the
  renderer re-emits at the current nesting depth and `load(render(x))`
  equals `x` *post-dedent* by construction — which retires the standing
  "a quoted word is not byte-verbatim" `Loss` class for prose fields
  entirely. Equality in the studio gate is post-dedent equality for
  prose-typed values, byte equality everywhere else.
- **Unbalanced braces in prose** remain the one honest wart (Tcl's, not
  ours): the rule stays "balance your braces or use `-verbatim` with
  backslash escapes", and the loader's existing reported-`Loss` path
  covers the pathological cases.
- **F is the natural winner for prose** — doc lines are comment lines
  directly above the stub (`## Sorts the elements…`, Rust-style), so
  indentation never enters the value at all; **D is the loser** (prose
  inside nested dict braces indents twice); A/B/C/E share the block rule
  above unchanged.
- **1.x compatibility**: existing single-line hovers are unaffected (no
  margin to strip); existing multi-line 1.x hovers — all currently
  column-0 — dedent to themselves. The upgrade tool may *re-indent*
  prose to nesting depth as part of `--restyle`, purely cosmetic under
  post-dedent equality.
- The same class covers the other rendering-sensitive shapes the
  complaint gestures at: per-paragraph deprecation guidance, multi-line
  `sink` rationales, and KCS-bound long descriptions all become prose
  values; grammar-bearing strings (synopses, patterns, version sets)
  are **never** prose-typed and stay byte-exact.

## Typing arguments and options — statically, and dynamically in hooks

### The static vocabulary

One closed, structured type-expression language shared by every
surface (types are model data; the surfaces only spell them):

```text
any  string  int  wide  double  boolean  index  version  requirement
list ?(T)?   dict ?(K V)?   tuple(T1 T2 …)   or(T1 T2 …)
enum(a b c)  script  command-prefix  varname ?(T)?  namespace  channel
path  pattern(glob)  pattern(regex)  class(Name)  object(Name)  widget-path
```

Spelling per design: the annotation form is uniform —
`type list(int)` on a named parameter, `takes count:int` /
`option {-stride count:int}` on options — and A/B/F additionally admit
the **colon shorthand inside signatures** for the simple cases,
inherited from the existing inline-stub DSL (`foreach_in_collection
{varName:var collection body:body}` is already shipping syntax):

```tcl
command {lrepeat count:int element:any ...} { returns list }
command {lsort ?options? list:list}        { returns list }
spec proc lappend {varName:varname(list) ?value:any ...?} -returns list
```

`varname(T)` types the *variable being named*, not the word — which is
how `-textvariable` and `upvar`-shaped options type the cell they bind.
Existing bespoke fields (`pattern_type`, `format_string_type`,
`var_write_typing`, `inferred_storage_type`) become instances of this
one vocabulary rather than parallel mechanisms — a ledger-style
retirement inside the format itself.

### Dynamic types: the `types` hook

A type resolver is a hook like the role resolver — tclvm-compiled,
shape-cached over declared inputs, addressing parameters **by name** —
whose one soundness rule is: **a hook may narrow a static type, never
widen it, and abstains by saying nothing** (falling back to the static
declaration). The sandbox exposes three verbs: `literal NAME` (the
statically-known literal value of a parameter, or abstention),
`type NAME TYPE` (repetition tails addressed `type {arg 0} T`), and
`returns TYPE`.

The canonical case — `format`, whose argument types depend on the
format-string literal:

```tcl
command {format formatString:string ?arg:any ...?} {
    returns string
    types -inputs {literal formatString} {call} {
        set fmt [literal formatString]        ;# abstains when not literal
        set i 0
        foreach spec [format::specs $fmt] {
            type [list arg $i] [format::spec-type $spec]   ;# %d→int, %s→string, %f→double
            incr i
        }
    }
    ;# shipped packs use the hot path:  types -native format-arg-types
}
```

`string is`, where one argument's *value* selects another's type:

```tcl
command {string is class:enum(alpha alnum integer double boolean list dict ...) ?options? value:string} {
    returns boolean
    types -inputs {literal class} {call} {
        switch -- [literal class] {
            integer  { type value int }
            double   { type value double }
            boolean  { type value boolean }
            list     { type value list }
            dict     { type value dict }
        }
    }
}
```

The same contract covers the domain sweep's needs: `dict get`'s return
type from path depth, `lindex`'s element type, Tk `-validatecommand`'s
per-widget `%`-substitution types, and SpiceGenTcl's `-model` value
narrowing to an `enum(...)` computed from a sibling option. In C the
hook is a scope member (`hook format-types … { … }` referenced by
`types format-types`); in E it is an ordinary proc
(`proc types::format {call} {…}; command format … -types types::format`);
in D the body is a dict value; in F a real `@`-proc in the stub file —
identical semantics everywhere, compiled once to the VM at pack load.

Downstream, hook-narrowed types feed the type-inference pass as
spec-grade facts with provenance; a conflict between a hook narrowing
and an inferred value is a diagnostic, never a silent override — the
same discipline as every other semantic hook under invariant I4.

## Operator commands: `tcl::mathop::` and `tcl::mathfunc::`

These two namespaces look like ordinary command families a pack would
author, and no design should author them. Each is one canonical entity
with **two projections** — an expr-grammar surface and a command-table
surface — and the projections have *different* availability, so a
hand-written `command ::tcl::mathop::+ …` row is wrong twice over: it
duplicates a fact the resolved `CoreProfile` already owns, and it
carries only one of the two gates.

**Ruling: derived, never authored.** The registry assembly emits the
command projections from the resolved core's `ExprGrammar` — every
operator spelling (`precedence` + `word_operators` +
`symbolic_operators`) becomes a `tcl::mathop::` command entry, and
every member of the `mathfuncs` set becomes a `tcl::mathfunc::` command
entry — exactly as the VM already registers its bindings from the
derived tables (ledger B3) and the runtime before it. The
hand-maintained generated files and the old `Traits::OPERATOR_COMMAND`
/ `DialectProfile::operators_as_commands` gating axis retire into this
derivation alongside C12. Design E's templating strength
(`foreach op {+ - * /} {command ::tcl::mathop::$op …}`) is therefore a
non-goal for this family: the loop would re-author a derived surface.

The two availability axes stay typed and distinct, as the registry's
`mathfunc` module already insists:

| axis | question | gate |
|---|---|---|
| expr grammar | is `abs(…)` a function, is `+` an operator, here? | `ExprGrammar` membership — `abs(1)` works in Tcl 8.4 and in iRules (fork of 8.4.6) |
| command table | does the command `::tcl::mathfunc::abs` / `::tcl::mathop::+` exist here? | family `tcl` ∧ release ≥ 8.5 (TIP 232 / TIP 174); TIP 461's `lt`/`le`/`gt`/`ge` from 9.0 on our ladder; absent in iRules and Jim |

So an 8.5–9.0 range target renders `expr {abs($x)}` clean everywhere
but flags `::tcl::mathfunc::abs 1` nowhere, while an 8.4-inclusive or
iRules target flags only the command spelling — both answers falling
out of one entity's two gates, not two entries.

**Resolution is already centralised and stays so.** In-expr `NAME(` is
*not* an ordinary command lookup: it dispatches on `tcl::mathfunc::NAME`
resolved relative to the calling namespace, then globally (TIP 232 —
the rule is oracle-verified in `tcl-registry/src/mathfunc.rs`, the
single home for the prefix and both gates). Under R-c this is just one
more candidate shape feeding the one `exists` oracle: a user
`proc ::tcl::mathfunc::f {x} {…}` is a `Must` binding that makes
`expr {f(2)}` resolve with no spec involved; a namespace-local
`::foo::tcl::mathfunc::f` shadows it for expr sites inside `::foo`; and
`is_in_mathfunc_namespace` keeps the reverse door shut — a proc living
there is never reachable as a bare command word. `namespace path
::tcl::mathop` making bare `+` a command needs no special case at all:
the derived entries are real commands, so ordinary path candidates find
them. Note the asymmetry: `mathfunc` is extensible in both directions
(defining the command adds the expr function), but `mathop` is
one-directional — the grammar projects into commands, and creating a
command named `::tcl::mathop::foo` never adds an `expr` operator.

**What SpecTcl does author.** Two things only. Prose: hover and doc
text attaches to the canonical entity and renders on both projections
(`+` at an expr site and `::tcl::mathop::+` at a command site share one
description). And *package-added math functions*, via a first-class
`mathfunc` declaration — first-class because authoring the qualified
command spelling by hand would under-declare, registering only one
projection:

```tcl
# A / C — an EDA or numerics pack adding an expr function
mathfunc db20 {x:double} {
    returns double
    doc {20·log₁₀(|x|) — the dB conversion used throughout the
         measurement commands.}
}
```

The loader registers both projections (the expr-function name and the
`::tcl::mathfunc::db20` command), infers the Tcl-table gate
(`available {tcl 8.5-}` — script-level extension does not exist
before TIP 232, so a pack claiming 8.4 for a `mathfunc` fails the load
with an availability diagnostic), and the declaration takes the same
static types and `types` hooks as any command. The derived core
entries need no hooks: their types come from the operator signatures
themselves (`+` variadic `numeric`, `eq`/`in` exactly-two returning
`boolean`, `!` exactly-one, TIP 461's ordering four string-typed).

One terminology guard: none of this is an *alias* in the `interp
alias` sense — the projections are real commands derived from one
entity. Actual `interp alias` creation in user source stays where the
redesign put it: an analyser transition producing alias bindings
through the same R-c oracle, with registry alias data reserved for
historical spellings.

## What the domain sweep shows

- **Hooks separate the designs sharply.** E and F make Tcl-body hooks
  *real procs* (natural, testable in isolation); A/B/C give them named
  declarative slots with the body as one block; D stores the body as a
  dict value — honest about what it is, worst to author. Native-ID
  references are equally clean everywhere.
- **A's name-keyed resolvers** are a quiet correctness win: the `set`
  resolver speaks `varName`/`value`, and renaming a parameter in the
  synopsis breaks the hook *visibly* at load rather than silently
  shifting indices.
- **Tk and SpiceGenTcl stress the same joint** — class members carrying
  command-grade semantics (effects, sinks, timing). C handles it most
  gracefully (classes are just scopes); F strains visibly (`@Class::`
  naming conventions); all six confirm the shared-`InvocationSpec`
  ruling: the *model* must let methods carry what commands carry, or
  every surface fakes it.
- **tcllib's `walk`** (scoped completion code + substitutions on a
  deferred callback) fits every design once the model carries it — the
  surface differences are cosmetic here.
- **iRules needs almost nothing special** at the surface: family
  availability + event requirements + taint rows; the closed world stays
  environment policy, exactly as ruled (B12).
- **E's templating** is the only design that *shrinks* the Tk example
  (the themed-widget loop) — and the only one whose surface can hide a
  registration bug behind control flow.

