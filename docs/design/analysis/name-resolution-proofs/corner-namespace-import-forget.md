# Does forgetting this import remove the imported callable after the first call?

Proof ID: `naming.corner.namespace-import-forget`

## Problem statement

An imported command is called and then forgotten. Retaining the original declaration forever would leave a stale callable; the forget grammar and resulting removal must be observed separately. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does forgetting this import remove the imported callable after the first call?

## Exact control

```tcl
namespace eval ::source {proc leaf {} {return SRC}; namespace export leaf}; namespace eval ::target {namespace import ::source::leaf; set before [leaf]; namespace forget ::source::leaf; list $before [info commands leaf]}
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 535243207b7d; result rendering "SRC {}". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 535243207b7d; result rendering "SRC {}". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 535243207b7d; result rendering "SRC {}". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 535243207b7d; result rendering "SRC {}". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 535243207b7d; result rendering "SRC {}". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 6e616d6573706163652c20756e6b6e6f776e20636f6d6d616e642022666f72676574223a2073686f756c642062652063616e6f6e6963616c2c20636f64652c2063757272656e742c2064656c6574652c20656e73656d626c652c206576616c2c206578706f72742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c2075707661722c207768696368; result rendering "namespace, unknown command \"forget\": should be canonical, code, current, delete, ensemble, eval, export, import, inscope, origin, parent, qualifiers, tail, upvar, which". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The captured C programs distinguish the successful imported call from the later empty command enumeration; Jim preserves the actual unsupported forget-option result.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
