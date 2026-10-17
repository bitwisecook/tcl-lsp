# Source basis and scope

This packet implements the existing catalogue unit
`lsp-server-code-action-wire-source-version` (original index724). It extends
neither Core action admissibility nor Native execution. Root owns integration,
counts and ordinary validation. No build, test, listing, trace or gate was run.

The original `Backend::code_action` read one DocumentState and asynchronous
analysis/settings/export inputs, then called `lift_code_actions` with only its
URI. That lift emitted WorkspaceEdit.changes. The ordinary path now retains
actual original document/version/raw source, the existing DB/Fresh source-input
receipt, both actual source-read and analysis provider admissions, input/factory
currency and the same captured export snapshot/generation. Its final shared
transaction rechecks those actual owners before lifting versioned document edits.
The existing Core workers/report and all their edit guards are unchanged.

The existing private workspace-symbol currency kernel is reused. The new
OriginalCodeActionSource is a transport/purpose adapter, not a second source,
Registry, provider or edit resolver. Final document equality includes the
existing canonical analysis-normalisation view beside exact original raw text;
bare-CR normalisation cannot substitute a different source. Strict request
positions use the document's actual LineIndex with a UTF16 roundtrip.

The installed vscode-languageclient10.1.1 converter discards
TextDocumentEdit.version in protocolConverter.js:840-864. Consequently the
shared Node/browser middleware must not return its converted automatic edit for
ordinary actions. It retains the actual provider TextDocument, original complete
source/version, existing session and existing full per-resource Tcl settings
fingerprint BEFORE the provider await. Server intent data must equal that
original capture. It renders the actual supplied single-document edit plan in
stable source/LSP order, refusing invalid Unicode boundaries and overlap, then
uses a bounded opaque one-use local receipt. JSON metadata is not itself an
editor receipt or an edit-subject issuer.

The actual `replaceDocumentFromSnapshot` writer is reused without modification.
Its source/session/dialect checks and TextEditor.edit remain the sole local
write sink. An additional admission checks the ORIGINAL whole resource settings
and settings-event invalidation across all awaits. Actual post-write snapshots
returned by that writer own optional rename continuation. The old active-editor
three-number command cannot mint a rename intent. No future source version is
invented in the adapter. The shared writer's existing strict accepted-version
check is retained unchanged. Node/browser Generate Docstring convenience callers
also consume the checked staged action instead of applying a version-less edit.

The explicit existing f5-bigip server branch keeps its old worker, lift and
command routing; an explicit legacy response marker leaves that dedicated editor
adapter unchanged. No BIG-IP migration is claimed. Unknown/older responses with
missing intent metadata are not silently treated as this selected legacy route.

All original test source/expectations are preserved. Existing synthetic ordinary
DocumentState::new fixtures have versionNone and cannot prove versioned edit
publication. Existing editor tests that require a converted CodeAction.edit may
expose the old version-less API assumption; those assertions have not been
weakened or rewritten. Four new actual Server/source-owner controls and three
host controls are included, unexecuted. Their source checks do not establish
typecheck, efficacy, lifecycle execution, performance or passing gate results.

Server full-input/provider/index currency is a publication-time transaction.
The later client applies the retained original plan under actual local source,
session and resource-settings currentness. This does not claim a distributed
atomic transaction over remote project/configuration mutations after publication.

The canonical contract extends the existing workspace-symbol source/input
currency row and adds its actual new self-call consumer of the unchanged shared
transaction; private adapter methods remain implementation narrative. No public
visibility or checker predicate changes, extra annex or automatic count are
introduced. The separate durable proof specifies the API boundary.
