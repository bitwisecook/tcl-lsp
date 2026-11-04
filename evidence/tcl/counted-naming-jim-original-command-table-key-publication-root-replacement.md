# Tcl counted-source execution evidence

## Question

> A publicly reported Jim command key can preserve a particular spelling through same-slot replacement while deletion/fresh publication reports another spelling. Namespace forwarding can also prevent the original inventory query from reaching a key observation.

> Which exact public Jim command-name/value reports follow original root procedure/alias replacement, rename/delete/fresh publication and namespace-relative replacement, and which namespace query remains limited by its caught arity failure?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Four exact ASCII LF originals use fresh full provider initialisation and the unchanged original counted-source driver. The namespace sources remain independent and retain their original limited versus successful public outcomes. Exact sources/requests/driver/launcher/ELFs/whole streams/receipts and required executable/archive/header/build/source pins remain inspectable. Literal triple roots are intentional; Unicode/NUL, BIG-IP and executed Rust comparisons are not tested. The linked shared NativeJimCommandTableKey API retains report bytes separately from constructed comparison bytes and refuses lookup or another engine as an original publication issuer. Runtime actual CommandBinding/NativeCommandCreation capture same-slot key metadata before replacement; VM NativeNameWorld retains key metadata under the exact backend slot and edges to the occupied node. Rename supplies its original destination key, removal clears the actual metadata, and enumeration filters before reporting retained units. These are software ownership/API controls, not proof of a provider key/header pointer identity. Runtime and VM each link eighteen whole original public code/result comparisons across the three successful source controls; the limited namespace original remains independent. Linked status claims no executed Rust pass.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {[catch {info commands -all r2286_key}]} {
    list NOT_APPLICABLE
} else {
    proc ::r2286_key {} {return FIRST}
    set first [list [lsort [info commands -all r2286_key]] [r2286_key]]
    proc r2286_key {} {return SECOND}
    set replaced [list [lsort [info commands -all r2286_key]] [r2286_key]]
    alias :::r2286_key list ALIAS
    set aliased [list [lsort [info commands -all r2286_key]] [r2286_key]]
    proc r2286_key {} {return LAST}
    set final [list [lsort [info commands -all r2286_key]] [r2286_key]]
    list $first $replaced $aliased $final
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
ORIGINAL|0|7b3a3a72323238365f6b65792046495253547d207b3a3a72323238365f6b6579205345434f4e447d207b3a3a72323238365f6b657920414c4941537d207b3a3a72323238365f6b6579204c4153547d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{::r2286_key FIRST} {::r2286_key SECOND} {::r2286_key ALIAS} {::r2286_key LAST}⟦EOF⟧
```
