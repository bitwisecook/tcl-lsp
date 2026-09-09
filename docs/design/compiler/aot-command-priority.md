# AOT WASM compiler: command usage census

A real-corpus census of Tcl command use, kept to show which forms the WASM
backend emits directly and which fall back to the interpreter. It reports
both **total call sites** and **repository breadth** (how many of the
corpus's twenty-one repositories use the form at all), because a command
that appears fifty thousand times in one generated file says less about real
Tcl than one used a few hundred times across eighteen unrelated repositories.

## 1. Corpus

Twenty-one repositories, cloned shallow into `experiments/aot-corpus/`
(git-ignored; `experiments/aot-corpus/fetch_corpus.sh` reconstitutes it and
records each clone's resolved commit and size in a git-ignored
`provenance.txt`):

| Group | Repositories | Revisions fetched (short SHA) |
|---|---|---|
| georgtree | SpiceGenTcl, tclopt, ruff, argparse, tcl_tools, tclinterp, tclmeasure, extexpr | `e8aa45cee705`, `ff9a56074272`, `b63da69eed62`, `139d695c9012`, `6be5470f73d8`, `ccd894bbcf75`, `648b0ccf18de`, `1d53317eb9a7` |
| nico-robert | ticklecharts, tomato, pix, zesty, haru, implottk | `b49f014cb6c2`, `1a41d9b36822`, `a859536d06ef`, `7cc87f3f21b4`, `1d3f44abb568`, `505b824ac966` |
| tcltk core | tcllib, tklib, tk | `6093f8d62465`, `627bfe1e8dcc`, `d2311aed9c6a` |
| aplsimple | pave, alited | `875de1f1539d`, `a1fae46fb7a7` |
| EDA | Xilinx/XilinxTclStore, OSVVM/OSVVM-Scripts | `f1e8fe54482b`, `60a2751c1d20` |

## 2. Tool

[`rust/tcl-compiler/examples/aot_command_priority.rs`](../../../rust/tcl-compiler/examples/aot_command_priority.rs):

```
cargo run --release -p tcl-compiler --example aot_command_priority -- \
    experiments/aot-corpus/georgtree experiments/aot-corpus/nico-robert \
    experiments/aot-corpus/tcltk experiments/aot-corpus/aplsimple \
    experiments/aot-corpus/eda \
    --csv docs/design/compiler/data/aot-command-usage.csv
```

It walks every `.tcl` and `.test` file with the real parsers, never a
regular expression:

1. **`tcl_compiler::segmenter::segment_commands`** — the same per-command
   word segmentation the lowerer and analyser consume — splits each script
   into commands and records, per word, whether it is a single literal token
   (`Esc` or `Str` in `tcl-lexer`'s `TokenType` terms) or carries a
   `Var`/`Cmd` substitution fragment.
2. **`tcl_registry::CommandRegistry::arg_indices_for_role`** resolves which
   argument positions hold a `Body` for every command that has one, so the
   walk recurses into nested scripts using the same argument-role data the
   LSP highlighter and analyser use, never a hardcoded command list.
3. **`tcl_syntax::case_list`** (`clause_list_call` / `split_case_list`)
   splits `switch`'s combined `{pattern body pattern body …}` form into its
   real pattern/body pairs, driven by the registry's `CaseListSpec`
   (`CommandSpec::case_list`). Only the body half of each clause is walked,
   and a bare `-` fall-through marker (`CaseListSpec::SWITCH.fallthrough_body`)
   is skipped; segmenting an arm list as an ordinary script would count every
   *pattern* word as a command.

The walk also recurses into every `[...]` command substitution in any word's
lexical fragments, always counted as **value position**. Body and clause
recursions are always counted as **statement position**.

A command's head is counted only when it is a **literal** word — no
`$var`/`[cmd]` substitution and no `{*}` expansion. A dynamically dispatched
head (`$cmd arg`, `[namespace which foo] arg`) is tallied separately as a
"dynamic-head" call site: 18,411 corpus-wide (5,526 value, 12,885 statement),
roughly 3.5% of all call sites.

Each row of the census is a **command form**: `(command, subcommand)` for
ensemble commands the registry declares subcommands for, or bare
`(command, —)` otherwise. `<dynamic>` in the subcommand column means the
subcommand word was itself substituted. Every row carries total call sites,
the value-vs-statement split, repository breadth (out of 21), the modal arity
(ties broken towards the smaller arity), whether the call sites' *arguments*
were literal or substituted, and `in_registry` — whether the literal name
resolves in `tcl-registry` at all. A `false` row names a proc the corpus
itself defines (ticklecharts' `setdef`, Vivado's `get_property`, tcltest's
`test`, TclOO body-context keywords such as `method`) — out of scope for
direct emission, since the compiler routes those through ordinary proc-call
or TclOO member dispatch.

The committed run covers 3,015 files (~43 MiB): 500,818 literal-head call
sites, 401,209 at 1,966 registry-command forms and 99,609 at 10,544
non-registry forms. The committed CSV keeps only the 1,966 `in_registry`
rows.

### 2.1 Known gaps and exclusions

- **Braced `expr`/condition substitutions are not walked.** `if {$x > 0}`
  and `expr {$x + [foo]}` carry their condition as one braced (`Str`) word,
  so the lexer emits no `Cmd` fragment for the `[foo]` inside it; the
  `expr`/`if` **value-position** counts in §3 are a lower bound.
- **tcllib's `modules/fumagic/filetypes.tcl` is excluded**: an 85,041-line
  generated encoding of the `file`(1) magic database as a private DSL whose
  "commands" (`>`, `<`, `emit`, `mime`, …) are real proc names within that
  file only. None would enter the ranked table, but by line count alone it
  dominated raw totals for real registry commands.
- Any single file over 8 MB is skipped and logged; nested recursion stops at
  depth 96; per-file panics are caught (`catch_unwind`).
- This is a **static lexical census, not a dynamic profile**: a command
  inside a loop body is counted once per call *site*, which is the right
  measure for direct emission (it pays off per site compiled) but means
  `test`/`testTemplate` rank high because tcltest and the Xilinx harness
  write one statement per test case.

## 3. Ranked command forms

Top 60 registry-command forms by total call sites. `repos` is out of 21.
`args` summarises whether call-site arguments were literal (`L`) or
contained a substitution (`S`) more often; `≈all-L`/`≈all-S` means at
least 95% of sites agreed. Full counts for all 1,966 registry forms are in
[`data/aot-command-usage.csv`](data/aot-command-usage.csv), ordered by call
count descending, then command, then subcommand; re-running the census
reproduces that file byte for byte.

| # | Form | Total | Value | Stmt | Repos | Arity | Args |
|--:|---|--:|--:|--:|--:|--:|---|
| 1 | `set` | 101,221 | 606 | 100,615 | 21 | 2 | mixed (71,893 S / 29,328 L) |
| 2 | `if` | 50,068 | 8 | 50,060 | 21 | 2 | ≈all-L (condition text is one word) |
| 3 | `return` | 27,889 | 1 | 27,888 | 21 | 1 | mixed (13,791 S / 14,098 L) |
| 4 | `proc` | 20,743 | 0 | 20,743 | 21 | 3 | ≈all-L |
| 5 | `list` | 15,126 | 14,981 | 145 | 20 | 2 | mostly S (9,890 / 5,236) |
| 6 | `variable` | 14,940 | 1 | 14,939 | 20 | 1 | ≈all-L |
| 7 | `expr` | 13,036 | 12,784 | 252 | 20 | 1 | mostly L (11,901 / 1,135) |
| 8 | `lappend` | 12,102 | 14 | 12,088 | 20 | 2 | mostly S (8,471 / 3,631) |
| 9 | `foreach` | 11,302 | 1 | 11,301 | 20 | 3 | mostly S (10,068 / 1,234) |
| 10 | `puts` | 10,144 | 1 | 10,143 | 19 | 2 | mostly S (8,386 / 1,758) |
| 11 | `lindex` | 8,464 | 8,420 | 44 | 20 | 2 | ≈all-S |
| 12 | `incr` | 6,202 | 578 | 5,624 | 21 | 1 | mostly L (5,402 / 800) |
| 13 | `file join` | 4,154 | 4,150 | 4 | 20 | 3 | ≈all-S |
| 14 | `upvar` | 3,907 | 0 | 3,907 | 10 | 2 | mostly S (3,200 / 707) |
| 15 | `append` | 3,616 | 65 | 3,551 | 18 | 2 | mostly S (2,605 / 1,011) |
| 16 | `package require` | 3,462 | 10 | 3,452 | 21 | 2 | ≈all-L |
| 17 | `file dirname` | 2,849 | 2,847 | 2 | 21 | 2 | all-S |
| 18 | `bind` | 2,608 | 57 | 2,551 | 6 | 3 | mostly L (1,624 / 984) |
| 19 | `dict set` | 2,416 | 3 | 2,413 | 13 | 4 | mostly S (1,528 / 888) |
| 20 | `catch` | 2,293 | 596 | 1,697 | 18 | 1 | ≈all-L |
| 21 | `switch` | 2,172 | 18 | 2,154 | 12 | 4 | all-S |
| 22 | `split` | 2,034 | 2,024 | 10 | 15 | 2 | ≈all-S |
| 23 | `dict get` | 1,980 | 1,979 | 1 | 19 | 3 | all-S |
| 24 | `namespace eval` | 1,916 | 9 | 1,907 | 19 | 3 | ≈all-L |
| 25 | `break` | 1,820 | 0 | 1,820 | 17 | 0 | — |
| 26 | `array set` | 1,817 | 0 | 1,817 | 9 | 3 | mixed (933 S / 884 L) |
| 27 | `error` | 1,778 | 0 | 1,778 | 17 | 1 | mostly S (1,064 / 714) |
| 28 | `for` | 1,754 | 0 | 1,754 | 20 | 4 | ≈all-L |
| 29 | `info script` | 1,754 | 1,752 | 2 | 21 | 1 | ≈all-L |
| 30 | `continue` | 1,658 | 0 | 1,658 | 14 | 0 | — |
| 31 | `lassign` | 1,547 | 57 | 1,490 | 16 | 3 | ≈all-S |
| 32 | `string range` | 1,473 | 1,467 | 6 | 12 | 4 | all-S |
| 33 | `format` | 1,409 | 1,265 | 144 | 13 | 2 | mostly S (1,202 / 207) |
| 34 | `lrange` | 1,404 | 1,402 | 2 | 13 | 3 | ≈all-S |
| 35 | `while` | 1,387 | 0 | 1,387 | 16 | 2 | ≈all-L |
| 36 | `llength` | 1,349 | 1,344 | 5 | 17 | 1 | all-S |
| 37 | `unset` | 1,347 | 13 | 1,334 | 11 | 1 | mixed (856 L / 491 S) |
| 38 | `join` | 1,345 | 1,333 | 12 | 17 | 2 | ≈all-S |
| 39 | `string map` | 1,281 | 1,256 | 25 | 17 | 3 | ≈all-S |
| 40 | `file tail` | 1,198 | 1,197 | 1 | 13 | 2 | all-S |
| 41 | `file normalize` | 1,150 | 1,147 | 3 | 20 | 2 | all-S |
| 42 | `image create` | 1,063 | 795 | 268 | 6 | 6 | mixed (638 L / 425 S) |
| 43 | `global` | 1,056 | 0 | 1,056 | 15 | 1 | ≈all-L |
| 44 | `close` | 1,015 | 15 | 1,000 | 15 | 1 | ≈all-S |
| 45 | `lsort` | 1,005 | 998 | 7 | 11 | 2 | ≈all-S |
| 46 | `source` | 1,003 | 4 | 999 | 17 | 1 | mostly S (985 / 18) |
| 47 | `eval` | 970 | 471 | 499 | 9 | 1 | ≈all-S |
| 48 | `string trim` | 948 | 942 | 6 | 14 | 2 | all-S |
| 49 | `namespace export` | 942 | 1 | 941 | 14 | 2 | ≈all-L |
| 50 | `regsub` | 920 | 313 | 607 | 14 | 4 | all-S |
| 51 | `linsert` | 915 | 911 | 4 | 11 | 3 | all-S |
| 52 | `pack <dynamic>` | 896 | 0 | 896 | 5 | 5 | all-S (option name itself dynamic) |
| 53 | `uplevel` | 872 | 383 | 489 | 15 | 2 | mostly S (791 / 81) |
| 54 | `package provide` | 779 | 16 | 763 | 19 | 3 | ≈all-L |
| 55 | `package ifneeded` | 744 | 7 | 737 | 14 | 4 | ≈all-S |
| 56 | `lreplace` | 678 | 678 | 0 | 10 | 3 | all-S |
| 57 | `array names` | 674 | 672 | 2 | 8 | 2 | mostly L (572 / 102) |
| 58 | `open` | 668 | 662 | 6 | 15 | 2 | mostly S (630 / 38) |
| 59 | `concat` | 642 | 641 | 1 | 11 | 2 | ≈all-S |
| 60 | `grid <dynamic>` | 608 | 0 | 608 | 6 | 7 | all-S |

### 3.1 Dominant forms of the ensemble commands

`string`, `dict`, `array`, `info`, `namespace`, and `file` are each a family
of unrelated operations. Their per-subcommand breadth (top forms only; full
breakdown in the CSV):

| Ensemble | Total calls | Distinct subforms | Top subcommands (total / repos) |
|---|--:|--:|---|
| `string` | 6,525 | 24 | `range` 1,473/12, `map` 1,281/17, `trim` 948/14, `length` 556/9, `tolower` 398/13, `first` 312/6, `repeat` 303/8, `trimleft` 243/8, `trimright` 231/9, `index` 208/9, `toupper` 169/8 |
| `dict` | 5,697 | 21 | `set` 2,416/13, `get` 1,980/19, `create` 470/16, `remove` 192/5, `keys` 147/10, `for` 110/12, `lappend` 83/6, `append` 51/3, `merge` 48/7 |
| `file` | 11,086 | 32 | `join` 4,154/20, `dirname` 2,849/21, `tail` 1,198/13, `normalize` 1,150/20, `rootname` 411/13, `delete` 292/13, `root` 224/3, `extension` 157/8, `split` 145/7, `mkdir` 133/6, `copy` 101/6, `rename` 70/11, `exists` 24/5 |
| `namespace` | 4,993 | 20 | `eval` 1,916/19, `export` 942/14, `upvar` 539/4, `import` 487/15, `current` 346/8, `ensemble` 203/8, `code` 121/4, `origin` 100/3, `delete` 88/7, `tail` 52/8, `which` 43/5 |
| `array` | 3,429 | 12 | `set` 1,817/9, `names` 674/8, `get` 514/7, `unset` 390/7, `size` 15/4 |
| `info` | 2,482 | 29 | `script` 1,754/21, `level` 251/9, `commands` 103/7, `exists` 90/6, `hostname` 43/3, `body` 35/6, `nameofexecutable` 34/5, `coroutine` 33/3, `class` 31/3 |
| `clock` | 730 | 9 | `format` 251/9, `seconds` 210/9, `scan` 157/6, `milliseconds` 59/7 |

`file`'s breakdown is the sharpest illustration of why this matters for
codegen: `join`/`dirname`/`tail`/`normalize`/`rootname`/`extension`/
`split`/`root` (10,288 of 11,086 calls, ~93%) are **pure string
manipulation of a path** — no filesystem touched — while the other ~7%
(`delete` 292, `mkdir` 133, `copy` 101, `size` 37, `attributes` 35,
`mtime` 26, `exists` 24, and two dozen smaller subcommands, 798 calls in
total) genuinely need `Host::filesystem()`. One `file` row in a naive
"top commands" table would hide that the large majority of real `file`
calls are as cheap to compile directly as `string range`.

## 4. What the WASM backend emits directly

`compile_wasm` (`rust/tcl-compiler/src/codegen/wasm/pipeline.rs`) first tries
the semantic-plan ladder over the unit's executable IR — the guarded boxed
`string length` region and the sealed constant `add` native pilot — and
otherwise records a typed decline and uses general structured lowering
(`backend.rs`). By default — `tcl compwasm`, the Explorer, and hosted
compilation all leave every `SemanticOptimisationPassId` off — that general
lowering receives no analysis facts: `try_emit_typed_statement` returns
`false` and every resolved call goes through `emit_command`. The direct
forms below need the `LegacyAnalysisSpecialisation` pass enabled
(`--codegen-passes`), the opt-in analysis tier that
[wasm-native-lowering-plan.md](wasm-native-lowering-plan.md) describes. With
it on, `try_emit_typed_statement` handles:

- **`set NAME LITERAL`** (`Statement::AssignConst` whose span is in the
  unit's `direct_assignments`) — `aot.var_set` at the top level, or
  `aot.local_set` on a local slot inside a `DirectProc`.
- **`return EXPR`** inside a `DirectProc` — a procedure whose body is a
  single `return`, not namespace-scoped, with a parameter list the runtime
  can bind; `emit_expr_value` accepts only `Var`, `Literal`, and binary `+`
  (`aot.expr_add`).
- **`proc` registration** (`SemanticOperationId::StructuredLowering(Proc)`)
  — name, parameters, and body are registered at runtime; the body's own
  statements are compiled on their own terms.
- **Every other resolved call** through the leaf-invocation path
  (`try_emit_leaf_invocation`, planned by `leaf_invoke.rs`): each word is
  evaluated into a transient call frame and the complete argv is dispatched
  through the runtime's ordinary command resolution — no source re-lexing.

A statement none of those admits, and every condition text, lowers through
`emit_command` — `tcl_eval_code(<source span>)` plus completion dispatch.
`if`/`while`/`for`/`break`/`continue`/`return` get real WASM control flow
(blocks, loops, branches) through the `Emit` trait
(`rust/tcl-compiler/src/codegen/emit.rs`); `foreach` does not.

The native tier (`rust/tcl-compiler/src/native_lowering/`, emitted by
`wasm/native_emit.rs`) is gated by four `SemanticOptimisationPassId` passes
that are off by default and enabled together by the pipeline's native tier.

## 5. Cannot be direct — needs interpreter or host facilities

Forms that need a channel, the filesystem, a process, or the event loop are
bounded by the host, not by compiler work (the WASI-versus-browser matrix is
in [wasm-target-surfaces.md](wasm-target-surfaces.md); the registry↔runtime
dispatch backing is [`docs/generated/wasm-command-backing.md`](../../generated/wasm-command-backing.md)):

| Reason | Forms | Fixable by more compiler work? |
|---|---|---|
| Needs a real channel or filesystem (WASI-only; `BrowserHost` has none today) | `puts` (non-default channel), `open`/`close`/`read`/`gets`/`eof`/`flush`/`fconfigure`/`seek`, `file exists`/`delete`/`mkdir`/`copy`/`rename`, `glob`, `cd`, `pwd` | Yes, once targeting WASI; no, for a bare browser until host wiring lands |
| Needs the package/library system and the `MemFs` stdlib seed | `source`, `package require`/`provide`/`ifneeded` | Partially — `WasiHost` already seeds `MemFs`; `BrowserHost` does not yet (wasm-target-surfaces.md §3) |
| Explicit "not supported under the WASM runtime" stub on both hosts | `exec`, `socket`, `load`/`unload`, `fileevent`, `fcopy` | No — no sandboxed meaning, not a missing feature |
| Compiles but is not functionally correct on the browser target | `after`/`vwait`/event loop (no real sleep primitive), `clock` (epoch-0 stub on both hosts) | No — needs a JS-side host import (clock, and an async/shared-memory sleep primitive), not compiler work |

## Related

- [WASM code generation](wasm-codegen.md) — the `compile_wasm` pipeline and
  the plan-selection/decline machinery.
- [WASM target surfaces](wasm-target-surfaces.md) — the WASI-vs-browser
  host-capability matrix.
- [`docs/generated/wasm-command-backing.md`](../../generated/wasm-command-backing.md)
  — the drift-gated registry↔runtime *dispatch* backing table (does the
  interpreter have a handler at all), a different question from this
  document's *direct-emission* coverage.
