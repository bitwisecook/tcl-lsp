# Does this dynamically generated NUL-bearing rename destination remain callable?

Proof ID: `naming.corner.rename-counted-nul`

## Problem statement

A command moves to a generated NUL-bearing destination and is called there. A clipped destination or a retained full destination gives different publication state; this is an evaluated-object control. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does this dynamically generated NUL-bearing rename destination remain callable?

## Exact control

```tcl
proc leaf {} {return ORIGINAL}; set dest "moved[format %c 0]tail"; rename leaf $dest; eval [list $dest]
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 4f524947494e414c; result rendering "ORIGINAL". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 4f524947494e414c; result rendering "ORIGINAL". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 4f524947494e414c; result rendering "ORIGINAL". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 4f524947494e414c; result rendering "ORIGINAL". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 4f524947494e414c; result rendering "ORIGINAL". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 4f524947494e414c; result rendering "ORIGINAL". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

All six captured programs return ORIGINAL through the generated destination. The control supplies no raw00 source-channel or native pointer claim.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
