# Slice 7 (broader execution and runtime consumers) — adversarial review

Tree: `/home/user/tcl-lsp`, branch `claude/spectcl-optimization-discussion-5qhf42`,
HEAD at review time `fb005d9c5` (the hand-off page after the landing, docs only),
the landing `8f301f00e`, slice commits `9fc56e4b9`, `3419e5ce2`, `e1100706c`,
`d55ef78a2`, `3dff86cbd`, `379c0230a`, `708d1128e`, `31b9a661c`, `8f301f00e`,
base `4872a52c7`; `git diff 4872a52c7..8f301f00e`, 387 files, +11564/-2434.
Read: the lane doc's slice 7 plan, record and decisions (D315-D347, D330's row,
the slice 13 review-fix rows), the hand-off page, `value-evaluation.md` (the
catalogue, the capability declaration, the routes, the `PLATFORM`/`WALL_CLOCK`
exclusions), `value-transfers.md`, `value-transfers-migration.md` (the slice 7
ledger and tier paragraphs), `registry-consumer-contracts.md`,
`optimisation-passes.md`, `precision-limitations.md` (the four slice 7
entries), `interprocedural-analysis.md` (Step 2c), `docs/generated/value-transfers.md`
(0 rows read `pure, no route`), the O126/O129/IRULE3103 KCS notes, the slice
13 and 7a reviews; the diff in full for `tcl-cmd-core/src/{index,list,path,
irules,base32}.rs`, `tcl-registry/src/value_transfer/{irules,list_update,path,
builtins,route,declared,tcllib,mod}.rs`, the `file`, `lset`, `ledit`, `lpop`,
`split`, `string` and iRules spec modules, `const_subst.rs`,
`interprocedural/completion.rs`, `optimiser/elimination.rs`, the codegen
helpers and values, both runtimes' `cmd_list`/`cmd_file`/`cmd_fs`,
`tcl-irule-test/src/{pure_functions,lib,live}.rs`, `irule_gen.rs`,
`loader/semantics.rs`, `tcl-lsp-db/src/lib.rs`, the xtask gates, the bench,
the lowering's `return` hook, `command_binding.rs`'s binding rule, the O129
interpolation fold, and tcllib 2.0's own `base32` sources.

