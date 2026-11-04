# Tcl counted-source execution evidence

## Question

> A namespace-aware command inventory needs evidence for the currently selected mutable scripted helper, its public declaration and actual original caller/vector routing. A synthetic name list or rooted written head alone cannot supply those facts.

> What exact helper formals/body and caller/vector reports do the original namespace info declaration and replacement controls produce, and which original ordinary, rooted, root-pattern, root-all and explicit -nons calls reach the replacement?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Three unchanged ASCII LF originals run through the unchanged counted-source driver with fresh full provider initialisation. Their exact source/request/driver/launcher/ELF bytes, whole command receipts/streams and required archive/header/build/source pins are retained. The public returned formal/body tuple retains every tab, blank line and byte; the independently inspected source declaration is not substituted for that tuple. No outer catch/eval/puts envelope is added. Other helper behaviour, arbitrary Unicode/NUL, private object/cache/identity, compiler or Normal admission, BIG-IP and executed Rust assertions are not measured. Linked Runtime and VM controls execute the unchanged source fixtures and compare all138 whole ORIGINAL completion/result windows across the four inventory/helper groups per backend, with private identity and META/version excluded from the behavioural result. Their linked status records source coverage, not an executed software pass. The Registry asset control compares the entire public two-field declaration tuple through the shared native list parser. Its full909-byte body is kept as the selected NativeScriptedLibrary asset; the current mutable helper lookup remains independent from authentic Core startup and actual library installation. Broad C/other-helper/core presence is not inferred from that asset. The shared body has138 public code/result comparisons across23 unchanged originals, but each question owns only its matching fixture group. It selects an actual software core/compiler and explicitly installs NamespaceEnsemble; this does not infer a full distribution, current helper roster, token/cache identity, native opcode or entered external handler. It launches no new original process and supplies no assertion outcome. Existing independent original channels remain unchanged.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {[catch {::info body {namespace info}}]} {
    list NOT_APPLICABLE
} else {
    proc ::r2286_global {} {return GLOBAL}
    namespace eval ::r2286_holder {
        proc r2286_local {} {return LOCAL}
    }
    proc {namespace info} args {list HELPER [uplevel 1 {namespace current}] $args}
    namespace eval ::r2286_holder {
        set ordinary [info commands -all r2286*]
        set rooted [::info commands -all r2286*]
        set direct [lsort [::info -nons commands -all r2286*]]
        list $ordinary $rooted $direct
    }
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
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|7b48454c504552203a3a72323238365f686f6c646572207b636f6d6d616e6473202d616c6c2072323238362a7d7d207b48454c504552203a3a72323238365f686f6c646572207b636f6d6d616e6473202d616c6c2072323238362a7d7d207b3a3a72323238365f676c6f62616c2072323238365f686f6c6465723a3a72323238365f6c6f63616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{HELPER ::r2286_holder {commands -all r2286*}} {HELPER ::r2286_holder {commands -all r2286*}} {::r2286_global r2286_holder::r2286_local}⟦EOF⟧
```
