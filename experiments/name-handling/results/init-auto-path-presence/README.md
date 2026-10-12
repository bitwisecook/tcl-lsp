# InitAutoPath command presence

Physical C Tcl controls report `::tcl::InitAutoPath` absent in 8.4.20, 8.5.19, 8.6.18 and 9.0.4, and present in 9.1.0. The latest built Jim reports it absent (0.84-9-g5bac7c9). The exact query, stdout, stderr and invocation records are retained here.

Both VM and runtime register this command only when the shared native core bootstrap protocol grants `registers_core_init_auto_path`. VM selects that protocol from its actual native engine profile; runtime selects it from the constructor's native invocation dialect. Its registry surface is Tcl9.1. The registration predicate is shared metadata, rather than relying on stock registration to filter an unavailable command spec.

The focused VM and runtime tests pass. Each verifies actual core command presence for all five C releases against the original C query and always checks the positive and negative release cases even when an oracle is unavailable. VM also checks retained native registration-token consistency for C. For Jim, the native core registration table is compared with the real Jim command-presence query: VM uses its original registered-command token owner and runtime its command backing inventory. Bare Jim `info commands` in the emulated cores delegates to a scripted `namespace info` command that is not loaded in these constructor-only tests; that independent distribution prerequisite grants no registration waiver.

These tests cover command registration. They make no claim that complete Tcl distribution initialization or upstream naming suites pass. `test-receipt.json` identifies the focused commands and source hashes; the full test logs are retained alongside the C and Jim observations.
