# Additional naming-consumer review

Review base: `work` at `2d456966b`, 15 October 2026. This is an additive
source/caller review beyond the finite330 roster. No Rust build, test, native
oracle execution, generated drift check or repository gate ran in this review.
Concurrent implementation changes need their own final source binding and
execution receipts. Root owns Git, shared tracking and issue actions.

## Actionable consumers

| Call site | Input, purpose and current owner | Required work and controls | Disposition |
| --- | --- | --- | --- |
| `rust/tcl-registry/src/value_transfer/reference_body.rs::variable_name` (649), called by `expression_text` (579) | Reference-body sandbox source; private first-close `${…}` and ASCII/colon scanning decides whether expression variables can reach outside the local frame. `Lexed::new` (315) explicitly selects the hook host's fixed plain-Tcl default grammar. | Replace the private variable parser with `tcl_lexer::word_parts::scan_var_ref` under the actual hook-host configuration. Verify that host before changing the public derive/config contract: `ContextDependency::TclProfile` alone does not prove the hook language varies. Keep malformed/non-reference `$`, arrays, nested braced names, lone colons and complete consumed extent distinct. Loader production caller is `rust/tcl-spectcl/src/loader/reference.rs::derive` (116); it passes command and definition, without a document configuration. | Root claimed implementation; pending execution. |
| `rust/tcl-vm-wasm/src/lib.rs::tcl_eval` (134–146) | Host `(ptr,len)` source bytes are converted with `String::from_utf8_lossy` before `run(&str)`. Distinct opaque bytes in literal command/variable names can become the same replacement text. | Use actual supported source-byte ingress, or reject unsupported UTF-8 explicitly before execution. Preserve byte-valued completion output independently. Add discriminating malformed-source and literal replacement-character controls; existing module tests exercise completion output only. | Migration required; communicated to VM owner/root, no edit by auditor. |
| `rust/tcl-spec-studio-wasm/src/archive.rs::extract` (154) | Archive Tcl source members are decoded lossily into `{name,text}` before package/spec inference; `Entry::text` explicitly documents this policy. | Retain original source bytes at a supported owner or report unsupported encoding in the existing skipped-member channel. A valid UTF-8 literal replacement character must remain distinct from malformed name bytes. Do not infer original naming identity from lossy imported text. | Migration required; communicated to root/VM owner. |
| `rust/tcl-vm/src/cmd_namespace.rs` Jim `canonical` arms | VM owner reports no-argument arm creates a fresh String from `namespace_object_bytes(current_ns_id)` and named arm derives current reporting bytes through `jim_canonical_namespace_bytes(None,name)`. Runtime already retains actual Jim namespace object through `jim_current_namespace_object` and `jim_canonical_namespace_object`. | Preserve held original identity, unresolved current context, name-first absolute short circuit, original append and getter order through the existing owner. VM owner/root coordinate after current source freeze. | VM-owner finding; migration required, not independently executed by auditor. |
| `rust/tcl-compiler/src/analyser/handlers.rs::dynamic_command_name_may_equal` (573), consumed by `static_provenance_command_is_trusted` (9449,9466,9480) | Private first-close `${…}` scan and copied colon-run canonicaliser exclude command-table mutations from static provenance. Input comes from reconstructed `subject_word`, without original word/configuration. Tcl9 `${a{b}}` leaves a false fixed `}` suffix in this matcher although the complete reference can produce an arbitrary name. | Repair the authentic original word/configuration/purpose transport through shared source-name pattern owners. Existing `dynamic_names` source APIs retain VariableRoot/NamespaceName purposes; command mutation may need its own purpose extension. Preserve finite same-tail/constructed-key controls; add nested-brace, brace-literal sigil, mutation and unchanged fixed-fragment positives. | Migration required; root and Compiler owner notified, no auditor source edit. |
| `rust/tcl-compiler/src/dynamic_names.rs::name_word_pattern` (368+) | Shared source-pattern owner still privately scans bare variable and bracket syntax. `$a($b)` retains fixed parentheses for NamespaceName purpose; bracket counting can stop at a literal `]` inside a braced command argument. These can rule out a possible dynamic name. | Use actual configured source substitution components/retained lexical geometry at this existing owner. Missing/malformed/unrepresented portions remain possible, not a fixed fragment. Do not migrate the analyser helper into this owner without repairing this geometry. | Migration required; root notified. |
| `runtime/rust/src/namespace.rs::{resolve_fqn,which_command}` (2296,3337) | Runtime owner identified duplicate namespace/simple reporting joins beside existing per-policy `command_fqn` (1497). Independent review confirms actual `home_of` selection and separate generation joins in `import_query_binding`, `find_command_id` and builtin checks. | Delegate reporting to the existing command FQN owner, retaining one selected home and separate generation/lifetime. Stale trace-key comments need correction. This does not establish a token-identity defect from rendered bytes alone. | Shared-owner migration required; Runtime owner/root coordinate after freeze. |

