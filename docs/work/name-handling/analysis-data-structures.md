# Analysis data structures and a typed AST

Two related questions: does each analysis pass (IR, lowering, CFG, SSA, memory SSA, SCCP, GVN, intervals, value transfer, type inference, shimmer, dead stores, the optimiser, interprocedural summaries, taint, place resolution, command binding, diagnostics) use the right data structure instead of comparing strings; and does the IR carry enough typed information, with the registry and packs, for those comparisons to be typed. Part of [tidying the architecture](architecture-tidy-plan.md); faults 1 and 7 of the [cross-cutting review](cross-cutting-review.md).

**Verdict:** the lattices are typed (SCCP's `LatticeValue`, taint's bit flags, `TypeLattice`, `Interval`), and the core keys exist (`BlockId`, `Symbol`, `ValueKey`, `ProcedureId`, `SyntheticMarker`). The problem is not literal comparisons, which are few in production code, but **string-keyed collections** around those typed cores, **typed IDs converted back to names**, **keys built by rendering**, and an IR whose command nodes are strings with the typed information held beside them, so every consumer asks the registry again by name.

## Per pass

Counts exclude test modules (a first count that included inline tests overstated literal comparisons several-fold). Production literal comparisons: analyser 63, optimiser 23, command binding 18, taint 7, SCCP 5, lowering 4, shimmer 2, SSA 2. Production string-keyed maps and sets: analyser 423, optimiser 84, command binding 78, interprocedural 72, CFG builder 48, SCCP 45, shimmer 44, SSA 43.

| Pass | Typed core | String or rendered parts |
| --- | --- | --- |
| IR | `SyntheticMarker` for synthetic statements | `Call { command: String, canonical_command: Option<String>, args, defs, reads: Vec<String> }`; `AssignConst { name, value: String }`; `Barrier { reason: String }`; marker text such as `<unseen-call>` kept beside the typed marker (display only); structured `word_exprs` held beside flat `argv_texts` in an optional side record |
| CFG | `BlockId` | blocks in a `HashMap` although IDs are dense; a name-to-ID map; flow facts and frame-effect sets as `BTreeSet<String>`; the top level identified by the name `"::top"` (*read*; a procedure named `::top` was analysed like any other in a probe, so a latent risk only) |
| CFG queries | — | `predecessors()` builds a fresh `HashMap<BlockId, HashSet<BlockId>>` on every call (*read*), from about 18 production sites; two private dominance walks duplicate the existing constant-time dominator intervals |
| SSA | `Symbol`, `Version`, `ValueKey` | name-to-symbol maps, one per statement point; use and def discovery returns `HashSet<String>` mapped to symbols afterwards; uses re-scanned from `argv_texts` although `word_exprs` exists, deduplicated with `Vec::contains`; a `"trace"` literal fallback in production |
| Memory SSA | `StateSsa<Place>` | locations with name and qualifier strings; phis keyed by block **name**; operations with a block-name string and an `i32` index |
| SCCP | `LatticeValue`, `ConstValue` | constant sets deduplicated by stringifying (`(tag, String)` keys); escape and parameter sets keyed by name; per statement, symbols mapped back to names to probe those sets; a legacy text fold re-parsing `[cmd …]` (waived under `registry-axes`) |
| GVN | — | **`ExprKey = Vec<String>`** (*read*): `["call", command spelling, canonicalised argument text…]`, re-lexed from argument text, with variables rewritten as `${name}@version`; entries and occurrences hold block-name strings. Hazards (inferred, not reproduced: GVN hints did not surface through the analyser or optimiser in a probe): a command's identity is its spelling, so a shadowing procedure or a `rename` between two calls keys the same; brace-stripped argument text can make `{$x}` and `$x` key the same |
| Intervals | `Interval` | a fresh `HashMap<String, Interval>` per statement in the fixpoint; finding reasons and the division operator as strings compared by diagnostics |
| Type inference | `TypeLattice` | class identity by name; expression types keyed by `(String, u32, u32)` |
| Shimmer | — | block IDs turned into name sets; successor and reachability maps keyed by block name (its own comment calls name-keyed maps the measured cost); `"<phi>"` and `"interpolation"` labels as strings |
| Dead stores and def-use | — | sites hold block-name strings; creation order recovered by **parsing the `_N` suffix of block names** (*read*) though `BlockId` already is creation order |
| Optimiser | — | procedure CFGs keyed by name; operand types as `"$name"` strings; command trust by name over a whole-module name set; **structural equality by rendering** (`render_expr(l) == render_expr(r)`, *read*), because `ExprNode`'s derived equality includes offsets |
| Interprocedural | `ProcedureId` summaries, `BodyFrameId` escape graph | parameter traits and defaults keyed by name; an index filled from name suffixes; escape tags keyed by `(String, u32)`, a `ValueKey` with its symbol replaced by a name; call-by-name scanning with `split_whitespace` |
| Taint | `TaintColour` bit flags | instance classes and guards keyed by name; command-name fallbacks that ignore `rename` |
| Places | `VariableCellKey` (mostly) | `Place { ns, name, owner: String }` with the sentinel `"<local>"`; `Authored(String)` and string activation identities |
| Command binding | `SourceNamespaceKey`, allocations, incarnations | root checks on the `Authored(String)` arm; a few name literals |
| Diagnostics | `DiagCode` | the class model, command rename and alias maps and disabled codes keyed by name; one comparison of a code through `as_str()`; command spellings compared with both `x` and `::x` by hand; a line-and-whitespace scan for `source` and `namespace` |

