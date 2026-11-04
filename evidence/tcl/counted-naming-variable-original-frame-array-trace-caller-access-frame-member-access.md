# Tcl counted-source execution evidence

## Question

> An unset callback during a procedure return can use upvar1 while a caller has a same-spelled array. Complete before-return values and callback logs are needed to distinguish those public caller accesses from mutation of the retiring owner's old members.

> What whole before-return/caller values and callback logs do the two original frame-teardown sources produce when unset callbacks use upvar1 and add traces through the resulting caller aliases?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Two ASCII LF sources were evaluated independently in fresh fully initialized interpreters through the counted-source C API driver, without an added source envelope. Opaque, NUL and Unicode names, private identities, BIG-IP and executed software controls were not tested. Public logs do not prove private identities.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
set log {}; proc parentNew args {lappend ::log PARENTNEW}; proc ownerOld args {upvar 1 a(x) seenX; upvar 1 a(y) seenY; lappend ::log [list OLD $seenX $seenY]; trace add variable seenX unset parentNew; trace add variable seenY unset parentNew}; proc owner {} {set a(x) OWNERX; set a(y) OWNERY; trace add variable a(x) unset ownerOld; trace add variable a(y) unset ownerOld; list $a(x) $a(y)}; proc parent {} {set a(x) PARENTX; set a(y) PARENTY; set before [owner]; list $before $a(x) $a(y)}; set before [parent]; list $before $log
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|7b7b4f574e455258204f574e4552597d20504152454e545820504152454e54597d207b7b4f4c4420504152454e545820504152454e54597d207b4f4c4420504152454e545820504152454e54597d20504152454e544e455720504152454e544e455720504152454e544e455720504152454e544e45577d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{{OWNERX OWNERY} PARENTX PARENTY} {{OLD PARENTX PARENTY} {OLD PARENTX PARENTY} PARENTNEW PARENTNEW PARENTNEW PARENTNEW}⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|7b7b4f574e455258204f574e4552597d20504152454e545820504152454e54597d207b7b4f4c4420504152454e545820504152454e54597d207b4f4c4420504152454e545820504152454e54597d20504152454e544e455720504152454e544e455720504152454e544e455720504152454e544e45577d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{{OWNERX OWNERY} PARENTX PARENTY} {{OLD PARENTX PARENTY} {OLD PARENTX PARENTY} PARENTNEW PARENTNEW PARENTNEW PARENTNEW}⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|7b7b4f574e455258204f574e4552597d20504152454e545820504152454e54597d207b7b4f4c4420504152454e545820504152454e54597d207b4f4c4420504152454e545820504152454e54597d20504152454e544e455720504152454e544e455720504152454e544e455720504152454e544e45577d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{{OWNERX OWNERY} PARENTX PARENTY} {{OLD PARENTX PARENTY} {OLD PARENTX PARENTY} PARENTNEW PARENTNEW PARENTNEW PARENTNEW}⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|7b7b4f574e455258204f574e4552597d20504152454e545820504152454e54597d207b7b4f4c4420504152454e545820504152454e54597d207b4f4c4420504152454e545820504152454e54597d20504152454e544e455720504152454e544e455720504152454e544e455720504152454e544e45577d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{{OWNERX OWNERY} PARENTX PARENTY} {{OLD PARENTX PARENTY} {OLD PARENTX PARENTY} PARENTNEW PARENTNEW PARENTNEW PARENTNEW}⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|7b7b4f574e455258204f574e4552597d20504152454e545820504152454e54597d207b7b4f4c4420504152454e545820504152454e54597d207b4f4c4420504152454e545820504152454e54597d20504152454e544e455720504152454e544e455720504152454e544e455720504152454e544e45577d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{{OWNERX OWNERY} PARENTX PARENTY} {{OLD PARENTX PARENTY} {OLD PARENTX PARENTY} PARENTNEW PARENTNEW PARENTNEW PARENTNEW}⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|1|696e76616c696420636f6d6d616e64206e616d652022747261636522
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
invalid command name "trace"⟦EOF⟧
```
