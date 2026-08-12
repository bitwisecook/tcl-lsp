# AOT WASM compiler: command emission priority

> **Status:** design study, backed by a real-corpus census (issue #1181).
> Establishes which Tcl commands the AOT WASM compiler should learn to emit
> directly next, ranked by real-world call-site count and repository
> breadth rather than by how interesting a command is to implement. No
> compiler codegen changed.

## 1. Why breadth-weighted, not just raw counts

A command that appears fifty thousand times in one generated data file
tells the compiler nothing about real Tcl usage; a command used a few
hundred times in eighteen of twenty-one unrelated repositories tells it a
great deal. This study therefore reports both **total call sites** and
**repository breadth** (how many of the corpus's twenty-one repositories
use the form at all, independent of how often), and the tiering in §4
weighs breadth over any single repository's hot loop, per the brief.

## 2. Method

### 2.1 Corpus

The corpus is exactly the set named in issue #1181, cloned shallow
(`git clone --depth 1`) into `experiments/aot-corpus/` (git-ignored;
reconstitute with `experiments/aot-corpus/fetch_corpus.sh`, which mirrors
`experiments/corpus/fetch_corpus.sh`'s idempotent, provenance-recording
style):

| Group | Repositories | Revisions fetched (short SHA) |
|---|---|---|
| georgtree | SpiceGenTcl, tclopt, ruff, argparse, tcl_tools, tclinterp, tclmeasure, extexpr | `e8aa45cee705`, `ff9a56074272`, `b63da69eed62`, `139d695c9012`, `6be5470f73d8`, `ccd894bbcf75`, `648b0ccf18de`, `1d53317eb9a7` |
| nico-robert | ticklecharts, tomato, pix, zesty, haru, implottk | `b49f014cb6c2`, `1a41d9b36822`, `a859536d06ef`, `7cc87f3f21b4`, `1d3f44abb568`, `505b824ac966` |
| tcltk core | tcllib, tklib, tk | `6093f8d62465`, `627bfe1e8dcc`, `d2311aed9c6a` |
| aplsimple | pave, alited | `875de1f1539d`, `a1fae46fb7a7` |
| EDA | Xilinx/XilinxTclStore, OSVVM/OSVVM-Scripts | `f1e8fe54482b`, `60a2751c1d20` |

All twenty-one repositories cloned successfully on the first attempt — no
network failures to record. None was large enough to skip on disk grounds
(largest single clone: `tcltk/tcllib` at 106 MB; the full corpus is
~436 MB by `du -sh` per repository, `.git` included). Free disk went from
~30 GB before the fetch to ~26 GB after — comfortably clear of the
"watch disk" concern the brief raised. `fetch_corpus.sh` records the
resolved commit and clone size of each repository to a git-ignored
`provenance.txt` on every run, so a later session can confirm what it is
analysing without re-deriving the table above.

### 2.2 Tool

Counting is done by a purpose-built Rust example,
[`rust/tcl-compiler/examples/aot_command_priority.rs`](../../../rust/tcl-compiler/examples/aot_command_priority.rs):

```
cargo run --release -p tcl-compiler --example aot_command_priority -- \
    experiments/aot-corpus/georgtree experiments/aot-corpus/nico-robert \
    experiments/aot-corpus/tcltk experiments/aot-corpus/aplsimple \
    experiments/aot-corpus/eda \
    --csv docs/design/compiler/data/aot-command-usage.csv
```

It walks every `.tcl` and `.test` file with **real parsers**, never a
regular expression:

1. **`tcl_compiler::segmenter::segment_commands`** — the same per-command
   word segmentation the lowerer and analyser consume — splits each script
   into commands and, for every word, records whether it is a single
   literal token (an `Esc` plain-word or a `Str` braced string, in
   `tcl-lexer`'s `TokenType` terms) or carries a `Var`/`Cmd` substitution
   fragment.
2. **`tcl_registry::CommandRegistry::arg_indices_for_role`** resolves which
   argument position holds a `Body` (nested script) for every command that
   has one — `proc`, `if`/`elseif`, `while`, `for`, `foreach`, `catch`,
   `try`, `namespace eval`, and more — so the walk recurses into real
   nested scripts using the same argument-role data the LSP highlighter
   and analyser use, never a hardcoded command list.
3. **`tcl_syntax::case_list`** (`clause_list_call` / `split_case_list`)
   splits `switch`'s combined `{pattern body pattern body …}` form into its
   real pattern/body pairs, driven by the registry's `CaseListSpec` shape
   (`CommandSpec::case_list`) — the same clause-list parser the semantic
   token walker and fold walker use. This step is not optional: segmenting
   a switch arm-list directly, as if it were an ordinary script, reads
   every arm's *pattern* word as a spurious command name. During
   development this put `file`, `default`, and other pattern text in the
   top thirty "commands" — an artefact of one large
   `switch -exact -- $scheme { … }` URI-scheme dispatcher in tcllib's
   `uri.tcl`, not a real usage signal. The fix — walking only the body
   half of each clause, and skipping a bare `-` fall-through marker
   (`CaseListSpec::SWITCH.fallthrough_body`), which is punctuation meaning
   "same as the next clause", not code — lives in the census tool itself,
   not as a hand exclusion of that one file.

The walk additionally recurses into every `[...]` command substitution
found in any word's lexical fragments — regardless of whether the
enclosing command's own head is itself literal or dynamic — always
counted as **value position**. Body and clause recursions (proc, control
flow, and switch-clause bodies) are always counted as **statement
position**.

The run analysed in this document segmented **3,015 files** (`.tcl` and
`.test`, `filetypes.tcl` excluded per §2.3) totalling **45,400,192 bytes**
(~43.3 MiB) of Tcl source, with zero panics and zero files skipped on the
8 MB oversize guard.

A command's head is counted only when it is a **literal** word — no
`$var`/`[cmd]` substitution and no `{*}` expansion. A dynamically
dispatched command (`$cmd arg`, `[namespace which foo] arg`) cannot be
attributed to a specific command name and is tallied separately as a
"dynamic-head" call site rather than folded into any row of the ranked
table: **18,411** such sites corpus-wide (5,526 in value position, 12,885
in statement position) — roughly 3.5% of all call sites, consistent with
Tcl's normal mix of `$obj method`/callback-prefix dispatch.

Each row of the census is a **command form**: `(command, subcommand)` for
ensemble commands the registry declares subcommands for (`string range`,
`dict get`, `array set`, `file join`, …), or bare `(command, —)`
otherwise. `<dynamic>` in the subcommand column means the subcommand word
was itself substituted (e.g. `$widget $op ...`) and so could not be
resolved statically. Every row carries: total call sites, the
value-vs-statement split, repository breadth (out of 21), the modal
arity, and whether every call site's *arguments* were literal versus at
least one being substituted. Every row is also flagged **`in_registry`**:
whether the literal command name resolves in `tcl-registry`'s
`CommandRegistry` at all. A `false` row names a proc the corpus itself
defines — a project helper or vendor macro (ticklecharts' own `setdef`;
Xilinx Vivado Tcl App Store commands such as `get_property`/
`send_msg_id`; tcltest's `test`; TclOO body-context keywords such as
`method`/`constructor`/`superclass`, which are definer-grammar members,
not standalone commands — see [`AGENTS.md`](../../../AGENTS.md)'s definition-body-grammar
section) — never a Tcl builtin, and therefore out of scope for AOT direct
emission: the compiler routes these through ordinary proc-call dispatch
or TclOO member dispatch, not builtin codegen.

Across the whole corpus the tool found **500,818** literal-head call
sites: 401,209 at 1,966 distinct registry-command forms, and 99,609 at
10,544 distinct non-registry forms (mostly one-off project procs — a long
tail, not a few dominant helpers). §3's ranked table and the committed CSV
keep only the 1,966 `in_registry = true` rows; the non-registry rows are
reproducible by re-running the tool without the CSV size trimmed (§2.3),
but are not committed, since they characterise these twenty-one
codebases' own procs rather than the Tcl language surface this study is
about.

### 2.3 Known gaps and exclusions

- **Braced `expr`/condition mini-language substitutions are not walked.**
  `if {$x > 0}` and `expr {$x + [foo]}` write their condition/expression
  as a single braced (`Str`-typed) word at the Tcl word-parsing level —
  Tcl does not substitute inside braces at that layer, so the lexer never
  emits a `Cmd` fragment for the `[foo]` inside it; the substitution only
  happens later, inside `expr`'s own mini-parser. This tool does not
  re-lex expression text looking for embedded `[...]`, so command
  substitutions written only inside a braced condition/expression are
  undercounted (bare/unbraced conditions, and any `[...]` written as an
  ordinary command argument, are unaffected). This makes the `expr`/`if`
  **value-position** counts in §3 a lower bound.
- **One file would have skewed every metric if walked as ordinary
  script, and is excluded from the numbers below**: tcllib's
  `modules/fumagic/filetypes.tcl` (85,041 lines) is a mechanically
  generated encoding of the Unix `file`(1) magic-number database as a
  private Tcl-embedded DSL (`fileutil::magic::rt`), whose "commands" are
  single/short tokens such as `>`, `<`, `emit`, `mime`, `indirect`,
  `offset`. These are real proc names *within that one file's own
  runtime*, not Tcl language surface — none of them are `in_registry`, so
  none would have entered the ranked table regardless — but at 85k lines
  in one file they still dominated raw totals for real registry commands
  by pure line-count weight before exclusion. The tool does not
  special-case it; a corpus with this file present is simply a different
  (and less representative) corpus, so it is left out of the fetch.
- **Oversize-file guard**: any single file over 8 MB is skipped and
  logged to stderr (none in this corpus, after the exclusion above,
  triggered it).
- **Recursion-depth guard**: nested body/clause/substitution recursion
  stops at depth 96 (no file in this corpus approached it).
- **Panics are caught per file** (`catch_unwind`), so one adversarial or
  malformed file cannot abort the run; none occurred.
- This is a **static lexical census, not a dynamic profile**: a command
  inside a loop body is counted once per call *site*, not once per
  execution. That is the right measure for a compiler-prioritisation
  question — direct emission pays off per call site compiled, not per
  runtime iteration — but it means a rarely-written, runtime-hot `expr`
  is not over-weighted, and it means `test`/`testTemplate` rank high on
  raw count precisely because tcltest and the Xilinx test harness both
  write one statement per test case, not because compiling `test` faster
  matters to a shipped program.
- **The committed CSV is filtered to `in_registry = true` rows** (1,966
  of 12,510 total distinct forms) to stay near the ~200 KB budget — the
  full unfiltered dump is 496 KB and its extra 10,544 rows are corpus-
  specific proc names with no bearing on Tcl command-surface priority (see
  above). Re-run the tool without post-filtering to reproduce the full
  table.

## 3. Ranked command forms

Top 60 registry-command forms by total call sites. `repos` is out of 21.
`args` summarises whether call-site arguments were literal (`L`) or
contained a substitution (`S`) more often; `≈all-L`/`≈all-S` means at
least 95% of sites agreed. `Arity` is the most frequent argument count for
the form, ties broken towards the smaller arity — 37 of the 1,966 registry
forms have two equally common arities, so the tie needs a stated rule to
keep the table and the CSV reproducible. Full counts for all 1,966 registry
forms are in [`data/aot-command-usage.csv`](data/aot-command-usage.csv),
whose rows are ordered by call count descending, then command, then
subcommand; re-running the census reproduces that file byte for byte.

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

### 3.1 "One number is useless" — dominant forms of the ensemble commands

`string`, `dict`, `array`, `info`, `namespace`, and `file` are each a
family of unrelated operations, exactly as the brief warned. Their real
per-subcommand breadth (top forms only; full breakdown in the CSV):

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

## 4. Recommended AOT direct-emission ordering

Grouped into tiers by (a) how much of §3's traffic each tier's forms
cover, and (b) whether the operation is pure-value (variables only, no
host capability) or needs frame/namespace state, a channel, the
filesystem, or a process — per the brief's requirement to flag what
cannot run in a bare-browser WASM target (no WASI file descriptors; see
[wasm-target-surfaces.md](wasm-target-surfaces.md) §2).

### Tier 1 — pure value operations, no interpreter state beyond reading/writing the current variable frame

The highest-value next work: no channel, filesystem, or process facility
involved, so every form here works identically under WASI and in a bare
browser once the compiler can emit it, and today's fallback cost
(`tcl_eval_code` re-lexing a boxed source string on every call) is paid on
some of the hottest, highest-breadth forms in the corpus.

- **`set` beyond a literal RHS** (currently only `AssignConst` — a literal
  value — is direct; #1 by total calls, and 71,893 of 101,221 sites (71%)
  have a substituted RHS, i.e. are *not* covered by today's narrow case).
  A `$var`-only or single-`[cmd]`-only RHS is the highest-leverage next
  step.
- **`list`, `lindex`, `lappend`, `lassign`, `llength`, `lrange`,
  `linsert`, `lreplace`, `lsort`, `concat`** — the list-value core. All
  breadth ≥ 10/21, several ≥ 20/21, and (`lindex`, `llength`, `lrange`,
  `linsert`, `lreplace`) are ≥95% substituted-argument value-position
  calls — exactly the shape today's fallback pays the most re-parse cost
  on.
- **`string range`/`map`/`trim(left/right)`/`length`/`tolower`/
  `toupper`/`first`/`index`/`repeat`** — the string-value core, same
  shape as the list core above.
- **`dict get`/`set`/`create`/`keys`/`remove`/`merge`** — pure value
  operations over the dict representation (`dict set`'s target is a
  variable write, everything else is a value read).
- **`split`, `join`, `format`, `concat`** — pure string/list
  transformation, no I/O.
- **`incr`, `append`** — variable read-modify-write, no I/O; both are
  high-breadth (21/21, 18/21) and today entirely fall back except through
  the narrow `AssignConst` special case.
- **`expr` beyond binary `+`** — comparisons (`==`, `<`, …), the other
  arithmetic operators, and unary `-`/`!` cover the great majority of
  real `expr` bodies; #7 by total calls and 98% of sites are already
  value-position, meaning `expr` is overwhelmingly used exactly the way
  direct emission is designed for.
- **`return EXPR` generalisation** beyond `Var`/`Literal`/`Binary Add`, so
  the existing `direct_proc_eligible` single-statement fast path covers
  more real proc bodies (comparisons and other arithmetic, matching the
  `expr` extension above, would directly widen this too since both share
  `ExprNode`).
- **`file dirname`/`tail`/`join`/`normalize`/`rootname`/`extension`/
  `split`/`root`** — despite the `file` name, these eight subcommands are
  pure path-string manipulation (~93% of all `file` traffic in this
  corpus; see §3.1) and belong in this tier, not the filesystem tier
  below.

### Tier 2 — needs variable-frame, namespace, or non-local control-flow machinery, still no host I/O

Still zero channel/filesystem/process dependency, but each needs more
compiler machinery than "read a value, call a runtime value op": frame
introspection, namespace state, or non-local exits.

- **`variable`, `global`, `upvar`** — scoping declarations. High breadth
  (20/21, 15/21, 10/21) and a prerequisite for widening how much of the
  Tier 1 list above can resolve variables that cross proc/frame
  boundaries rather than only locals.
- **`foreach`** — #9 by total calls, 20/21 breadth; needs list
  decomposition into loop variables layered onto the existing structured-
  loop `Emit` shape (`begin_loop`/`loop_test`/`begin_loop_body`).
  `for`/`while` already get real WASM loop structure via that shape (see
  §5); `foreach` does not use it yet, since it additionally needs to bind
  one or more loop variables per iteration from a list, not just evaluate
  a boolean condition. `for`'s own 1,754 sites already get structural
  loop emission today — its remaining cost is the body statements inside,
  covered by Tier 1's generalisations, not the loop shape itself.
- **`array set`/`get`/`names`/`unset`** — associative variable storage;
  9/21–8/21 breadth but a real gap once `variable`/`global` widen.
- **`namespace eval`/`export`/`import`/`upvar`/`current`/`ensemble`** —
  namespace-qualified state; `eval` alone is 19/21 breadth.
- **`catch`, `try`, `error`, `unset`** — non-local control flow /
  exception paths. `catch` is 98% literal-argument (its own body word is
  what varies), so a direct `catch {body}` shape is a bounded, well-scoped
  addition once the body's own statements are more often direct.
- **`switch`** — structured dispatch, 12/21 breadth. This tier's most
  concrete unlock from this study: `tcl_syntax::case_list` (used to build
  §3 itself, see §2.2) already gives the compiler a real, registry-shaped
  clause-list parser, so the analysis-time blocker for compiling `switch`
  as a real WASM `br_table`/chained-`if` is smaller than it looked before
  this census — the same parser this document's own tool depends on.

### Tier 3 — needs a channel or the filesystem (WASI-only; meaningless or stubbed in a bare browser)

Every form here needs `Host::filesystem()` or a real channel, which
`WasiHost` provides only via the embedded-stdlib `MemFs` (not a real
filesystem) and `BrowserHost` does not provide at all today (see
[wasm-target-surfaces.md](wasm-target-surfaces.md) §2 — `open`'s VFS
fallback has nothing to read from on the browser target; `file exists`/
`glob` report false/empty, not an error). Direct-emitting these forms is
real work, but the payoff is WASI-target-only until that wiring gap
closes; **do not** prioritise them for a browser deployment ahead of
Tier 1/2.

- **`puts`** to anything but the already-covered single-argument default-
  channel form (multi-argument, `-nonewline`, or an explicit channel
  argument) — #10 by total calls, 19/21 breadth, so this is the highest-
  value item in this tier by far.
- **`open`, `close`, `read`, `gets`, `eof`, `flush`, `fconfigure`,
  `seek`** — channel I/O.
- **`file exists`/`delete`/`mkdir`/`copy`/`rename`/`size`/`attributes`/
  `mtime`/…** — the ~7% of `file` traffic that is genuinely
  filesystem-backed (contrast §3.1's ~93% that is pure path-string
  manipulation and belongs in Tier 1).
- **`source`, `package require`/`provide`/`ifneeded`** — `package
  require` alone is 21/21 breadth (every repo in the corpus does it at
  least once) but needs the package/library system, not just a value op;
  `source`/`package require` additionally need the `MemFs` stdlib seed
  that (per wasm-target-surfaces.md §2) is wired for `WasiHost` but not
  yet for `BrowserHost`.
- **`glob`, `cd`, `pwd`**.

### Tier 4 — cannot be direct-emitted usefully on either WASM surface today

Not a compiler-effort question: these either have **no runtime backing at
all** on WASM (both hosts return an explicit "not supported under the
WASM runtime" stub — see `docs/generated/wasm-command-backing.md`'s
`not-required` rows for `exec`/`socket`/`load`/`fcopy`), or depend on host
facilities that plain `wasm32-unknown-unknown` structurally cannot
provide without new JavaScript-side imports:

- **`exec`, `socket`, `load`/`unload`, `fileevent`, `fcopy`** — explicit
  unsupported stubs on **both** WASI and browser hosts; not a target gap,
  a genuine "this operation has no meaning inside this sandbox" case (no
  child process, no TCP stack, no dynamic loader, no OS event loop).
- **`after`/`vwait`/the event loop** — compiles and "runs" on the browser
  target, but is not functionally a timer: `std::thread::sleep` is a
  no-op under `wasm32-unknown-unknown` (no blocking without an async
  runtime or a shared-array-buffer `Atomics.wait`), so deadline ordering
  has nothing to order against. Works under WASI (`poll_oneoff` backs a
  real sleep there).
- **`clock`** (`seconds`/`format`/`scan`/…) — currently wrong on **both**
  hosts (`BrowserClock`/the WASI clock stub hard-return the Unix epoch),
  not merely unimplemented; direct-emitting it before a real host clock
  import lands would compile a command that always reports 1970-01-01.
- **Bare `open` of a real host file** — under WASI this is meaningless
  without preopens (which this runtime does not use) and always falls
  back to the in-memory `MemFs`; under the browser it always fails. Only
  the `MemFs`-backed forms (Tier 3) are real targets.

None of Tier 4 belongs on an AOT-emission roadmap; they are host/runtime-
capability work (tracked in
[wasm-target-surfaces.md](wasm-target-surfaces.md) §3's proposed
four-function browser host-import surface), not compiler-codegen work.

## 5. Already covered today

Cross-referenced against `try_emit_typed_statement`, `emit_expr_value`,
and `emit_word_value` in
[`rust/tcl-compiler/src/codegen/wasm/backend.rs`](../../../rust/tcl-compiler/src/codegen/wasm/backend.rs).
The direct tier is narrow and every entry below is a **special case**, not
a general form — most of the traffic at each of these commands in §3
still falls through to `tcl_eval_code`:

- **`set NAME LITERAL`** (`Statement::AssignConst`) — only when the whole
  right-hand side is a literal with no substitution. §4 Tier 1's first
  item is precisely the 71% of `set` sites this does not cover.
- **`proc` registration** — the proc's name, parameter list, and body are
  captured and registered at runtime; this does not mean the *body*'s
  statements are compiled directly (each still needs its own direct-
  emission path, or falls back).
- **`puts $x` / `puts [cmd]`** (single-argument only) — the `ChannelWrite`
  intrinsic when the sole argument is a bare variable read or a "pure"
  command-substitution word (`is_pure_cmd_subst`) that itself resolves to
  a direct proc call. Multi-argument `puts` (an explicit channel,
  `-nonewline`), and a braced-literal argument, both fall back.
- **binary `+`** inside an expression (`emit_expr_value`'s only `Binary`
  arm) — operands must themselves be `Var`, `Literal`, or nested `+`.
- **`return EXPR`** — only inside a `DirectProc`-eligible procedure: a
  single-statement `{ return expr }` body, non-namespaced, not redefined,
  with an inferred `Int`/`Double`/`Numeric` return type, and `expr`
  restricted to the same `Var`/`Literal`/`Binary Add` grammar as above.
- **direct proc-to-proc calls** — a word that is a bare call or a "pure"
  command substitution resolving to another `DirectProc`-eligible
  procedure with matching arity; arguments are recursively emitted through
  the same word-value path.
- **`if`/`while`/`for`/`break`/`continue`/`return`** get real structured
  WASM control flow (blocks, loops, branches) via the `Emit` trait
  (`begin_if`/`begin_loop`/`loop_test`/`emit_break`/…) — but a condition
  and any body statement not matching one of the forms above still lowers
  through `emit_command`'s `tcl_eval_code(<source span>)` fallback. The
  *shape* is direct; most of the *content* today is not.

Reading this against §3's top 20 forms (`set`, `if`, `return`, `proc`,
`list`, `variable`, `expr`, `lappend`, `foreach`, `puts`, `lindex`,
`incr`, `file join`, `upvar`, `append`, `package require`, `file
dirname`, `bind`, `dict set`, `catch`): only `set` (narrow literal-RHS
case), `if` (structural shape only — the condition itself still falls
back), `return` (narrow single-statement direct-proc case), and `puts`
(narrow single-argument case) have *any* direct-emission path today, and
each of those four is a special case, not the general form. The other
sixteen of the top twenty — `proc` bodies beyond the single-return-expr
shape, `list`, `variable`, `expr` beyond binary `+`, `lappend`,
`foreach`, `lindex`, `incr`, `file join`, `upvar`, `append`, `package
require`, `file dirname`, `bind`, `dict set`, `catch` — fall entirely to
the `tcl_eval_code` fallback today.

## 6. Cannot be direct — needs interpreter/host facilities

Restated from §4 for a single reference point, split by *why*:

| Reason | Forms | Fixable by more compiler work? |
|---|---|---|
| Needs a real channel or filesystem (WASI-only; `BrowserHost` has none today) | `puts` (non-default channel), `open`/`close`/`read`/`gets`/`eof`/`flush`/`fconfigure`/`seek`, `file exists`/`delete`/`mkdir`/`copy`/`rename`, `glob`, `cd`, `pwd` | Yes, once targeting WASI; no, for a bare browser until host wiring lands |
| Needs the package/library system and the `MemFs` stdlib seed | `source`, `package require`/`provide`/`ifneeded` | Partially — `WasiHost` already seeds `MemFs`; `BrowserHost` does not yet (wasm-target-surfaces.md §3) |
| Explicit "not supported under the WASM runtime" stub on both hosts | `exec`, `socket`, `load`/`unload`, `fileevent`, `fcopy` | No — no sandboxed meaning, not a missing feature |
| Compiles but is not functionally correct on the browser target | `after`/`vwait`/event loop (no real sleep primitive), `clock` (epoch-0 stub on both hosts) | No — needs a JS-side host import (clock, and an async/shared-memory sleep primitive), not compiler work |

## Related

- [WASM code generation](wasm-codegen.md) — the `compile_wasm` pipeline
  §4's tiers feed into, and the plan-selection/decline machinery direct
  emission extends.
- [WASM target surfaces](wasm-target-surfaces.md) — the WASI-vs-browser
  host-capability matrix §4/§6 draw on, and why direct emission's
  clearest browser win is startup latency, not module size or throughput.
- [`docs/generated/wasm-command-backing.md`](../../generated/wasm-command-backing.md)
  — the drift-gated registry↔runtime *dispatch* backing table (does the
  interpreter have a handler at all), a different and prior question from
  this document's *direct-emission* tiering (does the compiler skip the
  interpreter for this form).
