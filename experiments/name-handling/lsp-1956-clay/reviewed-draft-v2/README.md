# Clay v2 development checkpoint — UNBUILT / UNVERIFIED

Work stopped under the user's stop directive. The latest clarification allows
all current reviewed work to be pushed. Only v2 was restored; older stashes
remain preserved. No Cargo, generator, new oracle run, or new feature work was
performed for this handoff.

The exact source patch has SHA-256
`afaf71a8b637ecb0bf3374fe73495bf28e507fe47050e9136f7d9edc31b4b1cc`.
Its original base is `cc6262bed73765f88c2e563465ea6fe996cd939e`.
The manifest retains all 18 exact resulting source hashes,
the initial review patch, and the preserved v2 stash. Restoration onto
`68df69c92cd2a9116c866b1fce14e592bd476d73` left every recorded source hash
identical. The patch is an archived draft artifact; it is not an instruction
to apply it again over the development checkpoint.

Four compiler tests, two registry tests, two language-server controls, and one
exact HTTPD corpus assertion are authored but **UNRUN**. Compiler Clippy,
code generation, `make rust-check`, and `make prep-pr` are also **UNRUN** for
this Clay draft. The [generated-file plan](../generated-file-plan.md) records
the remaining snapshots and gates. No runtime success is inferred from static
review or a clean patch application.

The draft uses registry-owned form/member data and existing original source
owners. It is limited to class inventory and audited lowercase method bodies.
Inheritance, transformed/generated members, installed factories, receiver
entry, and rename authority remain incomplete or unavailable. Native package
loading is not inferred from registry presence.

The existing [source/native audit](../README.md) and
held-server protocol receipt retain the
limits: the baseline HTTPD file has no class symbols, and even the plain TclOO
`my connect` definition control returns null. That provider gap remains
unresolved. Native C Tcl 8.6/9.0/9.1 load Clay and HTTPD; the
[portable package probe](../../../../evidence/tcl/lsp-1956-clay-package-loading.md)
records the actual TclOO availability in each tested host and Jim's Clay
header rejection. It establishes no universal Tcl 8.5 language limitation.

#1956 remains open. The independent published seed-map repair does not close
#2243: both exact IMAP4 `argc` reads still lack the required original-read
coverage. Stashes and frozen seed/audit binaries remain preserved.
