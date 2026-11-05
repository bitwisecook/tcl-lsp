# Does this package loader install the variable and command visible to the requiring namespace?

Proof ID: `naming.corner.package-loader-namespace`

## Problem statement

A package loader modifies namespace state and publishes a procedure while a namespace body requires it. Assuming that a require operand is merely metadata would miss the entered loader effects and its lookup context. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does this package loader install the variable and command visible to the requiring namespace?

## Exact control

```tcl
namespace eval ::n {variable stage BEFORE}; package ifneeded Demo 1.0 {namespace eval ::n {set stage LOADED; proc leaf {} {return INSTALLED}}; package provide Demo 1.0}; namespace eval ::n {list [package require Demo] $stage [leaf]}
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 312e30204c4f4144454420494e5354414c4c4544; result rendering "1.0 LOADED INSTALLED". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 312e30204c4f4144454420494e5354414c4c4544; result rendering "1.0 LOADED INSTALLED". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 312e30204c4f4144454420494e5354414c4c4544; result rendering "1.0 LOADED INSTALLED". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 312e30204c4f4144454420494e5354414c4c4544; result rendering "1.0 LOADED INSTALLED". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 312e30204c4f4144454420494e5354414c4c4544; result rendering "1.0 LOADED INSTALLED". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 7061636b6167652c20756e6b6e6f776e20636f6d6d616e64202269666e6565646564223a2073686f756c6420626520666f726765742c206e616d65732c2070726f766964652c2072657175697265; result rendering "package, unknown command \"ifneeded\": should be forget, names, provide, require". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The five C programs return 1.0 LOADED INSTALLED; Jim retains its unavailable ifneeded subcommand result.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
