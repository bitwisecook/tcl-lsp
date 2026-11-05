# Does the namespace path provider precede the global command in this program?

Proof ID: `naming.corner.command-path-precedence`

## Problem statement

A namespace path contains a provider that competes with the global command. Local-before-root reasoning is insufficient unless the path creation door exists and its ordered lookup has been measured. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does the namespace path provider precede the global command in this program?

## Exact control

```tcl
namespace eval ::n {}; namespace eval ::path {proc leaf {} {return PATH}}; proc ::leaf {} {return ROOT}; namespace eval ::n {namespace path ::path; leaf}
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | unsupported | Process exit 0; guest completion 1; captured result hex 626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368; result rendering "bad option \"path\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 50415448; result rendering "PATH". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 50415448; result rendering "PATH". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 50415448; result rendering "PATH". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 50415448; result rendering "PATH". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 6e616d6573706163652c20756e6b6e6f776e20636f6d6d616e64202270617468223a2073686f756c642062652063616e6f6e6963616c2c20636f64652c2063757272656e742c2064656c6574652c20656e73656d626c652c206576616c2c206578706f72742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c2075707661722c207768696368; result rendering "namespace, unknown command \"path\": should be canonical, code, current, delete, ensemble, eval, export, import, inscope, origin, parent, qualifiers, tail, upvar, which". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

C8.5–9.1 return PATH; C8.4 and Jim preserve the actual unavailable path-option errors.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
