# Does package present distinguish this generated NUL-bearing package name from Pkg?

Proof ID: `naming.corner.package-counted-nul-key`

## Problem statement

A package is provided under a generated NUL-bearing name and queried both by that name and its prefix. Package key selection is independent of command publication and the present subcommand may be unavailable. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does package present distinguish this generated NUL-bearing package name from Pkg?

## Exact control

```tcl
set name "Pkg[format %c 0]Tail"; package provide $name 1.0; list [package present $name] [catch {package present Pkg}]
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 312e302031; result rendering "1.0 1". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 312e302031; result rendering "1.0 1". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 312e302031; result rendering "1.0 1". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 312e302031; result rendering "1.0 1". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 312e302031; result rendering "1.0 1". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 7061636b6167652c20756e6b6e6f776e20636f6d6d616e64202270726573656e74223a2073686f756c6420626520666f726765742c206e616d65732c2070726f766964652c2072657175697265; result rendering "package, unknown command \"present\": should be forget, names, provide, require". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

C programs report full-name version1.0 and plain-name absence; Jim preserves its unavailable present subcommand result, so no Jim key-equivalence conclusion follows.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
