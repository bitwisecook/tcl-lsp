# Tcl counted-source execution evidence

## Question

> A rendered object name can combine holder and simple-name colon boundaries into equal bytes. Public reporting must be considered separately from the original relative creation and method calls.

> What returned names, namespace reports and original relative method-call results distinguish p created in current a: from :p created in current a, including two creations with equal displayed names?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Four unchanged ASCII LF sources run in separate fresh full-init counted C API processes. Three finite creation/call/query controls and one failed setup specimen retain their independent original completion/result bytes. Original requests, driver, launcher, executable/required provider build pins and whole streams/receipts are retained. The copy-labelled setup refusal cannot donate copy evidence; original-copy-holder-and-collision records independently reached copy inputs. BIG-IP and executed Rust assertions are not tested. The Runtime original-source comparison explicitly installs the shared NativeScriptedLibrary::ALL distribution for its selected Jim full-initialisation source ingress, matching the original driver's full core/static-extension setup. This conformance bootstrap cannot turn a core-only constructor, authoring profile or metadata source carrier into a loaded library, current helper binding or Native worker. The exact original sources, provider rows and completion boundaries remain unchanged; the linked assertion is not a claimed executed pass. The fixture selects an actual software core/compiler and compares public original bytes, without launching new original processes or observing private native token/header/cache identity. Constructed-holder reporting and collision results remain separate original questions. No passing software outcome, oo::copy availability, copy permission, activation/frame or compiler instruction admission is attached.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {![llength [info commands ::oo::object]]} {
    list NOT_APPLICABLE
} else {
    namespace eval a {
        set ::created [::oo::object create :p]
        ::oo::objdefine :p method marker {} {return SECOND}
        list $::created [namespace current] [info object isa object :p] [:p marker]
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
ORIGINAL|0|3a3a613a3a3a70203a3a612031205345434f4e44
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
::a:::p ::a 1 SECOND⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|3a3a613a3a3a70203a3a612031205345434f4e44
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
::a:::p ::a 1 SECOND⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|3a3a613a3a3a70203a3a612031205345434f4e44
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
::a:::p ::a 1 SECOND⟦EOF⟧
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
