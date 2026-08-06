# TclOO method-resolution + object→class lattice (experiment design)

**Status:** experiment / prototype behind a separate module. **Not** wired
into shipping diagnostics. See `experiments/mro_eval/RESULTS.md` for the
measurements and the ship/no-ship recommendation.

This note describes the *two-halves model* the experiment tests, the C3 /
mixin / filter / `next` semantics actually implemented, and the ⊤
(abstention) taxonomy.

## The two-halves model

TclOO "what does `$obj method` call?" splits cleanly into two composable
questions that want *different* machinery:

1. **MRO is a precomputed graph algorithm, not a lattice.** For a fixed
   class, "which class provides `method`?" is a deterministic walk over
   the linearised superclass/mixin graph. It does not need a fixpoint; it
   needs C3-style linearisation, memoised in the class index.

2. **Object→class binding *is* a lattice.** "What class does the variable
   `$o` hold *here*?" is a flow-sensitive dataflow fact that merges at
   control-flow joins and must be able to say "I don't know". That is a
   lattice with a top element.

3. **Dispatch = (lattice value) × (MRO table).** Join the two: for each
   candidate class in the lattice value, index the MRO/provider table.
   When the lattice value is ⊤, abstain — make no claim.

### Half 1 — MRO graph (already implemented, reused)

The MRO half already exists in the tree and is reused verbatim:

- `analyser/mro.rs` — `tcloo_linearise()` implements TclOO's **actual**
  linearisation from `generic/tclOOCall.c`: a two-pass DFS
  (`BUILDING_MIXINS` then non-mixin) with **late-placement dedup** (a
  re-encountered class moves to the *end* of the chain). This is faithful
  to Tcl 8.6/9.0 and is *not* the same as Python's C3 — the task framed it
  as "C3", but TclOO's own algorithm is what we model, because matching
  the interpreter is what makes a resolution correct. Diamond, mixin
  order, and unknown-parent cases are unit-tested there.

- `analyser/class_hierarchy.rs` — `build_class_hierarchy()` consumes the
  `ClassDef` index (registry-as-source-of-truth: `superclasses`, `mixins`
  come straight off `ClassDef`, populated uniformly for TclOO **and** snit
  by the definer registry) and precomputes:
  - `mro_map`: class → linearised MRO;
  - `method_providers`: `(class, method)` → providing class (the
    devirtualisation table);
  - direct + transitive subclass closures.

- **`next` / `nextto`** (added by this experiment,
  `class_lattice::next_provider`): given the receiver class, the method,
  and the current provider, walk the receiver's MRO *past* the current
  provider to the next class that provides the method. `nextto C m`
  restarts the search at `C`. Unit-tested for the super-chain walk, the
  `nextto` restart, and skipping classes that don't provide the method.

- **Filters** are modelled at the registry/`ClassDef` level (`filters`
  field) and stripped/kept by the `mixins_filters` ablation. A filter
  wraps every call; for *resolution* purposes it does not change which
  concrete method a plain dispatch reaches, so the prototype folds filters
  into the same ablation switch as mixins and measures them together.

Because half 1 reads only `ClassDef` + the definer family, it generalises
to snit (and itcl, if indexed) with no `match cmd_name` — a snit type's
`superclasses`/`mixins` populate the same maps.

### Half 2 — object→class binding lattice (new)

`analyser/class_lattice.rs` defines:

```text
           ⊤   (abstain — tagged with WHY, a TopReason)
          /|\
    Set{A,B,…}      ← control-flow JOIN of concrete bindings
        |
    Concrete(A)     ← set o [A new]
        |
        ⊥   (never seen to hold an object)
```

`ClassValue::join` is the least-upper-bound: `⊥` is identity, two distinct
concretes widen to a `Set` (**not** ⊤ — a finite class set is still
resolvable, and every member is checked against the MRO table), and ⊤
absorbs. The **primary object-typing signal is read out of the existing
SSA type lattice** (`FunctionUnit::types`, which already infers
`TclType::Object { class_name }` per SSA version) — we reuse that dataflow
rather than building a second engine. The module only adds:

- the explicit ⊤ **taxonomy** the scalar type lattice can't express;
- the **JOIN** at merges and the single-class ablation;
- **sound namespace resolution** of bare class names (`NsContext`): a bare
  `Circuit new` resolves via (1) the enclosing `namespace eval`, (2)
  `namespace import`ed prefixes, (3) the global namespace — else it
  abstains (`cross-file-miss`). It is **never** matched to a same-tailed
  class in an unrelated namespace, so cross-file resolution cannot
  manufacture a confident false resolution from a namespace collision;
- the **⊤-reason attribution** by scanning IR assignment RHS shapes.

The design bias is **sound-by-abstention**: anything we can't prove
collapses to ⊤ and the resolver declines. A confident wrong resolution is
strictly worse than ⊙ abstention.

### Half 3 — the resolver

`class_lattice::analyse_dispatch` combines the two per `$obj method` site:

- `ClassValue::Top(reason)` → `Abstain(reason)`.
- `ClassValue::{Concrete,Set}` → `Resolved { classes, providers,
  method_known }`, where `method_known` is `false` exactly when a named
  class services neither the method directly, nor via an OO builtin
  (`new`/`create`/`destroy`/`configure`/`cget`), an inherited `unknown`
  handler, or an unindexed external superclass — i.e. the W308 candidate.

⊤-rate = `Abstain` sites / all sites is the make-or-break number.

## The ⊤ taxonomy

