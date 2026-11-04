# Empty-loop variable reads under caught load errors

## Question

Does an empty `foreach` leave its procedure variable absent when the containing source load raises a separate top-level error, and do an existence guard, prior binding, formal parameter or by-name writer change the later read? These runtime controls support the expected W210 boundaries for #2441; they do not measure analyser diagnostics or establish source/Native admission.

## Assumptions

Each fixed source is loaded with `namespace eval` into its own new namespace. The driver catches that load, then calls the retained procedure under a separate `catch`; it deliberately continues after the original source's top-level error. The `exact` source is byte-for-byte the issue script, including its final LF, passed as the `namespace eval` argument. Its `source-bytes` row records that actual argument with Tcl's `binary scan`, not a reconstructed source. Namespace execution and the catch wrappers are part of this experiment, so this is not an unhandled global evaluation of the report. Other cases are fixed controls. Procedure parameters are supplied only in the formal-parameter control. The by-name writer uses `upvar` in its caller's procedure frame. No timing or internal object representation was measured. Additional interpreter startup/environment assumptions: Not recorded. Each recorded stdout/stderr stream is captured as bytes; empty stderr is recorded explicitly. C Tcl paths below are invocation wrappers, and their hashes identify those wrapper files.

## Script

```tcl-visible
set cases {
    {exact {proc p {} {
    foreach x {} {}
    if {[info exists x]} { puts yes }
    puts $x
}
foreach y {} {}
puts $y
} {}}
    {without-guard {
proc p {} {foreach x {} {}; return $x}
error fail
    } {}}
    {guarded-only {
proc p {} {foreach x {} {}; if {[info exists x]} {return $x}; return quiet}
error fail
    } {}}
    {guarded-later-read {
proc p {} {foreach x {} {}; if {[info exists x]} {set other 1}; return $x}
error fail
    } {}}
    {nonempty-loop {
proc p {} {foreach x {7} {}; return $x}
error fail
    } {}}
    {initialised-before-empty-loop {
proc p {} {set x 7; foreach x {} {}; return $x}
error fail
    } {}}
    {formal-before-empty-loop {
proc p {x} {foreach x {} {}; return $x}
error fail
    } {7}}
    {mutation-after-guard {
proc p {} {set x 7; if {[info exists x]} {unset x}; foreach x {} {}; return $x}
error fail
    } {}}
    {by-name-writer {
proc operation {name} {upvar 1 $name cell; set cell 7}
proc p {} {operation x; foreach x {} {}; return $x}
error fail
    } {}}
}
set ordinal 0
foreach case $cases {
    set id [lindex $case 0]
    set script [lindex $case 1]
    set args [lindex $case 2]
    if {$id eq "exact"} {
        binary scan $script H* sourceHex
        puts [list source-bytes exact $sourceHex]
    }
    set ns ::case[incr ordinal]
    set loadRc [catch {namespace eval $ns $script} loadResult]
    set call [linsert $args 0 ${ns}::p]
    set callRc [catch {eval $call} callResult]
    puts [list $id $loadRc $loadResult $callRc $callResult]
}
```

## Results

### tcl8.4 8.4.20

- Interpreter invocation path: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4`; SHA-256 `704c2caa06eddb67f97e128572395e1ed81a87bc07544b3b55efcd3139a1dc98`.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4 /tmp/lsp-2441-boundary-native.tcl`.
- Version observation: `puts [info patchlevel]` through standard input, exit 0.
- Exit status: 0.
- Standard output:

```text-visible
source-bytes exact 70726f632070207b7d207b0a20202020666f72656163682078207b7d207b7d0a202020206966207b5b696e666f2065786973747320785d7d207b207075747320796573207d0a20202020707574732024780a7d0a666f72656163682079207b7d207b7d0a707574732024790a
exact 1 {can't read "y": no such variable} 1 {can't read "x": no such variable}
without-guard 1 fail 1 {can't read "x": no such variable}
guarded-only 1 fail 0 quiet
guarded-later-read 1 fail 1 {can't read "x": no such variable}
nonempty-loop 1 fail 0 7
initialised-before-empty-loop 1 fail 0 7
formal-before-empty-loop 1 fail 0 7
mutation-after-guard 1 fail 1 {can't read "x": no such variable}
by-name-writer 1 fail 0 7
```

- Standard error:

```text-visible
⟦EOF⟧
```

### tcl8.5 8.5.19

- Interpreter invocation path: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5`; SHA-256 `ec41a6a74436030c15a3c45b9eeae821509148223711e9dd0993907fe5a59eb6`.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5 /tmp/lsp-2441-boundary-native.tcl`.
- Version observation: `puts [info patchlevel]` through standard input, exit 0.
- Exit status: 0.
- Standard output:

```text-visible
source-bytes exact 70726f632070207b7d207b0a20202020666f72656163682078207b7d207b7d0a202020206966207b5b696e666f2065786973747320785d7d207b207075747320796573207d0a20202020707574732024780a7d0a666f72656163682079207b7d207b7d0a707574732024790a
exact 1 {can't read "y": no such variable} 1 {can't read "x": no such variable}
without-guard 1 fail 1 {can't read "x": no such variable}
guarded-only 1 fail 0 quiet
guarded-later-read 1 fail 1 {can't read "x": no such variable}
nonempty-loop 1 fail 0 7
initialised-before-empty-loop 1 fail 0 7
formal-before-empty-loop 1 fail 0 7
mutation-after-guard 1 fail 1 {can't read "x": no such variable}
by-name-writer 1 fail 0 7
```

