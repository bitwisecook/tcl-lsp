# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Rust bytecode VM — tcltest parity scoreboard

_Regenerate with `cargo xtask tcltest-sweep --backend both` (Tcl 9.0.4)._

Goal: the VM's (passed/skipped/failed) per stem matches C Tcl 9 exactly.
`CRASH` = an uncaught error / timeout aborts the file (highest leverage — one
fix unlocks it). `gap` = ran but the counts differ. Columns: **C P/S/F** vs
**VM P/S/F**. Grouped by the capability ladder
([`tcl-test-tiers.md`](tcl-test-tiers.md)).

**Tally: 29 MATCH · 63 gap · 7 crash · 2 timeout** of 101 stems.

## Tier 1 — Parsing

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| parse | 90/181/0 | 68/181/22 | gap |
| parseOld | 158/0/0 | 151/0/7 | gap |
| parseExpr | 67/219/0 | 3/219/64 | gap |
| word | 55/0/0 | 55/0/0 | MATCH |
| subst | 62/1/0 | 54/1/8 | gap |

## Tier 2 — Interpretation

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| basic | 70/77/0 | 50/78/19 | gap |
| compile | 138/33/0 | 64/33/74 | gap |
| execute | 79/78/0 | 64/78/15 | gap |
| eval | 12/0/0 | 12/0/0 | MATCH |
| obj | 8/76/0 | 8/76/0 | MATCH |
| nre | 5/23/0 | 3/23/2 | gap |
| appendComp | 43/5/0 | 40/5/3 | gap |
| lsetComp | 19/0/0 | 19/0/0 | MATCH |
| regexpComp | 179/0/0 | 167/1/11 | gap |
| compExpr | 80/2/0 | 71/2/9 | gap |
| compExpr-old | 183/1/0 | 140/2/42 | gap |
| misc | 2/299/0 | 1/299/1 | gap |

## Tier 3 — Fundamentals

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| var | 200/21/0 | 127/21/73 | gap |
| set | 63/1/0 | 63/1/0 | MATCH |
| set-old | 153/0/0 | 91/0/62 | gap |
| append | 49/3/0 | 45/3/4 | gap |
| incr | 69/0/0 | 61/0/8 | gap |
| incr-old | 14/0/0 | 11/0/3 | gap |
| upvar | 62/8/0 | 29/8/33 | gap |
| uplevel | 57/0/0 | 54/0/3 | gap |
| get | 6/17/0 | 6/17/0 | MATCH |
| namespace | 311/3/0 | 171/3/140 | gap |
| namespace-old | 126/0/0 | 102/0/24 | gap |
| resolver | 0/10/0 | 0/10/0 | MATCH |
| trace | 273/17/0 | 121/17/152 | gap |
| rename | 11/8/0 | 10/8/1 | gap |
| info | 282/5/0 | 106/6/175 | gap |
| cmdInfo | 0/12/0 | 0/12/0 | MATCH |
| indexObj | 0/65/0 | 0/65/0 | MATCH |
| chan | 42/0/0 | 0/0/42 | gap |
| io | 480/404/0 | 122/527/235 | gap |

## Tier 4 — Data types

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| abstractlist | 0/123/0 | 0/123/0 | MATCH |
| assocd | 0/11/0 | 0/11/0 | MATCH |
| concat | 9/0/0 | 9/0/0 | MATCH |
| dict | 367/6/0 | 324/6/43 | gap |
| dstring | 0/46/0 | 0/46/0 | MATCH |
| format | 269/0/0 | 239/0/30 | gap |
| join | 10/0/0 | 10/0/0 | MATCH |
| lindex | 47/37/0 | 46/37/1 | gap |
| linsert | 28/0/0 | 28/0/0 | MATCH |
| list | 78/0/0 | 78/0/0 | MATCH |
| listObj | 42/17/0 | 42/17/0 | MATCH |
| listRep | 4/227/0 | 4/227/0 | MATCH |
| llength | 6/0/0 | 6/0/0 | MATCH |
| lmap | 66/0/0 | 55/0/11 | gap |
| lpop | 17/2/0 | 17/2/0 | MATCH |
| lrange | 1764/2/0 | 1760/2/4 | gap |
| lrepeat | 11/1/0 | 11/1/0 | MATCH |
| lreplace | 3579/0/0 | 3579/0/0 | MATCH |
| lsearch | 165/0/0 | 165/0/0 | MATCH |
| lseq | 131/5/0 | 90/6/40 | gap |
| lset | 0/89/0 | 0/89/0 | MATCH |
| range | ERROR | ERROR | CRASH |
| reg | 34/1107/0 | 32/1107/2 | gap |
| regexp | 253/4/0 | 231/5/21 | gap |
| scan | 184/1/0 | 169/1/15 | gap |
| split | 18/0/0 | 18/0/0 | MATCH |
| stack | 3/0/0 | 0/0/3 | gap |
| string | 693/12/0 | 677/12/16 | gap |
| stringObj | 0/81/0 | 0/81/0 | MATCH |
| cmdIL | 163/5/0 | 132/7/29 | gap |
| cmdMZ | 96/1/0 | 54/3/40 | gap |

## Tier 5 — Control flow & procs

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| apply | 38/4/0 | 28/4/10 | gap |
| error | 309/8/0 | 301/8/8 | gap |
| expr | 2137/31/0 | TIMEOUT | TIMEOUT |
| expr-old | 430/31/0 | 394/31/36 | gap |
| for | 64/24/0 | 58/24/6 | gap |
| for-old | 9/0/0 | 8/0/1 | gap |
| foreach | 43/0/0 | 37/0/6 | gap |
| if | 73/0/0 | 71/0/2 | gap |
| if-old | 33/0/0 | 33/0/0 | MATCH |
| mathop | 385/0/0 | TIMEOUT | TIMEOUT |
| proc | 29/9/0 | 26/9/3 | gap |
| proc-old | 74/0/0 | 62/0/12 | gap |
| result | 4/22/0 | 1/22/3 | gap |
| switch | 113/0/0 | 99/0/14 | gap |
| unknown | 7/0/0 | 7/0/0 | MATCH |
| while | 46/0/0 | 45/0/1 | gap |
| while-old | 15/0/0 | 15/0/0 | MATCH |

## Tier 6 — TclOO

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| oo | 372/16/0 | 49/16/323 | gap |
| ooNext2 | 57/5/0 | 2/5/55 | gap |
| ooProp | 55/0/0 | 0/0/55 | gap |
| ooUtil | 33/0/0 | 2/0/31 | gap |

## Tier 7 — Packages & loading

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| source | 23/0/0 | 10/0/13 | gap |

## Tier 8 — Interpreters

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| interp | 341/14/0 | 173/15/167 | gap |
| safe | 147/8/0 | 3/8/144 | gap |
| safe-stock | 11/0/0 | 0/0/11 | gap |
| safe-stock86 | ERROR | ERROR | CRASH |

## Tier 9 — Advanced I/O & events

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| chanio | 392/387/0 | ERROR | CRASH |
| ioCmd | 274/103/0 | ERROR | CRASH |

## Tier 10 — Concurrency

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| coroutine | 74/3/0 | 60/3/14 | gap |
| tailcall | 29/8/0 | 21/8/8 | gap |

## Tier 11 — Platform & library

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| aaa_exit | 2/0/0 | 0/0/2 | gap |
| brodnik | 0/422/0 | ERROR | CRASH |
| cmdAH | ERROR | ERROR | CRASH |
| opt | 31/0/0 | ERROR | CRASH |