Source peer review of root's subsequent `reference_body` draft, SHA-256
`80e5abc10a27218fc6ccdd02ff206f563ff2bd6d9f871998b90acfd7f4547290`,
finds retained `Lexed.config`, `scan_var_ref` and raw-index `decompose` with
explicit recursive part checking appropriate for the bounded change. The
actual raw-index braces remain literal while nested substitutions are
inspected; ParseError and unsupported expression parts decline. Consumer depth
32 rejects nested parts before the compatibility scanner's depth64 truncation.
No blocking source finding remains for that hunk. This supplies no executed
oracle or Rust acceptance.

Peer review of Compiler's Installed coverage change finds the separate actual
Installed snapshot and prepared-CFG selector preserve declaration-only API
purposes, complete inventories, issuer/currentness and exact source/allocation
mapping. Param-trait command-body enumeration needs source order: arena
`all_parts()` visits root lists before index child lists, while this consumer
mutates alias state. In `return "$array([set alias $v])[set $alias]"`, the
earlier array-index command must precede the later root command. Compiler owner
is sorting genuine body spans and adding both-order controls. Acceptance remains
pending root's fresh build/execution.

## Inspected adapters and domain boundaries

These rows establish the inspected caller purpose, not whole-surface acceptance.
Existing test names below are authored controls; none ran in this review.