- Standard error:

```text-visible
⟦EOF⟧
```

### tcl8.6 8.6.18

- Interpreter invocation path: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6`; SHA-256 `d98e632bcfbb411d33841cd75873d374dc950bf1a34ce4b20b657208d51f746c`.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6 /tmp/lsp-2441-boundary-native.tcl`.
- Version observation: `puts [info patchlevel]` through standard input, exit 0.
- Exit status: 0.
- Standard output:

```text-visible
source-bytes exact 70726f632070207b7d207b0a20202020666f72656163682078207b7d207b7d0a202020206966207b5b696e666f2065786973747320785d7d207b207075747320796573207d0a20202020707574732024780a7d0a666f72656163682079207b7d207b7d0a707574732024790a
exact 1 {can't read "y": no such variable} 1 {can't read "x": no such variable}
without-guard 1 fail 1 {can't read "x": no such variable}
guarded-only 1 fail 0 quiet
guarded-later-read 1 fail 1 {can't read "x": no such variable}
nonempty-loop 1 fail 0 7
initialised-before-empty-loop 1 fail 0 7
formal-before-empty-loop 1 fail 0 7
mutation-after-guard 1 fail 1 {can't read "x": no such variable}
by-name-writer 1 fail 0 7
```

- Standard error:

```text-visible
⟦EOF⟧
```

### tcl9.0 9.0.4

- Interpreter invocation path: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0`; SHA-256 `843e6c3f41064c654932a3677be76baddf10e59d340fd6de2a0ea56cffaf5f35`.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0 /tmp/lsp-2441-boundary-native.tcl`.
- Version observation: `puts [info patchlevel]` through standard input, exit 0.
- Exit status: 0.
- Standard output:

```text-visible
source-bytes exact 70726f632070207b7d207b0a20202020666f72656163682078207b7d207b7d0a202020206966207b5b696e666f2065786973747320785d7d207b207075747320796573207d0a20202020707574732024780a7d0a666f72656163682079207b7d207b7d0a707574732024790a
exact 1 {can't read "y": no such variable} 1 {can't read "x": no such variable}
without-guard 1 fail 1 {can't read "x": no such variable}
guarded-only 1 fail 0 quiet
guarded-later-read 1 fail 1 {can't read "x": no such variable}
nonempty-loop 1 fail 0 7
initialised-before-empty-loop 1 fail 0 7
formal-before-empty-loop 1 fail 0 7
mutation-after-guard 1 fail 1 {can't read "x": no such variable}
by-name-writer 1 fail 0 7
```

- Standard error:

```text-visible
⟦EOF⟧
```

### tcl9.1 9.1.0

- Interpreter invocation path: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1`; SHA-256 `f185a54d2fff0c33aa3d8fd5b7f867be7eceba237dea736b86c24e15b3fae0fa`.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1 /tmp/lsp-2441-boundary-native.tcl`.
- Version observation: `puts [info patchlevel]` through standard input, exit 0.
- Exit status: 0.
- Standard output:

```text-visible
source-bytes exact 70726f632070207b7d207b0a20202020666f72656163682078207b7d207b7d0a202020206966207b5b696e666f2065786973747320785d7d207b207075747320796573207d0a20202020707574732024780a7d0a666f72656163682079207b7d207b7d0a707574732024790a
exact 1 {can't read "y": no such variable} 1 {can't read "x": no such variable}
without-guard 1 fail 1 {can't read "x": no such variable}
guarded-only 1 fail 0 quiet
guarded-later-read 1 fail 1 {can't read "x": no such variable}
nonempty-loop 1 fail 0 7
initialised-before-empty-loop 1 fail 0 7
formal-before-empty-loop 1 fail 0 7
mutation-after-guard 1 fail 1 {can't read "x": no such variable}
by-name-writer 1 fail 0 7
```

- Standard error:

```text-visible
⟦EOF⟧
```

### jim 0.84-10-g82fb4d0

- Interpreter invocation path: `/workspace/tcl-lsp/tmp/lsp-oracles/jimtcl/jimsh`; SHA-256 `4086bc7223d3d0e853f7c094fdc6cf6f35f7e4c1837a3ff8f6abd0792cdedca8`.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/jimtcl/jimsh /tmp/lsp-2441-boundary-native.tcl`.
- Version observation: `puts [info patchlevel]` through standard input, exit 0.
- Exit status: 0.
- Standard output:

```text-visible
source-bytes exact 70726f632070207b7d207b0a20202020666f72656163682078207b7d207b7d0a202020206966207b5b696e666f2065786973747320785d7d207b207075747320796573207d0a20202020707574732024780a7d0a666f72656163682079207b7d207b7d0a707574732024790a
exact 1 {can't read "y": no such variable} 1 {can't read "x": no such variable}
without-guard 1 fail 1 {can't read "x": no such variable}
guarded-only 1 fail 0 quiet
guarded-later-read 1 fail 1 {can't read "x": no such variable}
nonempty-loop 1 fail 0 7
initialised-before-empty-loop 1 fail 0 7
formal-before-empty-loop 1 fail 0 7
mutation-after-guard 1 fail 1 {can't read "x": no such variable}
by-name-writer 1 fail 0 7
```

- Standard error:

```text-visible
⟦EOF⟧
```
