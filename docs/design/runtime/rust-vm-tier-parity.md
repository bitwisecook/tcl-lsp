# Rust bytecode VM — Tier 1/2/3 tcltest parity scoreboard

_Snapshot 2026-06-23 — regenerate with `TCL_LIBRARY=tmp/tcl9.0.3/library python scripts/dev/rust_vm_tier_gap.py --json tmp/g.json`._

Goal: the VM's (passed/skipped/failed) per stem EXACTLY matches C Tcl 9 (`tests/baselines/tcl9-tcltest/c-tclsh.ndjson`; `*` = captured live from tclsh9.0).

`CRASH` = an uncaught error/timeout aborts the file (highest leverage — one fix unlocks it). `no-ref` = no C baseline captured. Columns: **C P/S/F** vs **VM P/S/F**.

**Tally: 19 MATCH · 53 gap · 22 crash · 3 no-ref** of 97 stems.


## Tier 1

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| abstractlist | 0/123/0 | 0/123/0 | MATCH |
| append | 49/3/0 | 44/3/5 | gap |
| appendComp | 43/5/0 | 38/5/5 | gap |
| apply | 38/4/0 | 18/4/20 | gap |
| assemble | (no ref) | ERROR | CRASH |
| assocd | 0/11/0 | 0/11/0 | MATCH |
| basic | 69/77/0 | 43/78/25 | gap |
| cmdAH | 16820/181/0 | ERROR | CRASH |
| cmdIL | 163/5/0 | ERROR | CRASH |
| cmdInfo | 0/12/0 | 0/12/0 | MATCH |
| cmdMZ | 96/1/0 | 40/3/54 | gap |
| compExpr | 80/2/0 | 64/2/16 | gap |
| compExpr-old | 183/1/0 | 122/2/60 | gap |
| compile | 138/33/0 | 61/33/77 | gap |
| concat | 9/0/0 | 9/0/0 | MATCH |
| dict | 367/6/0 | 282/6/85 | gap |
| dstring | 0/46/0 | 0/46/0 | MATCH |
| error | 309/8/0 | 278/8/31 | gap |
| eval | 12/0/0 | 11/0/1 | gap |
| execute | 79/78/0 | 49/78/30 | gap |
| expr | 2137/31/0 | ERROR | CRASH |
| expr-old | 430/31/0 | 328/31/102 | gap |
| for | 64/24/0 | 57/24/7 | gap |
| for-old | 9/0/0 | 8/0/1 | gap |
| foreach | 43/0/0 | 37/0/6 | gap |
| format | 269/0/0 | 174/19/76 | gap |
| get | 6/17/0 | 4/17/2 | gap |
| if | 73/0/0 | 66/0/7 | gap |
| if-old | 33/0/0 | 32/0/1 | gap |
| incr | 69/0/0 | 61/0/8 | gap |
| incr-old | 14/0/0 | 10/0/4 | gap |
| indexObj | 0/65/0 | ERROR | CRASH |
| info | 282/5/0 | 83/6/198 | gap |
| join | 10/0/0 | 10/0/0 | MATCH |
| lindex | 47/37/0 | 44/37/3 | gap |
| linsert | 28/0/0 | 25/0/3 | gap |
| list | 78/0/0 | 78/0/0 | MATCH |
| listObj | 42/17/0 | 42/17/0 | MATCH |
| listRep | 4/227/0 | 4/227/0 | MATCH |
| llength | 6/0/0 | 6/0/0 | MATCH |
| lmap | 66/0/0 | 63/0/3 | gap |
| lpop | 17/2/0 | 17/2/0 | MATCH |
| lrange | 1764/2/0 | 1759/2/5 | gap |
| lrepeat | 11/1/0 | 11/1/0 | MATCH |
| lreplace | 3579/0/0 | 3579/0/0 | MATCH |
| lsearch | 165/0/0 | 163/0/2 | gap |
| lseq | 132/2/0 | 95/3/36 | gap |
| lset | 0/89/0 | 0/89/0 | MATCH |
| lsetComp | 19/0/0 | 19/0/0 | MATCH |
| mathop | 385/0/0 | ERROR | CRASH |
| misc | 2/299/0 | 1/299/1 | gap |
| namespace | 311/3/0 | 132/3/179 | gap |
| namespace-old | 126/0/0 | 99/0/27 | gap |
| nre | 5/23/0 | 2/23/3 | gap |
| obj | (no ref) | ERROR | CRASH |
| parse | 90/181/0 | 60/181/30 | gap |
| parseExpr | 67/219/0 | 3/219/64 | gap |
| parseOld | 158/0/0 | 148/0/10 | gap |
| proc | 29/9/0 | TIMEOUT | CRASH |
| proc-old | 74/0/0 | 57/0/17 | gap |
| regexp | 253/4/0 | 231/5/21 | gap |
| regexpComp | 179/0/0 | 167/1/11 | gap |
| rename | 11/8/0 | ERROR | CRASH |
| result | 4/22/0 | 0/22/4 | gap |
| scan | 184/1/0 | 125/1/59 | gap |
| set | 63/1/0 | 63/1/0 | MATCH |
| set-old | 153/0/0 | 88/0/65 | gap |
| split | 18/0/0 | 18/0/0 | MATCH |
| string | 693/12/0 | 670/12/23 | gap |
| stringObj | 0/81/0 | 0/81/0 | MATCH |
| subst | 62/1/0 | 23/1/39 | gap |
| switch | 113/0/0 | 99/0/14 | gap |
| trace | 273/17/0 | 80/17/193 | gap |
| unknown | 7/0/0 | 7/0/0 | MATCH |
| uplevel | 57/0/0 | 33/0/24 | gap |
| upvar | 62/8/0 | 28/8/34 | gap |
| util | (no ref) | 133/152/177 | no-ref |
| var | 198/21/0 | 58/21/140 | gap |
| while | 46/0/0 | 45/0/1 | gap |
| while-old | 15/0/0 | 14/0/1 | gap |
| word | 55/0/0 | 0/0/55 | gap |

## Tier 2

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| binary | (no ref) | 578/91/81 | no-ref |
| coroutine | 74/3/0 | 0/3/74 | gap |
| interp | 340/14/0 | ERROR | CRASH |
| oo | 372/16/0 | ERROR | CRASH |
| ooNext2 | 57/5/0 | ERROR | CRASH |
| ooProp | 55/0/0 | ERROR | CRASH |
| ooUtil | 33/0/0 | ERROR | CRASH |
| tailcall | 29/8/0 | ERROR | CRASH |

## Tier 3

| stem | C P/S/F | VM P/S/F | status |
|---|---|---|---|
| clock | (no ref) | ERROR | CRASH |
| encoding | (no ref) | ERROR | CRASH |
| msgcat | (no ref) | ERROR | CRASH |
| safe | 147/8/0 | ERROR | CRASH |
| safe-stock | 11/0/0 | ERROR | CRASH |
| safe-stock86 | 0/0/0 | ERROR | CRASH |
| safe-zipfs | (no ref) | ERROR | CRASH |
| utf | (no ref) | 133/251/15 | no-ref |
