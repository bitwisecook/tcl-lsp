# Does this forced import follow its original procedure after that procedure is renamed?

Proof ID: `naming.corner.namespace-import-force-rename`

## Problem statement

An import replaces a local command and its source procedure is then renamed. Resolving by the source's old spelling or preserving the imported command's origin leads to different calls and origin reports. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does this forced import follow its original procedure after that procedure is renamed?

## Exact control

```tcl
namespace eval ::source {proc leaf {} {return SRC}; namespace export leaf}; namespace eval ::target {proc leaf {} {return OLD}; namespace import -force ::source::leaf}; rename ::source::leaf ::source::moved; list [::target::leaf] [namespace origin ::target::leaf]
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 535243203a3a736f757263653a3a6d6f766564; result rendering "SRC ::source::moved". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 535243203a3a736f757263653a3a6d6f766564; result rendering "SRC ::source::moved". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 535243203a3a736f757263653a3a6d6f766564; result rendering "SRC ::source::moved". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 535243203a3a736f757263653a3a6d6f766564; result rendering "SRC ::source::moved". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 535243203a3a736f757263653a3a6d6f766564; result rendering "SRC ::source::moved". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022736f757263653a3a6c65616622; result rendering "invalid command name \"source::leaf\"". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The five C programs call SRC and report ::source::moved; Jim preserves the original missing source::leaf error instead of borrowing C token behaviour.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
