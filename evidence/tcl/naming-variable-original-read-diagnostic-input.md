# Tcl execution evidence

## Question

> A variable Read and a write to the same displayed receiver have different failure presenters. C array-root reads can fail where Jim dictionary-root reads succeed, and scalar-element failures must retain the original requested read name. Literal, dynamic-head and procedure source entries need separate recorded controls.

> For the six original ASCII source forms, what exact caught Read result does each provider return for an array/dictionary root a and for b(k) when b is a scalar?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Six fixed complete ASCII LF source-file CLI forms per actual C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim process. Actual queried versions identify each provider; outer process status is distinct from the six catches. No opaque/counting name channel, private receiver/object/header/cache identity, source compiler admission, captured increment, callback lifetime, general dictionary ordering or BIG-IP result is measured. The recreated-element callback question remains independent.

## Script

```tcl-visible
if {[catch {info patchlevel} probeVersion]} {
    set probeVersion unavailable
}
puts "PATCHLEVEL $probeVersion"
foreach {label script} {
    literal_array_read {array set a {k OLD}; set a}
    dynamic_array_read {array set a {k OLD}; set probeHead set; $probeHead a}
    procedure_array_read {proc probeArrayRead {} {array set a {k OLD}; set a}; probeArrayRead}
    literal_scalar_element {set b SCALAR; set b(k)}
    dynamic_scalar_element {set b SCALAR; set probeHead set; $probeHead b(k)}
    procedure_scalar_element {proc probeScalarRead {} {set b SCALAR; set b(k)}; probeScalarRead}
} {
    set probeCode [catch $script probeResult]
    puts [list RESULT $label $probeCode $probeResult]
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.4.20
RESULT literal_array_read 1 {can't read "a": variable is array}
RESULT dynamic_array_read 1 {can't read "a": variable is array}
RESULT procedure_array_read 1 {can't read "a": variable is array}
RESULT literal_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT dynamic_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT procedure_scalar_element 1 {can't read "b(k)": variable isn't array}
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.5.19
RESULT literal_array_read 1 {can't read "a": variable is array}
RESULT dynamic_array_read 1 {can't read "a": variable is array}
RESULT procedure_array_read 1 {can't read "a": variable is array}
RESULT literal_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT dynamic_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT procedure_scalar_element 1 {can't read "b(k)": variable isn't array}
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 8.6.18
RESULT literal_array_read 1 {can't read "a": variable is array}
RESULT dynamic_array_read 1 {can't read "a": variable is array}
RESULT procedure_array_read 1 {can't read "a": variable is array}
RESULT literal_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT dynamic_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT procedure_scalar_element 1 {can't read "b(k)": variable isn't array}
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 9.0.4
RESULT literal_array_read 1 {can't read "a": variable is array}
RESULT dynamic_array_read 1 {can't read "a": variable is array}
RESULT procedure_array_read 1 {can't read "a": variable is array}
RESULT literal_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT dynamic_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT procedure_scalar_element 1 {can't read "b(k)": variable isn't array}
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 9.1.0
RESULT literal_array_read 1 {can't read "a": variable is array}
RESULT dynamic_array_read 1 {can't read "a": variable is array}
RESULT procedure_array_read 1 {can't read "a": variable is array}
RESULT literal_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT dynamic_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT procedure_scalar_element 1 {can't read "b(k)": variable isn't array}
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 0.84-9-g5bac7c9
RESULT literal_array_read 0 {k OLD}
RESULT dynamic_array_read 0 {k OLD}
RESULT procedure_array_read 0 {k OLD}
RESULT literal_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT dynamic_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT procedure_scalar_element 1 {can't read "b(k)": variable isn't array}
```

- Standard error: empty.
