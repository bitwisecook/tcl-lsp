# BIG-IP Tcl log: nh20261012exact2, tmm-procedure-10

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **namespace-captured-prefix:** Does a namespace code prefix retain the defining namespace when evaluated elsewhere, including after a same-named global command is defined, and preserve its argument elements?

## Assumptions

- **namespace-captured-prefix:** The byte-identical original program was evaluated sequentially. Complete recorded binary-scan errorInfo bytes contain reversible ASCII quoted-command and eval-body-line markers locating the authored final operation. Reaching that source line establishes that preceding authored top-level commands completed without propagating a nonzero completion. Code1/result belongs to evaluating the final operation, including substitutions; an argument-substitution failure does not imply outer command dispatch. ASCII is an explicit reversible byte projection for locating stack markers only. The original result/error encodings remain binary-scan-tcl-value; no UTF8 or Tcl Unicode projection is claimed. Successful setup means prior source evaluation completed, not that every expected binding exists. Ownership collisions and earlier program errors remain unpromoted; this review applies only to this exact identity.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-procedure-1](fixtures/bigip-tmm-procedure-1.md)

- run: `nh20261012exact2`
- case: `namespace-captured-prefix`

### nh_setup

```tcl-visible
set nh_guard_scope [namespace current]
set nh_guard_procs {}
set nh_guard_absent {}
foreach nh_guard_name {::nh} {
    if {[namespace exists $nh_guard_name] || [llength [info commands $nh_guard_name]]} {error [list ownership-collision namespace-or-command $nh_guard_name]}
    lappend nh_guard_absent [list namespace-and-command $nh_guard_name]
}
foreach nh_guard_tail {emit} {
    set nh_guard_name [string trimright $nh_guard_scope :]::$nh_guard_tail
    if {[llength [info commands $nh_guard_name]] || [string length [namespace which -command $nh_guard_tail]]} {error [list ownership-collision procedure $nh_guard_name resolved [namespace which -command $nh_guard_tail]]}
    lappend nh_guard_procs $nh_guard_name
    lappend nh_guard_absent [list procedure $nh_guard_name fallback $nh_guard_tail]
}
foreach nh_guard_name {} {
    if {[namespace exists $nh_guard_name] || [llength [info commands $nh_guard_name]]} {error [list ownership-collision command-or-namespace $nh_guard_name]}
    lappend nh_guard_procs $nh_guard_name
    lappend nh_guard_absent [list command-and-namespace $nh_guard_name]
}
foreach nh_guard_name {} {
    if {[info exists $nh_guard_name] || [llength [info vars $nh_guard_name]] || [llength [trace vinfo $nh_guard_name]]} {error [list ownership-collision frame-variable $nh_guard_name]}
    lappend nh_guard_absent [list frame-variable $nh_guard_name]
}
list ownership-guard execution-namespace $nh_guard_scope absent $nh_guard_absent
```

### nh_operation

```tcl-visible
namespace eval ::nh {
    proc emit {value args} {list local $value $args}
    set prefix [namespace code [list emit {a b}]]
}
proc emit {args} {return global}
eval [linsert $::nh::prefix end {c d}]

```

### nh_cleanup

```tcl-visible
if {$nh_setup_code == 0} {
    foreach nh_guard_name $nh_guard_procs {
        if {[llength [info commands $nh_guard_name]]} {rename $nh_guard_name {}}
    }
    foreach nh_guard_name {::nh} {
        if {[namespace exists $nh_guard_name]} {namespace delete $nh_guard_name}
    }
    foreach nh_guard_name {} {unset -nocomplain $nh_guard_name}
    set nh_guard_remaining {}
    foreach nh_guard_name $nh_guard_procs {
        if {[llength [info commands $nh_guard_name]]} {lappend nh_guard_remaining [list command $nh_guard_name]}
    }
    foreach nh_guard_name {::nh} {
        if {[namespace exists $nh_guard_name]} {lappend nh_guard_remaining [list namespace $nh_guard_name]}
    }
    foreach nh_guard_name {} {
        if {[info exists $nh_guard_name]} {lappend nh_guard_remaining [list frame-variable $nh_guard_name]}
    }
    list ownership-cleanup execution-namespace $nh_guard_scope remaining $nh_guard_remaining
} else {
    list ownership-not-acquired original-source-unreached
}
```

## Results

### tmm-procedure, unit 0:0, invocation once; tmm-procedure, unit 0:1, invocation once; tmm-procedure, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-captured-prefix, setup, completion 0, result:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {procedure ::emit fallback emit}}⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, result:

```text-visible
can't read "::nh::prefix": no such variable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorinfo:

```text-visible
can't read "::nh::prefix": no such variable
    while executing
"linsert $::nh::prefix end {c d}"
    ("eval" body line 6)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-captured-prefix, runtime-cleanup, completion 0, result:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- namespace-captured-prefix: intended operation reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- namespace-captured-prefix, setup, completion 0, result:

```text-visible
ownership-guard execution-namespace :: absent {{namespace-and-command ::nh} {procedure ::emit fallback emit}}⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, result:

```text-visible
can't read "::nh::prefix": no such variable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- namespace-captured-prefix, lookup, completion 1, errorinfo:

```text-visible
can't read "::nh::prefix": no such variable
    while executing
"linsert $::nh::prefix end {c d}"
    ("eval" body line 6)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- namespace-captured-prefix, runtime-cleanup, completion 0, result:

```text-visible
ownership-cleanup execution-namespace :: remaining {}⟦EOF⟧
```

- namespace-captured-prefix: intended operation reached.
