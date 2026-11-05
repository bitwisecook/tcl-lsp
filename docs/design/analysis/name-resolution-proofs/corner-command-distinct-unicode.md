# Do precomposed and combining-character command names select distinct declarations?

Proof ID: `naming.corner.command-distinct-unicode`

## Problem statement

Two command names use precomposed e-acute and e plus a combining acute. A Unicode-normalised display key could merge the declarations even when this provider selects them separately. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Do precomposed and combining-character command names select distinct declarations?

## Exact control

```tcl
set one [format %c 233]; set two "e[format %c 769]"; proc $one {} {return ONE}; proc $two {} {return TWO}; list [eval [list $one]] [eval [list $two]]
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

These authored source receipts are separate from the original public Unicode command-identity experiment. They establish no native allocation, lookup/call completion, Unicode normalisation, object cache or frame, and attach no assertion or provider result.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 4f4e452054574f; result rendering "ONE TWO". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 4f4e452054574f; result rendering "ONE TWO". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 4f4e452054574f; result rendering "ONE TWO". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 4f4e452054574f; result rendering "ONE TWO". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 4f4e452054574f; result rendering "ONE TWO". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 4f4e452054574f; result rendering "ONE TWO". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

All six captured programs return ONE TWO for the two separately generated names; no Unicode equivalence is inferred.

A linked source constructor control preserves precomposed and decomposed class name bytes as distinct source slots and retains literal multiword Jim class names. Source reporting uses the genuine selected class name input rather than an ASCII label gate.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

- [rust/tcl-compiler/src/registry_invocation/source_structure.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_structure.rs), `source_constructor_call_at`: Join genuine source class declaration/factory call geometry while retaining exact producer name units independently of original runtime identity.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs), `command_binding::source_transition_advice::source_class::tests::original_native_class_names_keep_distinct_source_units_without_ascii_report_gate` (linked): Selected class source receipts preserve distinct UTF8 precomposed/decomposed name bytes and source slots without an ASCII report filter; a literal multiword Jim class name remains complete. Source constructor geometry does not execute or normalize either name.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.
