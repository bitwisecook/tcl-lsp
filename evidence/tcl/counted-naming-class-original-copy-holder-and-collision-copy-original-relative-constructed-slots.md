# Tcl counted-source execution evidence

## Question

> An original OO copy must be reached before destination-call and collision results can be measured. The source object has its own explicit name ::r2286_original_oo_source, distinct from the stock source command.

> What original relative destination-call and occupied-destination results follow TclOO copy of the explicit source object into p in current a: and :p in current a?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Two unchanged original ASCII LF sources use the explicit original source object name and retain actual relative destination/source calls and caught collision. Fresh full original provider initialisation and counted C API evaluation preserve complete original code/result bytes. All sources/requests/driver/launcher/ELFs/whole streams/receipts and independently required provider/archive/header/build/source pins are retained. Whole pinned C holder/name/publication source is independent inspection; other originals and failed setup outcomes cannot supply these reached copy observations. BIG-IP and Rust assertions are not tested. The Runtime original-source comparison explicitly installs the shared NativeScriptedLibrary::ALL distribution for its selected Jim full-initialisation source ingress, matching the original driver's full core/static-extension setup. This conformance bootstrap cannot turn a core-only constructor, authoring profile or metadata source carrier into a loaded library, current helper binding or Native worker. The exact original sources, provider rows and completion boundaries remain unchanged; the linked assertion is not a claimed executed pass.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {![llength [info commands ::oo::object]]} {
    list NOT_APPLICABLE
} else {
    ::oo::object create ::r2286_original_oo_source
    ::oo::objdefine ::r2286_original_oo_source method marker {} {return SOURCE}
    namespace eval a: {
        set ::first [::oo::copy ::r2286_original_oo_source p]
        ::oo::objdefine p method marker {} {return FIRST}
    }
    namespace eval a {
        set ::second [::oo::copy ::r2286_original_oo_source :p]
        ::oo::objdefine :p method marker {} {return SECOND}
    }
    list [::r2286_original_oo_source marker] $::first $::second [namespace eval a: {p marker}] [namespace eval a {:p marker}] [namespace eval a: {info object isa object p}] [namespace eval a {info object isa object :p}]
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
ORIGINAL|0|534f55524345203a3a613a3a3a70203a3a613a3a3a70204649525354205345434f4e4420312031
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
SOURCE ::a:::p ::a:::p FIRST SECOND 1 1⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|534f55524345203a3a613a3a3a70203a3a613a3a3a70204649525354205345434f4e4420312031
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
SOURCE ::a:::p ::a:::p FIRST SECOND 1 1⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|534f55524345203a3a613a3a3a70203a3a613a3a3a70204649525354205345434f4e4420312031
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
SOURCE ::a:::p ::a:::p FIRST SECOND 1 1⟦EOF⟧
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
