# Tcl counted-source execution evidence

## Question

> An explicitly rooted ::info head inside a nonroot namespace does not by itself establish that a direct-core inventory handler is selected. Public option and reported-name results need to retain the actual original caller context.

> What public option/arity and caller-owned inventory results does the original rooted ::info head produce inside the actual nonroot namespace for absent, pattern, -all, -al and extra-operand forms?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Six untouched rooted-head ASCII LF sources retain the actual nonroot namespace activation. Whole public completions, source/request/driver/launcher bytes, ELFs, receipts/streams and required provider pins remain independently inspectable. Literal rooting is a measured spelling difference, not an actual handler/frame receipt. Root/explicit-nons controls have a distinct complete question; BIG-IP and executed software assertions are not tested. Linked Runtime and VM controls execute the unchanged source fixtures and compare all138 whole ORIGINAL completion/result windows across the four inventory/helper groups per backend, with private identity and META/version excluded from the behavioural result. Their linked status records source coverage, not an executed software pass. The shared body has138 public code/result comparisons across23 unchanged originals, but each question owns only its matching fixture group. It selects an actual software core/compiler and explicitly installs NamespaceEnsemble; this does not infer a full distribution, current helper roster, token/cache identity, native opcode or entered external handler. It launches no new original process and supplies no assertion outcome. Existing independent original channels remain unchanged.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
proc ::r2286_global {} {return GLOBAL}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
    set code [catch {::info commands -all} result]
    set selected {}
    if {$code == 0} {foreach name $result {if {[string match r2286* $name]} {lappend selected $name}}}
    list $code [lsort $selected]
}
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|30207b7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 {}⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|30207b7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 {}⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|30207b7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 {}⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|30207b7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 {}⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|30207b7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 {}⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|302072323238365f686f6c6465723a3a72323238365f6c6f63616c
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 r2286_holder::r2286_local⟦EOF⟧
```
