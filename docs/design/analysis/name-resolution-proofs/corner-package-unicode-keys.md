# Which versions are returned for these two generated Unicode package names?

Proof ID: `naming.corner.package-unicode-keys`

## Problem statement

Two Unicode package names are provided with different versions. A command-name distinctness result does not establish package-name key comparison or version selection for these evaluated names. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Which versions are returned for these two generated Unicode package names?

## Exact control

```tcl
set one [format %c 233]; set two "e[format %c 769]"; package provide $one 1.0; package provide $two 2.0; list [package require $one] [package require $two]
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 312e3020322e30; result rendering "1.0 2.0". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 312e3020322e30; result rendering "1.0 2.0". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 312e3020322e30; result rendering "1.0 2.0". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 312e3020322e30; result rendering "1.0 2.0". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 312e3020322e30; result rendering "1.0 2.0". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 312e3020312e30; result rendering "1.0 1.0". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

C programs return 1.0 2.0; Jim returns 1.0 1.0 in this exact control. These observations alone do not identify the internal key comparison mechanism.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
