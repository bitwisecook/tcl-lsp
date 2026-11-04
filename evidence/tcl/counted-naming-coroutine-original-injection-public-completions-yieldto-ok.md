# Tcl counted-source execution evidence

## Question

> Queued coroutine injections need public completion codes and values together with the exact reached callback log. A result byte equal to a success value cannot establish that an Error branch occurred or was replaced.

> For the original yield/yieldto controls, what public callback order, suspension kind, passed value and terminal or replaced Error completion are observed?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Two independently retained four-control source groups run exact ASCII LF originals in fresh full provider initialisation through the unchanged counted C API driver, with no added outer source envelope. Their catches and explicit reached-error markers are original source inputs. The original four inputs and independent marked error inputs stay collision-free under original/ and errors/. Whole original requests/sources/driver/launchers/ELFs/commands/receipts/stdout/stderr and required provider/source/header/archive/build pins remain exact. Public suspension-kind/callback/value observations supply no private frame/token or abstract interpreter identity.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {[llength [info commands coroinject]] == 0} {
    set result UNAVAILABLE
} else {
    set log {}
    proc inject {tag mode kind value} {
        lappend ::log [list $tag $kind $value]
        if {$mode eq "error"} {error $tag}
        return $tag
    }
    coroutine c apply {{} {set ::value [yieldto list ready]; return $::value}}
    coroinject c inject A ok
    coroinject c inject B ok
    set completion [catch {c ORIGINAL} result options]
    list $completion $result $log
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
ORIGINAL|0|554e415641494c41424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
UNAVAILABLE⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|554e415641494c41424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
UNAVAILABLE⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|554e415641494c41424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
UNAVAILABLE⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|302041207b7b42207969656c64746f204f524947494e414c7d207b41207969656c64746f20427d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 A {{B yieldto ORIGINAL} {A yieldto B}}⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|302041207b7b42207969656c64746f204f524947494e414c7d207b41207969656c64746f20427d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 A {{B yieldto ORIGINAL} {A yieldto B}}⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|554e415641494c41424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
UNAVAILABLE⟦EOF⟧
```
