# Registry name adapters

`probe.tcl` measures command words independently of registry descriptors. Each
capture uses the source-built shell selected by the shared reference-toolchain
manifest: C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0, and pinned Jim
`5bac7c99ad65864c87da513e22e2f01703fa4e03`. `capture.json` records shell and input
hashes, patchlevels, exit codes and output hashes. These custom programs do not
load Tcltest or substitute any upstream test/library file.

All five C shells accept complete leading and interior colon runs as namespace
separators for the measured procedure words. Jim accepts the measured leading
runs but preserves the interior runs: `adapter:::f` and `adapter::::f` fail while
`adapter::f` succeeds. Both issuers reject a lone leading colon and a lone
trailing colon in the measured words. These observations do not establish
unmeasured name extents, registration identity, namespace incarnation or shadowed
bindings.

The measured private `tcl::dict::create` worker resolves on C8.5–9.1, including
the wider separator spellings. C8.4 and Jim report no such worker. Written-source
private-namespace classification is a metadata diagnostic; it uses the shared
written component owner and the selected registry's public-package exclusion.
Its explicit Jim boundary declines these C implementation-namespace suggestions.
It does not issue a native binding or editable occurrence receipt.

The measured `if`, `::if`, `:::if` and `::::if` all execute their script body on
all six shells. `:if` and `::::if:` fail. The dialect scanner's bounded source
hint uses these written-name rules; it does not prove that a builtin is current
or that a conditional branch runs.

Selected registration descriptors and analytical constructed keys have a
separate contract. The registry's native compiler/TclOO/string adapters remove
exactly one root marker using `naming::unroot_rooted_key`; they retain every
remaining byte and all independent binding/hook prerequisites. Math-function
metadata uses `key_holder_and_tail` and `key_segments`, preserving literal-colon
simple names. Reference-body admission matches exact descriptor labels after
one optional root marker and refuses a mismatching implementation name.
Constructed-key controls express these software contracts; written-word oracle
results do not turn a rendered key into a native identity.

`source-inputs.json` seals the frozen Rust/manifest input set used for scoped
Registry execution. Test logs and `validation.json` record the executed filters,
results and omissions. No full-workspace gate is established by this capture.
The original `named_ensemble_grammar` operand-layout classifier requires its own
purpose-correct original-word validation: an unknown head cannot safely inherit
the public ensemble's argument offset. This experiment establishes no repair or
acceptance for that classifier.

`constructed-actual.tcl` joins each actual `namespace current` result with the
simple name of a procedure created and invoked in that namespace. On all six
shells, `::tcl::mathfunc` plus `:f`, and `::tcl::mathfunc:` plus `f`, produce the
same displayed string. C refuses written relookup of that display in this
fixture; Jim reaches the second namespace's `f`. The shared analytical split
therefore cannot recover a unique global math-function identity from that
string. The metadata controls require refusal rather than inventing a private
inverse. `constructed-display.tcl` is a separate constant-string control; its
hardcoded identical strings establish no namespace or procedure ownership.

The scoped filters passed 129 distinct Registry controls. The first mathfunc
run passed thirteen and failed the new control's incorrect assumption that an
ambiguous display uniquely identified `mathfunc/:f`. The corrected fourteen
controls retain the shared owner's refusal. Both logs remain separate.
`source-refresh.json` identifies exactly two test-only changes between captures:
the Registry expectation and an unrelated Compiler import repair. Registry
production inputs remained unchanged. `source-current.json` seals the corrected
capture. Compiler/Core/CLI/VM/Runtime integration and full gates are outside this
Registry result.
