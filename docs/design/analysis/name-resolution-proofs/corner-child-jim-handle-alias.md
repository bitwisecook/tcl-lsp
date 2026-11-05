# Which prefix argv does the Jim child-interpreter handle alias enter?

Proof ID: `naming.corner.child-jim-handle-alias`

## Problem statement

A Jim child handle installs an alias with a baked PREFIX. C interp grammar does not establish that this handle syntax exists or that it preserves the same argv layout. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Which prefix argv does the Jim child-interpreter handle alias enter?

## Exact control

```tcl
set child [interp]; $child alias a list PREFIX; set result [$child eval {a ARG}]; $child delete; set result
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727020636d64203f617267202e2e2e3f22; result rendering "wrong # args: should be \"interp cmd ?arg ...?\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.5 8.5.19 | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727020636d64203f617267202e2e2e3f22; result rendering "wrong # args: should be \"interp cmd ?arg ...?\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.6 8.6.18 | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727020636d64203f617267202e2e2e3f22; result rendering "wrong # args: should be \"interp cmd ?arg ...?\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl9.0 9.0.4 | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727020636d64203f617267202e2e2e3f22; result rendering "wrong # args: should be \"interp cmd ?arg ...?\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl9.1 9.1.0 | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727020636d64203f617267202e2e2e3f22; result rendering "wrong # args: should be \"interp cmd ?arg ...?\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 50524546495820415247; result rendering "PREFIX ARG". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

Jim returns PREFIX ARG; the five C captures preserve the distinct interp grammar errors for this handle syntax.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