| Call site | Actual shared route and representation/context boundary | Evidence and remaining validation |
| --- | --- | --- |
| `codegen/backend.rs::BytecodeBackend::{lower_function,lower_module}` | The only `Backend` implementation delegates to existing emitter owners. Whole-module emission receives actual IR+CFG+Registry; single-function emission cannot invent declaration ownership. | Current trait/callers read. `backend_lower_module_retains_original_declaration_order_and_body_identity` remains required. |
| `codegen/emitter/mod.rs::codegen_procedures` | Procedure map keys transported into function/provenance maps are actual issued body IDs, despite the local variable name `qname`; `ProcedureProvenance.name` is separate presentation. Original source/namespace/formals remain attached. | Historical `codegen-consumers.md` String-map claims are superseded here; final docs need current typed terminology. No executable acceptance inferred. |
| `codegen/mod.rs::CodegenCtx::command_binding_identity` (615) | Original static command word comes from retained invocation tokens; selected descriptor matching delegates `command_descriptor_labels_match`; compiled namespace context and binding guard remain independent. | Historical trim-based comparison is removed. Existing selected/replaced/foreign-context controls still required. |
| `codegen/wasm/backend.rs` procedure dispatch (311–313, 2640+) | `proc_indices`, `direct_procs`, bound/source-function joins use `ProcedureId`; `qualified_name` at report/emission calls is a label. | Body-ID collision/currentness and actual WASM linker checks remain required. This is general/native WASM emission, separate from the VM WASM host. |
| `codegen/wasm/pipeline.rs::compile_wasm`; `wasm/native_emit.rs::emit_function` | Existing plan registry and typed native IR select executable operations; original argv/publication and source ownership remain separate. | Registered leaf/mixed/native selectors are parts of this pipeline, not additional independent Tcl name resolvers. No fresh backend execution. |
| `bpf-tcl-ir/src/frontend.rs::compile_module` | Actual `bpf_registry` and `source_declaration_kind` select sealed DSL declarations; the BPF event namespace is separate from iRules. | Follow through `source.rs`/lowering for unexamined DSL consumers; this entry review does not certify them. |
| `bpf-tcl-codegen/src/lib.rs` and eBPF emission | Actual implemented target is eBPF. The comment that typed IR could support WASM does not register a second BPF target. | ELF/BTF names are artifact symbols; typed map/instruction relocations remain separate from Tcl command lookup. Deeper emission checks remain unexecuted. |
| `tcl-lsp-server-wasm/src/lib.rs::LspWorker::new` | `Backend::with_store` supplies the same server providers over `MemoryStore`; `normalise_request_uris` and capability shims are protocol adapters. | Browser host source/currentness reaches backend owners. No local Tcl name resolver found in this entry; complete browser regression remains unexecuted. |
| `runtime/rust/src/host_wasm.rs::{BrowserHost,WasiHost}` | Host capabilities, environment and I/O adapters; WASI stdlib VFS supplies source files to the existing interpreter. | No Tcl namespace lookup here. Native source/library startup and profile tests are separate obligations. |
| `tcl-engine-wasm/src/session.rs::{define_command_with_context,replay_command,replay_remove}` | Counted original name bytes enter `command_receipts`; actual replay context and root interpreter owner stay separate from public qualified spelling. | `command_receipts.rs::replay_input` validates purpose and current interpreter identity. No display-name inverse introduced at this inspected seam. Native object callbacks explicitly refuse across WASM. |
| `editors/vscode/src/nameGeneration.ts::packageScaffoldNamespace` and `scaffold.ts` | Generated `scaffoldNamespacePolicy.json` owns a deliberate, lossy new-name authoring convention. It does not resolve existing Tcl commands. | Existing scaffold tests and generator drift are required. Splitting its policy separator is not a competing runtime resolver. |
| `tcl-debugger/src/backend.rs::{record,evaluate}` | Recording explicitly selects Tcl9.0 for VM+compiler; replay `evaluate` uses that same grammar through `whole_var_ref`, rejects unsupported indices and requires exactly one captured reporting label. | The hardcoded profile agrees with the current fixed launch API. Do not turn reporting labels into physical variable identity. Existing replay name/current-frame/collision controls remain required. |
| `tcl-registry/src/native_argument_role_hook.rs::native_argument_role_hook` | `::` separates authored hook ID fields. Exact command metadata takes precedence over subcommand interpretation; not a written Tcl name parser. | Existing `native_role_ids_keep_qualified_command_precedence_over_members` covers this purpose. |
| `tcl-registry/src/f5/rule_identity.rs::RuleProcedureTarget::{resolve,referenced_rule_spelling}` | Registry owns BIG-IP rule/partition/procedure path grammar; this is a different name domain from Tcl namespaces. | Keep native `call` route evidence and actual provider context; do not substitute Tcl colon-run normalisation. |
| `runtime/rust/src/interp/native_variable_names/native_tcloo.rs::original_c_oo_varname` | Shared `native_oo_varname_lookup` selects the original TclOO operand recipe before actual variable owner lookup; resulting qualified name is a report of an already selected home. | Constructed operand and reporting bytes do not independently recover namespace identity. Native input/lifetime tests remain separate. |
| `runtime/rust/src/cmd_oo/native_properties.rs` default accessor | Actual object namespace and selected property recipe call `object_variable_operand`, then existing original C variable read/store owner. | No consumer-local Tcl namespace parser found at this inspected seam. Native property allocation/currentness remains independent. |
| `rust/tcl-spec-studio/src/infer.rs::draft_for_proc` | Shared `unroot_rooted_key` changes only draft presentation; source facts use retained original declaration/body owners. | `by_name` output aggregation at704 is an authoring collision policy needing explicit review, not proof of current source identity. Imported non-UTF8 source boundary above remains unresolved. |
| `rust/xtask/src/gen_ai.rs::irules_vendor_surface` | Actual ingress context/store select `vendor_command_surface`; generated namespace/count prose is metadata presentation. | Generated templates/copies require their drift gate and final prose review. No local Tcl target resolver found in this inspected generator. |
| `.claude/skills/lsp-client/lsp_client.py::{open_document,cmd_command_info}` | Source/cursor and literal requested command labels pass through LSP; `document_uri` uses `Path.as_uri`. Source file decoding raises on invalid default UTF-8 instead of replacing source bytes. | Reader/log display decoding is separate from source names. `cmd_context` extension-based dialect text is presentation; it must not establish actual selected execution policy. |
| `.claude/skills/compiler-explorer/explore.py::_read_source` and explorer WASM `compile_with_optimisations` | File/text ingress forwards to existing CLI/Explorer pipeline. Local CST/source slices are reports; optimisations IDs are authoring settings. | UTF-8 document input is separate from native opaque names. No local Tcl resolver found at these inspected seams. |
| WASI server `driver.rs` | JSON-RPC driver uses the same server service and request-URI/capability adapters; no Tcl semantic parsing in the inspected input/dispatch path. | Framing, liveness and full WASI execution remain separate gates. |

## F5 admission and outstanding questions