String comparison is correct where Tcl semantics are string-based and must not be "fixed": literal-spelling preservation in SCCP (`01`, `+5`, `" 5"` keep their identity), GVN keeping `1`/`01`/`0x1` apart, `switch` patterns, `--` on literal words, Tk paths as text, raw-source syntax checks, conservative over-approximations, and display strings as long as nothing parses them back.

## Is the IR typed enough?

**What a command node carries today:** the command spelling and an optional canonical spelling, argument texts, variable names read and defined, and an optional `CommandTokens` side record holding the structured words (`WordExpr`: literal, braced literal, variable, command substitution, template, expansion, opaque), spans, the resolved `SourceInvocationBinding`, nested bindings by offset, variable accesses and taint context. 48 production files read the structured words; 79 read the flat `args`.

**What it does not carry:** the resolved command as an identity; the selected form or subcommand; and, per word, its **role** (script body, expression, variable name, command name or prefix, index, option, value), the **place** it names, its **value type or domain**, its **evaluation context**, and the command's **result type and effects**.

**What the registry and packs already know:** per-word roles (`ArgRole`), option tables, subcommand forms, result types (`TclType`, `ReturnElements`), integer argument domains (`arg_domains`, added on this branch), variable-effect and frame-effect descriptors in part, side-effect classes, value transfers, and per-release availability; SpecTcl packs supply the same for user and vendor commands. About 990 production lines query `ArgRole` or role helpers directly, each by command name and argument strings, at each consumer.

**The gap** is that this knowledge is never projected onto the IR once. Every pass re-asks the registry by name, re-splits the words, and keys its own tables by the strings it gets back; that is why GVN keys by spelling, SCCP and type inference key escape and class sets by name, and diagnostics compare `x` and `::x` by hand.

**What the IR should carry** (the "annotated command" of the [architecture plan](architecture-tidy-plan.md)'s structured words and shared structures):

- `command: CommandRef` — the resolved identity (or an explicit unknown), from the lookup owner.
- `form: FormRef` — the selected subcommand or grammar form from the registry.
- per word: the structured `WordExpr` with source span; `role: WordRole` (body with its evaluation context, expression, variable place, command reference, index, option, value); for values, `ValueType`/domain and, where proven, a `ValueId`; for variable roles, the `PlaceId`; for bodies, the body-unit ID.
- `result: ValueType` and `effects: EffectSet` (variables, frames, commands, instances, I/O), from registry and pack descriptors.

Computed once at binding and lowering with the selected dialect, environment and packs, this lets every pass compare identities and typed values: GVN keys become `Call { command: CommandRef, operands: [Literal(ValueId) | Read(ValueKey)] }`, SSA uses and defs come straight from word roles as places, SCCP and type inference key by symbol and place, taint and diagnostics read roles instead of spellings, and the optimiser compares trees structurally. The registry gaps this exposes are the ones the other studies found: complete effect descriptors, evaluation contexts, value grammars and types for option values and results, and package and object-system identities.

## Plan

1. **Annotated commands:** add `CommandRef`, `FormRef` and per-word role, place, type and body annotations to the IR, computed once from the registry and packs; make `word_exprs` mandatory and retire `argv_texts`/`args` consumers; drop sentinel text where a `SyntheticMarker` exists.
2. **Typed keys in every pass:** GVN `ExprKey` over command references and operand values; SSA use and def discovery yielding places and symbols directly (no per-point name maps); SCCP constant sets with a typed hashable value; memory SSA, def-use, dead stores, shimmer and GVN entries by `BlockId`, with block names as display only and never parsed; intervals, escape and parameter sets keyed by symbol or place; finding reasons, labels and barrier reasons as enums; a `FunctionKind` instead of the `"::top"` name.
3. **Shared structural queries:** cache predecessors and dominance on the function; offset-insensitive structural equality for `ExprNode` instead of rendering; dense index vectors and bitsets for blocks and symbols.
4. **Gates:** extend `registry-axes` from spellings to types — forbid new `String`-keyed maps and sets of commands, variables, blocks and namespaces in analysis passes, comparisons of rendered text, and parsing of display names — with the identity layer and annotated commands as the sanctioned replacements.
