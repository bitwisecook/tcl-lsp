# Do generated NUL-bearing and plain array indices remain distinct in this program?

Proof ID: `naming.corner.array-counted-nul-index`

## Problem statement

An array contains a generated NUL-bearing index and a plain index. If an index purpose clips or normalises the full object, the two cells would collapse despite different intended values. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Do generated NUL-bearing and plain array indices remain distinct in this program?

## Exact control

```tcl
set key "k[format %c 0]tail"; set a($key) FULL; set a(k) SHORT; list [set a($key)] [set a(k)] [llength [array names a]]
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 46554c4c2053484f52542032; result rendering "FULL SHORT 2". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 46554c4c2053484f52542032; result rendering "FULL SHORT 2". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 46554c4c2053484f52542032; result rendering "FULL SHORT 2". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 46554c4c2053484f52542032; result rendering "FULL SHORT 2". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 46554c4c2053484f52542032; result rendering "FULL SHORT 2". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 46554c4c2053484f52542032; result rendering "FULL SHORT 2". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

All six return FULL SHORT and two enumerated indices. No native pointer or raw source00 relationship is established.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
