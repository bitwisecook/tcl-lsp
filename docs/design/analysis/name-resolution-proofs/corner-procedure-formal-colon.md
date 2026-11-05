# Are :odd and n::q accepted as formal parameters by this declaration?

Proof ID: `naming.corner.procedure-formal-colon`

## Problem statement

A procedure formal list contains :odd and n::q. Qualified variable-name acceptance does not establish that the procedure's formal grammar accepts these names at declaration. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Are :odd and n::q accepted as formal parameters by this declaration?

## Exact control

```tcl
proc p {:odd n::q} {list ${:odd} ${n::q} [info locals]}; p A B
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 1; captured result hex 70726f636564757265202270222068617320666f726d616c20706172616d6574657220226e3a3a71222074686174206973206e6f7420612073696d706c65206e616d65; result rendering "procedure \"p\" has formal parameter \"n::q\" that is not a simple name". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 1; captured result hex 666f726d616c20706172616d6574657220226e3a3a7122206973206e6f7420612073696d706c65206e616d65; result rendering "formal parameter \"n::q\" is not a simple name". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 1; captured result hex 666f726d616c20706172616d6574657220226e3a3a7122206973206e6f7420612073696d706c65206e616d65; result rendering "formal parameter \"n::q\" is not a simple name". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 1; captured result hex 666f726d616c20706172616d6574657220226e3a3a7122206973206e6f7420612073696d706c65206e616d65; result rendering "formal parameter \"n::q\" is not a simple name". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 1; captured result hex 666f726d616c20706172616d6574657220226e3a3a7122206973206e6f7420612073696d706c65206e616d65; result rendering "formal parameter \"n::q\" is not a simple name". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 412042207b3a6f6464206e3a3a717d; result rendering "A B {:odd n::q}". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The C programs reject n::q as a nonsimple formal; Jim accepts the two names and returns A B with its captured local roster.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
