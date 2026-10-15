# Original Jim Import source completion

Question: how does the original unmodified namespace Import procedure complete
for direct commands, one/two alias hops, empty pattern matches and no arguments;
what does replacing the inventory implementation change?

Fresh original interpreters use original core commands plus static extensions.
Both captures retain six exact setups/inputs and 18 code/result/command/prefix
reports. The mutable-info case succeeds in original Jim with no imported command;
Runtime's missing-producer refusal is a separate architecture control, not a
conformance success. The commands/prefix reports occur after Import completion.
Capture02 additionally attempts the exact retained two-alias cycle. Setup succeeds,
then the original `Jim_Eval` call does not complete within the recorded two-second
observer budget. Its raw `CYCLE_EVAL_ENTER` establishes that call boundary only;
there is no invented Tcl error or assertion of a reached inner phase. This bounded
noncompletion is separate from successful semantic rows and setup failure.

Upstream was rechecked at each actual execution (14:33 and 14:39 UTC on
2026-10-15), both matching clean local
`5bac7c99ad65864c87da513e22e2f01703fa4e03`. The manifests preserve original
source/executable/configuration/harness/library hashes, exact compiler and provider
invocations, stdout/stderr, exit/completion and unchanged 247-provider-input
closures. Original namespace.test/testing.tcl/tcltest.tcl ran 79/79 each time.
The C observer itself does not use tcltest. No original provider file was changed.

Runtime fixtures retain exact observed rows and setup/report inputs. Internal
entered-body controls exercise the genuine installed Proc/body/frame/Rest owners;
they do not establish public dispatch, publication lifetime parity or compiled
Source transport. Public original Import remains guarded until actual completion
controls and producer/consumer review support its conditional admission.
`verify.py --current-provider` checks package/fixture/provider hashes without
executing an interpreter. Rebuildable probe binaries are excluded; their captured
hashes remain mandatory when binaries are available. Evidence remains bounded to
the retained inputs, configuration and upstream capture.
