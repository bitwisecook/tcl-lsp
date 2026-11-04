# Tcl counted-source execution evidence

## Question

> Jim alias prefix queries and inventories depend on exact selected info words and the actual namespace forwarding helper. Whole public query and error results must distinguish stored-prefix queries from flat, all-mode and namespace-forwarded inventory calls.

> What whole prefix values, missing/non-alias errors and inventory results do four original sources produce for current Jim info alias and exact info aliases -all/-nons words, including actual namespace forwarding?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Four unchanged ASCII LF source windows independently evaluated in fresh fully initialised interpreters under the counted-source driver. All six provider/compile/executable/archive/header/source pins and entire streams/receipts are retained, including actual namespace helper errors and the explicit unsupported C branch. Whole pinned jim.c, jim-namespace.c and nshelper.tcl copies are separate source artifacts, not a claim about returned loaded procedure-body bytes or arbitrary helper currency. BIG-IP, native command-table identity and executed Rust controls are not tested. Runtime and VM linked controls compare24 original completion codes and23 whole original results each, including the twenty explicit unavailable C branches. For the one Jim namespace source, the helper failure code/message and explicit -nons completion remain exact first-three fields; only the final alias inventory field is compared as an alias-member multiset. The complete raw observed field retains its original order. This bounded comparison does not assert a matching native hash iteration order, table seed/capacity or object/header ownership. The whole original captured streams remain unchanged. Sorting that one member field does not establish original alias hash order, capacity, seed, private prefix object/cache identity or complete linked-source roster. Explicit selected scripted libraries are fixture premises, not authority from catalogue labels. No original process is rerun, passing assertion or native identity is attached.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {[catch {info -commands} r2286_selectors] || [lsearch -exact $r2286_selectors alias] < 0} {
    set r2286_result NOT_APPLICABLE
} else {
    proc r2286_proc {} {return PROC}
    alias ::r2286_alias_a list A
    alias {::r2286 spaced alias} list SPACE
    alias r2286_holder::r2286_alias_b list B
    list [lsort [info aliases r2286*]] [lsort [info aliases -all r2286*]] [lsort [info aliases -all r2286_holder::*]]
}
```

## Results

### jim, run 1

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|7b3a3a72323238365f616c6961735f612072323238365f686f6c6465723a3a72323238365f616c6961735f627d207b7b3a3a72323238362073706163656420616c6961737d203a3a72323238365f616c6961735f612072323238365f686f6c6465723a3a72323238365f616c6961735f627d2072323238365f686f6c6465723a3a72323238365f616c6961735f62
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{::r2286_alias_a r2286_holder::r2286_alias_b} {{::r2286 spaced alias} ::r2286_alias_a r2286_holder::r2286_alias_b} r2286_holder::r2286_alias_b⟦EOF⟧
```
