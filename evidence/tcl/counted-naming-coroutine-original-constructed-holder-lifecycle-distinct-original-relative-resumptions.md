# Tcl counted-source execution evidence

## Question

> Coroutine reports can combine holder and simple-name colon boundaries into equal bytes. Resumption, relative deletion and relative rename need their own original public controls rather than identity inferred from displayed names.

> Can original coroutine commands created as p in current a: and :p in current a resume independently, and do relative deletion or rename of the first preserve the second despite equal coroutine reports?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Three untouched ASCII LF sources retain original current-holder activation, creation spelling, yielded reporting bytes and independent relative resume/delete/rename operations. Each runs separately in a fresh full-init counted C API process without an added source envelope. Exact originals/requests/driver/launcher/ELFs/whole output/receipts and independently required executable/archive/header/build/source pins are retained. Actual unsupported stock capability branches remain separate from supported coroutine observations. BIG-IP and Rust assertions are not tested. The source definition launches no original provider process and inspects no private native holder/token pointer, refcount or coroutine frame. Complete code/result bytes preserve availability and independent relative-resume/move/delete branches; reporting-label equality supplies no identity. No executed software outcome, compiled opcode or broader runtime admission is attached.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {![llength [info commands coroutine]]} {
    list NOT_APPLICABLE
} else {
    proc ::body {tag} {yield [list $tag [info coroutine]]; return [list DONE $tag]}
    namespace eval a: {set ::first [coroutine p ::body FIRST]}
    namespace eval a {set ::second [coroutine :p ::body SECOND]}
    set ::firstDone [namespace eval a: {p}]
    set ::secondDone [namespace eval a {:p}]
    list $::first $::second $::firstDone $::secondDone
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
ORIGINAL|0|7b4649525354203a3a613a3a3a707d207b5345434f4e44203a3a613a3a3a707d207b444f4e452046495253547d207b444f4e45205345434f4e447d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{FIRST ::a:::p} {SECOND ::a:::p} {DONE FIRST} {DONE SECOND}⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|7b4649525354203a3a613a3a3a707d207b5345434f4e44203a3a613a3a3a707d207b444f4e452046495253547d207b444f4e45205345434f4e447d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{FIRST ::a:::p} {SECOND ::a:::p} {DONE FIRST} {DONE SECOND}⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|7b4649525354203a3a613a3a3a707d207b5345434f4e44203a3a613a3a3a707d207b444f4e452046495253547d207b444f4e45205345434f4e447d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{FIRST ::a:::p} {SECOND ::a:::p} {DONE FIRST} {DONE SECOND}⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```
