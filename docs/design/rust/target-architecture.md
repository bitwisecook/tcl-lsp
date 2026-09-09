# Target architecture — zero-copy, single-parse, incremental, MVCC

The architectural destination the Rust workspace is converging on, and the
tensions and decisions that shape it. Companion to
[`current-architecture.md`](current-architecture.md), which describes the crate
graph as it stands. Everything below is a **gap**: where a layer is partly
built, the section says what is missing.

The properties the design is aiming at:

- aggressively **zero-copy** — borrow from one source buffer, never
  re-materialise text;
- **parse to tokens once**, where the parse is statically knowable;
- build **single trees** — one CST, no second parse representation;
- derive positional data (cursor ↔ byte ↔ line/column) from the **tokens
  underlying the CST**, not from per-feature indexes;
- let edits **cascade**: changing an input invalidates exactly the derived data
  that depends on it, rebuilt on demand;
- use **MVCC** where concurrent reads and writes need a consistent view;
- apply all of the above to **every embedded grammar** — BIG-IP config, format
  and `scan` strings, regexps, globs, `expr` — not just the Tcl surface.

## The one tension, stated up front

Two of the goals pull against each other, and the resolution is a deliberate
decision, not a free lunch:

- **Aggressive zero-copy** wants tree nodes to hold a `Span` into a single
  `Arc<str>` source and slice text on demand (`&source[span]`), copying nothing.
- **Structural sharing across edits and files** (rowan's trick: identical
  subtrees share one allocation, so an edit reuses untouched subtrees verbatim)
  wants green leaves to **own** their text (interned `SmolStr`), because a
  position-independent node cannot borrow from a position-bearing buffer.

rust-analyzer chose owned text to get sharing; the stated priority here is
zero-copy, which points the other way. This is decision **D1** below; the rest
of the architecture is the same either way.

## Layered model

### Layer 0 — Versioned, immutable source

Each document version owns its bytes once: `Source { version, text: Arc<str> }`.
That is the only allocation of the source for that version and the unit of
MVCC — versions coexist, and a reader holds an `Arc` to the version it started
against. The document store shares `text` and its line index as `Arc`-backed
handles installed together per revision. **Missing:** the version is not a
first-class key a reader can hold and compare.

### Layer 1 — Tokens, once

One lex pass per version produces a structure-of-arrays token stream:
`Tokens { source: Arc<str>, kinds: Vec<TokenKind>, spans: Vec<Span> }`, so a
token's text is `&source[span]` — everything downstream consumes one stream, and
only the edited region re-lexes.

`Token` is already span-only. **Missing:** the stream itself, and the owned
`text` / `raw` strings `GreenToken` stores beside it (D1).

### Layer 2 — One CST, everything else a view

The red-green CST is built from the token stream and is the **single** parse
representation. This is already true of the segmenter: `segment_commands_local`
derives `SegmentedCommand` from the CST, and the old token-loop segmenter
survives only as a frozen oracle in the differential tests. **Missing:** the IR
item tree and the lowering input each rebuild rather than being views over one
reused CST, and the sub-word lexer scans are not folded onto the tree.

Green stores structure; the text-storage choice is **D1**. Red anchors absolute
positions lazily and carries the one line index beside the tree.

### Layer 3 — Positions come from the tree

A single position service lives with the CST and is the only place byte ↔
(line, column) and the UTF-16 conversion happen:

- cursor `(line, UTF-16 column)` → byte offset via the one line-start index;
- byte offset → token / node via binary search over token spans, or red-tree
  descent.

One index, built once per version. **Missing:** `LineIndex::new` is called from
over two hundred sites outside `tcl-lexer`, so features still build their own.

### Layer 4 — Demand-driven incremental graph (the cascade)

Derived data is expressed as **memoised queries** over inputs, with dependencies
tracked so a change invalidates exactly its dependents and recomputes them
lazily, when next demanded:

