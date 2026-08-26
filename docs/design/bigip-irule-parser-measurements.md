# BIG-IP iRule parser measurements ([#1631](https://github.com/bitwisecook/tcl-lsp/issues/1631))

> **Purpose.** Live evidence for
> [`dialect-and-package-registry-redesign-bigip-evidence-review.md`](dialect-and-package-registry-redesign-bigip-evidence-review.md),
> whose §E3 recorded the appliance transcript as *pending*. This document
> supplies that transcript and answers §F3's discriminating matrix directly.
>
> **Probe corpus:** [`scripts/dev/bigip-probes/`](../../scripts/dev/bigip-probes/)
> — 378 iRules, the drivers that ran them, the stock-Tcl controls, and the raw
> result files.
>
> **Methodology caveat up front:** this run predates and does **not** implement
> the §E4 probe contract. See [§11](#11-relationship-to-the-evidence-review) for
> the exact delta before relying on any row here as E4-conforming evidence.

Measured against a live appliance, not inferred from documentation.

| | |
| --- | --- |
| **Host** | `bigip1` |
| **Version** | BIG-IP 21.1.0.1, build 0.0.26 (2026-07-14) |
| **Probed** | 2026-08-26 |
| **Probes** | 97 syntax · 41 word-formation · 25 feature-availability · 85 command-availability · 120 event-context · 30 runtime-semantic |
| **Controls** | `tclsh8.4` (8.4.6) and `tclsh8.5` (8.5.13), both on the same host |
| **Corpora** | 170 public files; ~559 iRule-bearing files in `~/src/tcl-lsp-testsrc`; vendor manpages in `~/src/bigip-extract` |
| **Traffic lab** | `dev` (192.168.9.80) as client and backend, through virtual servers on `bigip1`; torn down afterwards |

Nothing was persisted: `save sys config` was never run and every probe object was
deleted afterwards.

There are **two** independent parser divergences, not one. Both are lexical, both
are silent, and either alone is enough to make a stock Tcl parser wrong about
real iRules.

---

## 1. The implicit word break (R-rules)

In stock Tcl, a word that begins with `{` or `"` **must** be followed by
whitespace, `;`, `]`, or end-of-line. Anything else is a hard parse error
(`extra characters after close-brace` / `close-quote`). F5's parser removes that
error and inserts a word boundary instead.

This is what makes `if {1}{log local0. true}` legal, and it is why `{*}` silently
misbehaves.

### Normative rules

**R1.** A word beginning with `{` ends at its *matching* close brace; a word
beginning with `"` ends at its closing quote. Brace counting and backslash
escaping are unchanged from Tcl.

**R2.** If the character immediately following that close delimiter is not
whitespace or a command terminator, F5 **ends the word there and begins a new
word** at that character. Stock Tcl raises a parse error.

**R3.** The new word is parsed from scratch under ordinary word-start rules:
a leading `{` makes it brace-quoted (no substitution), a leading `"` makes it
quoted, anything else makes it a **bare word** — in which case any `{` or `}`
inside it are literal characters.

> This is why `}else{log` becomes the single bare word `else{log`, and why the
> `if` branch of `if {0}{a}else{b}` compiles while the `else` branch does not.

**R4.** R2 applies **repeatedly** and in **every word position, including the
command name**. `{set}zz 7` executes `set zz 7`.

**R5.** R2 applies **only** when the word *started* with `{` or `"`. A bare word,
a `$` variable, a `${name}` variable, or a `[` command substitution followed by a
brace is unchanged from Tcl. `a{b}`, `$v{b}`, `${v}b` and `[list a]{b}` all
remain a single word.

**R6.** No diagnostic is emitted. A rule containing `{a}{b}` loads with **zero
warnings**.

**R7.** R1–R6 describe the **script parser only**. The `expr` sub-parser is
*unmodified* — see [§7 negative results](#7-what-is-not-divergent).

### How words actually come out

```tcl
if {1}{log local0. true}
#  -> [if] [1] [log local0. true]                       valid 3-word if

if {0}{log local0. a}else{log local0. b}
#  -> [if] [0] [log local0. a] [else{log] [local0.] [b}]   REJECTED

catch {set x 1}errvar
#  -> [catch] [set x 1] [errvar]                        valid, silently correct

list {a}"b"c
#  -> [a] [b] [c]                                       R2 fires twice
```

### The dangerous consequence: `{*}`

`{*}` expansion does not exist in this interpreter, and it does not error either.
R2/R3 turn it into two words — a literal asterisk and the unexpanded list.

```tcl
set l {a b c}
list {*}$l
#  F5:      "* {a b c}"   2 elements, no warning
#  Tcl 8.5: "a b c"       3 elements
#  Tcl 8.4: parse error
```

Code ported from Tcl 8.5+ that relies on `{*}` will **load cleanly on BIG-IP and
produce wrong data at runtime**. This is the only divergence found with no
failure signal at all.

### Word-formation evidence

Each snippet was evaluated inside `RULE_INIT` and its resulting word list logged,
then run verbatim through stock `tclsh8.4` on the same host. `$v` is `XX`.

| Source | F5 words | n | Stock Tcl 8.4 |
| --- | --- | --- | --- |
| `{a}{b}` | `a` · `b` | 2 | extra chars after close-brace |
| `{a}b` | `a` · `b` | 2 | extra chars after close-brace |
| `{a}bc{d}` | `a` · `bc{d}` | 2 | extra chars after close-brace |
| `{a}"b"` | `a` · `b` | 2 | extra chars after close-brace |
| `{a}$v` | `a` · `XX` | 2 | extra chars after close-brace |
| `{a}[list b]` | `a` · `b` | 2 | extra chars after close-brace |
| `{a}{b}{c}` | `a` · `b` · `c` | 3 | extra chars after close-brace |
| `{}{}` | *(empty)* · *(empty)* | 2 | extra chars after close-brace |
| `{a b}{c d}` | `a b` · `c d` | 2 | extra chars after close-brace |
| `{a{b}}{c}` | `a{b}` · `c` | 2 | extra chars after close-brace |
| `{a}#b` | `a` · `#b` | 2 | extra chars after close-brace |
| `{a}\ b` | `a` · `␣b` | 2 | extra chars after close-brace |
| `{a}"b"c` | `a` · `b` · `c` | 3 | extra chars after close-brace |
| `{a}}` | `a` · `}` | 2 | extra chars after close-brace |
| `{a}]` | `a` · `]` | 2 | extra chars after close-brace |
| `{a}{` | **missing close-brace** | — | extra chars after close-brace |
| `"a"{b}` | `a` · `b` | 2 | extra chars after close-quote |
| `"a"b` | `a` · `b` | 2 | extra chars after close-quote |
| `"a""b"` | `a` · `b` | 2 | extra chars after close-quote |
| `"a"$v` | `a` · `XX` | 2 | extra chars after close-quote |
| `"a"}` | `a` · `}` | 2 | extra chars after close-quote |
| `{set}zz 7` | `set` · `zz` · `7` | 3 | extra chars after close-brace |
| `{$v}$v` | `$v` · `XX` | 2 | extra chars after close-brace |
| `a{b}` | `a{b}` | 1 | **identical** — `a{b}` |
| `a{b}c` | `a{b}c` | 1 | **identical** — `a{b}c` |
| `$v{b}` | `XX{b}` | 1 | **identical** — `XX{b}` |
| `$v"b"` | `XX"b"` | 1 | **identical** — `XX"b"` |
| `[list a]{b}` | `a{b}` | 1 | **identical** — `a{b}` |
| `[list a]b` | `ab` | 1 | **identical** — `ab` |
| `${v}b` | `XXb` | 1 | **identical** — `XXb` |
| `@${v}c` | `@XXc` | 1 | **identical** — `@XXc` |
| `${v}${v}` | `XXXX` | 1 | **identical** — `XXXX` |
| `${v}{b}` | `XX{b}` | 1 | **identical** — `XX{b}` |
| `${v}"b"` | `XX"b"` | 1 | **identical** — `XX"b"` |
| `{a}<TAB>{b}` | `a` · `b` | 2 | **identical** — `a` · `b` |

The split is exact: all 23 cases where the word *starts* with `{` or `"` diverge;
all 11 cases where it starts with a bare character, `$`, `${` or `[` are
byte-identical. The final row confirms ordinary whitespace separation is
untouched.

---

## 2. The brace-line continuation (N-rules)

This is a **second, independent** divergence, and arguably the more consequential
one: it makes K&R brace style — which stock Tcl has never allowed — legal.

```tcl
while {$n < 30}
{
    set n [expr {$n + 30}]
}
#  F5:      loop runs; n ends at 31
#  Tcl 8.4: wrong # args: should be "while test command"
```

### Normative rules

**N1.** A newline does **not** terminate a command when the next line's first
non-whitespace character is `{`. Such a line is absorbed as further arguments to
the preceding command.

**N2.** N1 is **unconditional**. It does not depend on the command being
incomplete, on the command's identity, or on its arity. Verified: `list a b`
followed by a line `{c}` yields the three-element list `a b c`.

**N3.** N1 applies at **any nesting depth** — inside `when` bodies, `if` bodies,
`proc` bodies.

**N4.** Because the test is purely "does the next line start with `{`", a line
that is **blank, whitespace-only, or a comment** terminates the command normally.
`if {1}` followed by a blank line and then `{body}` fails with
`missing a script after "if"`.

**N5.** `else` / `elseif` are a **separate** lookahead performed by `if` itself:
they are picked up across a single newline (a line starting with `else` is not a
`{` line), but **not** across a blank line, where they fall back to being an
unknown command (`undefined procedure: else`).

### The rule is lexical, not command-specific

The decisive experiment. The same commands flip behaviour purely on whether the
next line starts with `{`, and the continued forms execute correctly:

| Command | next line `q 5` style | next line `{q} 5` style |
| --- | --- | --- |
| `set` | stops — `wrong # args` | **continues** → `q=5` |
| `incr` | stops — `wrong # args` | **continues** → `c=1` |
| `append` | stops — `wrong # args` | **continues** → `s=ab` |
| `list` | stops — `undefined procedure: a` | **continues** → `a b` |
| `string` | stops — `wrong # args` | **continues** → `3` |
| `expr` | **stops** — `wrong # args` | continues → `2` |
| `lindex` | **stops** — `wrong # args` | continues → `b` |
| `llength` | **stops** — `wrong # args` | continues → `2` |

Stock Tcl 8.4 rejects *every* cell in this table.

### Runtime-verified continuation

Each was run in `RULE_INIT` and its observable effect logged, so these are
execution results, not merely compile acceptance:

| Construct | Result |
| --- | --- |
| `while {$n < 30}` ⏎ `{…}` | `n=31` — loop ran |
| `if {1}` ⏎ `{…}` | `body-ran` |
| `if` ⏎ `{1}` ⏎ `{…}` | `body-ran` — even the condition may be on its own line |
| `foreach i {a b c}` ⏎ `{…}` | `acc=abc` |
| `for` ⏎ `{set x 0}` ⏎ `{$x<3}` ⏎ `{incr x} {…}` | `acc=012` |
| `switch "a"` ⏎ `{…}` | `matched-a` |
| `proc p {a}` ⏎ `{…}`, called via `call` | returns correctly |
| `if {0} {…}` ⏎ `else {…}` | `else-branch` |
| `if {0} {…}` ⏎ `elseif {1} {…}` | `elseif-branch` |
| `if {1}` ⏎ *(blank)* ⏎ `{…}` | **REJECT** `missing a script after "if"` |
| `if {1}` ⏎ `# comment` ⏎ `{…}` | **REJECT** `missing a script after "if"` |
| `if {1}` ⏎ *(spaces only)* ⏎ `{…}` | **REJECT** `missing a script after "if"` |
| `if {0} {…}` ⏎ *(blank)* ⏎ `else {…}` | **REJECT** `undefined procedure: else` |
| `catch {set zz 1}` ⏎ `errv` | **REJECT** `undefined procedure: errv` |
| backslash-newline before `{` | ACCEPT (as in Tcl) |
| backslash-**space**-newline | **REJECT** (as in Tcl) |

---

## 3. §F3's discriminating matrix, answered on TMM

The evidence review asks six specific questions and says the dialect-level
separator should be retained **only if the live generic cases establish it**.
They do. Run with the `__tcl_lsp_probe_*` prefix, a collision check before each
create and an absence proof after each delete
([`irules/f3-matrix/`](../../scripts/dev/bigip-probes/irules/f3-matrix/)):

| Case | Stock 8.6.18 / 9.0.4 | **TMM 21.1.0.1** | What it settles |
| --- | --- | --- | --- |
| `if {1} {expr {6*7}}` | `42` | `42` | control |
| `if {1}{expr {6*7}}` | parse error | **`42`** | the documented oddity is real |
| `list {a}{b}` | parse error | **`a b`** | **generic separator, not `if` grammar** |
| `set x {a}{b}` | parse error | **parses, then `wrong # args`** | parser acceptance *plus* ordinary arity checking |
| `if{1}{expr {6*7}}` | invalid command | **`undefined procedure: if{1}{set`** | **no separator before the first `{`** |
| `list {*}{a b}` | expands to `a b` | **`* {a b}`** | **the separator wins; there is no expansion** |

Three consequences for the redesign:

1. **Keep the dialect-level separator.** `list {a}{b}` and `set x {a}{b}` both
   split on TMM, so the rule is not `if`-specific and does not belong in
   per-command argument grammar. `Lexer::parse_brace` checking only "iRules flag
   on, next byte is `{`" matches the appliance.
2. **The separator is gated on the word having *started* with `{` or `"`.** The
   `if{1}{…}` row is the proof: a bare word glued to a brace is one word, exactly
   as in stock Tcl. This is R5 below, and it is what stops the rule
   overclaiming — `${name}`, `$v{b}` and `[cmd]{b}` are all untouched.
3. **`{*}` must not be implemented in the iRules dialect.** On TMM the separator
   wins and `{*}` is inert, yielding a literal `*` plus the unexpanded list. A
   lexer that implements 8.5 expansion here would silently disagree with the
   appliance. This is the expansion/separator interaction the review noted was
   uncovered.

The generic rule that produces all six rows is R1–R7 in §1. A second, independent
divergence — brace-line continuation, §2 — is not covered by the review's matrix
at all.

---

## 4. Interpreters — four Tcls on one appliance

Every F5-hosted context reports 8.4.6. They are not the same interpreter: each is
sandboxed differently and each fabricates `tcl_platform` its own way.

| Context | `patchlevel` | `info commands` | `tcl_platform` | Notes |
| --- | --- | --- | --- | --- |
| **iRule** (TMM) | 8.4.6 | — | 7 keys, fabricated | `os BIG-IP`, `tmmVersion 26`, `wordSize 8`, `machine` = *hostname*, `nameofexecutable` empty |
| **iApp** (scriptd) | 8.4.6 | 95 | 7 keys, real-ish | `os Linux`, `machine x86_64`, but `wordSize 4` — a 32-bit build |
| **tmsh cli script** | 8.4.6 | 95 | **empty** (0 elements) | Adds a non-standard `info vartype` subcommand |
| `/usr/bin/tclsh` | 8.5.13 | — | standard | Stock system Tcl, unrelated to any F5 execution context; `tclsh8.4` also present |

TMM and scriptd are **behaviourally identical** on every case probed: a tmsh cli
script reproduces both R2 (`list {a}b` → `a b`) and N1 (`while {$n<30}` ⏎ `{…}` →
`n=31`).

### No 8.5 features anywhere

25 features were probed in the tmsh and iApp interpreters against both stock
builds. Sixteen cleanly separate 8.4 from 8.5, and **all sixteen behave as 8.4**
in both F5 contexts. The lone apparent pass, `{*}`, is the R2 artifact above
rather than real expansion.

| Feature | tcl8.4 | tcl8.5 | tmsh | iApp |
| --- | --- | --- | --- | --- |
| `{*}` expansion | no | yes | **false pass** | **false pass** |
| `dict` | no | yes | no | no |
| `lassign` | no | yes | no | no |
| `apply` | no | yes | no | no |
| `lreverse` / `lrepeat` | no | yes | no | no |
| `string reverse` | no | yes | no | no |
| `**` operator | no | yes | no | no |
| `in` / `ni` operators | no | yes | no | no |
| `::tcl::mathop` | no | yes | no | no |
| `chan` | no | yes | no | no |
| `switch -matchvar` | no | yes | no | no |
| `string is wideinteger` | no | yes | no | no |
| `info frame` | no | yes | no | no |
| `namespace ensemble` | no | yes | no | no |

---

## 4a. Do the tmsh and iApp parsers match the iRule parser?

**Yes — exactly, on every grammar and newline case.** The three F5 execution
contexts are one parser; they differ only in command surface and environment.

A single 34-case list
([`suites/10-context-parity.cases`](../../scripts/dev/bigip-probes/suites/10-context-parity.cases))
is compiled into four wrappers by
[`gen-context-parity.py`](../../scripts/dev/bigip-probes/lib/gen-context-parity.py)
— an iRule, a `cli script`, an iApp template+service, and a plain `tclsh`
script — so any difference in the transcripts is a real context difference and
not a difference in what was asked. Raw output:
[`results/10-context-parity.txt`](../../scripts/dev/bigip-probes/results/10-context-parity.txt).

### Parser behaviour — identical across all three F5 contexts

| Case | TmmIRule | TmshCliScript | IAppImplementation | host 8.4 | host 8.5 |
| --- | --- | --- | --- | --- | --- |
| `if {1} {…}` (control) | `42` | `42` | `42` | `42` | `42` |
| `if {1}{…}` | **`43`** | **`43`** | **`43`** | parse error | parse error |
| `list {a}{b}` | **`a b`** | **`a b`** | **`a b`** | parse error | parse error |
| `set zq {a}{b}` | **wrong # args** | **wrong # args** | **wrong # args** | parse error | parse error |
| `if{1}{…}` | invalid command | invalid command | invalid command | invalid command | invalid command |
| `list {*}{a b}` | **`* {a b}`** | **`* {a b}`** | **`* {a b}`** | parse error | `a b` |
| `list "a"b` | **`a b`** | **`a b`** | **`a b`** | parse error | parse error |
| `list ${zz}b` | `XXb` | `XXb` | `XXb` | `XXb` | `XXb` |
| `if {1}` ⏎ `{…}` | **`45`** | **`45`** | **`45`** | wrong # args | wrong # args |
| `while {$n<30}` ⏎ `{…}` | **`31`** | **`31`** | **`31`** | wrong # args | wrong # args |
| `if {0} {…}` ⏎ `else {…}` | **`else`** | **`else`** | **`else`** | invalid command `else` | invalid command `else` |
| `list a b` ⏎ `{c}` | **`a b c`** | **`a b c`** | **`a b c`** | invalid command `c` | invalid command `c` |
| `if {1}` ⏎ *(blank)* ⏎ `{…}` | error | error | error | error | error |
| `if {1}` ⏎ `# c` ⏎ `{…}` | error | error | error | error | error |
| `expr {"abc" starts_with "a"}` | **`1`** | **`1`** | **`1`** | syntax error | invalid bareword |
| `expr {1 and 1}` | **`1`** | **`1`** | **`1`** | syntax error | invalid bareword |
| `expr {010}` / `{0x10}` / `{1e3}` | `8` / `16` / `1000.0` | same | same | same | same |
| `expr {0b101}` | error | error | error | error | **`5`** |

Every R-rule and N-rule row is identical across `TmmIRule`, `TmshCliScript` and
`IAppImplementation`, and every one of them differs from both host builds. The
word-form `expr` operators are **not** an iRules-only extension — they are
present in the tmsh and iApp interpreters too. Numeral handling is 8.4
throughout (`0b101` fails everywhere except host 8.5).

### Where the contexts genuinely differ

Not in the parser — in identity, environment, and command surface:

| | TmmIRule | TmshCliScript | IAppImplementation | host 8.4 |
| --- | --- | --- | --- | --- |
| `info patchlevel` | 8.4.6 | 8.4.6 | 8.4.6 | **8.4.13** |
| `tcl_patchLevel` | 8.4.6 | **UNSET** | 8.4.6 | 8.4.13 |
| `tmsh::version` | n/a | **21.1.0.1** | **21.1.0.1** | n/a |
| `llength [info commands]` | **152** | 95 | 95 | 85 |
| `tcl_platform` keys | 7, fabricated | **0 (empty)** | 7, real Linux | 8, real |
| `tcl_platform(wordSize)` | 8 | — | **4** | 8 |
| `tcl_platform(machine)` | *hostname* | — | x86_64 | x86_64 |
| `exec` | **absent** | **works** | **works** | works |
| `package names` | `Tcl` | `Tcl` | **tclparser, xml::tcl, http, uri, uuencode, xslt::libxslt, sha256, …** | `Tcl` |

Three consequences:

1. **`exec` is available in `TmshCliScript` and `IAppImplementation` but not in
   `TmmIRule`.** A command-availability fact measured in one context must never
   be promoted to another. This is the concrete case behind F1 and F4.
2. **The host `tclsh` is a different Tcl build entirely** — 8.4.13, not the
   8.4.6 embedded in all three F5 contexts. Reading a version off the host
   `tclsh` would have produced a wrong answer for every F5 row.
3. **`tcl_patchLevel` does not exist in `TmshCliScript`**, whose `tcl_platform`
   is also empty. Any probe that reads either without guarding aborts there —
   this one did, on its first run.

### Command resolution happens at rule load — even inside `catch`

Found by breaking the probe: a literal reference to a command TMM does not have
is rejected when the **rule is loaded**, regardless of `catch`.

```
ltm rule … { when RULE_INIT { if {[catch {tmsh::version} e]} { … } } }
  -> 01070151:3: error: [undefined procedure: tmsh::version]
```

The other three contexts resolve at runtime and accept the same body. For an
editor this is the difference between a warning and a hard error: in iRules an
unknown command is a load-time failure that `catch` cannot soften, so
"unknown command" is safe to surface as an error rather than a hint. The
standalone case is
[`ctx_unknown_cmd.conf`](../../scripts/dev/bigip-probes/irules/context-parity/ctx_unknown_cmd.conf).

This is also why the `s_exec` row above reads `invalid command name "exec"` for
`TmmIRule` rather than `command is disabled: "exec"`: reached through `eval` at
runtime the command is simply absent, whereas a literal `exec` in rule source is
rejected at load with the "disabled" wording (§5).

---

## 5. Command availability

All 85 stock Tcl 8.4 builtins were probed individually against the iRule
compiler. **31 are disabled**, 2 are absent, 52 are present.

**Disabled** (`command is disabled: "X"` at rule load):

```
auto_execok  auto_import  auto_load  auto_qualify  cd     eof     exec
exit         fblocked     fconfigure fcopy         file   flush   gets
glob         interp       load       namespace     open   package pid
pwd          rename       seek       socket        source tell    time
unknown      update       vwait
```

Note `namespace`, `time`, `rename`, `interp` and `package` — these are not
obviously I/O-related and are easy to assume available.

**Absent** (not builtins at all — they are `init.tcl` procs in a stock
interactive `tclsh`): `auto_load_index`, `tclLog`.

**Present**: the remaining 52, including `after`, `eval`, `subst`, `clock`,
`regexp`/`regsub`, `array`, `upvar`, `binary`, `puts`, `catch`, `trace`
(8.3-era form only — `trace add variable …` fails with `wrong # args`).

---

## 6. Other divergences

### Accepted by iRules, rejected by stock Tcl

The `expr` language is extended with word-form operators, valid both inside
`if {…}` and in a bare `expr`.

```tcl
if {[HTTP::uri] starts_with "/api"}    ;# starts_with  ends_with  contains  equals
if {[HTTP::host] matches_glob "*.io"}  ;# matches_glob  matches_regex  matches
if {$a and $b}                         ;# and  or  not  — word-form booleans
set r [expr {"abcdef" starts_with "abc"}]
```

### Rejected by iRules, legal in stock Tcl

| Construct | F5 diagnostic |
| --- | --- |
| the 31 disabled commands above | `command is disabled: "…"` |
| bare command outside any `when` | `"set" unknown property` — rejected by the config layer, not Tcl |
| two `when` blocks, same event | `Duplicate event` |
| nested `when` | `command is not valid in the current scope` |
| unknown event name | `unknown event (NOT_A_REAL_EVENT)` |
| text after a `when` block's brace | `unexpected extra argument "junk"` |
| `#` comment after a command, same line | `invalid 'local0.'; expected:-noname` |
| `#` in argument position | `invalid 'local0.'; expected:-noname` |
| unbalanced `{` or `}` inside a comment | `incomplete command` / `"log" unknown property` |
| `expr { SomeInvalidOperator() }` | rejected **at load**; stock Tcl defers to runtime (`unknown math function`) |

That last row is a genuine strictness gain: F5 validates `expr` math functions at
compile time.

### iRule-only structural rules

| Rule | Behaviour |
| --- | --- |
| `when EVENT priority N` | **N must be 0–1000 inclusive.** `1001` and `-1` are rejected — misleadingly, as `unexpected extra argument "1001"` rather than a range error. |
| top-level `proc` | Definition is accepted, but it is reachable **only** through `[call myproc …]`. Direct invocation `[myproc …]` fails at compile with `undefined procedure: myproc`. |
| comments and braces | Brace counting inside comments is **identical to Tcl**: an unbalanced `{` or `}` in a comment breaks the enclosing block, but unbalanced quotes and apostrophes are harmless, and commenting out a whole balanced block works. |

### Warnings the compiler does emit

F5 has a warning channel, which makes R6's and N1's silence notable — it chose
not to warn on either parser divergence.

| Trigger | Warning |
| --- | --- |
| `if $v { … }` | `use curly braces to avoid double substitution` |
| 4th word of `if` not `else`/`elseif` | `deprecated usage, use else or elseif` |
| stray `}` reaching a command word | `unmatched closing character` |

---

## 7. What is *not* divergent

Negative results matter as much as positive ones for a parser implementation —
these are places it would be easy to wrongly special-case:

- **The `expr` sub-parser is unmodified.** `expr {"a"eq"a"}`,
  `expr {[string length "xy"]eq"2"}` and `expr {1+1}` all work identically in F5
  and in stock 8.4/8.5. Operator adjacency inside `expr` has always been legal in
  Tcl — it tokenises rather than word-splits. R2 is a *script parser* phenomenon
  only.
- **`${name}` substitution is unchanged.** See R5. `@${offset}c` is one word
  everywhere.
- **Comment brace-counting is unchanged.** Identical to Tcl.
- **Backslash-newline continuation is unchanged**, including the rule that a
  trailing space after the backslash breaks it.
- **`switch -glob` / `-regexp` / `--`, dash-fallthrough bodies, and mixed
  quoted/braced patterns** all behave as in Tcl.

---

## 8. Traffic lab — event-context and runtime behaviour

Everything above §6 is compile-time or `RULE_INIT`. To test real event handlers, a
lab was built: a virtual server on the BIG-IP, with `dev` (192.168.9.80) acting as
both client and backend.

```
dev (curl) ──► 192.168.9.241:80  ltm virtual lab_vs2 ──► pool ──► dev:8000 (python http.server)
dev (curl) ──► 192.168.9.240:80  ltm virtual lab_vs   ──► HTTP::respond (no backend)
```

Both virtual servers, the pool, and the backend were removed afterwards. Note that
on this 1-NIC VE, a pool member may be neither a self IP (`01070080`) nor a
loopback address (`01020061`), and VIP-targeting-VIP did not loop back — an
off-box backend was required. `source-address-translation { type automap }` is
needed for the return path.

### Both divergences confirmed over the wire

A single request through TMM, with the iRule returning its own parse results to
the client. This is end-to-end evidence, not compile acceptance:

```
GET /parsecheck  ->  r2a=<a b> r2b=<a b> expand=<* {a b c}> n1=<a b c>
                     nospace=<yes> whilenl=<3> krelse=<else>
```

- `{a}{b}` and `{a}b` → `a b` — **R2 holds in event context**
- `{*}{a b c}` → `* {a b c}` — **the `{*}` trap is real at runtime**, not just at load
- `list a b` ⏎ `{c}` → `a b c` — **N1/N2 hold in event context**
- `if {1}{…}` ran; `while` with the brace on the next line looped 3 times; `else`
  on its own line took the else branch

### Event-context validity is enforced at compile time

120 (command × event) pairs were probed. Invalid pairs are rejected at rule load
with `command is not valid in current event context (EVENT)`.

| command | RULE_INIT | CLIENT_ACCEPTED | CLIENT_DATA | HTTP_REQUEST | LB_SELECTED | SERVER_CONNECTED | HTTP_RESPONSE | CLIENT_CLOSED |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `HTTP::uri` | yes | – | – | yes | yes | yes | **–** | – |
| `HTTP::status` | yes | – | – | **–** | yes | – | yes | – |
| `HTTP::respond` | yes | – | – | yes | – | – | yes | – |
| `HTTP::collect` | yes | – | – | yes | yes | – | yes | – |
| `IP::client_addr` | **–** | yes | yes | yes | yes | yes | yes | yes |
| `IP::server_addr` | – | yes | yes | yes | yes | yes | yes | yes |
| `TCP::client_port` | – | yes | yes | yes | yes | yes | yes | yes |
| `TCP::collect` | yes | yes | yes | yes | yes | yes | yes | yes |
| `TCP::payload` | yes | yes | yes | yes | yes | yes | yes | yes |
| `LB::server` | – | yes | yes | yes | yes | yes | yes | yes |
| `SSL::cipher` | – | – | – | yes | yes | – | yes | – |
| `pool` | – | yes | yes | yes | yes | yes | yes | yes |
| `node` | – | yes | yes | yes | yes | yes | yes | yes |
| `table` | – | yes | yes | yes | yes | yes | yes | yes |
| `static::` | yes | yes | yes | yes | yes | yes | yes | yes |

Two things to note before using this as an LSP table:

- **It is compile-time only.** `RULE_INIT` permits the `HTTP::*` commands at load
  but they are meaningless there; the same run logged
  `TCL error: … can't read "static::probesv": no such variable` from a
  `RULE_INIT` probe that had compiled cleanly. Compile acceptance in `RULE_INIT`
  should not be read as "valid to use".
- The asymmetries are the useful part: `HTTP::uri` is rejected in `HTTP_RESPONSE`
  and `HTTP::status` in `HTTP_REQUEST`, which are exactly the mistakes an editor
  should catch.

### Rule priority

`priority` overrides the order rules are attached to the virtual server. Attached
as `{ lab_p_low lab_p_default lab_p_high }`, execution order was:

```
lab_p_high    (priority 100)
lab_p_default (no priority given)
lab_p_low     (priority 900)
```

So **lower number runs first**, and the **default is 500** — the midpoint of the
0–1000 range from §5. `HTTP::respond` in the first rule did **not** prevent the
remaining rules from running.

### Uncaught runtime errors

A rule that divides by zero mid-event:

- executes normally up to the failing command (the preceding `log` fired),
- logs `01220001:3: TCL error: /Common/lab_err <HTTP_REQUEST> - divide by zero
  while executing "expr {1/0}"` at **err** level,
- aborts the remainder of that event handler, and
- **resets the client connection** (`curl: (56) Recv failure: Connection reset by
  peer`).

---

## 9. Corpus check

### Public corpus — 170 `when`-bearing files, 9 repositories

Fetched with `scripts/dev/fetch-irules-corpus.sh`.

| Occurrences | Construct | Verdict |
| --- | --- | --- |
| 15 | `}{` — R2 in production, e.g. `if {$x eq ""}{ set x NA }` | Load-bearing, not a curiosity. |
| 18 | `${var}` + bareword chars, mostly `binary scan @${off}c` | **safe** — byte-identical to stock. R2 must not fire here. |
| 14 | comments with unbalanced braces or quotes | **safe** — commented-out blocks that balance across lines, or unbalanced quotes. |
| 332 | `equals`, `contains`, `starts_with`, `ends_with`, `matches` | Word-form `expr` operators are pervasive. |
| 49 | `and` / `or` / `not` in expressions | Confirmed accepted. |
| 53 | `when EVENT priority N` | Prompted the range probe: 0–1000. |
| 10 | `[call proc]` against 5 top-level `proc` definitions | Prompted the direct-invocation probe, which fails. |

### Private aggregate — `~/src/tcl-lsp-testsrc`

~559 iRule-bearing files (1050 candidates). Counts only; contents are
access-controlled and are not reproduced here or in any published artifact.

- **144** occurrences of `}{` — an order of magnitude more than the public
  corpus. R2 is thoroughly mainstream.
- **46** occurrences of `"{`.
- **2** occurrences of `{*}`, both benign on inspection: one inside a JSDoc-style
  comment, one a `set … {*}` where `*` is the intended CORS wildcard value. **No
  real `{*}` expansion usage exists in either corpus** — consistent with it never
  having worked.

### `simonkowallik/irulescan` test suite — publicly available

The single most valuable find, reached via the aggregate but public in its own
right at [github.com/simonkowallik/irulescan](https://github.com/simonkowallik/irulescan);
nothing below depends on access-controlled material. `tests/bigip/syntax/` is a
hand-curated set of
parser torture cases — `while {…}` with the brace on the next line, backslash
continuations with and without a trailing space, `for` split across four lines,
and `switch {4}{…}` brace adjacency. **This suite is what surfaced the entire
N-rule divergence**, which none of the production corpora would have revealed,
because production code is written to look normal.

Worth adopting as regression fixtures (BSD-style upstream; check the licence
before vendoring).

### `~/src/bigip-extract`

Extracted vendor manpages plus `irule-command-synopisis-21.0.0.1-0.0.13.md`
(a per-command reference for 21.0.0.1). It is a *command* reference and does not
document parser behaviour or restricted commands, so the disabled-command list in
§5 was derived empirically instead.

---

## 10. Method

Rules were fed to the real compiler with `tmsh load sys config merge file`
(~0.3 s per probe), then `tmsh list` distinguished *accepted* from *accepted with
warnings* from *rejected*, and the rule was deleted. Word lists and runtime
effects were captured by executing the construct inside `RULE_INIT` and logging
the observable result to `/var/log/ltm` — so every semantic claim above is an
execution result, not merely compile acceptance. Cases whose literal braces would
unbalance the tmsh config layer were reached by building the script text at
runtime and passing it to `eval`.

Controls ran the identical snippets through `tclsh8.4` and `tclsh8.5` on the same
host.

Useful driver details, if this is re-run:

- `tmsh -f <file>` does not exist, and piping a rule into `tmsh` on stdin fails —
  `tmsh load sys config merge file` is the working path.
- `tmsh load sys config merge file` **creates** an iApp service without running its
  implementation; force it with
  `tmsh modify sys application service <name> execute-action definition`.
- iApp implementations log via `tmsh::log <level> <msg>` (a level keyword, not
  iRules' `local0.`), and info-level messages do not reach `/var/log/ltm` — use
  `err`.
- syslog collapses repeated identical lines, so emit one joined line per probe
  rather than one line per case.
- Probes were wrapped in `when HTTP_REQUEST` (compiled, never executed) except
  where runtime values were needed, which used `RULE_INIT` (runs at load, once
  per TMM).
- Stubbing `unknown` in a tclsh control silently swallows misuse of *builtins*
  too (an `else` command, for instance). Stub only the iRule-specific commands.

### Caveats

Compile-time acceptance and runtime behaviour are distinct. Where only compile
acceptance was measured, that is what is claimed; every N-rule and word-formation
result was additionally verified by execution. A few probes — notably
namespace-qualified `set` and `static::` reads — were excluded from divergence
claims because they differ only through this asymmetry.

---

## 11. Relationship to the evidence review

### What this closes

| Review finding | What was measured | Effect |
| --- | --- | --- |
| **F1** — the proposal conflates six language contexts | Four of the six measured with **one shared 34-case list** (§4a): `TmmIRule`, `TmshCliScript`, `IAppImplementation`, `HostShellTcl`. | **Refines F1.** The three F5 contexts are *one parser* — every grammar and newline case is identical — but they are **not** one environment: `exec` is absent in `TmmIRule` and works in the other two, `info commands` counts 152/95/95, and `tcl_platform` is fabricated / **empty** / real-Linux respectively. So split the key on *command surface and environment*, not on grammar. Two contexts remain **unmeasured**: `IAppPresentationApl` and `IAppPresentationTclCallback`, recorded as `Unknown` by the driver itself. |
| **F2** — Tcl release defaults have no provenance | All three F5 contexts report `8.4.6`; 16 features that cleanly separate 8.4 from 8.5 behave as 8.4 in every one (§4); numeral handling is 8.4 throughout (§4a). Controls are `tclsh8.4`/`tclsh8.5` **on the same appliance**. | Gives 21.1.0.1 a measured row rather than a guessed one, and **directly vindicates the review's "one observed `tclsh`" objection**: `/usr/bin/tclsh8.4` is **8.4.13**, not the 8.4.6 embedded in all three F5 contexts. Reading the version off the host would have been wrong for every F5 row. Still one build; see F8. |
| **F3** — `}{` overfits one command, overclaims all | The full six-row matrix, run on TMM (§3). | **Retain the dialect-level separator.** Gate it on the word having started with `{` or `"`. Do not implement `{*}` in the iRules dialect. |
| **F4** — tmsh policy is not core Tcl availability | All 85 stock 8.4 builtins probed individually against the iRule compiler: 31 disabled, 2 absent, 52 present (§5). Cross-context: `exec` absent in `TmmIRule`, working in `TmshCliScript` and `IAppImplementation` (§4a). | Supplies the `TmmIRule` **rule-load** surface as data and proves it does **not** generalise. Also separates two mechanisms that look alike: a literal disabled command is refused at load with `command is disabled`, while the same command reached through `eval` at runtime is simply `invalid command name`. |
| **F5** — `tcl_platform` has iRules-specific semantics | Measured in all three F5 contexts (§4): TMM fabricates it (`machine` = hostname, `os BIG-IP`, `tmmVersion 26`, `wordSize 8`), iApp reports a real-ish Linux with `wordSize 4`, and a tmsh cli script's array is **empty**. | Confirms F5, and shows the divergence is three-way, not two-way. |
| **F8** — one build must become a fixture | [`scripts/dev/bigip-probes/`](../../scripts/dev/bigip-probes/) — 378 iRules, drivers, controls, raw transcripts. | Partially discharges F8: it is a re-runnable fixture, but see the delta below before treating it as the E4 artefact. |

**F6** (BIG-IP release vs tmsh syntax release) and **F7** (iApp target and
execution policy as action-local data) were not addressed; nothing here bears on
them.

### What this run did *not* do

Two runs are described here and they differ in rigour. The **§3 F3 matrix** and
the **§4a four-context parity probe** were run under the E4 contract —
`__tcl_lsp_probe_*` names, an exact-name absence check before every create, an
`EXIT` trap deleting only those names, an absence proof after every delete, an
explicit "attached to a virtual server?" check, and the APL contexts recorded as
`Unknown` rather than inferred. The driver is
[`lib/e4-context-probe.sh`](../../scripts/dev/bigip-probes/lib/e4-context-probe.sh).

The **earlier bulk run** (§5–§9) answered the same questions under a looser
procedure. Its differences, in full:

- Probe objects used `probe_*`, `lab_*` and `probe_ws_*` prefixes, **not**
  `__tcl_lsp_probe_*`. Only `irules/f3-matrix/` uses the reserved prefix.
- There was **no exact-name absence check before each create** and **no `EXIT`
  trap**, except in the F3 matrix run. A name collision would have been silently
  overwritten by the merge rather than aborting.
- The traffic lab (§8) **deliberately attached rules to virtual servers**, which
  E4 step 7 forbids. It also created a pool, two virtual servers, and an off-box
  backend.
- Role and command visibility were recorded only as "run as the SSH login user";
  policy settings such as `systemauth.disablebash` were not captured, so the
  §5 command surface carries no role annotation.
- The APL contexts of E4 step 6 were not exercised at all.

What it did satisfy: `save sys config` was never run; every created object was
deleted and the absence verified; the stock-Tcl controls were run on the
appliance rather than on a developer machine; and no conclusion about a missing
command rests on a single `info commands` result — §5 probes each builtin as a
separate rule load and distinguishes `command is disabled` from
`undefined procedure`.

**Recommendation.** Treat §3 and §4a as E4-grade evidence and everything else as
a strong but non-conforming transcript. One caveat on §4a: the payload needed two
corrections mid-session (each a finding in its own right — see the provenance
note in `results/10-context-parity.txt`), so its four context blocks come from
two consecutive runs rather than one, and the E4.4b command-resolution probe has
not yet been run standalone. Re-running the suites under the full E4
contract is mechanical — `lib/runner.sh` needs only a prefix change and a
pre-create absence check — and would upgrade the whole document.

---

## 12. Open questions

- Whether the `switch` *body* list is re-parsed by the same script parser
  (`switch "a" {a{log …}}` is rejected, but the failure mode was not isolated to
  either parser).
- The disabled-command list was derived from the 85 stock 8.4 builtins; F5's own
  command namespace was not enumerated from the other direction.
- Whether TMM and scriptd share one parser *build*, or merely agree on every case
  probed here.
- The event-context matrix covers 15 commands across 8 events. The full matrix
  (hundreds of `NAMESPACE::command` entries across ~100 events) would need the
  manpage extraction in `~/src/bigip-extract` cross-checked against live probes.
- Whether N1 interacts with `priority` ordering or with event-time versus
  load-time compilation.
