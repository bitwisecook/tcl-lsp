# Tcl counted-source execution evidence

## Question

> The inventory result of a rooted written head remains sensitive to the actual caller context. Root execution and explicit Jim -nons source controls need independent measured option, arity and command-name values, including names containing a literal space.

> How do original info commands option and reported-name results differ between actual root execution and explicit -nons inside a nonroot namespace, including exact -all and caller-owned names containing a space?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Eight unchanged original ASCII LF sources retain genuine root versus nonroot activation, explicit original -nons words, caller-owned global/local commands and a literal-space command name. Counted C API evaluation introduces no outer source envelope. All exact inputs/requests/driver/launcher/ELFs/whole streams/receipts and independently required provider/archive/header/build/source pins are retained, with full Jim source and exact excerpt as separate source inspection. The two nonroot question records retain their own limited public forwarding answers. BIG-IP and executed software tests are not tested. Linked Runtime and VM controls execute the unchanged source fixtures and compare all138 whole ORIGINAL completion/result windows across the four inventory/helper groups per backend, with private identity and META/version excluded from the behavioural result. Their linked status records source coverage, not an executed software pass. This policy control grants no selected operation success, original handler table, Native entry or inventory execution outcome. Whole original inventory comparisons retain independent request and executable provenance. The shared body has138 public code/result comparisons across23 unchanged originals, but each question owns only its matching fixture group. It selects an actual software core/compiler and explicitly installs NamespaceEnsemble; this does not infer a full distribution, current helper roster, token/cache identity, native opcode or entered external handler. It launches no new original process and supplies no assertion outcome. Existing independent original channels remain unchanged.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
proc ::r2286_global {} {return GLOBAL}
proc {::r2286 spaced} {} {return SPACE}
namespace eval ::r2286_holder {
    proc r2286_local {} {return LOCAL}
}
set code [catch {::info commands r2286*} result]
if {$code} {list $code $result} else {list $code [lsort $result]}
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|30207b7b7232323836207370616365647d2072323238365f676c6f62616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 {{r2286 spaced} r2286_global}⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|30207b7b7232323836207370616365647d2072323238365f676c6f62616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 {{r2286 spaced} r2286_global}⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|30207b7b7232323836207370616365647d2072323238365f676c6f62616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 {{r2286 spaced} r2286_global}⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|30207b7b7232323836207370616365647d2072323238365f676c6f62616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 {{r2286 spaced} r2286_global}⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|30207b7b7232323836207370616365647d2072323238365f676c6f62616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 {{r2286 spaced} r2286_global}⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|30207b3a3a72323238365f676c6f62616c2072323238365f686f6c6465723a3a72323238365f6c6f63616c7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 {::r2286_global r2286_holder::r2286_local}⟦EOF⟧
```
