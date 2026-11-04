# Tcl execution evidence

## Question

> An existing array root is a different variable receiver from an array element. Literal, dynamic and procedure source entries need their actual release-specific write result and errorCode; a read-failure tuple or presentation label cannot establish that boundary.

> What catch result, errorCode and remaining array state follow the three fixed source entries that write an existing array root?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One actual CLI interpreter per selected provider executes the same fixed ASCII LF source in literal, dynamic-command and procedure order; each form starts with a recreated array a containing k=OLD. Six separately printed patchlevels and eighteen complete result/error/state rows are retained. No arbitrary array names, opaque input, observers, physical cell/object/header/cache, original counted-argv ingress, compiler entry, Native Normal, pointer layout or BIG-IP answer is inferred.

## Script

```tcl-visible
puts [list PATCHLEVEL [info patchlevel]]

proc reportArrayWrite {label script} {
    uplevel #0 {array set a {k OLD}}
    set status [uplevel #0 [list catch $script result]]
    set result [uplevel #0 {set result}]
    if {[info exists ::errorCode]} {
        set code $::errorCode
    } else {
        set code {}
    }
    puts [list RESULT $label $status $result $code [uplevel #0 {array exists a}]]
    uplevel #0 {unset a}
}

reportArrayWrite literal_array_write {set a NEW}
reportArrayWrite dynamic_array_write {set cmd set; $cmd a NEW}
proc writeArrayRoot {} {
    global a
    set a NEW
}
reportArrayWrite procedure_array_write {writeArrayRoot}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: ASCII LF source-file CLI; three existing-array root variable write cases
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.4.20
RESULT literal_array_write 1 {can't set "a": variable is array} NONE 1
RESULT dynamic_array_write 1 {can't set "a": variable is array} NONE 1
RESULT procedure_array_write 1 {can't set "a": variable is array} NONE 1
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: ASCII LF source-file CLI; three existing-array root variable write cases
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.5.19
RESULT literal_array_write 1 {can't set "a": variable is array} NONE 1
RESULT dynamic_array_write 1 {can't set "a": variable is array} NONE 1
RESULT procedure_array_write 1 {can't set "a": variable is array} NONE 1
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: ASCII LF source-file CLI; three existing-array root variable write cases
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.6.18
RESULT literal_array_write 1 {can't set "a": variable is array} {TCL WRITE VARNAME} 1
RESULT dynamic_array_write 1 {can't set "a": variable is array} {TCL WRITE VARNAME} 1
RESULT procedure_array_write 1 {can't set "a": variable is array} {TCL WRITE VARNAME} 1
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: ASCII LF source-file CLI; three existing-array root variable write cases
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 9.0.4
RESULT literal_array_write 1 {can't set "a": variable is array} {TCL WRITE VARNAME} 1
RESULT dynamic_array_write 1 {can't set "a": variable is array} {TCL WRITE VARNAME} 1
RESULT procedure_array_write 1 {can't set "a": variable is array} {TCL WRITE VARNAME} 1
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: ASCII LF source-file CLI; three existing-array root variable write cases
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 9.1.0
RESULT literal_array_write 1 {can't set "a": variable is array} {TCL WRITE VARNAME} 1
RESULT dynamic_array_write 1 {can't set "a": variable is array} {TCL WRITE VARNAME} 1
RESULT procedure_array_write 1 {can't set "a": variable is array} {TCL WRITE VARNAME} 1
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: ASCII LF source-file CLI; three existing-array root variable write cases
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 0.84-9-g5bac7c9
RESULT literal_array_write 0 NEW NONE 0
RESULT dynamic_array_write 0 NEW NONE 0
RESULT procedure_array_write 0 NEW NONE 0
```

- Standard error: empty.
