# Does global ::n expose a plain n local and the rooted value in this program?

Proof ID: `naming.corner.global-rooted-local`

## Problem statement

global receives ::n while the procedure asks whether plain n exists. Assuming all providers manufacture the same local-tail alias from a rooted operand can misidentify the local cell. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does global ::n expose a plain n local and the rooted value in this program?

## Exact control

```tcl
set ::n VALUE; proc p {} {global ::n; list [info locals] [info exists n] [set ::n]}; p
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 7b7d20312056414c5545; result rendering "{} 1 VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 7b7d20312056414c5545; result rendering "{} 1 VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 7b7d20312056414c5545; result rendering "{} 1 VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 7b7d20312056414c5545; result rendering "{} 1 VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 7b7d20312056414c5545; result rendering "{} 1 VALUE". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 7b7d20302056414c5545; result rendering "{} 0 VALUE". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The C programs report n existence 1 and VALUE; Jim reports n existence 0 while the explicitly rooted read returns VALUE.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
