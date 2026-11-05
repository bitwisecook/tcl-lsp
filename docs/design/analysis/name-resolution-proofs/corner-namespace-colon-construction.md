# What current, parent and tail names result from this repeated-colon namespace construction?

Proof ID: `naming.corner.namespace-colon-construction`

## Problem statement

A namespace is constructed using repeated colons and then reports current, parent and tail. Publication geometry and reporting text might canonicalise differently, so the command-name recipe cannot simply be reused. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

What current, parent and tail names result from this repeated-colon namespace construction?

## Exact control

```tcl
namespace eval ::n::::child {list [namespace current] [namespace parent] [namespace tail ::n::::child]}
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 3a3a6e3a3a6368696c64203a3a6e206368696c64; result rendering "::n::child ::n child". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 3a3a6e3a3a6368696c64203a3a6e206368696c64; result rendering "::n::child ::n child". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 3a3a6e3a3a6368696c64203a3a6e206368696c64; result rendering "::n::child ::n child". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 3a3a6e3a3a6368696c64203a3a6e206368696c64; result rendering "::n::child ::n child". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 3a3a6e3a3a6368696c64203a3a6e206368696c64; result rendering "::n::child ::n child". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 3a3a6e3a3a3a3a6368696c64203a3a6e3a3a206368696c64; result rendering "::n::::child ::n:: child". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

C8.4–9.1 report current ::n::child, parent ::n and tail child; Jim reports ::n::::child, parent ::n:: and tail child. These are script reports, not namespace token grants.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