```
input    source(file)          = version + Arc<str>
input    registry              = built-in command shapes (static)
derived  tokens(file)          ← source(file)
derived  cst(file)             ← tokens(file), command_shape(head)*  // shape-directed descent
derived  command_shape(name)   ← registry, else proc_shape(name)
derived  proc_shape(name)      ← analysis(def of name)               // discovered
derived  item_tree(file)       ← cst(file)        // whitespace-stable firewall
derived  analysis(item)        ← item_tree(file), registry
derived  diagnostics(file)     ← analysis(item)*
derived  semantic_tokens(file) ← cst(file), registry
derived  workspace_index       ← item_tree(file)*
```

Three properties do the heavy lifting:

- **Firewall queries.** `item_tree` is keyed on structure, not bytes, so a
  whitespace-only edit re-lexes and re-parses but leaves `item_tree` unchanged,
  and every analysis downstream is reused. This is what makes "edit one
  character" cheap.
- **Per-item granularity.** `analysis(item)` is keyed per proc or class, so
  editing one proc invalidates only that proc's analysis. Combined with CST
  subtree reuse, a keystroke recomputes a bounded slice.
- **Shape-directed descent.** How a command's arguments are parsed depends on
  `command_shape(head)` — registry-known for built-ins, discovered for user
  procs (**D4**). A proc's shape becoming known cascades a targeted re-parse of
  just its call sites.

The analysis half runs as salsa queries today: the analyser walk, the
per-procedure lattices, the interprocedural taint cascade, and the cross-file
signature table ([`incremental-analysis.md`](incremental-analysis.md)).
**Missing:** the *parse* half — tokens and CST are rebuilt per version rather
than being queries with their own firewalls.

The failure mode to avoid is adopting the *vocabulary* (snapshots, queries)
without the firewall and per-item granularity that make the cascade cheap: a
memoised graph with one coarse whole-file analysis query re-runs the whole file
on every keystroke and buys nothing.

### Layer 5 — MVCC where reads and writes overlap

Reads run against an immutable snapshot; writes publish a new version:

- The current derived state is an immutable snapshot behind an atomic handle
  swap (lock-free reads). A request loads the current handle and holds it for
  its duration, so it sees one consistent version, and handing work to a worker
  is a reference-count bump rather than a deep clone.
- An edit builds the next version and swaps it in atomically. In-flight reads
  either complete against their older snapshot and have their result dropped by
  version tag, or are cancelled once they are pointless.

This is MVCC **where necessary** — the document and its derived snapshots, not
every small map. **Missing:** the derived state is a `Mutex<TclDatabase>`, not
an atomic handle swap, so a write takes global exclusivity rather than
publishing beside in-flight readers.

## Embedded sub-languages — one model, applied everywhere

Tcl hosts a dozen embedded grammars, and nothing above is specific to the Tcl
surface. A `format` template, a `regexp` pattern, a `string match` glob, an
`expr` expression, a `clock` / `scan` specifier, a BIG-IP `.conf` stanza, a
data-group body — every one is a small language sitting in a span of the same
source.

**Today each is an ad-hoc re-scanner.** Sub-languages are parsed by scattered
functions that take a raw `&str` and return owned copies: format-spec parsing in
several places, regexp and pattern handling across the taint, optimiser, codegen
and URI paths, variable-reference scans returning fresh `Vec<String>`. The
re-derivation problem repeats once per sub-language.

**Target — typed sub-trees, injected, sharing the spine.** Each embedded grammar
is parsed *once* into a typed sub-tree hanging off its host CST node, with the
same guarantees as the host:

- **Dispatched by the registry.** Which sub-grammar a word belongs to is a
  command-shape fact the registry schema already carries (`arg_roles`,
  `body_kind`, `options`, `forms`). This is D4's shape-directed descent
  generalised from script and expr to regexp, format, glob, and config: the host
  parser sees `regexp $re` and descends `$re` into a regexp sub-tree, `format
  $fmt` into a format sub-tree — once, memoised, re-parsed only when the typing
  or the text changes.
- **Zero-copy.** Sub-parsers yield spans into the one `Arc<str>`, not owned
  strings.
