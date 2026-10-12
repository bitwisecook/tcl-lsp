# W210 versioned fixture boundary (temporary source review)

Read-only comparison; no new build, trace or compiler change. Retained-token
forwarding passes the owner regression; Tcltest remains 6pass/3fail. The native
fixture's post-forwarding factory/advice guard results have not been measured.

The passing regression (`script_binds.rs:619`) uses plain
`Analyser::new().analyse("proc p {arg} {puts $missing}", "tcl")`.
Its analyser has no explicit entry (`analyser/state.rs:1637`). Logical input
routes to `document_lexical_realm_with_resolved_input` (`realm.rs:734`), which
retains SuppliedSource metadata and logical_source_input at 747–751.
This is source/header advice, not a native interpreter activation.

The failing direct control (`diagnostics/tests.rs:13286`) uses `tcl8.4`, empty
formals, package loading/import and a Tcltest provider (`provider_fixtures:153`).
`provider_fixtures.rs:144–148` explicitly constructs an entry with loaders,
versioned invocation dialect and Direct/ScriptCode compilation mode, then
`SourceAnalysisEntry::default()` supplies no native_entry, Logical or complete input.
Default metadata is Standalone (`registry_invocation/metadata_context.rs:17`).
Do not call this fixture's loader attestation an actual Native execution entry.

This is the first concrete ownership difference: the default Logical path
retains the complete input; the explicit provider path passes entry.options()
directly to analysis (`realm/declared_source.rs:21–28`). That option retains
the entry's metadata owner (`source_analysis_entry.rs:453–470`).
The final `.with_resolved_analysis_input` changes only the outer realm's
source_input (`realm.rs:152–158`), not the entry metadata or point snapshots.
No profile label, provider tag or outer-input equality reconstructs that owner.

The factory (`declaration_preview.rs:430–445`) first creates request metadata
from the current analysis input, then requires binding/site/image and checked
source purpose. The source-purpose join (`origin_inventory.rs:69–92`) asks
the retained binding for metadata matching that complete request. A Standalone
owner and a supplied full-input owner are distinct purposes. The selected point
owner and exact failed join in the post-fix fixture remain unobserved.

Further differences are release/dialect (unversioned versus C8.4), native
formal topology versus Logical parameters, and empty versus one-name formals.
The native branch additionally checks original row/config/vector, target,
header shape, original parameter-word policy/topology and written literal body
(`declaration_preview.rs:454–483`, `1507–1580`). Their verdicts are unmeasured;
the passing Logical regression does not validate that branch.

Concrete next question: for the SAME direct C8.4 control after binding
forwarding, does factory request metadata exist, and does source-purpose join
refuse because the retained proc-header owner is Standalone while the request
has supplied complete input? Record only request/point owner tags, original-site
presence, input equality and source-purpose result. If that is the refusal,
repair the explicit fixture's input manufacturer with its genuine resolved
input; retain invalid-owner refusal and all Native/formal/body guards.
