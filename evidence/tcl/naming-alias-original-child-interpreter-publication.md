# Tcl execution evidence

## Question

> Parent-to-child alias creation uses provider-specific APIs and can retain different returned values while accepting the same original call spellings. Cross-interpreter ports must preserve each API result instead of inferring shared storage or a single returned-name protocol.

> How do rooted, relative-qualified, unqualified and extra-root alias names publish and resolve for parent-to-child aliases under each provider's actual supported child-interpreter API?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One original fixed ASCII LF file per fresh provider process selects an explicit C versus Jim branch by its reported patchlevel, creates one child, tests four original alias names and deletes the child. The aliases target the parent list VALUE prefix; child evaluation is exactly [list $name]. No nonroot child current namespace, arbitrary target, target rename/delete/replacement, alias loop, NUL, Unicode, continuation/control escape, entered receiver frame or interpreter lifetime after deletion is measured. API return values are retained independently and not equated.

## Script

```tcl-visible
puts [list version [info patchlevel]]
set is_jim [string match 0.* [info patchlevel]]
if {$is_jim} {set child [interp]} else {set child [interp create]}
foreach name {::A::a A::relative plain ::::extra} {
    if {$is_jim} {
        set code [catch {$child alias $name list VALUE} result]
        set call_code [catch {$child eval [list $name]} call_result]
    } else {
        set code [catch {interp alias $child $name {} list VALUE} result]
        set call_code [catch {interp eval $child [list $name]} call_result]
    }
    puts [list child_alias $name $code $result $call_code $call_result]
}
if {$is_jim} {$child delete} else {interp delete $child}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim
- Exit status: 0
- Standard output:

```text-visible
version 8.4.20
child_alias ::A::a 0 ::A::a 0 VALUE
child_alias A::relative 0 A::relative 0 VALUE
child_alias plain 0 plain 0 VALUE
child_alias ::::extra 0 ::::extra 0 VALUE
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim
- Exit status: 0
- Standard output:

```text-visible
version 8.5.19
child_alias ::A::a 0 ::A::a 0 VALUE
child_alias A::relative 0 A::relative 0 VALUE
child_alias plain 0 plain 0 VALUE
child_alias ::::extra 0 ::::extra 0 VALUE
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim
- Exit status: 0
- Standard output:

```text-visible
version 8.6.18
child_alias ::A::a 0 ::A::a 0 VALUE
child_alias A::relative 0 A::relative 0 VALUE
child_alias plain 0 plain 0 VALUE
child_alias ::::extra 0 ::::extra 0 VALUE
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim
- Exit status: 0
- Standard output:

```text-visible
version 9.0.4
child_alias ::A::a 0 ::A::a 0 VALUE
child_alias A::relative 0 A::relative 0 VALUE
child_alias plain 0 plain 0 VALUE
child_alias ::::extra 0 ::::extra 0 VALUE
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim
- Exit status: 0
- Standard output:

```text-visible
version 9.1.0
child_alias ::A::a 0 ::A::a 0 VALUE
child_alias A::relative 0 A::relative 0 VALUE
child_alias plain 0 plain 0 VALUE
child_alias ::::extra 0 ::::extra 0 VALUE
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim
- Exit status: 0
- Standard output:

```text-visible
version 0.84-9-g5bac7c9
child_alias ::A::a 0 {} 0 VALUE
child_alias A::relative 0 {} 0 VALUE
child_alias plain 0 {} 0 VALUE
child_alias ::::extra 0 {} 0 VALUE
```

- Standard error: empty.