- **Positions from the sub-tree.** Because a sub-tree carries spans into the
  shared buffer, a diagnostic like "unknown conversion `%q` at column 7 of the
  format string" resolves through the *same* position service to a correct LSP
  range *inside* the embedded language — which unlocks per-sub-language
  diagnostics, hover, and semantic-token colouring (capture groups in a regexp,
  conversions in a format string) that ad-hoc scanners cannot place.
- **In the cascade.** A sub-tree is a query keyed on its span and typing, so
  editing one regexp invalidates only that sub-tree.

**Injection runs both ways.** BIG-IP config is itself a host: a `.conf` file is
config syntax with Tcl (iRules) injected inside `ltm rule { … }`, and within
that Tcl the sub-grammars are injected again. The model is therefore a single
CST of typed nodes joined by language-injection edges (the tree-sitter injection
model) — every node spanning the one source, every position resolved by the one
service, every node a participant in the one cascade. One parser per language,
no duplicates.

## Decisions

| | Decision | Options | State |
|---|---|---|---|
| **D1** | Green-leaf text storage | (A) `Span` into `Arc<str>` — true zero-copy, weaker cross-file dedup, incremental reuse by re-spanning (tree-sitter model); (B) owned interned `SmolStr` — rowan-style sharing and hash-consing, not zero-copy | **Open.** Leans **A** per the zero-copy priority, but only if single-document zero-copy matters more than cross-file subtree sharing |
| **D4** | "Parse once" boundary | see below | once per *(span, known-as-script)*, memoised |

### D4 — "where knowable" is progressive

Tcl is contextually parsed — a `{…}` word is a string *or* a script depending on
the command — so "parse once" has two tiers of knowability that resolve at
different times.

- **Registry-known shapes — pass one.** For any command in the registry (every
  built-in), `CommandSpec` already encodes which arguments are bodies, scripts,
  expressions, or variable names (`BodyKind`, `arg_roles`,
  `arg_role_resolver`). Descent is *deterministic*, not a guess: the first pass
  descends `if`'s condition as an expression and its arms as scripts, `proc`'s
  body as a script, `foreach`'s variable list, each parsed once and memoised.

- **Discovered shapes — cascade on first analysis.** A call to a *user* proc
  whose definition has not been analysed has an **unknown** shape.
  `customcmd $x {maybe a body}` is parsed conservatively — the braced word left
  opaque — because nothing yet says it is a script. When that proc's definition
  is analysed (signature scan and parameter traits infer that the parameter is
  used as a script via `eval $body`, an expression via `expr $cond`, or a
  variable via `upvar $name`), its shape becomes known and **the call sites that
  parsed it conservatively re-parse the affected argument**.

In the graph this is one edge: a call site's descent depends on
`command_shape(head)` — the registry shape for built-ins, the derived
`proc_shape(name)` for user procs. When `proc_shape` transitions unknown →
known, the engine invalidates exactly the call-site parses that read it.

The apparent cycle — parsing a proc's body needs its shape, but the shape is
inferred from analysing the body — is shallow and terminating: a parameter's
shape is fixed by how it is used with *built-in* commands (`eval`, `expr`,
`upvar`, …), which are registry-known, so `proc_shape` is computable from the
pass-one, registry-only descent; no fixpoint over user code is needed. Where a
shape is genuinely undecidable (computed command names, dynamic dispatch), the
parse stays conservative — opaque word, generic call — matching the existing
"fall through to the generic call when it cannot be proven safe" rule.

This is also a *latency* win. The conservative pass-one parse needs only the
static registry, so it paints fast (good for time-to-first-semantic-tokens); the
refined parse arrives later as an ordinary cascade once shape discovery
completes. Progressive enhancement, not a blocking dependency.

## Related

- [`current-architecture.md`](current-architecture.md) — the crate graph and
  ownership rules this builds on.
- [`incremental-analysis.md`](incremental-analysis.md) — Layer 4 as it exists
  today: the per-item firewall, the query graph, and the fallback contract.
- [`engineering-guide.md`](engineering-guide.md) — the engineering rules.
