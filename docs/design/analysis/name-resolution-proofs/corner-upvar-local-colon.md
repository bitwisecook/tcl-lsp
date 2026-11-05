# Can this colon-leading local alias read the rooted target?

Proof ID: `naming.corner.upvar-local-colon`

## Problem statement

upvar installs a colon-leading local alias for a rooted variable. Treating the alias operand as a namespace publication or an ordinary local string can lose the selected target binding. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Can this colon-leading local alias read the rooted target?

## Exact control

```tcl
set ::target ROOT; proc p {} {upvar #0 ::target :alias; list ${:alias} [info locals]}; p
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 524f4f54207b7d; result rendering "ROOT {}". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 524f4f54207b7d; result rendering "ROOT {}". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 524f4f54207b7d; result rendering "ROOT {}". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 524f4f54207b7d; result rendering "ROOT {}". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 524f4f54207b7d; result rendering "ROOT {}". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 524f4f54207b7d; result rendering "ROOT {}". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

All six captured programs return ROOT with their captured local roster; this establishes alias access rather than a new namespace declaration.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
