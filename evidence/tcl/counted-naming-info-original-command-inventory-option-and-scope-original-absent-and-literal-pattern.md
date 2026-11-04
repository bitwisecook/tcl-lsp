# Tcl counted-source execution evidence

## Question

> A command inventory request inside a nonroot namespace can follow selected namespace forwarding rather than the root inventory handler. Option/arity and sorted caller-owned names must be measured for that actual source invocation.

> What original absent, literal-pattern, exact -all, abbreviated -al and extra-operand info commands results are observed inside the actual nonroot namespace, including caller-owned global and local command reports?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Six unchanged original ASCII LF caught/public-list controls execute independently in fresh full-init counted C API processes. Exact originals, requests, driver, launcher, ELFs, whole streams/receipts and required provider pins are retained. The runtime problem hypothesis in the raw request remains an input premise, not a proved source cause. Rooted-nonroot and root/explicit-nons inputs retain independent question records; sorted caller-owned reports grant no successful scripted helper or Native handler/compiler admission. BIG-IP and executed Rust assertions are not tested. Linked Runtime and VM controls execute the unchanged source fixtures and compare all138 whole ORIGINAL completion/result windows across the four inventory/helper groups per backend, with private identity and META/version excluded from the behavioural result. Their linked status records source coverage, not an executed software pass. The renderer definition supplies no public helper dispatch, current binding, Native object/frame identity or original-provider completion. Actual helper arity windows retain their original measured scope. The shared body has138 public code/result comparisons across23 unchanged originals, but each question owns only its matching fixture group. It selects an actual software core/compiler and explicitly installs NamespaceEnsemble; this does not infer a full distribution, current helper roster, token/cache identity, native opcode or entered external handler. It launches no new original process and supplies no assertion outcome. Existing independent original channels remain unchanged.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
proc ::r2286_global {} {return GLOBAL}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
    set selected {}
    foreach name [info commands] {if {[string match r2286* $name]} {lappend selected $name}}
    list [lsort $selected] [lsort [info commands r2286*]]
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
ORIGINAL|0|7b72323238365f676c6f62616c2072323238365f6c6f63616c7d207b72323238365f676c6f62616c2072323238365f6c6f63616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{r2286_global r2286_local} {r2286_global r2286_local}⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|7b72323238365f676c6f62616c2072323238365f6c6f63616c7d207b72323238365f676c6f62616c2072323238365f6c6f63616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{r2286_global r2286_local} {r2286_global r2286_local}⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|7b72323238365f676c6f62616c2072323238365f6c6f63616c7d207b72323238365f676c6f62616c2072323238365f6c6f63616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{r2286_global r2286_local} {r2286_global r2286_local}⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|7b72323238365f676c6f62616c2072323238365f6c6f63616c7d207b72323238365f676c6f62616c2072323238365f6c6f63616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{r2286_global r2286_local} {r2286_global r2286_local}⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|7b72323238365f676c6f62616c2072323238365f6c6f63616c7d207b72323238365f676c6f62616c2072323238365f6c6f63616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{r2286_global r2286_local} {r2286_global r2286_local}⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|7b72323238365f686f6c6465723a3a72323238365f6c6f63616c2072323238365f6c6f63616c7d207b3a3a72323238365f676c6f62616c2072323238365f686f6c6465723a3a72323238365f6c6f63616c2072323238365f6c6f63616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{r2286_holder::r2286_local r2286_local} {::r2286_global r2286_holder::r2286_local r2286_local}⟦EOF⟧
```
