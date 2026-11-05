# Which interpreter target and baked argv does the C child-interpreter alias enter?

Proof ID: `naming.corner.child-c-style-alias`

## Problem statement

A child interpreter alias targets a parent command with PREFIX baked before ARG. Equal command labels do not identify the target interpreter or the captured versus callsite argv roles. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Which interpreter target and baked argv does the C child-interpreter alias enter?

## Exact control

```tcl
set child [interp create]; proc target args {return [list PARENT $args]}; interp alias $child a {} target PREFIX; set result [interp eval $child {a ARG}]; interp delete $child; set result
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 504152454e54207b505245464958204152477d; result rendering "PARENT {PREFIX ARG}". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 504152454e54207b505245464958204152477d; result rendering "PARENT {PREFIX ARG}". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 504152454e54207b505245464958204152477d; result rendering "PARENT {PREFIX ARG}". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 504152454e54207b505245464958204152477d; result rendering "PARENT {PREFIX ARG}". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 504152454e54207b505245464958204152477d; result rendering "PARENT {PREFIX ARG}". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727022; result rendering "wrong # args: should be \"interp\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The five C captures return PARENT with PREFIX ARG; Jim retains the creation-door interp grammar error.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