Every abstention is tagged so the harness can report *why* the lattice
gave up, which decides which layer (if any) is worth building:

| reason | trigger |
|---|---|
| `dynamic-assign` | object binding + a non-object literal reach the same var; class not stable |
| `factory-return` | bound from `[proc …]` whose object class we don't model |
| `runtime-oo-define` | class mutated by a runtime `oo::define` after binding |
| `introspection` | bound from `info object class` / `oo::copy` / `[$cls new]` / `self` — class is a runtime value |
| `per-object-mixin` | receiver touched by `oo::objdefine` — per-instance method table extends the class |
| `forward` | bound from a `forward` / alias we don't model |
| `cross-file-miss` | class *name* known (`[Foo new]`) but `Foo` has no `ClassDef` in the (per-file or merged) index |
| `unknown` | bare parameter / global / `upvar` / `⊥`-dispatch — no local evidence at all |

An orthogonal sub-count records how many `unknown` abstentions are on
receivers *never assigned in the file* (parameters / globals / `upvar`) —
the sites only **interprocedural** object-type flow could ever bind.

## Ablations

The harness measures the marginal value of each layer:

- **A0 MRO-only** — single class from the first `[C new]`, no join.
- **A1 +join** — the CFG-merge lattice (`Set` widening).
- **A2 +mixins/filters** — full MRO (vs. pure superclass chain).
- **A3 +cross-file index** — resolve against a corpus-merged class index
  instead of the per-file one.

## Abstentions on the class *definition* side

The ⊤ taxonomy above is about the receiver — "which class is this object?".
A second, independent family of abstentions is about the class itself —
"what does this class contain, and is this even a class?".  Both are
recorded on the `ClassDef` so every consumer abstains from one fact rather
than each re-deriving its own guess.

| flag / state | trigger | consumer effect |
|---|---|---|
| `ClassDef::inheritance_unknown` | manufactured by a user metaclass whose `create` override could not be read, so the spliced superclass list is unknown (audit idx 96/97) | W308 abstains: an inherited method is not a missing one |
| `ClassDef::member_set_incomplete` | the class body installs members the walk cannot read (audit idx 53) | W308 abstains: the member tables are a lower bound |
| *no `ClassDef` at all* | `X create Name … Body` where `X`'s own definition is in another file (audit idx 97, multi-file half) | nothing is recorded, and nothing is diagnosed |

### Reflective member installation (`member_set_incomplete`)

ticklecharts' `chart3D` builds its whole public surface by reflection:

```tcl
oo::class create ticklecharts::chart3D {
    constructor {*}[info class constructor "ticklecharts::chart"]
    foreach method {Render toHTML options get toJSON …} {
        method $method {*}[ticklecharts::classDef "chart" $method]
    }
}
```

Tcl expands both shapes at definition time, so the members are entirely
real — `info class methods ::ticklecharts::chart3D` lists all of them on
9.0.4 and 8.6.16 alike.  What the analyser cannot do is *name* them: a
`{*}` over a command substitution has no statically-known element list, and
the installer loop's body is a script the member walk does not descend into.

Two registry-driven signals set the flag (`Analyser::member_declaration_is_opaque`):

- a **member** word (per the definer grammar) whose declaration arrives
  through a `{*}` expansion that would not splice statically, or with a
  computed word in one of its declaring roles (`Name` / `ParamList` /
  `Body`);
- a **non-member** word that either has no registry spec or declares an
  `ArgRole::Body` argument — a script that can install members out of
  sight (`foreach`, `if`, a helper proc).

Neither signal names a command or a keyword.  What is recorded stays
recorded: the readable members, the class itself, and its inheritance are
all still modelled — the flag only says "there may be more", which is
exactly the premise W308 needs to be sound.

*Not* built: recovering the member *names* from a literal `foreach` list
while leaving their signatures unknown.  It is tempting (the names really
are static) but it buys an outline entry at the cost of a `MethodDef` whose
parameter list is a fabrication, and every arity-shaped consumer would have
to learn a second "unknown signature" state.  The abstention is the honest
answer until that state exists.

### Cross-file class factories

Single-file, a user metaclass is fully resolved: `handle_oo_class_command`
recognises a `create` call whose command is a recorded class reaching
`oo::class` through its own superclass chain, reads the body/name argument
positions off the `self method create` override, and splices the
superclasses that override injects.  Tk's `library/megawidget.tcl`
(`SimpleWidget`, `FocusableWidget`) is entirely covered.

Across files it is not, and deliberately so.  The analyser is per-file, and
a bare `::tk::Megawidget create IconList FocusableWidget {…}` in a file that
never mentions `::tk::Megawidget`'s definition is syntactically
indistinguishable from `interp create`, `image create`, or any ordinary proc
that happens to take a script argument.  Recognising it on shape alone would
invent classes out of unrelated commands — a wrong answer, and one that
would then feed W308, the outline, and rename.  So nothing is recorded and
nothing is diagnosed: `symbols` is empty for such a file, navigation returns
no locations, and no false "unknown method" appears.

Closing it needs the analyser to consult a *workspace* class index during
the walk — the same seed the cross-document command-resolution lattice
already builds after the fact — which is a larger change than the per-file
walk this abstention lives in.

## What is deliberately *not* built

- No second dataflow engine — the SSA type lattice is reused.
- No wiring into W307/W308 — zero behaviour change to shipping
  diagnostics until the data says ship.
- No interprocedural object-type propagation through proc/method
  parameters — the measurements are what decide whether that (larger)
  investment is the right next step.
