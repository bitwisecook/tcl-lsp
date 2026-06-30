# Rust bytecode VM — Tier 1/2/3 tcltest parity scoreboard

_Snapshot 2026-06-30 — regenerate with `TCL_LIBRARY=tmp/tcl9.0.3/library python scripts/dev/rust_vm_tier_gap.py --json tmp/g.json`._

Goal: the VM's (passed/skipped/failed) per stem EXACTLY matches C Tcl 9 (`tests/baselines/tcl9-tcltest/c-tclsh.ndjson`).

`CRASH` = an uncaught error/timeout aborts the file (highest leverage — one fix unlocks it). Columns: **C P/S/F** vs **VM P/S/F**.

**Tally: 28 MATCH · 58 gap · 11 crash · 0 no-ref** of 97 stems.

## Tier 1

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| parse | 90/181/0 | 69/181/21 | gap |
| parseOld | 158/0/0 | 148/0/10 | gap |
| parseExpr | 67/219/0 | 3/219/64 | gap |
| word | 55/0/0 | 55/0/0 | MATCH |
| subst | 62/1/0 | 54/1/8 | gap |
| compile | 138/33/0 | 63/33/75 | gap |
| execute | 79/78/0 | 65/78/14 | gap |
| basic | 69/77/0 | 45/78/23 | gap |
| misc | 2/299/0 | 1/299/1 | gap |
| compExpr | 80/2/0 | 71/2/9 | gap |
| compExpr-old | 183/1/0 | 138/2/44 | gap |
| appendComp | 43/5/0 | 40/5/3 | gap |
| lsetComp | 19/0/0 | 19/0/0 | MATCH |
| regexpComp | 179/0/0 | 167/1/11 | gap |
| assemble | (no ref) | ERROR | CRASH |
| obj | 8/76/0 | 8/76/0 | MATCH |
| nre | 5/23/0 | 2/23/3 | gap |
| eval | 12/0/0 | 12/0/0 | MATCH |
| if | 73/0/0 | 71/0/2 | gap |
| if-old | 33/0/0 | 33/0/0 | MATCH |
| for | 64/24/0 | 57/24/7 | gap |
| for-old | 9/0/0 | 8/0/1 | gap |
| foreach | 43/0/0 | 37/0/6 | gap |
| while | 46/0/0 | 45/0/1 | gap |
| while-old | 15/0/0 | 15/0/0 | MATCH |
| switch | 113/0/0 | 99/0/14 | gap |
| error | 309/8/0 | 296/8/13 | gap |
| result | 4/22/0 | 0/22/4 | gap |
| expr | 2137/31/0 | 1921/32/215 | gap |
| expr-old | 430/31/0 | 393/31/37 | gap |
| mathop | 385/0/0 | 350/0/35 | gap |
| incr | 69/0/0 | 61/0/8 | gap |
| incr-old | 14/0/0 | 11/0/3 | gap |
| proc | 29/9/0 | 27/9/2 | gap |
| proc-old | 74/0/0 | 61/0/13 | gap |
| apply | 38/4/0 | 26/4/12 | gap |
| uplevel | 57/0/0 | 54/0/3 | gap |
| upvar | 62/8/0 | 29/8/33 | gap |
| namespace | 311/3/0 | 149/3/162 | gap |
| namespace-old | 126/0/0 | 102/0/24 | gap |
| var | 198/21/0 | 119/21/79 | gap |
| info | 282/5/0 | 103/6/178 | gap |
| cmdInfo | 0/12/0 | 0/12/0 | MATCH |
| trace | 273/17/0 | 120/17/153 | gap |
| rename | 11/8/0 | 10/8/1 | gap |
| unknown | 7/0/0 | 7/0/0 | MATCH |
| string | 693/12/0 | 676/12/17 | gap |
| set | 63/1/0 | 63/1/0 | MATCH |
| set-old | 153/0/0 | 91/0/62 | gap |
| append | 49/3/0 | 45/3/4 | gap |
| format | 269/0/0 | 239/0/30 | gap |
| scan | 184/1/0 | 169/1/15 | gap |
| split | 18/0/0 | 18/0/0 | MATCH |
| join | 10/0/0 | 10/0/0 | MATCH |
| concat | 9/0/0 | 9/0/0 | MATCH |
| regexp | 253/4/0 | 231/5/21 | gap |
| stringObj | 0/81/0 | 0/81/0 | MATCH |
| dstring | 0/46/0 | 0/46/0 | MATCH |
| list | 78/0/0 | 78/0/0 | MATCH |
| lindex | 47/37/0 | 46/37/1 | gap |
| linsert | 28/0/0 | 28/0/0 | MATCH |
| llength | 6/0/0 | 6/0/0 | MATCH |
| lrange | 1764/2/0 | 1759/2/5 | gap |
| lrepeat | 11/1/0 | 11/1/0 | MATCH |
| lreplace | 3579/0/0 | 3579/0/0 | MATCH |
| lsearch | 165/0/0 | 165/0/0 | MATCH |
| lset | 0/89/0 | 0/89/0 | MATCH |
| lmap | 66/0/0 | 63/0/3 | gap |
| lpop | 17/2/0 | 17/2/0 | MATCH |
| lseq | 132/2/0 | 96/3/35 | gap |
| listObj | 42/17/0 | 42/17/0 | MATCH |
| listRep | 4/227/0 | 4/227/0 | MATCH |
| abstractlist | 0/123/0 | 0/123/0 | MATCH |
| cmdAH | 16820/181/0 | 589/190/16222 | gap |
| cmdIL | 163/5/0 | 132/7/29 | gap |
| cmdMZ | 96/1/0 | 49/3/45 | gap |
| util | 310/152/0 | 280/152/30 | gap |
| dict | 367/6/0 | 324/6/43 | gap |
| indexObj | 0/65/0 | 0/65/0 | MATCH |
| get | 6/17/0 | 6/17/0 | MATCH |
| assocd | 0/11/0 | 0/11/0 | MATCH |

## Tier 2

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| coroutine | 74/3/0 | 0/3/74 | gap |
| tailcall | 29/8/0 | 21/8/8 | gap |
| interp | 340/14/0 | 84/15/255 | gap |
| binary | 660/90/0 | 600/91/59 | gap |
| oo | 372/16/0 | ERROR | CRASH |
| ooNext2 | 57/5/0 | ERROR | CRASH |
| ooProp | 55/0/0 | ERROR | CRASH |
| ooUtil | 33/0/0 | ERROR | CRASH |

## Tier 3

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| encoding | (no ref) | ERROR | CRASH |
| utf | 148/251/0 | 133/251/15 | gap |
| clock | (no ref) | ERROR | CRASH |
| msgcat | (no ref) | ERROR | CRASH |
| safe | 147/8/0 | ERROR | CRASH |
| safe-stock | 11/0/0 | 0/0/11 | gap |
| safe-stock86 | 0/0/0 | ERROR | CRASH |
| safe-zipfs | (no ref) | ERROR | CRASH |

