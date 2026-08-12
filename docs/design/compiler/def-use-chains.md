# KCS: Def-Use Chains

## Overview

Def-use chains link each SSA variable definition to all statements that
read (use) it.  They are built in two passes over the SSA function
after SSA construction, and stored in `FunctionAnalysis.def_use_chains`.

## Data Structure

```
DefUseResult
  chains: dict[SSAValueKey, DefUseChain]

DefUseChain
  key: SSAValueKey          # (variable_name, ssa_version)
  definition: DefSite       # where the value is defined
  uses: list[UseSite]       # all sites that read it

DefSite
  block: BlockName
  kind: DefKind             # STATEMENT | PHI | PARAMETER
  statement_index: int      # -1 for phi/parameter

UseSite
  block: BlockName
  kind: UseKind             # OPERAND | PHI_INCOMING | TERMINATOR
  statement_index: int
  variable: str             # for phi: the phi variable
  phi_version: int          # for phi: the phi's version
  class: UseClass           # SUBSTITUTED | QUOTED
```

## Use classification (`UseClass`)

A use is **classified**, not merely present or absent, because the two
families of consumer need opposite conservatism about the same word.

Tcl substitutes `$name` in a bare or `"`-quoted word and never in a
brace-quoted one: `puts {$y}` prints the two characters `$y` and reads
nothing.  A braced word's contents may still be *evaluated* later — by
`expr`, by `if`, by an `after` callback, by an unknown definer — but when,
and in which frame, is the callee's business.

| `UseClass` | Meaning | Who honours it |
|---|---|---|
| `SUBSTITUTED` | A genuine read here | everyone |
| `QUOTED` | Carried only by a brace-quoted word nothing evaluates in this frame | liveness / dead-store only |

Liveness, W211, W220 and store elimination must assume a quoted word *may*
be evaluated, so the use has to exist.  Read-before-set (W210 / W213) must
assume it *may not* be, or may be evaluated in a frame that binds the name,
so it skips `QUOTED` uses.  Filtering at either end breaks the other:
dropping the use resurrects `W211 set but never used` on `set a(k) 1; puts
{$a(k)}`, and recording the name as a self-initialising def deletes the
feeding store outright (issues #1142, #1237).

### The three kinds of braced word

A braced word is never read *at* the call site.  What it **is** decides the
class, and the answer is registry data rather than a command list
(`ssa::braced_word_class`):

| Kind | Test | Example | Class |
|---|---|---|---|
| script, this frame | the position's `ArgRole` answers `braced_word_evaluated_in_frame` (`Body` / `Expr`) | `expr {$a + $b}`, `if {$c} …` | `SUBSTITUTED` |
| data | the registry **describes** the command, and the role is not one of those | `puts {$y}`, `string match {$pat*} …`, `lsort -command {cmp $x}` | `QUOTED` |
| unclassified | the registry does **not** describe the command | `mywrapper {puts $myf}` | `SUBSTITUTED`, unless the word `set`s the name itself → `QUOTED` |

The unclassified row is the conservative one, and deliberately so: a user
proc's braced argument may be a script, and a wrapper that hands it to an
`uplevel`-ing worker runs it in *this* frame, where a free read of an unset
name is a genuine error tclsh reproduces.  A name the word sets itself is
that script's own local whichever frame it runs in — the shape an un-hooked
definer body takes — so only that is demoted.  It is the `Statement::Call`
twin of the analyser's `barrier_body_locally_sets`.

A braced `return` value (`return {$y}`) is `QUOTED` on both the statement
and the terminator read.

### Collapsed bodies keep their classification

Almost every script in a Tcl file is *lowered* into ordinary CFG blocks
before SSA runs, so the classification above is derived once, per word, at
the statement that owns it.  The one exception is a non-lowered `switch`
(`-glob` / `-regexp`, or `-exact` with a fall-through arm): its arms stay
inside a single opaque `Statement::Switch`, and `ssa::switch_reads` recovers
the names they read so a variable used only in an arm is not reported unused.

That recovery walk (`switch_reads` → `free_reads_in_script` →
`reads_in_script` → `reads_in_stmt`) carries the **same `ClassifiedUses`
pair** the lowered path produces, rather than a bare name set (issue #1266).
Collapsing to names alone made every brace-quoted data word inside an arm a
substituted read, so `switch -glob $z { a* { puts {$b} } }` drew a false
W210 that the identical body outside an arm did not.

The classification has to be *threaded*, not dropped: omitting the name
instead would take its liveness use with it and resurrect a false
`W220 assignment never read` on `set x 1; switch -glob $z { a* { foreach n
{$x} {} } }` — the guard rail issues #1237 and #1260 established.  The
braced loop value word (`ForeachIterator::list_braced`) is the only extra
input the walk needs beyond what `uses_of_classified` already answers,
because a `Statement::Foreach` inside an opaque arm is walked here rather
than lowered.

## Derivation from SSA

1. **Pass 1 — definitions**: Walk every block.  For each phi node,
   record `(name, phi.version)` with `DefKind.PHI`.  For each
   statement, record each `(name, ver)` in `stmt.defs` with
   `DefKind.STATEMENT`.

2. **Pass 2 — uses**: Walk every block again.  For each phi incoming
   edge `(pred_block, incoming_ver)`, record a `UseKind.PHI_INCOMING`
   use.  For each statement operand `(name, ver)` in `stmt.uses`,
   record a `UseKind.OPERAND` use.

3. **Version 0**: If a use references version 0 (read-before-set), the
   chain is lazily created with `DefKind.PARAMETER` in the entry block.

## Key Properties

| Property / Method | Meaning |
|-------------------|---------|
| `chain.is_dead` | No uses at all — candidate for dead-store elimination |
| `chain.use_count` | Number of use sites |
| `chain.has_phi_use` | At least one use is a phi incoming edge |
| `result.is_dead(name, ver)` | True if the SSA value has no uses |
| `result.uses_of(name, ver)` | All use sites for a given SSA value (or empty list) |
| `result.reaching_defs(name)` | All SSA definitions of a variable across the function |

## Consumer Contracts

| Consumer | What it reads | What it produces |
|----------|---------------|-----------------|
| Dead store detection | `chain.is_dead` | Improved `DeadStore` precision |
| Unused variable detection | `chain.use_count == 0` | Improved W213/W214 |
| Copy propagation (O127) | Single-def chains | Optimisation suggestions |
| Data-flow graph | All chains | Visualisation nodes and edges |
| Compiler explorer | Per-function chains | JSON for Data Flow tab |

## Module Location

- **Source**: `rust/tcl-compiler/src/def_use.rs`
- **Integration**: `rust/tcl-compiler/src/analyses.rs` / `rust/tcl-compiler/src/sccp.rs` (built in `analyse_function`)
- **Graph export**: `rust/tcl-compiler/src/dataflow_graph.rs`

## Example

Given:
```tcl
set x 1
set y [expr {$x + 1}]
```

Chains:
- `(x, 1)`: def=STATEMENT in entry, uses=[(OPERAND in entry, stmt 1)]
- `(y, 1)`: def=STATEMENT in entry, uses=[] → **DEAD**

With branching:
```tcl
if {$cond} { set a 1 } else { set a 2 }
set b $a
```

Chains:
- `(a, 1)`: def=STATEMENT in if_true, uses=[(PHI_INCOMING → a#3)]
- `(a, 2)`: def=STATEMENT in if_else, uses=[(PHI_INCOMING → a#3)]
- `(a, 3)`: def=PHI in if_next, uses=[(OPERAND in if_next)]
- `(b, 1)`: def=STATEMENT in if_next, uses=[] → **DEAD**
