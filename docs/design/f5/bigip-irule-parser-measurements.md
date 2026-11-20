# BIG-IP iRule parser measurements ([#1631](https://github.com/bitwisecook/tcl-lsp/issues/1631))

> **Purpose.** The appliance transcript behind the F5 rows of
> [`dialect-and-package-registry-redesign.md`](../registry/dialect-and-package-registry-redesign.md),
> including the answer to its F3 discriminating matrix.
>
> **Probe corpus:** [`scripts/dev/bigip-probes/`](../../../scripts/dev/bigip-probes/)
> — 378 iRules, the drivers that ran them, the stock-Tcl controls, and the raw
> result files.
>
> **Methodology caveat up front:** only §3 and §4a were run under the E4 probe
> contract ([`dialect-and-package-registry-redesign.md`](../registry/dialect-and-package-registry-redesign.md)
> §0.2). See [§11](#11-evidence-contracts) for the exact delta
> before relying on any other row as E4-conforming evidence.

Measured against a live appliance, not inferred from documentation.

| | |
| --- | --- |
| **Host** | `bigip1` |
| **Version** | BIG-IP 21.1.0.1, build 0.0.26 (2026-07-14) |
| **Probed** | 2026-08-26; §12 follow-up 2026-11-20 on the same build (VM 211, 192.168.9.24) |
| **Probes** | 97 syntax · 41 word-formation · 25 feature-availability · 85 command-availability · 120 event-context · 30 runtime-semantic · 15 proc-semantics · 14 dynamic-code |
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

## 3. Discriminating word-formation matrix

The six byte-controlled cases distinguish generic word formation from command
argument grammar. The fixture uses the `__tcl_lsp_probe_*` prefix, a collision
check before each create and an absence proof after each delete
([`irules/f3-matrix/`](../../../scripts/dev/bigip-probes/irules/f3-matrix/)):

| Case | Stock 8.6.18 / 9.0.4 | **TMM 21.1.0.1** | What it settles |
| --- | --- | --- | --- |
| `if {1} {expr {6*7}}` | `42` | `42` | control |
| `if {1}{expr {6*7}}` | parse error | **`42`** | the documented oddity is real |
| `list {a}{b}` | parse error | **`a b`** | **generic separator, not `if` grammar** |
| `set x {a}{b}` | parse error | **parses, then `wrong # args`** | parser acceptance *plus* ordinary arity checking |
| `if{1}{expr {6*7}}` | invalid command | **`undefined procedure: if{1}{set`** | **no separator before the first `{`** |
| `list {*}{a b}` | expands to `a b` | **`* {a b}`** | **the separator wins; there is no expansion** |

Current grammar contract:

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
   appliance. Expansion and word separation remain distinct grammar features.

The generic rule that produces all six rows is R1–R7 in §1. A second, independent
divergence — brace-line continuation — has its own controls in §2.

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

**The three contexts agree on all 34 entered parity cases.** This establishes
agreement for those exact grammar and newline inputs, not parser-build identity
or equivalence for unmeasured source. Command surface and environment remain
context-specific.

A single 34-case list
([`suites/10-context-parity.cases`](../../../scripts/dev/bigip-probes/suites/10-context-parity.cases))
is compiled into four wrappers by
[`gen-context-parity.py`](../../../scripts/dev/bigip-probes/lib/gen-context-parity.py)
— an iRule, a `cli script`, an iApp template+service, and a plain `tclsh`
script — so any difference in the transcripts is a real context difference and
not a difference in what was asked. Raw output:
[`results/10-context-parity.txt`](../../../scripts/dev/bigip-probes/results/10-context-parity.txt).

### Entered parser cases — agreement across three F5 contexts

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
   be promoted to another.
2. **The host `tclsh` is a different Tcl build entirely** — 8.4.13, not the
   8.4.6 embedded in all three F5 contexts. Reading a version off the host
   `tclsh` would have produced a wrong answer for every F5 row.
3. **`tcl_patchLevel` does not exist in `TmshCliScript`**, whose `tcl_platform`
   is also empty. A probe must guard these reads so missing globals do not
   abort the remaining cases.

### Command resolution happens at rule load — even inside `catch`

A literal reference to a command absent from TMM is rejected when the **rule
is loaded**, regardless of `catch`.

```
ltm rule … { when RULE_INIT { if {[catch {tmsh::version} e]} { … } } }
  -> 01070151:3: error: [undefined procedure: tmsh::version]
```

The other three contexts resolve at runtime and accept the same body. For an
editor this is the difference between a warning and a hard error: in iRules an
unknown command is a load-time failure that `catch` cannot soften, so
"unknown command" is safe to surface as an error rather than a hint. The
standalone case is
[`ctx_unknown_cmd.conf`](../../../scripts/dev/bigip-probes/irules/context-parity/ctx_unknown_cmd.conf).

This is also why the `s_exec` row above reads `invalid command name "exec"` for
`TmmIRule` rather than `command is disabled: "exec"`: reached through `eval` at
runtime the command is simply absent, whereas a literal `exec` in rule source is
rejected at load with the "disabled" wording (§5).

---

## 4b. Procedures, and what actually separates the contexts

### Ordinary procs work everywhere, including iRules

Fifteen proc cases run through the same four-wrapper harness
([`suites/11-proc-semantics.cases`](../../../scripts/dev/bigip-probes/suites/11-proc-semantics.cases),
transcript [`results/11-proc-semantics.txt`](../../../scripts/dev/bigip-probes/results/11-proc-semantics.txt)).
Fourteen of fifteen are **identical in all three F5 contexts**:

| Case | TmmIRule | TmshCliScript | IAppImplementation |
| --- | --- | --- | --- |
| define + direct call | `42` | `42` | `42` |
| no args / default arg / varargs | `plain` / `7` / `3` | same | same |
| recursion (`5!`) | `120` | `120` | `120` |
| `upvar` / `global` / `uplevel` | `9` / `11` / `4` | same | same |
| proc defined inside a proc | `in` | `in` | `in` |
| redefinition wins | `two` | `two` | `two` |
| `info procs` | `__p10` | `__p10` | `__p10` |
| `return -code error` | `boom` | `boom` | `boom` |
| `rename` a proc | `orig` | `orig` | `orig` |
| **`call <proc>`** | **`proc __p11 not found`** | **`invalid command name "call"`** | **`invalid command name "call"`** |

So the answer to "can tmsh and iApps have regular procs with regular proc
calls?" is **yes, unreservedly** — and so can iRules, *at runtime*. What iRules
add is not a restriction on `proc` semantics but a pair of **load-time** rules:
a proc written at rule top level is reachable only through `call` (a literal
command head must resolve when the rule loads, §4a), and `proc` is banned
outright inside a `when` body. Both are lexical, and both are escapable by
holding the script text in a variable — see §4c, which is where the fifteen
cases in this table actually ran.

`call` itself is an `f5-irules` command: it exists only in TMM, and even there it
resolves *rule* procs, not runtime-defined ones — hence
`proc __p11 not found` rather than `invalid command name`.

### The 31 "disabled" commands are two mechanisms, not one

Re-probing each of the 31 through `eval` at runtime splits them exactly
([`ctx_disabled_runtime.conf`](../../../scripts/dev/bigip-probes/irules/proc-semantics/ctx_disabled_runtime.conf)):

**Absent from TMM's interpreter (16)** — `invalid command name` even via `eval`:

```
auto_execok  auto_import  auto_load  auto_qualify  cd    exec  exit
fconfigure   file         glob       load          open  pwd   socket
source       unknown
```

**Present in the interpreter, refused only by the rule compiler (15)** —
reachable via `eval`, and `rename` demonstrably works:

```
eof  fblocked  fcopy  flush  gets  interp  namespace  package
pid  rename    seek   tell   time  update  vwait
```

The first group is a smaller interpreter build. The second is **pure load-time
policy** — the commands are right there, and only the compiler's opinion stops
you. That distinction matters for diagnostics: the second group is a policy
warning about rule *source*, not a statement about the language.

### Recommended model: `f5-tcl` with `f5-irules` as an offshoot

The measurements support splitting the F5 story in two, rather than treating
`f5-irules` as the sole F5 dialect:

**`f5-tcl` — the shared fork.** Everything the three contexts agree on:

- the parser: R-rules, N-rules, inert `{*}`, the exact `${name}` / bare-word
  gating (§1, §2, §4a)
- the `expr` extensions: `starts_with`, `ends_with`, `contains`, `equals`,
  `matches`, `matches_glob`, `matches_regex`, and word-form `and` / `or` / `not`
  — **present in tmsh and iApps too, not iRules-only**
- Tcl 8.4.6 semantics throughout, including 8.4 numerals (`0b101` fails)
- ordinary `proc` semantics in full

**`f5-irules` — an offshoot of `f5-tcl`,** adding, and only in TMM:

- closed-world command resolution **at rule load**, unaffected by `catch`
- the event model: `when` blocks, per-event command validity, duplicate/nested
  event rejection, `priority` 0–1000 (default 500)
- declaration-only top level; top-level `proc` reachable only via `call`
- the 15-command load-time policy ban, plus a 16-command smaller interpreter
- `expr` math-function validation at load

**`f5-tmsh` and `f5-iapps`** then become `f5-tcl` **environments**, differing
from each other only in ambient packages and host facts — `exec` present,
`tcl_platform` empty vs real-Linux, `package names` small vs large — with no
grammar delta between them at all.

This keys execution contexts independently of the shared language family, and
it puts each measured fact at the level where it was actually observed.

---

## 4c. Dynamic code: what iRules will and will not let you build at runtime

§4b established that ordinary procs work in all three contexts. This section
draws the line iRules actually enforce, because it is not where it first
appears. Probes: [`irules/dynamic-code/`](../../../scripts/dev/bigip-probes/irules/dynamic-code/),
transcript [`results/12-dynamic-code.txt`](../../../scripts/dev/bigip-probes/results/12-dynamic-code.txt).

### `proc` is banned inside a `when` body — and the ban is *lexical*

| Form | Result |
| --- | --- |
| `when RULE_INIT { proc p {} {…} }` | **REJECT** `command is not valid in the current scope` |
| `when RULE_INIT { eval {proc p {} {…}} }` | **REJECT** — same error |
| `when RULE_INIT { uplevel #0 {proc p {} {…}} }` | **REJECT** — same error |
| `when RULE_INIT { eval {set x 1} }` | ACCEPT — so `eval` itself is fine |
| `when RULE_INIT { uplevel #0 {set x 1} }` | ACCEPT — so `uplevel` itself is fine |
| top-level `proc p …` + `[p]` | **REJECT** `undefined procedure: p` |
| top-level `proc p …` + `[call p]` | **ACCEPT** |

The compiler **scans braced script literals**, so wrapping `proc` in
`eval {…}` or `uplevel #0 {…}` changes nothing. Nesting is not an escape hatch.

### …but the ban is only lexical, and that is the whole story

Put the script text in a **variable** and the compiler cannot see it. Then it
works — provided *both* the definition and the call are hidden:

```tcl
when RULE_INIT {
  set s "proc dpp \{\} \{ return persisted \}"
  uplevel #0 $s          ;# definition: compiler sees only $s
  set c "dpp"
  uplevel #0 $c          ;# call: compiler sees only $c
}
```

`uplevel #0 dpn` with a *literal* head is still rejected (`undefined procedure:
dpn`) — that is the §4a load-time resolution rule, not a proc rule. Measured
behaviour of the working form:

| Property | Result |
| --- | --- |
| define + call in `RULE_INIT` | `define_rc=0 call_rc=0 v=persisted` |
| survives into `HTTP_REQUEST` | **`persist=YES(persisted)`** |
| survives across separate requests | **yes** — identical on a second request |
| define a new proc mid-request | **`defineinreq=YES(madeinrequest)`** |

So a runtime-defined proc is a **persistent, per-TMM global**, created once in
`RULE_INIT` and reusable by every later event and every later connection.

### Runtime event creation is impossible

`when` is **not a runtime command at all**:

```
when_at_runtime=NO(invalid command name "when")
```

Measured both in `RULE_INIT` and inside `HTTP_REQUEST` on a live request. The
event table is fixed when the rule loads; nothing in user space can extend it.

### Consequence: the only user-space JIT is an expression string

With `proc` lexically banned and `when` absent at runtime, the one dynamic-
evaluation door left open is **building a Tcl `expr` string and letting a second
substitution pass evaluate it**.

That is exactly what KaiWilke's
[Natural Speech Expression](https://github.com/KaiWilke/F5-Natural-Speech-Expression)
does — an expression-language interpreter written as an iRule. It JIT-compiles a
natural-language search filter into a Tcl expression, caches that expression
string in a `static::` array, and evaluates it with an **unbraced** `if`. The
source pattern does not establish a physical TMM bytecode-cache observation:

```tcl
if $static::natural_speech_cache($search_string) { … }
```

Its own comments call this "controlled double substitution", and it carries
heavy two-stage escaping precisely because the input is untrusted and this
construct is a command-injection surface.

**Diagnostic consequence.** Unbraced `if $var` is the *one* construct F5 itself
warns about — §6 records `use curly braces to avoid double substitution` — and
it is simultaneously the load-bearing primitive for the only JIT technique the
dialect permits. An editor must surface it as a **warning, never an error**, and
ideally recognise the `static::`-cached-expression idiom rather than flagging it
as a bug.

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
dev (curl) ──► 192.0.2.101:80  ltm virtual lab_vs2 ──► pool ──► dev:8000 (python http.server)
dev (curl) ──► 192.0.2.100:80  ltm virtual lab_vs   ──► HTTP::respond (no backend)
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

### Current event policy

`CommandRegistry::is_irules_call_legal_in_event` agrees with the measured
matrix except for six `RULE_INIT` compile acceptances. Those commands retain
flow requirements: load acceptance alone does not establish usable connection
state. Executed in `RULE_INIT`, all six fail at runtime, and the four `HTTP::`
ones now load with a warning
([§12](#rule_init-runtime-for-the-six-load-accepted-flow-commands)). The corpus records that distinction as `RuleInitCompileAcceptance`.

| Command and event | Current constraint in `rust/tcl-registry/src/commands/irules/` |
| --- | --- |
| `HTTP::uri` in `HTTP_RESPONSE` | `excluded_events: ["HTTP_RESPONSE"]` on `http__uri.rs` |
| `HTTP::status` in `HTTP_REQUEST` | `excluded_events: ["HTTP_REQUEST"]` on `http__status.rs` |
| `SSL::cipher` in `CLIENT_ACCEPTED`, `CLIENT_DATA`, `SERVER_CONNECTED`, `CLIENT_CLOSED` | An explicit `excluded_events` list on `ssl__cipher.rs`; the accepted set cannot be selected by an `EventProps` predicate alone |
| `SSL::cipher` and `LB::server` in `RULE_INIT` | `event_requires { flow: true }` |
| `HTTP::uri`, `HTTP::status`, `HTTP::collect` in `LB_SELECTED` | Each command's `event_requires.also_in` includes `LB_SELECTED`; compiler acceptance does not imply that an HTTP profile is present |
| `IP::server_addr` in `CLIENT_ACCEPTED`, `CLIENT_DATA`, `HTTP_REQUEST`, `CLIENT_CLOSED` | `flow: true` without a server-side requirement; the documented result before a serverside connection exists is `0` |

The current per-command constraints and measured results are checked by
`rust/tcl-registry/src/f5/corpus.rs`. Compile acceptance, runtime availability
and event-state requirements remain independent.

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
  where runtime values were needed, which used `RULE_INIT`. Rule creation runs those initialisers; worker
  coverage requires recorded TMM identities rather than a per-TMM assumption.
- Stubbing `unknown` in a tclsh control silently swallows misuse of *builtins*
  too (an `else` command, for instance). Stub only the iRule-specific commands.

### Caveats

Compile-time acceptance and runtime behaviour are distinct. Where only compile
acceptance was measured, that is what is claimed; every N-rule and word-formation
result was additionally verified by execution. A few probes — notably
namespace-qualified `set` and `static::` reads — were excluded from divergence
claims because they differ only through this asymmetry.

---

## 11. Evidence contracts

### Context and build identity

`BigIpExecutionContext` in `rust/tcl-registry/src/f5/execution_context.rs`
identifies seven contexts independently: TMM iRules, tmsh CLI scripts, iApp
implementation scripts, iCall scripts, APL presentation, APL Tcl callbacks and
host Tcl. Grammar agreement on entered cases does not establish a shared parser
build or permit a command-surface fact to cross contexts. The two APL contexts
are unmeasured and supply no family, environment or core profile.

`EmbeddedRuntimeEvidence` in `rust/tcl-registry/src/f5/evidence.rs` retains
context, exact BIG-IP build, fact and provenance. `measured_fact` requires the
exact context and build. `assistance_fact` supplies only a labelled same-context
measured or nearest-known row; it grants no semantic authority for another
build. Host Tcl transcripts are separate controls, not embedded-runtime proof.
Reported Tcl patchlevels do not grant parser, object, compiler or ABI capability.

The [resolution-context transcripts](../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md)
contain 92 byte-attested scripts entered independently in tmsh, iApp
implementation and iCall contexts. `RuntimeFact::ParserCase` retains each
original source hex, completion code and result hex. iCall's own transcript
reports Tcl 8.4.6, 95 commands, `tmsh::version` 21.1.0.1 and a successful `exec`
invocation. It does not measure interpreter width or establish a registered
catalogue environment. tmsh's empty `tcl_platform` likewise supplies no width.
Both build profiles remain `Unknown`; iApp's measured width selects
`F5Scriptd32`. No iCall fact is borrowed from iApp implementation scripts.

### Source, runtime and environment axes

- TMM's word-separator grammar applies when a word starts with `{` or `"`.
  The iRules dialect does not gain Tcl argument expansion from `{*}`.
- Literal `namespace`, `interp`, `package` and `rename` invocations are refused
  by the measured rule loader, while dynamically constructed invocations run
  on BIG-IP 21.1.0.1 build 0.0.26. `namespace path` is unsupported and the
  logical package table reports Tcl 8.4. `irules_policy::rule_loader_refuses`
  and `runtime_surface_admits` retain these separate axes. Interpreter-absent
  commands have their own availability class.
- TMM's fabricated `tcl_platform`, tmsh's empty array and iApp's host-like array
  are separate `RuntimeFact::TclPlatform` observations. A fabricated platform
  field does not establish a native object ABI. Ordinary-global CMP demotion
  does not establish a shared cross-TMM variable cell.
- `rust/tcl-registry/src/f5/corpus.rs` checks measured grammar, command surface,
  event and priority cases against the current model. Agreement, explained
  divergence and unavailable model answers remain distinct outcomes.

`rust/tcl-registry/src/f5/tmsh_syntax.rs` owns the BIG-IP-independent tmsh syntax
axis and its `tmsh::modify cli version active` transition. Literal syntax
versions can select that axis; dynamic versions remain `Unknown`. The
transition is script-scoped: it lasts for the rest of the cli script and the
caller's version is restored on return ([§12](#tmsh-cli-version-scope-and-restricted-roles)). `rust/tcl-registry/src/f5/iapp_metadata.rs`
owns action-local target constraints, `role-acl` and `run-as`; an omitted
principal means the calling user. Those documentary contracts do not claim
unmeasured appliance behaviour.

### Provenance limits

The §3 word-formation matrix and §4a parity evidence satisfy the E4
probe-and-cleanup contract: reserved object names, exact-name absence checks,
cleanup traps, deletion verification and attachment checks. The driver is
[`lib/e4-context-probe.sh`](../../../scripts/dev/bigip-probes/lib/e4-context-probe.sh).
The §5–§9 surface, event and traffic evidence carries
`e4_conforming: false`. Its raw transcript remains usable with that qualification:
object names and pre-create checks do not meet the same contract, and the
traffic lab intentionally uses virtual servers and a backend. No role or
`systemauth.disablebash` annotation is available for that command surface.

Each observation keeps its own provenance; the resolution-context transcripts
also retain their own non-E4 label. No conforming suite upgrades another suite's
record. Appliance probe creation, traffic and cleanup details belong to the
checked-in drivers and raw reports, independently of the current semantic
contracts above.

---

## 12. Open questions answered on 2026-11-20

Measured on the same build as above: BIG-IP 21.1.0.1 build 0.0.26, now running
as Proxmox VM 211 at 192.168.9.24 (single NIC, 4 TMMs, FIPS licence). Each answer
links its execution evidence. The probe drivers are in
[`scripts/dev/bigip-probes/open-questions/`](../../../scripts/dev/bigip-probes/open-questions/).
Every probe object was named `tcllsp_oq_*`, checked absent before creation and
deleted afterwards. Nothing was saved, except a `save sys config user-only` after
resetting the admin and root passwords for lab access.

### `matches` and the F5 word operators

- **`matches` is a glob match, not equality.** `"abc" matches "a*"`, `"abc" matches "a?c"`
  and `"b" matches "[abc]"` are 1, while `"abc" matches "a.c"` and `"ABC" matches "a*"` are 0.
  It is case-sensitive and anchored at both ends, so `"abcd" matches "bc"` and
  `"abcd" matches "ab"` are 0. `matches_glob` agrees; `matches_regex` is an unanchored
  regex search. See [bigip-f5-matches-operator-semantics](../../../evidence/tcl/bigip-f5-matches-operator-semantics.md).
  The analyser's authored evaluator, which uses equality, is wrong for any operand
  containing `*`, `?` or `[`.
- **The registry binding powers are right.** `contains`, `starts_with`,
  `ends_with`, `equals`, `matches`, `matches_glob` and `matches_regex` sit at the
  `==` level and are left-associative. They bind tighter than `&` and `&&` and
  looser than `<` and `+`. Every discriminating bare expression equals the grouping
  `binary_bp` predicts ([bigip-f5-word-operator-binding-power](../../../evidence/tcl/bigip-f5-word-operator-binding-power.md)).
  `and` and `or` bind exactly like `&&` and `||`, including mixed with them and
  above `?:`, and they short-circuit. `not` is the tight unary `!`; its error message
  names `"!"` ([bigip-f5-and-or-not-binding-power](../../../evidence/tcl/bigip-f5-and-or-not-binding-power.md)).
  W154/W155's grouping is therefore measured, not assumed.
- The operators work the same in a computed `expr` string the loader cannot see.
  The loader then warns `use curly braces to avoid double substitution`
  ([bigip-f5-word-operators-in-computed-expr](../../../evidence/tcl/bigip-f5-word-operators-in-computed-expr.md)).

### `switch` bodies are parsed as lists, not scripts

The R-rules (implicit word break after a closing `}` or `"`) apply to script
words only. A `switch` pattern/body list is split with stock Tcl list rules,
both by the loader and by TMM at runtime:

- A literal `switch a {"a"{set r A}}` (or `{a}{…}`, or `{default}{…}`) is refused
  at load with Tcl's own `invalid list: list element in quotes followed by "{set" instead of space`
  ([bigip-switch-body-list-at-load](../../../evidence/tcl/bigip-switch-body-list-at-load.md)).
- The same value built at runtime fails `llength` and `switch` with the same list
  error ([bigip-switch-body-list-runtime](../../../evidence/tcl/bigip-switch-body-list-runtime.md)).
- Given as separate `switch` words, `"a"{…}`, `{a}{…}` and `{default}{…}` do split
  and match ([bigip-switch-separate-args-word-break](../../../evidence/tcl/bigip-switch-separate-args-word-break.md)).
- `a{set r A}` never splits: it does not *start* with a brace or quote. It is
  three ordinary list elements, which explains the original
  `switch "a" {a{log …}}` failure.

### N1 is independent of `priority` and of when the code runs

N1 holds at event time on a live request. That covers a rule with a rule-level
`priority 100` and a `when HTTP_REQUEST priority 900` handler, plus a proc called
from it (`while` ⏎ `{…}` ran, `[list a b` ⏎ `{c}]` gave `a b c`)
([bigip-brace-line-continuation-event-time-priority](../../../evidence/tcl/bigip-brace-line-continuation-event-time-priority.md)).
It also applies to the rule file's own header words: `priority` ⏎ `{200}` and
`when RULE_INIT priority` ⏎ `{10}` ⏎ `{…}` both load and run
([bigip-brace-line-continuation-priority-words](../../../evidence/tcl/bigip-brace-line-continuation-priority-words.md)).

### Parser builds: TMM, mcpd, tmsh and scriptd each embed their own Tcl 8.4.6

No process links a shared `libtcl`. The 64-bit `tmm64.pgo_use` (82 MB, stripped,
6,169 dynamic symbols) and the 64-bit `mcpd` each contain their own Tcl 8.4.6
(by patchlevel string). The 32-bit `tmsh` and `scriptd` each carry their own
copy too. The host's `tclsh8.4` is a different release, 8.4.13. So "same parser
build" is false at binary level, and grammar agreement between contexts is
per-case agreement between separate copies. The load-time verdict comes from
**mcpd**, not TMM ([§14](#14-how-the-rule-loader-decides)), so a load result
and a runtime result can disagree. `binary scan … cu` is accepted at load and
fails at runtime with `bad field specifier "u"`.

### Event context: the full matrix

The checker's command table enumerates F5's command namespace from the other
direction. It defines **984** iRule commands. The appliance's own
`/usr/local/www/docs/irule/irule-schema.json` documents 933 commands and 158
events. Of the 984, 931 are documented; 53 are recognised but undocumented
(`LB::prime`, `LB::select`, `REST::send`, `DIAG::test`, `tcpdump`, `md4`, …).
`ACCESS::flowid` and `listen` are documented but absent from the checker. The
command registry already names 983 of the 984; `radius_avp` is the gap.

Validity was measured for every documented command in every documented event,
158 events × 931 commands, 147,098 cells. 84,736 are valid, 60,852 are refused at
load and 1,510 load with only a JET warning. The rules held 4 commands each. The
772 rules whose error text hit mcpd's cap were re-run one command per rule, so no
cell is unknown. The method and caveats are in
[`evmatrix.py`](../../../scripts/dev/bigip-probes/open-questions/evmatrix.py); the
result is [`results/21.1-event-matrix.json`](../../../scripts/dev/bigip-probes/results/21.1-event-matrix.json).
Each cell is the loader's verdict for the bare command name: `invalid` (load
error), `invalid-warn` (the rule loads with a "not caught before" warning) or
`valid`. A bare name is checked for event context even when its arguments are
wrong, so the matrix covers the context check only, not argument validity.
`relate_client` and `relate_server` are excluded: given bare, they crash the
checker itself (`incr len -1` on an empty length in `checkBracesBlock`), which
aborts the whole rule check.

The 14 command rows of the §8 table are reproduced cell for cell, with one change since
2026-08-26: `HTTP::uri`, `HTTP::status`, `HTTP::respond` and `HTTP::collect` in
`RULE_INIT` now load with `command is not valid in RULE_INIT` as a **warning**
(they were recorded as accepted).

### `RULE_INIT` runtime for the six load-accepted flow commands

Executed in `RULE_INIT` under `catch`, all six fail at runtime. `HTTP::uri`,
`HTTP::status` and `HTTP::collect` raise `Invalid context`; `HTTP::respond`,
`TCP::collect` and `TCP::payload` raise `Operation not supported`. `errorCode`
is `NONE` for all of them, and the handler continues after the catch
([bigip-rule-init-flow-commands-runtime](../../../evidence/tcl/bigip-rule-init-flow-commands-runtime.md)).
The `flow: true` requirement is right. Load acceptance in `RULE_INIT` never
means usable.

### tmsh: `cli version` scope and restricted roles

- **`tmsh::modify cli version active` is script-scoped.** Inside a cli script it
  selects the schema for the rest of that script. The caller's version is
  restored when the script returns. In an interactive or `-c` session it persists
  for that tmsh process only: child processes, new processes and other sessions
  never see it, and it is not saved
  ([bigip-tmsh-cli-version-scope](../../../evidence/tcl/bigip-tmsh-cli-version-scope.md)).
  `TmshSyntaxTransition::scope_is_measured` can become `true` with script scope.
  F5's own 11.5.0/11.6.0 example versions are refused on 21.1, which accepts
  14.0.0 to 21.1.0.1.
- **The role changes the cli-script interpreter, not just tmsh authorisation.**
  For guest, operator, manager and resource-admin, `exec` does not exist
  (`invalid command name "exec"`); for root and admin it does (95 commands).
  Guest and operator get a syntax-level `"rule" unexpected argument` for
  `create ltm rule`, so the command is hidden, while `delete` reports an
  authorisation error. `run util bash` is a syntax error for all four roles
  ([bigip-tmsh-cli-script-role-command-surface](../../../evidence/tcl/bigip-tmsh-cli-script-role-command-surface.md)).
  A role must be applied through a real login: `su` from root keeps root's access.

### Signed integers reaching F5 commands

| Question | Measured on 21.1.0.1 |
| --- | --- |
| TMM `binary scan` vs stock 8.4 | Same signed fields (`c` of `\x81` is -127; `S` of `\x80\x00` is -32768). The table's original probe `\x81\x00\x80 cS` reads `S` from `00 80`, which is 128, not -32768. The `u` flag loads but fails at runtime (`bad field specifier "u"`). `scan ffffffff %x` is -1. **`scan ffffffff %lx` is -1 in TMM but 4294967295 in the host's tclsh 8.4.13 and 8.5.13** ([TMM](../../../evidence/tcl/bigip-tmm-signed-scan-and-format.md), [host control](../../../evidence/tcl/bigip-host-tclsh-signed-scan-and-format.md)) |
| TMM `format` width | 64-bit, like 8.4 on a 64-bit host: `format %02x -127` is `ffffffffffffff81` and `%u` is `18446744073709551489`. `format %c -127` gives a two-character string in TMM (one in host tclsh). Integers are 64-bit: `1 << 63` is INT64_MIN and `1 << 64` is 1 (shift count taken mod 64) |
| Lengths and offsets of -1 | `TCP::collect -1`: load warning (JET checker); at runtime CLIENT_DATA fires with all 8 bytes, as with no length. `TCP::release -1`: warning; releases everything (returns 8, payload 0 after). `TCP::payload 0 -1` and `UDP::payload 0 -1`: warning; the whole payload. `TCP::payload -1 4` and `UDP::payload -1 4`: warning; runtime error. `TCP::payload replace -1 1 x`: **refused at load**. `HTTP::collect -1`: warning; no runtime error. `HTTP::payload -1`: warning; runtime `Neither <offset> nor <length> may be negative`. `SSL::collect -1`: warning (`out of expected range(0 - 4294967295)`); at runtime `CLIENTSSL_DATA` **never fires**, unlike `TCP::collect -1`. `SSL::payload 0 -1`: the whole payload. `SSL::payload -1 4`: runtime error. `SSL::release -1`: releases everything. `SSL::payload replace -1 1 x`: refused at load ([SSL collect](../../../evidence/tcl/bigip-ssl-collect-negative.md), [SSL payload](../../../evidence/tcl/bigip-ssl-payload-release-negative.md), [SSL replace](../../../evidence/tcl/bigip-ssl-payload-replace-negative.md); [TCP](../../../evidence/tcl/bigip-tcp-payload-release-negative.md), [collect](../../../evidence/tcl/bigip-tcp-collect-negative.md), [replace](../../../evidence/tcl/bigip-tcp-payload-replace-negative-refused.md), [UDP](../../../evidence/tcl/bigip-udp-payload-negative.md), [HTTP](../../../evidence/tcl/bigip-http-collect-and-payload-negative.md)) |
| `IP::addr` with signed octets | A literal `192.-64.-1.1` is **refused at load** (`expected syntax spec: (IP_ADDR|parse)`) ([evidence](../../../evidence/tcl/bigip-ip-addr-literal-signed-octet-refused.md)). Built at runtime, it raises `bad IP network address format` / `bad IP address format` |
| Integer data group with a signed key | A group holding 129: `class match -- -127 equals dg` is 0, the masked key matches. **`class match -127 …` without `--` is an error** (`bad option "-127"`); `class lookup -127 dg` returns empty with no error ([evidence](../../../evidence/tcl/bigip-class-match-signed-key.md)) |
| Counters and ports | `table incr k -32768` from 0 stores and returns **18446744073709518848**: the table is unsigned 64-bit and wraps. `incr k -1` from 5 gives 4. `node 10.0.0.1 -1` (`invalid IP port:-1`) and `pool p member 10.0.0.1 -1` are refused at load ([table](../../../evidence/tcl/bigip-table-incr-negative.md), [ports](../../../evidence/tcl/bigip-node-and-pool-member-negative-port-refused.md)) |

These give W156 measured argument domains. Ports, `IP::addr` octets and the
`TCP::payload replace` offset are non-negative at load. TMM offsets and lengths
are non-negative at runtime, where -1 for a length means "all". `table` values are
unsigned 64-bit. A negative leading argument to `class match` needs `--`.

### How much data a `*::collect` delivers

[`collect.py`](../../../scripts/dev/bigip-probes/open-questions/collect.py)
drove `TCP::collect`, `SSL::collect` and `HTTP::collect` with no length, 0, 1,
4, 100, 1460, 1500, 3000, 65536 (and 1048576 for HTTP), each fired once or
re-armed after every release. Client write patterns ran from 1 byte to 70,000
bytes, including back-to-back and gapped writes, against a real backend sink.
The per-trial results are in
[`results/21.1-collect-matrix.json`](../../../scripts/dev/bigip-probes/results/21.1-collect-matrix.json).

- **`TCP::collect` (no length), `0` and `1`** fire `CLIENT_DATA` for whatever
  has arrived: one segment (1448 bytes here), or several if they arrived
  together (2896, 8688). `TCP::collect 0` loads with a JET warning but behaves
  like no length.
- **`TCP::collect N`** is a *minimum*. The event fires once at least N bytes
  are buffered and delivers everything buffered at that moment, which can be
  far more than N (`TCP::collect 1` with 10,000 bytes in flight gave 2896;
  `TCP::collect 65536` with 70,000 gave 69,504). If fewer than N bytes ever
  arrive, it fires **when the client closes**, with what it has: `TCP::collect 100`
  with 1 byte held open fired at 5,508 ms, as the client closed at about 5.5 s
  ([evidence](../../../evidence/tcl/bigip-tcp-collect-short-data-fires-at-close.md)).
  Re-armed, it fires once more at close with 0 bytes. Three 100-byte writes
  300 ms apart under `TCP::collect 1460` gave one 300-byte event at close.
- **`SSL::collect N`** fires once at least N *decrypted* bytes are buffered and
  delivers whole TLS records (at most 16,384 bytes each; `SSL::collect 65536`
  with 70,000 bytes gave exactly 65,536). If fewer than N bytes arrive it
  **never fires**, not even at close, though the data still reaches the server
  ([evidence](../../../evidence/tcl/bigip-ssl-collect-short-data-never-fires.md)).
  With no length, it fires per record.
- **`HTTP::collect N`** fires when the whole body has arrived if its length is
  under N, and otherwise once at least N body bytes are buffered (`1` with a
  70,000-byte body gave 2896). A body shorter than its `Content-Length` fires
  at client close with the partial body. A request with no body never raises
  `HTTP_REQUEST_DATA` and goes straight to the server. With a chunked body,
  `HTTP::payload` holds the **raw chunk framing** (106 bytes for one 100-byte
  chunk). A length above the body waits for the terminating chunk (217 bytes)
  ([evidence](../../../evidence/tcl/bigip-http-collect-length-and-short-body.md)).
- **`HTTP::collect` with no length never fires `HTTP_REQUEST_DATA` by itself**,
  even with the complete body present. The connection stays in TMM's table
  after the client has gone, until idle timeout, exactly as F5's description
  warns ([evidence](../../../evidence/tcl/bigip-http-collect-without-length-stalls.md)).
- Re-arming inside `HTTP_REQUEST_DATA` with `HTTP::collect 0` or
  `HTTP::collect 1048576` raised a runtime `ERR_ARG` (TCL error in the log).
  Other re-arm lengths were not checked for this.
- In every fire-once TCP and SSL run, all data after the first release reached
  the server; nothing was lost by not re-arming.

## 13. Still open

- **Other BIG-IP releases.** Everything here is 21.1.0.1 build 0.0.26. The
  extraction in [§14](#14-how-the-rule-loader-decides) makes a cheap per-release
  diff of the checker possible; behaviour still needs a run per release.
- **APL presentation and its Tcl callbacks** remain unmeasured as separate
  contexts.
- **Server-side collection** (`TCP::collect` in `SERVER_CONNECTED`,
  `SERVERSSL_DATA`, `HTTP_RESPONSE_DATA`), `TCP::collect`'s `<skip_bytes>`
  argument, and collect behaviour under client-side TCP options other than
  `TCP_NODELAY` were not exercised.
- **E4 provenance for §5–§9.** The new probes check absence first, clean up, and
  attach only to dedicated virtual servers. They do not use the full E4 driver
  (`lib/e4-context-probe.sh`), so they carry the same non-E4 qualification as
  the earlier traffic lab.
- Load-time verdicts are mcpd's. The event matrix is a statement about the
  checker, not about what TMM would do if a refused rule were somehow installed.

## 14. How the rule loader decides

mcpd validates every iRule with a **Tcl program**, derived from the Ajuba/TclPro
checker ("Copyright (c) 1998-2000 Ajuba Solutions", `package provide tclCheck 1.0`),
loaded from `libtclCheck_tcl.so`. Its sources are embedded as 17 exported
`*_tcl` data symbols (`checker_tcl`, `jet_tcl`, `f5Cmds_tcl`, `jet_event_tcl`,
`context_tcl`, …), about 61,000 lines, each XORed with the single byte `0xF5`.
[`extract-tclcheck.py`](../../../scripts/dev/bigip-probes/open-questions/extract-tclcheck.py)
recovers them. The output is F5 proprietary source: the 21.1.0.1 copy is kept
outside this repository (`~/src/bigip-extract/tclcheck-21.1.0.1-0.0.26/`) and must
not be committed here.

What it explains:

- **Two checkers run.** The legacy one (`f5Cmds_tcl`: `checkEvent`,
  `checkSimpleArgs`) and the newer JET one (`jet_tcl`'s `bigip_rule_syntax`: a
  full argument grammar per command, with `jet_event_profile_constraint`
  allow/deny lists over events, event categories and profiles). When JET rejects
  a command and the legacy checker accepts it, mcpd logs `warnOldCheckerFlaw`
  ("The following errors were not caught before…") as a **warning**, and the rule
  loads. That is the whole WARN-versus-REJECT split for argument and event
  errors in §5–§8 and above.
- **Event validity is declarative.** Events map to categories in
  `jet_event_tcl`'s `event_table` (for example `WS_REQUEST → {IP TCP HTTP WEBSOCKET}`),
  and each command lists `allow`/`deny` over events or categories, optionally per
  profile. `HTTP::uri` is
  `{allow HTTP} {deny HTTP_RESPONSE HTTP_RESPONSE_CONTINUE HTTP_RESPONSE_DATA HTTP_RESPONSE_RELEASE}`
  for the HTTP profile, plus explicit events for FASTHTTP, ICAP and HTTPROUTER.
- **The load message is capped.** mcpd stops the error part of its message
  around 1 KB, so a rule with many errors shows only the first few. Probe drivers
  that read load errors must keep each rule small. `evmatrix.py` detects the cut
  and re-runs truncated chunks one command at a time.
- The checker's Tcl itself uses `starts_with`, so mcpd's embedded interpreter
  has the F5 operators too. The R/N lexical rules and the expression grammar are
  in the C parser, not in these scripts.

The remaining resolution-specific identity, callable replacement and deletion
boundaries have exact probes in
[FOLLOWUP_APPLIANCE_CHECKS.md](../../../scripts/dev/bigip-probes/resolution-2286/FOLLOWUP_APPLIANCE_CHECKS.md).
They do not imply general Unicode name acceptance, counted-NUL key identity or
cross-worker state sharing.