The retained `f5/answers/outstanding-operation-audit.md` distinguishes 108
observed, 25 unsupported and 267 unreached original pattern operations. The
307-row completed investigation is not 307 reached operations. Existing
source hashes, input/context/unit/invocation pointers and cleanup limits remain
authoritative; no old receipt was rewritten here.

The highest-value new inputs for the separate appliance chat remain:

1. F5-N02/N06/N07/N08/N09: explicit absolute unique procedure names, with
   absence, catalogue, exact body, successful invocation and actual
   namespace/frame controls before lookup/link/import/alias/trace mutation.
   Hosted unqualified proc inside namespace eval has a measured global owner.
2. F5-N10: warmed descendant called from an entered frame across deletion and
   recreation, with distinct old/new cells; suspended frames and restart are
   independent variants.
3. F5-N12/N13: same protocol operation without its HTTP profile/event; actual
   traffic on another TMM worker with unit identity and distinct sentinels.
4. F5-N05/N15/N16: exact literal/constructed byte names, individual operation
   catches and one-word display roundtrip. Construction failure is separate
   from lookup failure. A stock tclsh answer supplies no TMM/MCPD answer.

Existing `REQUEST.md`, `ANSWER-TEMPLATE.md` and exact question files remain
the handoff. No appliance communication, run or answer admission occurred in
this audit. These bounded variants still prevent whole-scope completion.

## Upstream and issue metadata refresh

Read-only GitHub `msteveb/jimtcl` HEAD lookup on 15 October 2026 returns
`5bac7c99ad65864c87da513e22e2f01703fa4e03`, commit timestamp
`2026-09-30T22:54:24Z`, matching `rust/tcl-test-support/jim-reference.txt`.
This establishes current upstream revision only. No new Jim binary, build
configuration, extension inventory, original harness or semantic execution
result follows. Use `scripts/dev/ensure-jim-oracle.sh` and actual original
`testing.tcl`/`tcltest.tcl` suite provenance before execution claims.

Exact issue reads confirm #2286/#1778/#1755/#2424/#2426 OPEN and
#2287/#2300/#1469/#1455/#1606/#2450 CLOSED. Three read-only searches for
naming, namespace and W210 included both open and closed issues; search
snapshots return `state:null`, so only exact issue fetches establish state.
The issue catalogue already contains these relationships. #2286's latest
GitHub comment still describes an older failed source capture; the later
ordinary272 PASS is separate scoped implementation evidence. No issue was
changed or closed.

## Remaining census limits

A fresh tracked-file search outside the finite330 filenames finds 160 files
with candidate `qualified_name`, proc/class inventory, or colon split/trim
uses, including shared owners, test modules, examples, Rust metadata IDs,
artifact labels and genuine semantic consumers. This is a lexical count,
not 160 bugs, reviewed consumers or completed migrations. The inspected rows
above do not close that candidate set. Syntax owners, registry references,
all analyser/optimiser callers, every generated prompt and every editor
provider still need per-call-site reconciliation and fresh gates.

DB owner reports implementation of the positive retained Logical command
catalogue and graph-selected occupancy through existing owners during this
review. That work is pending integration/fresh execution and does not erase
earlier unknown effects or turn absent representation into proved absence.

Do not remove this report or the temporary inventory before remaining
consumers, supported oracle variants, repository documentation cleanup and
required gates are complete. Useful current contracts belong in durable
design documentation; this review history belongs in Git after completion.

## Reproducible baseline bindings

These are SHA-256 hashes of the exact `2d456966b` Git blobs read for this
review. Later implementation diffs and the reference-body peer review are
separate; these hashes do not certify concurrent changes.

