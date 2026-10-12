# Original command arguments

`OriginalArguments` captures the entered Runtime operation and allocation leases for the exact original `Tcl_Obj` headers. The leases preserve allocation safety without adding native references, changing sharing, producing strings, or granting a command-specific getter purpose.

A physical consumer calls `check_for(actual)` before and after a reached effect. The actual receiver must be the same interpreter allocation that captured the arguments; an interpreter clone is accepted, while another interpreter with equal context values is refused. `check_original_for(actual, ordinal, header)` additionally requires the exact captured header for that argument. Equal spelling and an independently retained allocation do not satisfy this relationship.

`original(ordinal)` checks currency and header liveness. `bytes(ordinal)` reaches the existing selected String getter for that one original and checks the operation and header after the getter. Neither method eagerly reads later arguments. Command-specific numeric, List, Unicode, name and result workers select their own purpose and use the receiver/header checks at their actual effect boundaries.

A mismatch is retained on the actual receiver as a structured `NativeExecutionError`. Its complete first cause is returned as `CmdError`; consumers must preserve this object rather than reconstructing a refusal from a message or reducing it to `ValueError`. Mandatory cleanup still occurs after refusal, followed by the original operation check. Cleanup grants no new native argument, result or working-reference role.

The software controls in `interp::native_arguments::original_receiver_tests` cover same-receiver clones, distinct receivers with equal contexts, distinct headers with equal spelling, unchanged native argument reference counts, retained result identity, and an existing first Host cause. These controls test the implementation contract. They do not claim native C/Jim callback or foreign-interpreter behaviour. The native evidence boundaries are documented in [the original Host publication proof](../analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md).