Measured with the CLI built from the tree (`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2
cargo build -p tcl-cli`, copied to `review-vt7/tcl-head`) and the base
`4872a52c7` (`tcl 2.2.5-737+g4872a52c`, a scratch worktree under the scratchpad
built into the existing `target/`, removed and pruned afterwards;
`review-vt7/tcl-base`) against tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0
(`tmp/tcl<ver>/unix/tclsh`, `TCL_LIBRARY` per release) and tcllib 2.0
(`tmp/tcllib-2.0/modules`). Oracle per program (`harness/oracle.sh`): stdout
and exit status under each release before and after `tcl opt --profile full`,
under the auto-detected dialect and under the release's own `--dialect`, every
run under one fixed file name so error traces compare; `tcl diag --dialect …`
read against the release; the lattice's own answers read through `tcl explore
--show sccp` (`harness/idx_probe.py`, `harness/b32_diff.py`). Programs under
`programs/` (`a` index grammar, `b` list cell updates, `c` path forms, `d` the
generated search fuzz, `e` base32, `f` encodings and byte arrays, `g`/`h`/`h2`
the completion proof at the top level, inside a procedure, and over malformed
words, `n` the `none` rows, `p` qualified writes, `s` shadowing and iRules
edges, `t` the bracket-in-braces family, `w` the wide-index witnesses);
outputs under `outputs/`.

- Hand batches (oracle, 5 releases x 2 dialect modes): a 8, b 35, c 7, d 35,
  e 4, f 7 (x3 locales), g 40, h 52, s 19, w 8: DIFF rows only on b07/b15/b16/
  b30 (auto dialect, P2), b31 (P3), b33 (P1), e03 (D344, accepted), h03/h04 (B1),
  h22 (B2), s15/s16/s19 (B3), d_match_uni (B4), the auto-dialect rows of
  d_err_* and d_split0-2 (#2373's class), and two trace-line-number shifts
  (`20_lists.tcl`, `d_split_ctl`) - each classified below; nothing else.
- Lattice probes: 173 index spellings x 5 dialects x 3 routes + 3 O129 folds
  (84 mismatches, all S1's integer-range family, 0 in the spelling grammar);
  2400 base32 encode/decode comparisons against tcllib 2.0 on 8.5-9.1 (0
  mismatches); 41 iRules edge calls (every answer right or a safe decline, N5).
- Every B/S/P item was checked against the issue list (titles fetched by
  number, 100 most recent): P1 is #2370, P2 is #2373's class, P3 is #2350's
  class, B1's definition-order root cause has siblings #2398/#2407 in other
  passes and #2346 in O126; nothing covers B1-B4, S1 (its spelling half is
  #2400, which D321 closes without saying so), P4-P6.

## B - blocking (a miscompile or a false report the slice introduces)

**B1 — The completion proof deletes an unused call to a procedure whose
definition has not run at the call.** `ProcSummary::completes`
(`rust/tcl-compiler/src/interprocedural/completion.rs`, `procedures_complete`)
and `RaiseProof::calls_complete` (`rust/tcl-compiler/src/optimiser/elimination.rs`,
`CallCompletion::completes`) take any procedure in `ir_module.procedures` as
callable; neither reads whether the `proc` statement that defines it has run
where the call is made. A call to a procedure defined later in the file, or
only inside a conditional, raises `invalid command name` on every tclsh, and
O126 deletes its store.

Repro `programs/h/h03.tcl` (every dialect, the auto dialect too):
```tcl
proc main {} { set a [label abc]; puts done }
main
proc label {x} {return [string length $x]}
```
`tcl opt --profile full --dialect tcl8.6`: `proc main {} { ; puts done }`
(`O126 Remove unused variable assignment`). tclsh 8.4.20, 8.5.19, 8.6.18,
9.0.4, 9.1.0: `invalid command name "label" … (procedure "main" line 1)`,
exit 1; the optimised program prints `done`, exit 0, under all five. The base
binary leaves the program unchanged. `programs/h/h04.tcl` (the definition under
`if {[info exists ::env(TCL_LSP_NOPE)]} { proc label … }`) is the same on all
five releases, and so is the per-procedure memo path (`tcl-lsp-db`'s
`OptCalleeSummary.completes`). The slice 13 summaries already treat "a
procedure defined later" as a barrier for O103 (slice 13 review, R7; #2398 and
#2407 are the pre-existing siblings in other passes); the completion proof
needs the same reachability fact (the `proc` statement dominating the call, at
the top level and in the calling procedure's definition order), or must decline
for a procedure whose definition is not proved to have run. The slice's own
witnesses (`a_call_that_cannot_raise_leaves_a_dead_store`: `expr`, a rejected
word count, recursion, `lindex` of a brace, an unset read, namespace
shadowing) cover neither shape; the O126 KCS note's new completion paragraph
states no definition condition either.

**B2 — A word the release cannot compile is counted as completing.**
`CompletionWalk::word_at` fails only on `WordExpr::Expand`/`Opaque`, and the
lexer hands it a word with text after a close-quote or close-brace as
neither, so a callee whose body is a compile error on every tclsh "completes"
and the store of its unused call goes.

Repro `programs/h2/m01.tcl` (every dialect):
```tcl
proc label {x} {return [string length "a"b]}
proc main {} { set a [label abc]; puts done }
main
```
`tcl opt --profile full --dialect tcl8.6`: `proc main {} { ; puts done }`
(O126); tclsh 8.4.20, 8.6.18, 9.0.4: `extra characters after close-quote …
(compiling body of proc "label", line 1)`, exit 1; the optimised program
prints `done`. `m02.tcl` (`{a}b`, `extra characters after close-brace`) is
the same, and `programs/h/h22.tcl` (`string length {*}$x`) under
`--dialect tcl8.4`, where `{*}` is no expansion and tclsh 8.4.20 raises
`extra characters after close-brace` (under 8.5+ the `Expand` rule keeps the
store, correctly). Base: unchanged on all three. Fix: the proof must fail
closed on any word the release's grammar rejects (the lexer should hand the
walk an `Opaque` for a quote or brace followed by text, and for `{*}` under
8.4); `m04`–`m07` (unbalanced `(`, `{`, `[`, `"`) already stay.

**B3 — A procedure of the module that shadows a tcllib command does not stop
its route.** `rust/tcl-compiler/src/command_binding.rs::default_binding` takes
any namespaced name that is not a `::tcl::mathfunc::` wrapper as `Opaque`, so
a `proc` on `::base32::encode` is never recorded as rebinding a registry
command, `trusts`/`observed_binding_is_the_builtin` stay true for the head,
and the slice's four `base32` routes (D343) — the first direct routes on a
namespaced package command — answer through the shadow in the lattice, O129
and codegen.

Repro `programs/s/s15.tcl` (`--dialect tcl8.5`, `8.6`, `9.0`, `9.1`, auto):
```tcl
namespace eval base32 {}
proc base32::encode {x} {return shadowed}
puts [base32::encode abc]
```
`tcl opt --profile full --dialect tcl8.6`: `puts MFRGG===` (`O129 Fold
constant builtin command substitution`); tclsh 8.5.19, 8.6.18, 9.0.4, 9.1.0:
`shadowed`; base: `puts shadowed` (O103 on the proc's constant return).
`s16.tcl` (`namespace eval base32 { proc encode … }`) is the same; `s19.tcl`
(`if {[base32::encode abc] eq "shadowed"} {puts yes} else {puts no}`) is also
a false report — `tcl diag`: `I230 Condition … is always false` and `tcl opt`:
`if {0} {} else {puts no}` where tclsh prints `yes`. Under `--dialect tcl8.4`
the fold is right only because tcllib is outside the 8.4 surface. The iRules
routes are not affected (`s18.tcl`: `proc b64encode` leaves `h#1 =
overdefined` under f5-irules), nor the builtin routes (`s01`, `s02`, `s06`,
`s07`, `s12`: `proc string`/`::string`/`split`/`file`/`lset` print `shadowed`
on HEAD and base). Fix: the binding scan must record a `proc`, `rename` or
alias on any registry-known qualified name (every tcllib spec) as a
rebinding, or the route resolution must consult the module's procedures
before the registry for a qualified head, as `CompletionWalk::procedure`
already does.

**B4 — A `string match -nocase` the route declines falls to O129's
interpolation fold, which cuts the substitution at a `]` inside a braced word
and emits an unparsable program.** The quoted-word fold in
`rust/tcl-compiler/src/optimiser/propagation.rs` (the `b'['` arm above the
"Fold constant builtin command substitution in interpolation" report, ~line
2560) tracks `[`/`]` depth with no regard to braces, so inside `[string match
-nocase {a]€} {€a}]` it takes `{a]` as the command's end, folds the truncated
`string match -nocase {a` (pattern `-nocase`, subject `{a`: 0) and splices the
answer over the truncated span. The word reaches that fold because the slice's
route declines `-nocase` over a non-ASCII operand (`Collation`, D337) and the
base's callback path — which folded the whole word — is gone (D338).

Repro `programs/t/t31.tcl` (`--dialect tcl8.6` and `tcl9.0`):
```tcl
puts [string match -nocase {a]€} {€a}]
```
tclsh 8.6.18, 9.0.4: `0`; HEAD `tcl opt --profile full --dialect tcl9.0`:
`puts "0€} {€a}]"]` (`O129 … in interpolation`), a program no tclsh parses;
base: `puts 0`. `t1.tcl` (`{ba]😀[€}`) is the same; without `-nocase`
(`t32`) the route answers and the rewrite is right. The scanner itself is a
pre-existing defect (P6: the base breaks `puts [string length {a]€}]` and the
ASCII `puts "x[string length {a]b}]y"` the same way, and HEAD's route path
fixes five of the base's bare-word cases under tcl9.0); the fix is the
scanner honouring braces, backslashes and quotes as the segmenter does, or
taking its spans from the lexer's tokens — and a route's decline must not
hand the word to a less careful rewriter (R7: "a route's decline is final").


## S - should fix before the slice is called complete

**S1 — The per-release index grammar (D321) models the spelling but not the
integer range each release accepts, so every route reading an index answers
where tclsh raises or wraps.** `index::parse` (`rust/tcl-cmd-core/src/index.rs`)
reads an integer as an `i64` under every grammar. Measured (`programs/w/wrap.tcl`,
`outputs/idx/report.txt`, `programs/w/wrap-lat.tcl`):

- 8.4.20, 8.5.19 and 8.6.18 read an index through `Tcl_GetIntFromObj`: a
  value with |v| ≥ 2^32 is `bad index "<spec>"` (so are `end-4294967296`,
  and `end+2147483648` on 8.4); a value with 2^31 ≤ |v| < 2^32 is accepted
  and **truncated to 32 bits** — `string range abcdefghijkl 0 2147483648` is
  the empty string on 8.x and the whole string on 9.0; `string range
  abcdefghijkl 0 -4294967295` is `ab` on 8.x; `lset x -4294967295 Z` writes
  element 1 on 8.x; `lset x end-4294967295 Z` appends on 8.6; `string first c
  abcdefghijklc 2147483648` is 2 on 8.x and −1 on 9.0.
- 9.0.4 and 9.1.0 saturate an overflowing `M+N` and a bignum `end+N` to
  `WIDE_MAX`, the internal encoding of `end+1` (`tclUtil.c`
  `GetEndOffsetFromObj`, `extreme:`), so `lset x 1+9223372036854775807 Z` and
  `lset x end+99999999999999999999 Z` **append** (`a b c Z`) where the
  lattice's `lset` route answers a raise.

At HEAD the lattice says, under tcl8.4 and tcl8.6 alike, `string range
abcdefghijkl 0 2147483648` = `abcdefghijkl`, `… 0 -4294967295` = ``, `string
first c abcdefghijklc 2147483648` = −1, `… 0 4294967296` = `abcdefghijkl`,
and `lset x -4294967295 Z` / `lset x end-4294967295 Z` = a raise; `tcl opt
--dialect tcl8.6` folds `puts [string range abcdefghijkl 0 4294967296]` to
`puts abcdefghijkl` and `set r [string first c abcdefghijklc 4294967296]; puts
$r` to `puts -1` (`programs/w/w01.tcl`, `w02.tcl`) where tclsh 8.6.18 raises
`bad index "4294967296"`; the release-blind O129 folds of `lindex`, `string
index` and `lrange` fold the same spellings under every dialect (45 rows);
under `--dialect tcl9.0` the `lset` route reports `evaluated: error` for an
index tclsh 9.0.4 appends with. Pre-existence: the base folds `w01`/`w02`
identically (its `string first` through the O129 callback, its `string range`
through the same route), and W231's false reports on these spellings (`lset
index '1+9223372036854775807' … raises 'index out of range' at runtime` under
tcl9.0; `lset index '-4294967295' is negative; raises …` under tcl8.6, where
8.6.18 writes element 1; `bounds_checks.rs` reads the same parser) are
identical at base (`w04.tcl`, `w06.tcl`, `w07.tcl`). It is an S rather than a P
because D321 is the slice's own "an index reads as its release reads it"
commit, its record says the grammar was "measured with `string index` on tclsh
8.4.20 to 9.1.0", the 51-row test `each_release_reads_an_index_as_its_tclsh_does`
has no row beyond 32 bits, and the slice's new `lset`/`ledit`/`lpop`/`string
first` routes read the same parser (and #2400's spelling half, which D321
closes, is not recorded as closed). The rule to add: under the 8.4 and 8.5
grammars, reject |v| > 4294967295 and wrap the accepted value to `i32`; under
the 9.0 grammar a saturated sum or a bignum offset is `end+1` (an `lset`
append, a clamp elsewhere), and a bignum alone is `WIDE_MAX`/`WIDE_MIN`.


## N - nits

**N1 — D343's and `tcllib.rs`'s sentence "a character past U+00FF, which 8.x
reads as its UTF-8 bytes and 9.0 refuses" is half wrong.** With tcllib 2.0
loaded, `base32::encode "€"` is `4KBKY===` (the UTF-8 bytes E2 82 AC) on
tclsh 8.5.19, 8.6.18, 9.0.4 **and** 9.1.0; no release refuses it. The route
declines such a word, which is safe (and a small precision gap, every release
agreeing), but the recorded reason is not what the shells do. Location: the
`Base32Semantics` doc comment in `rust/tcl-registry/src/value_transfer/tcllib.rs`
and the lane doc's D343.

**N2 — The Explorer labels a raising substitution-position route as "not
substituted: the outcome writes storage".** `tcl explore --dialect tcl8.4
--show sccp` on `set r5 [string range abcdefghijkl 0 1+1]` prints `route
string: direct string-range (registry) · answer: not substituted: the outcome
writes storage` for an outcome that is a raise with no store (the `lset` route
in the same view says `evaluated: error after 0 stores`). `programs/idx-sample.tcl`.

**N3 — `auto_path_eval.rs::file_route` resolves `file` through
`tcl_registry::default_registry()` rather than the unit's dialect registry.**
Harmless today (every dialect's `file` is the same spec), but it is the one
place in a `CLEAN_FILES` member that reaches past the unit's registry.

**N4 — The inventory's `Route`/`Enabled` columns do not say that a `none
(unauthored)` row still folds through its `const_fold` callback.** `string
toupper`, `dict size`, `lsort`, `regsub`, `namespace tail`, `scan` and 33
other calls in `programs/n/n01.tcl` (39 of 80) fold on HEAD, every one
identically on base, though their rows read `none (unauthored)` / `no`;
precision-limitations.md's "most pure commands have no route and say why"
records the callback folds, the generated page does not point at it. The two
new folds against base in that program, `regexp {a+} aaa` → 1 (D338) and
`base32::encode abc` (D343), are right; no `none (platform)` or `none
(declared)` row folds.

**N5 — `URI::host "http://A.B/"` answers `A.B`, outside the reference's
examples (D322's own rule).** `URI::protocol "HTTP://a/"` declines the
upper-case scheme; the host's case is not stated by the reference either and
should decline for the same reason, or be stated. `programs/s/irules_edges.tcl`.


## P - pre-existing defects found in passing (base binary identical)

**P1 — A qualified top-level write is not a write to the place** — `programs/b/b33.tcl`
(`set x {a b c}; lset ::x 1 Z; if {$x eq "a Z c"} …` folds to `puts other`
where tclsh prints `ok`), and `programs/p/p01.tcl` (`set ::s`, `incr ::y`,
`append ::z`, `lappend ::l`, `dict set ::d`, `unset ::u` all folded to the
stale value): **#2370**, open; `tcl diag` draws the matching false `I230
… always false` on b33. Not a new issue.

**P2 — Under the auto-detected dialect (tcl8.6 fallback) a 9.0-only command
is read as unavailable and the value it would write is propagated past it.**
`programs/b/b07.tcl` (`ledit`), `b15`/`b16`/`b30` (`lpop`, `ledit`): correct
under 8.6.18 (the command raises inside the `catch`), wrong under 9.0.4/9.1.0;
likewise `d_err_first5/6/7/8/13` (a start index 8.4 or 9.0 rejects folds
under the 8.6 fallback) and `d_split0-2` (a leading `#` element rendered
`{#}` where 8.4.20 prints `#`). Right under each release's own `--dialect`.
**#2373**'s class (the auto dialect's 8.6 assumption). Not a new issue.

**P3 — O109 deletes a store the next statement's raise names.**
`programs/b/b31.tcl`: `set x {a b c}; lset x(1) 0 v` loses the store, so the
raise reads `can't read "x(1)": no such variable` instead of `… variable isn't
array` (exit status unchanged). **#2350**'s class. Not a new issue.

**P4 — A `proc` on a `::tcl::dict::` spelling is not a rebinding of the
registry command.** `programs/s/s10.tcl`: `proc ::tcl::dict::size {d} {return
7}; puts [::tcl::dict::size {a 1}]` folds `1` on HEAD and base where tclsh
8.6.18 prints 7 — `command_binding.rs::default_binding`'s rule for namespaced
names (B3's root cause) on a pre-existing callback fold. New issue.

**P5 — A `proc ns::name` whose namespace does not exist is taken as defined.**
`programs/s/s03.tcl` (`proc base32::encode …` with no `namespace eval`) and
`s13.tcl` (`proc ip::version …`): tclsh raises `can't create procedure
"base32::encode": unknown namespace` at the `proc`, both binaries fold the
later call to the proc's constant return (HEAD to the route's answer in s03).
New issue.

**P6 — O129's quoted-word fold cuts a command substitution at a `]` inside a
braced argument and emits an unparsable program.** `programs/t/t40.tcl`:
`puts "x[string length {a]b}]y"` → `puts "x1b}]y"` on HEAD and base under
tcl8.6 and tcl9.0 (tclsh 8.6.18: `x3y`); the bare-word forms with a
multi-byte character after the `]` (`t5` `puts [string length {a]€}]`, `t11`,
`t29`, `t33`, `t24`) rewrite to `puts "1…}]"]` on the base (HEAD's route
path fixes them under tcl9.0 and still breaks `t5`/`t24` under tcl8.6), and
`t12`/`t18` (the multi-byte character outside the substitution) on both.
Location: `rust/tcl-compiler/src/optimiser/propagation.rs`, the `b'['` arm
counting `[`/`]` only. B4 is the slice's new exposure of it. New issue.


## What the slice gets right

- **The index spelling grammar per release (D321)** is right on every spelling
  tried short of the integer-range family: 173 spellings × `string range`,
  `lset`, `string first` on the lattice and `lindex`, `string index`, `lrange`
  on O129 under tcl8.4, 8.5, 8.6, 9.0 and 9.1 against the five shells — the
  8.4 `end` abbreviations and `end- 1`, the 8.5 sums and `end+N`, the 9.0
  `1_0`/`0d1`/no-abbreviation rules, octal `010`/`08`/`09`, `0X2`, `0B1`,
  `0O7`, `1+1+1`, every whitespace and tab placement, Unicode digits — with 0
  spelling mismatches (#2400's index half is closed by it).
- **The list cell updates (D329):** `lset` over nested index paths (`{0 1}`,
  `{0 end}`, `0 end`, `{end end}`, a braced one-element word, an unparsable
  `{`), the 8.6 append at a level's length and its nested form (measured:
  8.4.20 and 8.5.19 raise, 8.6.18 on append), a bad list under `lset x v` /
  `lset x {} v`, `ledit`'s insert/append/negative/`end+N` forms, `lpop` with no
  index, `end`, nested paths, an empty list and a non-list level: every outcome
  the lattice states agrees with the release it names (batch b, 35 programs ×
  5 releases), the written element renders canonically (`{x y}`, `\{`, `#`, a
  space, a newline), and both runtimes call the same cores under the release
  (D329, D335; the VM's `lset`/`lpop`/`ledit` units and the dict parity test
  pass).
- **The path routes (D331–D335):** 86 names through the six operations and 36
  joins — root-only, trailing-slash, doubled-slash, dot-segment and dotfile
  forms — every declining form (`~` anywhere, `:` anywhere incl. `a/b:c`,
  `\`, a leading `//`, `\\server\share`) and `file normalize`/`nativename`/
  `pathtype`/`separator` left alone: 0 DIFF rows on the five releases (batch
  c); `tcl diag`'s six folded path conditions true; `auto_path_eval.rs`
  carries exactly the three `irreducible` waivers.
- **The search routes (D337):** `split` on default and explicit separators
  incl. `""`, multi-character sets, `{`, `\`, `#`, control characters (`\v`,
  `\f`, NUL, `\x1f`, U+00A0, U+2028, U+0085), `string first` with every valid
  start grammar, `string match` with random patterns over `*?[]-^\` and the
  `-nocase`/`-noc`/`-no` spellings: 0 DIFF rows under each release's own
  dialect (batch d2, 35 programs, ~1600 folds per dialect), the raising forms
  (`-N`, `- `, `-x`, `-nocasex`, a bad start index, a wrong arity) declining
  under every named dialect; `split ""` is the empty list (#2418 closed, the
  base printed `1 1 {{}}`).
- **Non-ASCII admission and byte arrays (D337/D338):** under tcl9.0/9.1 the
  search, length, range and path routes fold `héllo`/`€` words (`string
  length "héllo"` → 5, `split "héllo" é` → `{h llo}`), under tcl8.4–8.6 they
  decline (0 rewrites); `binary format c* {128 195 255}`, `format %c 233` and
  `base32::decode` never reach a source rewrite; identical under the ambient
  locale, `LANG=C` and `C.UTF-8` (batch f, 7 programs × 3 locales).
- **The iRules cores (D322–D327):** the 71 published vectors answered alike by
  the simulator and the route, the cores registered ahead of the stub table so
  an unmodelled input falls back to the stub, `b64encode`/`crc32` against
  `binary encode base64`/`zlib crc32` on 8.6–9.1 (the crate's test, 29 + 1
  passing), and 41 edge calls — empty words, `crc32 "\x00"` =
  −771559539, `IP::addr 0.0.0.0/0 equals 1.2.3.4` = 1, `/32`, `/33`,
  `256.0.0.1`, port 0 and 65536, `%zz`, a parameter without `=` — each an
  answer the reference supports or a decline; `b64encode abc` in `RULE_INIT`
  folds to `YWJj` with `route b64encode: direct base64-encode (registry)` as
  the exit evidence says; a shadowing `proc b64encode` is honoured.
- **The tcllib `base32` routes (D343):** 2400 random encode/decode comparisons
  against tcllib 2.0 on 8.5–9.1 with 0 mismatches; canonical-only decoding
  (a set trailing bit, `======`-only, mid-padding, foreign alphabets, lowercase)
  exactly as the package's pure-Tcl implementation raises or answers;
  `base32::decode ""` the empty byte array; W120 names the missing `package
  require` as D344 says.
- **The completion proof (D330)** keeps the store in 49 of the 52 shapes inside
  a procedure where tclsh raises (batch h) — wrong arity, recursion, namespace
  shadowing, redefinition in eight forms incl. `proc $n`, `uplevel #0`,
  `namespace eval ::`, `eval $code`, `apply` and a helper proc, variable and
  execution traces, element and raising arguments, `return -code
  error|break|return`, `{*}` under 8.5+, `interp alias`, `rename`,
  `upvar`/`global`/`variable` links, `info exists` after the store, a
  shadowing `proc string` — and deletes it where it may (`string length`,
  `string le`, a defaulted parameter, `args`, a second procedure); the memo
  projection carries `completes` and `param_defaults` both ways.
- **The explicit `none` reasons (D340–D342):** no `none (platform)` or `none
  (declared)` row folds (`clock seconds`, `htonl`, `file pathtype`/`nativename`/
  `separator`, `info library`, `zlib compress`, `pid`, every iRules reader
  sampled); the inventory has no `pure, no route` row and `KNOWN_GAPS` is empty.
- **R1:** both xtask gates pass with the record's exact counts (29 files
  clean, 22 waived, 57 pinned, 6625 rows; 7834 words, 18 clean, 37 waived, 850
  pinned); `uri_split.rs` and `irule_gen.rs` read routes by `NativeEvalId`
  (`is_split_cmd`/`is_string_cmd` and the `"set"`/`"incr"` matches are gone).
- **R6:** `tcl opt --profile full` under tcl8.4–9.1 over the slice 13 review's
  55 witness programs (its batches a and c: the seven witnesses, the call
  shapes, the #2050/#2141/#2132 programs) byte-identical between HEAD and base,
  275 of 275; every `codegen_golden` (1 + 55 depth), `compiler_analysis_residual`
  (119), `dict_canonicalisation_parity` (4, engine leg) and `spec_corpus` test
  passes; the record's `20_lists.tcl`/`24_regex.tcl` differences are the
  stated trace-line shift and nothing else.
- **R7:** a platform- or clock-dependent core never answers (above); the
  extension route declines `Transient` with no host (`declared.rs`,
  `extension_host_installed()` a `None` thread slot by default) and
  `Unsupported` for a store-declaring implementation; a declared answer is
  cached on its identity, inputs and content hash
  (`a_changed_incoming_target_misses_the_cache`); a route's decline ends the
  fold in `const_subst.rs` (`route_literal(...)?` returns before any callback)
  — B4 is the one place a decline still reaches another rewriter.
- **D320/D345:** `cargo tree -i wasmtime` lists `tcl-engine-wasm` and
  `tcl-fuzz` only; `tcl-lsp-server`, `tcl-lsp-core`, `tcl-registry`,
  `tcl-compiler` and `tcl-cli` have no wasmtime edge; the loader's `extension
  FILE PREFIX` row and the `wasm_extension` host word are read and the
  artefact's hash enters the identity (`a_wasm_extension_implementation_names_its_artefact`,
  `a_wasm_extension_implementation_runs_on_the_extension_host` pass).
- **The bench (D346):** `cargo test -p tcl-compiler --bench value_transfers`
  runs and reports (the `list` route at 0.28–0.56 % of the optimiser over
  200–400 words, the four-variant comparison with `RouteTally` 40·0·0,
  0·40·0, 0·0·40); the large-literal workload timings on the two CLI builds
  are in the run log.


## Verdict

Rework. B1 and B2 are miscompiles the completion proof (D330) introduces,
each with the base binary leaving the program alone: an unused call to a
procedure whose `proc` has not run at the call — defined later in the file or
only under a condition — is deleted where every tclsh raises `invalid command
name` (B1), and a callee whose body holds a word no release compiles (`"a"b`,
`{a}b`, `{*}$x` under 8.4) is taken as completing (B2). B3 is a miscompile
and a false report the `base32` routes introduce: a procedure of the module
that shadows a namespaced package command is invisible to the binding scan,
so `[base32::encode abc]` folds to `MFRGG===` and I230 calls the shadowed
comparison always false where tclsh prints `shadowed`/`yes`. B4 is the
slice's new exposure of a pre-existing rewriter defect (P6): a `string match
-nocase` over a non-ASCII operand, which the route now declines, falls to
O129's interpolation fold, whose scanner ignores braces and emits an
unparsable program where the base emitted the right fold. S1 should be fixed
before the slice is called complete: D321's per-release index grammar
models the spelling but not the integer range (8.x's 32-bit truncation and
2^32 bound, 9.0's `end+1` saturation), so the routes it feeds — the slice's
`string first`, `lset`, `ledit`, `lpop` among them — answer where tclsh
raises, wraps or appends; the miscompile itself pre-exists at base and W231's
false reports with it. Everything else the slice decides was right under
every release: 0 spelling mismatches over 173 index spellings × 5 dialects,
0 DIFF rows over the path, search, encoding and iRules batches under each
release's own dialect, 0 mismatches over 2400 base32 comparisons against
tcllib 2.0, 275 of 275 R6 rows byte-identical to the base, every suite the
record names green (the witness binary 166, `differential_fold` 25,
`tcl-irule-test` 30, `stub_arg_roles` 35, the codegen goldens, both xtask
gates at the record's counts), and the language server links no wasmtime.
Of the seven planned mutations, the two that ran to completion die on the tests the record names — M1 (the 8.4 grammar reading `end+N`) on `each_release_reads_an_index_as_its_tclsh_does` (13 passed, 1 failed) and M6 (an explicit `none` reason changed) on `cargo xtask value-transfers --check` ("inventory is stale", exit 1); M2 was interrupted and M3, M4, M5, M7 were not run because the hand-back was enforced before the second driver reached them (the first driver's `pgrep -f 'cargo test'` guard matched its own argument list and refused them), so the mutation half of the review is incomplete and recorded as such in the run log.

S items: S1 (the index grammar's integer range per release: the 8.x
`Tcl_GetIntFromObj` bound and wrap, 9.0's saturated sums; W231 reads the same
parser).


## Run log

- 07:12 start; the coordinator's `make xtask-check` running (cargo lock); reading done first.
- 07:16 `cargo build -p tcl-cli` 2m44s, copied to `tcl-head`; 07:21 base `4872a52c7` built from a scratch worktree into `target/` (`tcl 2.2.5-737+g4872a52c`), copied to `tcl-base`; harness and batches written.
- 07:30–08:10 batches g, a, b, h, c, d through `oracle.sh` (two harnesses in parallel); `idx_probe.py` (lattice vs the five shells, 173 spellings); `b32_diff.py` (2400 comparisons).
- 08:00 `cargo test -p tcl-cmd-core` 145 + 1 doc; 08:05–08:25 the chained suites one at a time (`differential_fold` 25 with `TCL_LSP_REQUIRE_TCLSH=1`, `tcl-irule-test` 29 + 1, `stub_arg_roles` 35, `tcl-compiler --lib -- completion elimination` 86, `tcl-registry --lib` 1048, `tcl-spectcl` all binaries green); `cargo xtask value-transfers --check` and `registry-axes --check` OK.
- 08:20–08:45 `value_transfer_witnesses` 166 (671 s, `TCL_LSP_REQUIRE_TCLSH=1`), `dict_canonicalisation_parity` 4, `value_transfers_cli` 30, `codegen_golden` 1, `codegen_depth` 55, `compiler_analysis_residual` 119, the bench's default run; batches e, f (three locales), d2 (regenerated), s, t, w; R6 over the slice 13 review's 55 programs (275 of 275 identical).
- 08:30 the base worktree removed and pruned, its rlibs (1.7 GB) and the stale `target/debug/tcl` deleted; every test binary over 30 MB deleted after its run; disk never below 2.8 GB free.
- 08:50 chain 3 on the quiet machine: `tcl-spectcl --lib -- wasm_extension` 1, `tcl-vm --lib -- lset lpop ledit` 4, the large-literal timings (below); 09:15 the mutations: M6 and M1 complete and killed, M2 interrupted by the hand-back and restored by hand, M3/M4/M5/M7 not run.
- No container restart during the review; every background run completed from its own log.
- Tree: `git status` clean throughout; `git worktree list` the checkout and the coordinator's `tcl-lsp-gate2` only.

### Large-literal workload (two CLI builds, one run at a time on the quiet machine)

  - big6000 head opt round1 25.6 s
  - big6000 head diag round1 16.7 s
  - big6000 base opt round1 26.8 s
  - big6000 base diag round1 16.3 s
  - big6000 head opt round2 25.2 s
  - big6000 head diag round2 17.5 s
  - big6000 base opt round2 26.1 s
  - big6000 base diag round2 16.2 s
  - stringprep head opt 196.4 s
  - stringprep head diag 154.4 s
  - stringprep base opt 205.6 s
  - stringprep base diag 153.6 s

### Mutations (`harness/mutations.md`, `harness/run_mutations.sh`, logs under `outputs/mut/`)

| mutation | result | tallies | restored | tree |
|---|---|---|---|---|
| M6 | killed (exit 1) | see log | yes | clean |
| M1 | killed (exit 101) | 13 passed, 1 failed | yes | clean |
| M2 | interrupted (the run was stopped for the hand-back before `path_routes_match_every_release_on_path` finished) | — | yes, by hand from the saved original (`outputs/mut/M2/orig`, byte for byte) | clean |
| M3, M4, M5, M7 | not run (the hand-back was enforced before the second driver reached them; the first driver refused them on a self-matching guard) | — | untouched | clean |

