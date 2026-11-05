# Does this generated NUL-bearing formal create the plain x local?

Proof ID: `naming.corner.procedure-formal-counted-nul`

## Problem statement

The formal list contains a generated x-NUL-tail name. Treating a formal as a CString prefix could create x; formal-list storage must be measured independently of a variable operand or raw source channel. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does this generated NUL-bearing formal create the plain x local?

## Exact control

```tcl
set formal "x[format %c 0]tail"; proc p [list $formal] {list [info locals] [info exists x]}; p VALUE
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 78007461696c2030; result rendering "x\u0000tail 0". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 78007461696c2030; result rendering "x\u0000tail 0". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 78007461696c2030; result rendering "x\u0000tail 0". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 78007461696c2030; result rendering "x\u0000tail 0". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 78007461696c2030; result rendering "x\u0000tail 0". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 78007461696c2030; result rendering "x\u0000tail 0". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

All six captured programs enumerate the full formal and report plain x absent. This is a generated formal-name control, not a raw00 compiler preparation proof.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
