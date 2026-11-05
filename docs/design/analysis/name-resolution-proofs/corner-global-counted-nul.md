# What value and local roster does this generated NUL-bearing global binding expose?

Proof ID: `naming.corner.global-counted-nul`

## Problem statement

global receives a generated NUL-bearing variable name while a plain prefix variable also exists. Clipping a reporting name could select the wrong global; the formal name and generated operand retain separate roles. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

What value and local roster does this generated NUL-bearing global binding expose?

## Exact control

```tcl
set name "v[format %c 0]tail"; set $name FULL; set v SHORT; proc p {name} {global $name; list [info locals] [set $name]}; p $name
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 6e616d652046554c4c; result rendering "name FULL". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 6e616d652046554c4c; result rendering "name FULL". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 6e616d652046554c4c; result rendering "name FULL". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 6e616d652046554c4c; result rendering "name FULL". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 6e616d652046554c4c; result rendering "name FULL". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 6e616d652046554c4c; result rendering "name FULL". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

All six captures retain the observed name roster and FULL result; the binding purpose cannot be inferred from a rendered prefix.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
