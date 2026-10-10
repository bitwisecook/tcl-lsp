# Does an inherited destructor run after ordinary destruction and after inherited-constructor failure, with the failed command removed?

Proof ID: `naming.tcloo.inherited-destructor-cleanup`

## Problem statement

A child with an inherited constructor and destructor can fail during construction. Cleanup must identify the inherited destructor and remove the failed object rather than preserve a successful allocation.

## Question

Does an inherited destructor run after ordinary destruction and after inherited-constructor failure, with the failed command removed?

## Provider answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | unsupported | TclOO is unavailable; the semantic cases do not execute. |
| tcl8.5 8.5.19 | unsupported | TclOO is unavailable; the semantic cases do not execute. |
| tcl8.6 8.6.18 | observed | superclass-failure 1 BOOM CLEAN 0 |
| tcl9.0 9.0.4 | observed | superclass-failure 1 BOOM CLEAN 0 |
| tcl9.1 9.1.0 | observed | superclass-failure 1 BOOM CLEAN 0 |
| jim 0.84-9-g5bac7c9 | unsupported | TclOO is unavailable; the semantic cases do not execute. |
| bigip not recorded | not-tested | No appliance observation for this lifecycle question. |

## Conclusion

C Tcl 8.6.18, 9.0.4 and 9.1.0 run the inherited destructor in both controls and remove the failed object command.

## Scope

One-level TclOO superclass or single mixin, ASCII names and bodies, CLI file-script input. These observations do not prove general multiple inheritance, filters, next, namespace binding, traces, receiver mutation or compiler completion contracts.

## Evidence and reconfirmation

The [exact source](../../../../rust/tcl-compiler/tests/data/native_original_lifecycle_providers/source.tcl), [provider receipt](../../../../rust/tcl-compiler/tests/data/native_original_lifecycle_providers/receipt.json) and peer stream files retain the complete invocation. The receipt binds the exact source SHA-256 `b772996e165014327448be749f825ec35be0c6c18056bb23956ebb8e74dc889b`, executable hashes, input channel, exit status and stream hashes. Each process exits 0 with empty stderr.

Run `tclsh rust/tcl-compiler/tests/data/native_original_lifecycle_providers/source.tcl` using the exact intended provider executable. Compare the complete raw streams and executable hash to the receipt. This is file-script input; it supplies no counted native EvalEx or document-decoding comparison.

The native observation is distinct from the Rust source evaluator contract. Linked Rust selectors independently verify source ownership and completion; these native rows do not grant physical frames, argv references or compiler admission.

The current Jim control records patchlevel `0.84-9-g5bac7c9`, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03` and its own executable hash in [the separate tagged receipt](../../../../rust/tcl-compiler/tests/data/native_original_lifecycle_providers/tagged-jim/receipt.json). The initial invocation remains available with its original executable hash; its patchlevel was not recorded.
