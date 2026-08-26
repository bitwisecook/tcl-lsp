# SpecTcl authoring-surface alternatives: six designs

> **Status: EXPLORATION for an owner decision.** A user's complaint —
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
