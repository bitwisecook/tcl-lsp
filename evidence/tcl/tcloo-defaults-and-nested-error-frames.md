# TclOO defaults and nested error frames

## Question

What superclass does a fresh class report, what does renaming an absent method
report, and which invocation frames survive eval-list and child limit errors?

## Assumptions

Fresh source-built interpreters use their stock commands. `TCL_LIBRARY` is unset.
No Tk, extension, embedding or Jim behaviour is established by this script.

## Script

```tcl-visible
puts [list release [info patchlevel]]
if {[package vcompare [info patchlevel] 8.6] >= 0} {
    oo::class create C
    puts [list superclasses [info class superclasses C]]
    puts [list rename [catch {oo::define C renamemethod absent 42} message] $message]
}
proc p {} {error bad}
catch {eval [list p]} message
puts [list eval_list $message $::errorInfo]
if {[package vcompare [info patchlevel] 8.5] >= 0} {
    interp create c
    c eval {proc budget_loop {} {while {1} {incr x}}}
    interp limit c commands -value 1000
    catch {c eval budget_loop} message
    puts [list interp_limit $message $::errorInfo]
}
```

## Results

### C Tcl 8.4.20

Invocation: `env -u TCL_LIBRARY /workspace/tcl-lsp/tmp/tcl8.4.20/unix/tclsh < /tmp/goal-vm-gap-probes.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
release 8.4.20
eval_list bad {bad
    while executing
"error bad"
    (procedure "p" line 1)
    invoked from within
"p"
    ("eval" body line 1)
    invoked from within
"eval [list p]"}
```

- Standard error: empty.

### C Tcl 8.5.19

Invocation: `env -u TCL_LIBRARY /workspace/tcl-lsp/tmp/tcl8.5.19/unix/tclsh < /tmp/goal-vm-gap-probes.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
release 8.5.19
eval_list bad {bad
    while executing
"error bad"
    (procedure "p" line 1)
    invoked from within
"p"
    ("eval" body line 1)
    invoked from within
"eval [list p]"}
interp_limit {command count limit exceeded} {command count limit exceeded
    while executing
"while {1} {incr x}"
    (procedure "budget_loop" line 1)
    invoked from within
"budget_loop"
    invoked from within
"c eval budget_loop"}
```

- Standard error: empty.

### C Tcl 8.6.18

Invocation: `env -u TCL_LIBRARY /workspace/tcl-lsp/tmp/tcl8.6.18/unix/tclsh < /tmp/goal-vm-gap-probes.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
release 8.6.18
superclasses ::oo::object
rename 1 {method absent does not exist}
eval_list bad {bad
    while executing
"error bad"
    (procedure "p" line 1)
    invoked from within
"p"
    ("eval" body line 1)
    invoked from within
"eval [list p]"}
interp_limit {command count limit exceeded} {command count limit exceeded
    while executing
"incr x"
    (procedure "budget_loop" line 1)
    invoked from within
"budget_loop"
    invoked from within
"c eval budget_loop"}
```

- Standard error: empty.

### C Tcl 9.0.4

Invocation: `env -u TCL_LIBRARY /workspace/tcl-lsp/tmp/tcl9.0.4/unix/tclsh < /tmp/goal-vm-gap-probes.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
release 9.0.4
superclasses ::oo::object
rename 1 {method absent does not exist}
eval_list bad {bad
    while executing
"error bad"
    (procedure "p" line 1)
    invoked from within
"p"
    ("eval" body line 1)
    invoked from within
"eval [list p]"}
interp_limit {command count limit exceeded} {command count limit exceeded
    while executing
"incr x"
    (procedure "budget_loop" line 1)
    invoked from within
"budget_loop"
    invoked from within
"c eval budget_loop"}
```

- Standard error: empty.

### C Tcl 9.1.0

Invocation: `env -u TCL_LIBRARY /workspace/tcl-lsp/tmp/tcl9.1.0/unix/tclsh < /tmp/goal-vm-gap-probes.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
release 9.1.0
superclasses ::oo::object
rename 1 {method absent does not exist}
eval_list bad {bad
    while executing
"error bad"
    (procedure "p" line 1)
    invoked from within
"p"
    ("eval" body line 1)
    invoked from within
"eval [list p]"}
interp_limit {command count limit exceeded} {command count limit exceeded
    while executing
"incr x"
    (procedure "budget_loop" line 1)
    invoked from within
"budget_loop"
    invoked from within
"c eval budget_loop"}
```

- Standard error: empty.
