# Pattern 11: real Snit prefix captured into a child alias

Question: does the retained Tcllib `interp::snitLink` preserve a real Snit
`mymethod` prefix, method/fixed-argument word boundaries and child identity?
Does replacing that alias through `snitDictLink` with a missing method still
create a prefix while refusing the later call?

The source is `tcltk/tcllib` commit
`8a2e288fa02363639784c2f0a8ea6245896c2884`,
`modules/interp/interp.tcl:66–81`, already retained in the corpus catalogue.
The exact helper source is unmodified and loaded through its real package
index. The real Snit package supplies `mymethod`; no surrogate implements it.
The fixed `AliasProbe` type, simple object/method names, captured
argument and missing-method change are an authored caller composition, not an
extracted original application or an exact original complete invocation.

The positive installs one `echo` child alias whose stored target must
match the prefix returned in the same genuine Snit instance method context.
It invokes that alias with one `arg tail` word. The adverse overwrites that
same child alias through the retained `snitDictLink` with `missing method`,
checks the changed stored prefix, then measures the actual invocation error.
Alias creation alone does not prove the target method exists or will complete.

Run the same input in each available pinned C Tcl shell and pinned Jim.
Record package/setup refusals separately and leave the actual calls unreached;
do not rewrite package requirements, adapt namespace/interpreter APIs, or
substitute a list-only reduction for a refused original framework setup.
One process per provider; no Tcltest, compiler, Cargo or F5 execution.
Results cover only this fixed composition. Framework parity, prevalence,
other Snit lifecycle properties and native F5 remain unanswered.
