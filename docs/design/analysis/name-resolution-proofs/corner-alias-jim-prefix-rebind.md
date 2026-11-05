# Does this Jim alias prefix call the replacement target after rename and redefinition?

Proof ID: `naming.corner.alias-jim-prefix-rebind`

## Problem statement

A Jim alias prefix targets a procedure that is renamed and replaced. Its creation grammar and target retention cannot be inferred from C interp alias, even when both aliases have similar reporting names. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does this Jim alias prefix call the replacement target after rename and redefinition?

## Exact control

```tcl
proc target args {return OLD}; alias a target PREFIX; rename target original; proc target args {return NEW}; a ARG
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022616c69617322; result rendering "invalid command name \"alias\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.5 8.5.19 | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022616c69617322; result rendering "invalid command name \"alias\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.6 8.6.18 | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022616c69617322; result rendering "invalid command name \"alias\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl9.0 9.0.4 | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022616c69617322; result rendering "invalid command name \"alias\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl9.1 9.1.0 | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022616c69617322; result rendering "invalid command name \"alias\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 4e4557; result rendering "NEW". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

Jim returns NEW; C8.4–9.1 retain the missing alias command errors. This creation door is distinct from the C interp alias control.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.
