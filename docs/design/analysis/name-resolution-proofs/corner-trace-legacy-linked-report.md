# What root and element do legacy read traces report through this local alias?

Proof ID: `naming.corner.trace-legacy-linked-report`

## Problem statement

A legacy read trace is attached through a local alias to an array element. The callback's reported root and element may describe the alias rather than the underlying cell, and the legacy syntax is release-dependent. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

What root and element do legacy read traces report through this local alias?

## Exact control

```tcl
set a(k) VALUE; set log {}; proc observe {n e op} {lappend ::log [list $n $e $op]}; proc p {} {upvar #0 a(k) alias; trace variable alias r observe; set alias}; list [p] $log
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b616c696173207b7d20727d7d; result rendering "VALUE {{alias {} r}}". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b616c696173207b7d20727d7d; result rendering "VALUE {{alias {} r}}". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b616c696173207b7d20727d7d; result rendering "VALUE {{alias {} r}}". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | unsupported | Process exit 0; guest completion 1; captured result hex 626164206f7074696f6e20227661726961626c65223a206d757374206265206164642c20696e666f2c206f722072656d6f7665; result rendering "bad option \"variable\": must be add, info, or remove". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl9.1 9.1.0 | unsupported | Process exit 0; guest completion 1; captured result hex 626164206f7074696f6e20227661726961626c65223a206d757374206265206164642c20696e666f2c206f722072656d6f7665; result rendering "bad option \"variable\": must be add, info, or remove". The precise command/option creation door is unavailable in this capture; its error remains data. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022747261636522; result rendering "invalid command name \"trace\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

C8.4–8.6 retain the alias/empty-element/r report; C9 retains removed legacy syntax errors and Jim the missing trace command error.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
