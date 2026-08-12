# Def-Use Chains

## Overview

Def-use chains link each SSA variable definition to all statements that
read (use) it.  They are built in two passes over the SSA function
after SSA construction by `build_def_use_chains(&ssa, Some(&cfg))`
(`rust/tcl-compiler/src/def_use.rs`), and stored as `FunctionUnit::def_use`,
an `Arc<DefUseResult>` shared rather than deep-copied because the result is
span-free and therefore survives offset rebasing untouched.

The CFG argument is optional and carries the terminator reads — branch
conditions and `return` values — that the SSA function itself does not
record.  Passing `None` yields chains without them.

## Data Structure

```rust
// A *name*-keyed value key — distinct from `ssa::ValueKey`, which is
// `(Symbol, Version)` over the SSA function's interned variable names.
pub type SsaValueKey = (String, Version);

pub struct DefUseResult {
    pub chains: HashMap<SsaValueKey, DefUseChain>,
}

pub struct DefUseChain {
    pub key: SsaValueKey,       // (variable_name, ssa_version)
    pub definition: DefSite,    // where the value is defined
    pub uses: Vec<UseSite>,     // all sites that read it
}

pub struct DefSite {
    pub block: String,
    pub kind: DefKind,          // Statement | Phi | Parameter
    pub statement_index: i32,   // -1 for phi/parameter
}

pub struct UseSite {
    pub block: String,          // for PhiIncoming: the *predecessor* block
    pub kind: UseKind,          // Operand | PhiIncoming | Terminator
    pub statement_index: i32,   // -1 for phi-incoming and terminator uses
    pub variable: String,       // for phi: the phi variable
    pub phi_version: Version,   // for phi: the phi's version
    pub class: UseClass,        // Substituted | Quoted
}
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
| `Substituted` | A genuine read here | everyone |
| `Quoted` | Carried only by a brace-quoted word nothing evaluates in this frame | liveness / dead-store only |

The class is carried on the `SsaStatement` as the `quoted_uses` subset of
`uses`, and `build_def_use_chains` copies it onto each `UseSite`.

Liveness, W211, W220 and store elimination must assume a quoted word *may*
be evaluated, so the use has to exist.  Read-before-set (W210 / W213) must
assume it *may not* be, or may be evaluated in a frame that binds the name,
so it skips `Quoted` uses.  Filtering at either end breaks the other:
dropping the use resurrects `W211 set but never used` on `set a(k) 1; puts
{$a(k)}`, and recording the name as a self-initialising def deletes the
feeding store outright (issues #1142, #1237).

### The three kinds of braced word

A braced word is never read *at* the call site.  What it **is** decides the
class, and the answer is registry data rather than a command list
(`ssa::braced_word_class`):

| Kind | Test | Example | Class |
|---|---|---|---|
| script, this frame | the position's `ArgRole` answers `braced_word_evaluated_in_frame` (`Body` / `Expr`) | `expr {$a + $b}`, `if {$c} …` | `Substituted` |
| data | the registry **describes** the command, and the role is not one of those | `puts {$y}`, `string match {$pat*} …`, `lsort -command {cmp $x}` | `Quoted` |
| unclassified | the registry does **not** describe the command | `mywrapper {puts $myf}` | `Substituted`, unless the word `set`s the name itself → `Quoted` |

The unclassified row is the conservative one, and deliberately so: a user
proc's braced argument may be a script, and a wrapper that hands it to an
`uplevel`-ing worker runs it in *this* frame, where a free read of an unset
name is a genuine error tclsh reproduces.  A name the word sets itself is
that script's own local whichever frame it runs in — the shape an un-hooked
definer body takes — so only that is demoted.  It is the `Statement::Call`
twin of the analyser's `barrier_body_locally_sets`.

A braced `return` value (`return {$y}`) is `Quoted` on both the statement
and the terminator read — `terminator_read_vars` reads
`Terminator::Return`'s `braced` flag to decide.  tclsh-proof (8.6.14):
`proc f {} { return {$y} }; puts [f]` prints `$y` with `y` undefined.

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
   record `(name, phi.version)` with `DefKind::Phi` and
   `statement_index: -1`.  For each statement, record each
   `(name, ver)` in `stmt.defs` with `DefKind::Statement` and the
   statement's index.  Each `Symbol` is resolved to its display name
   through `SsaFunction::var_name`, since the chains are name-keyed.

2. **Pass 2 — uses**: Walk every block again.
   - Each phi incoming edge `(pred_block, incoming_ver)` records a
     `UseKind::PhiIncoming` use, filed against the *predecessor* block
     and always `UseClass::Substituted`.
   - Each statement operand `(name, ver)` in `stmt.uses` records a
     `UseKind::Operand` use, classed `Quoted` when the symbol is in the
     statement's `quoted_uses`.
   - When the CFG was supplied, `add_terminator_uses` records a
     `UseKind::Terminator` use for each name a `Terminator::Branch`
     condition or a `Terminator::Return` value reads.

3. **Version 0**: If a use references version 0 (read-before-set), the
   chain is lazily created with `DefKind::Parameter` in the entry block.

## Key Properties

| Property / Method | Meaning |
|-------------------|---------|
| `chain.is_dead()` | No uses at all — candidate for dead-store elimination |
| `chain.use_count()` | Number of use sites |
| `chain.has_phi_use()` | At least one use is a phi incoming edge |
| `result.is_dead(name, ver)` | `true` if the SSA value has no uses |
| `result.uses_of(name, ver)` | All use sites for a given SSA value (an empty slice when none) |
| `result.reaching_defs(name)` | All SSA definitions of a variable across the function |
| `result.chain_for(name, ver)` | The chain for one SSA value, if it exists |
| `result.dead_chains()` | Every chain with no uses |
| `result.total_defs()` / `result.total_uses()` | Whole-function counts |

## Consumer Contracts

| Consumer | What it reads | What it produces |
|----------|---------------|-----------------|
| Dead store detection (`dead_stores.rs`) | `chain.is_dead()` | `DeadStore` records → W220 |
| Unused variable detection (`analyser/diagnostics/dataflow.rs`) | `chain.use_count() == 0` | W211 |
| Read-before-set (`analyser/diagnostics/dataflow.rs`) | Version-0 chains, skipping `Quoted` uses | W210 |
| Store-to-load forwarding (O127) | Single-use chains | Optimisation suggestions |
| Data-flow graph (`dataflow_graph.rs`) | All chains | Visualisation nodes and edges |
| Compiler explorer | Per-function chains | JSON for the `dataflow` view |

## Module Location

- **Source**: `rust/tcl-compiler/src/def_use.rs`
- **Integration**: `rust/tcl-compiler/src/compilation_unit.rs` — built by
  `FunctionUnit::build` and held as `FunctionUnit::def_use`
- **Graph export**: `rust/tcl-compiler/src/dataflow_graph.rs`

## Example

Given:
```tcl
set x 1
set y [expr {$x + 1}]
```

Chains:
- `("x", 1)`: def = `Statement` in `entry_1`, uses = [`Operand` in `entry_1`, stmt 1]
- `("y", 1)`: def = `Statement` in `entry_1`, uses = [] → **DEAD**

With branching:
```tcl
if {$cond} { set a 1 } else { set a 2 }
set b $a
```

The CFG builder names these blocks `entry_1`, `if_end_2`, `if_then_3`,
`if_next_4` — the merge block is allocated first, and the `else` body is
lowered into the final `if_next`.  Chains:

- `("cond", 0)`: def = `Parameter` in `entry_1` (read before set), uses = [`Terminator` in `entry_1`]
- `("a", 1)`: def = `Statement` in `if_then_3`, uses = [`PhiIncoming` → `a`#3, filed at `if_then_3`]
- `("a", 2)`: def = `Statement` in `if_next_4`, uses = [`PhiIncoming` → `a`#3, filed at `if_next_4`]
- `("a", 3)`: def = `Phi` in `if_end_2`, uses = [`Operand` in `if_end_2`]
- `("b", 1)`: def = `Statement` in `if_end_2`, uses = [] → **DEAD**

The `a` phi exists because the trailing `set b $a` reads `a` in a block that
does not first redefine it — the upward-exposed use that makes `a` non-local
under semi-pruned SSA.
