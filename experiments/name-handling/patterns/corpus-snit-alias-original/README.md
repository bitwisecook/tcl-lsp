# Real Snit prefix and child-alias composition

The final fixed composition passes its positive and changed-prefix controls on
C Tcl 8.5.19, 8.6.18, 9.0.4 and 9.1.0. The unmodified retained Tcllib helpers
and real Snit 2.3.4 package supplied the prefix and installed the child alias.

The original `mymethod` prefix and stored alias target are both
`::snit::RT.CallInstance ::AliasProbe::Snit_inst1 echo {captured arg}`.
Calling `echo` in child `aliasChild` with one `arg tail` operand returns
`fixed {captured arg} {arg tail}`. Overwriting that same child alias through
`snitDictLink` preserves the newly captured prefix, changes its method word to
`missing method`, and the actual later call errors with an unknown-subcommand
result. Alias creation succeeds independently of target-method availability.
Both explicit final cleanup operations return code 0 on those four providers.

C8.4.20 refuses original package setup: its package-index loader reports the
multi-version `package vsatisfies` syntax as unsupported and cannot load
`interp 0.1.3`. Pinned Jim `5bac7c99ad65864c87da513e22e2f01703fa4e03`
refuses the original package machinery with the recorded `package require`
argument error. Their positive and adverse calls remain unreached. No alternate
package requirements, emulated Snit or adapted interpreter API was tried.

This is an authored caller composition over exact original helpers at Tcllib
`8a2e288fa02363639784c2f0a8ea6245896c2884`, `interp.tcl:66–81`.
It does not execute an extracted complete corpus application. The positive
uses supported simple object/method/child names and spaces only in argument
values; it establishes no arbitrary-name, multiword-method or framework parity.
Native F5, broader lifecycle and prevalence questions remain pending.

Earlier inputs are retained separately in `initial-child-path-refusal/`,
`quoted-child-path-refusal/` and `authored-spaced-method-refusal/`. They record
caller/setup failures for the authored spaced-child variants, then an actual
single-word spaced-method refusal and uncaught object cleanup error. They are
not positive controls. The first capture labelled C8.4 `capture-failure` because
its package warnings reached stderr; its raw setup refusal remains retained.
Historical command paths referred to the input at capture time; each directory
preserves those exact input and runner bytes independently of the final input.

See [question](question.md), [input](probe.tcl), [runner](run.py),
[final measurements](result.json) and [source provenance](source.json).
All eight retained package-source hashes match before/after. Each available
provider ran the final input once in a fresh process with a 30-second bound.