| Path | SHA-256 at base |
| --- | --- |
| `rust/tcl-registry/src/value_transfer/reference_body.rs` | `ff92d3114a0002c0434c1eb2cd63f90e024404d8bf93a3ca60208bca8e62a0c3` |
| `rust/tcl-spectcl/src/loader/reference.rs` | `6c1af76a30d9ef4ab84069ea7c37053fbe5b4a803ee5d93ea852a995e4669a9b` |
| `rust/tcl-vm-wasm/src/lib.rs` | `dac0c823a1f6e3acb4892ec61397483ae4a0bab304788e9743900c8301b57005` |
| `rust/tcl-spec-studio-wasm/src/archive.rs` | `ea873970fc765673e525e038b15d2b8ce4cabc5ef46cee48d9ba20344ee45ee4` |
| `rust/tcl-compiler/src/codegen/backend.rs` | `4ca3c8dedbbb17f548d5ee65716c6e44fbfcebb645a6d12a911f19b1efcd1a2e` |
| `rust/tcl-compiler/src/codegen/emitter/mod.rs` | `8cf5eb0f656bf32e352ed002ca18ca291c7eecda14dacb9a95a91a0838020414` |
| `rust/tcl-compiler/src/codegen/mod.rs` | `390ff07eda1485a879d8037368a89dddca55a5b6440f818631a42e9dcf7924e8` |
| `rust/tcl-compiler/src/codegen/wasm/backend.rs` | `568f37ccec9945ac9d31ce75710410f32890aed8fcf3e429b9487229c75b74b4` |
| `rust/tcl-compiler/src/codegen/wasm/pipeline.rs` | `97b37494e65733fd2c193857c8d42c7db5ff6860a0133911434706a4097ad0af` |
| `rust/tcl-compiler/src/codegen/wasm/native_emit.rs` | `40922d52e5c57e6617ed3f23c1d342e0c5801c625c4330cbe6a58cf82665dd5d` |
| `rust/bpf-tcl-ir/src/frontend.rs` | `89405e375c6e89114f489e679fce1227e98af9435273d920b1a46dc7258ceae9` |
| `rust/bpf-tcl-codegen/src/lib.rs` | `7b37f5c047dc2b88df6a790a6829557f3b5ad32aa2f0a899e7aba9b038a629cb` |
| `rust/tcl-lsp-server-wasm/src/lib.rs` | `1514ab9a517558c8091c7f341e5f41265094405cf9b200831e45e3d34e0ee886` |
| `runtime/rust/src/host_wasm.rs` | `2423177f8a926e59af7f2cba308c6009ee3b16f257b479870a3f0ee8bec3ddec` |
| `rust/tcl-engine-wasm/src/session.rs` | `fd37770243adbb03760557659823cc33a7b4330eb8e9049ebeedfb4bc678583d` |
| `rust/tcl-engine-wasm/src/command_receipts.rs` | `b534d79173ca7deee9f933621a6a6f894dc860285199696b53248f08634ae7bc` |
| `editors/vscode/src/nameGeneration.ts` | `ac64ea88109bf0240a130c1e52c04edb4ff8479f827cfad11c98a7a478d242ba` |
| `editors/vscode/src/scaffold.ts` | `f3942bdf5597a0378b00020540bd7692524387d226154d85963935126a0f9ecc` |
| `rust/tcl-debugger/src/backend.rs` | `0c5b7ef4f939f8433d60fb39cdebb6b234736b15c4428b18b7a485dc86d3f5a8` |
| `rust/tcl-registry/src/native_argument_role_hook.rs` | `d6a7c9c56f9467f6261ef18ac6db933cc6eb6f635da32dec697eaf7496bf0860` |
| `rust/tcl-registry/src/f5/rule_identity.rs` | `c72bafb9a236594321901f066f0ed0927d276ef34d600eee337b3ba8ac3d5734` |
| `runtime/rust/src/interp/native_variable_names/native_tcloo.rs` | `92bd760d9642edca80422f05542a8cfbd6d9eef75867b2bc19910ddf28016b42` |
| `runtime/rust/src/cmd_oo/native_properties.rs` | `5ca6975e30a7cafb1108939b66f19ed405ddcdc8b5318875e71d0bb6174294f7` |
| `rust/tcl-spec-studio/src/infer.rs` | `1727cef5b9429dbda2686a4a06c33cd29f538a58a8e18676be637c9acf13a408` |
| `rust/tcl-explorer-wasm/src/lib.rs` | `b813cd8404bb96f816f115105a2f3eaed82b6438493c46d1bd03447ea72a55f5` |
| `rust/tcl-lsp-server-wasi/src/driver.rs` | `05c66dcedc14107110eddf3c208d5db8a328115b012fe105006cad87fea25e4e` |
| `rust/xtask/src/gen_ai.rs` | `510b6f96c30ce052fd57bc22c71e83deb25671db152622001ad329146b3e14f1` |
| `.claude/skills/lsp-client/lsp_client.py` | `c10be772eca42e36970aa90dc86908fa4457967130080b897b15f045d18dccc0` |
| `.claude/skills/compiler-explorer/explore.py` | `76f8841ce18df2f2f63c66ca07b03e901ad5cedc6e07499a452d5dd5415e88fc` |
