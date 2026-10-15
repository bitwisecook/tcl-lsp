# Additional naming-consumer review

Review base: `work` at `2d456966b`, 15 October 2026. This is an additive
source/caller review beyond the finite330 roster. No Rust build, test, native
oracle execution, generated drift check or repository gate ran in this review.
Concurrent implementation changes need their own final source binding and
execution receipts. Root owns Git, shared tracking and issue actions.

## Actionable consumers

| Call site | Input, purpose and current owner | Required work and controls | Disposition |
| --- | --- | --- | --- |
| `rust/tcl-registry/src/value_transfer/reference_body.rs::variable_name` (649), called by `expression_text` (579) | Reference-body sandbox source; private first-close `${…}` and ASCII/colon scanning decides whether expression variables can reach outside the local frame. `Lexed::new` (315) explicitly selects the hook host's fixed plain-Tcl default grammar. | Replace the private variable parser with `tcl_lexer::word_parts::scan_var_ref` under the actual hook-host configuration. Verify that host before changing the public derive/config contract: `ContextDependency::TclProfile` alone does not prove the hook language varies. Keep malformed/non-reference `$`, arrays, nested braced names, lone colons and complete consumed extent distinct. Loader production caller is `rust/tcl-spectcl/src/loader/reference.rs::derive` (116); it passes command and definition, without a document configuration. | Root implemented and source-peer reviewed below; execution separately owned by root. |
| `rust/tcl-vm-wasm/src/lib.rs::tcl_eval` (134–146) | Host `(ptr,len)` source bytes are converted with `String::from_utf8_lossy` before `run(&str)`. Distinct opaque bytes in literal command/variable names can become the same replacement text. | Use actual supported source-byte ingress, or reject unsupported UTF-8 explicitly before execution. Preserve byte-valued completion output independently. Add discriminating malformed-source and literal replacement-character controls; existing module tests exercise completion output only. | VM implemented supported original byte-source ingress; bounded source peer below, execution separate. |
| `rust/tcl-spec-studio-wasm/src/archive.rs::extract` (154) | Archive Tcl source members are decoded lossily into `{name,text}` before package/spec inference; `Entry::text` explicitly documents this policy. | Retain original source bytes at a supported owner or report unsupported encoding in the existing skipped-member channel. A valid UTF-8 literal replacement character must remain distinct from malformed name bytes. Do not infer original naming identity from lossy imported text. | Root implemented full provenance chain and source-peer reviewed below; Rust execution separate. |
| `rust/tcl-vm/src/cmd_namespace.rs` Jim `canonical` arms | VM owner reports no-argument arm creates a fresh String from `namespace_object_bytes(current_ns_id)` and named arm derives current reporting bytes through `jim_canonical_namespace_bytes(None,name)`. Runtime already retains actual Jim namespace object through `jim_current_namespace_object` and `jim_canonical_namespace_object`. | Preserve held original identity, unresolved current context, name-first absolute short circuit, original append and getter order through the existing owner. VM owner/root coordinate after current source freeze. | VM implemented original current-holder owner route; bounded source inspection below, execution separate. |
| `rust/tcl-compiler/src/analyser/handlers.rs::dynamic_command_name_may_equal` (573), consumed by `static_provenance_command_is_trusted` (9449,9466,9480) | Private first-close `${…}` scan and copied colon-run canonicaliser exclude command-table mutations from static provenance. Input comes from reconstructed `subject_word`, without original word/configuration. Tcl9 `${a{b}}` leaves a false fixed `}` suffix in this matcher although the complete reference can produce an arbitrary name. | Repair the authentic original word/configuration/purpose transport through shared source-name pattern owners. Existing `dynamic_names` source APIs retain VariableRoot/NamespaceName purposes; command mutation may need its own purpose extension. Preserve finite same-tail/constructed-key controls; add nested-brace, brace-literal sigil, mutation and unchanged fixed-fragment positives. | Compiler implemented and source-peer reviewed below; execution pending root. |
| `rust/tcl-compiler/src/dynamic_names.rs::name_word_pattern` (368+) | Shared source-pattern owner still privately scans bare variable and bracket syntax. `$a($b)` retains fixed parentheses for NamespaceName purpose; bracket counting can stop at a literal `]` inside a braced command argument. These can rule out a possible dynamic name. | Use actual configured source substitution components/retained lexical geometry at this existing owner. Missing/malformed/unrepresented portions remain possible, not a fixed fragment. Do not migrate the analyser helper into this owner without repairing this geometry. | Compiler implemented and source-peer reviewed below; execution pending root. |
| `runtime/rust/src/namespace.rs::{resolve_fqn,which_command}` (2296,3337) | Runtime owner identified duplicate namespace/simple reporting joins beside existing per-policy `command_fqn` (1497). Independent review confirms actual `home_of` selection and separate generation joins in `import_query_binding`, `find_command_id` and builtin checks. | Delegate reporting to the existing command FQN owner, retaining one selected home and separate generation/lifetime. Stale trace-key comments need correction. This does not establish a token-identity defect from rendered bytes alone. | Runtime now delegates both report consumers to command_fqn; bounded source inspection below, execution separate. |
| `rust/tcl-compiler/src/analyser/handlers.rs::invalidate_static_facts_written_by_segment` (9220) | Static name-provenance invalidation splits every VarWrite name at the first `(`. Scalar `a(b` and `a(b)tail` are consequently treated as a different root from the shared evaluated-name owner. | Use actual role/name-data geometry through `split_element_ref` or the existing typed root projection, preserving original value/source proof. Add scalar unmatched/trailing-parenthesis invalidation controls. | Compiler implemented and source-peer reviewed below; execution pending root. |
| `rust/tcl-compiler/src/analyser/handlers.rs::substitute_bound_words` (435), used by `static_word_value` (9310) and parameterised class-name production | Braced closing syntax delegates to the shared owner, but bare reference scanning independently accepts every lone colon and Unicode alphanumeric. `$a:b` can select formal `a:b` instead of the shared Tcl `$a` reference followed by literal `:b`. Only BracedVarStyle is supplied, losing other full-configuration axes. | Retain the actual original source word/full configuration and use shared variable/component parsing. A proc with formal `a:b` and name expression `$a:b` must not manufacture a proved output name from that formal. Keep legitimate literal sigils, constructed data and source syntax separate. | Compiler implemented shared configured reference geometry; bounded source peer below, execution separate. |
| `rust/tcl-compiler/src/irules_checks.rs::{classify_stmt_for_collect_flow,drop_flow_leaf,http_flow_stmt_command,find_unnormalised_getter_warnings}` (943,621,1493,382) | Collection transfer classifies reconstructed/canonical command labels through a local colon trim and registry metadata; assigned substitutions are reparsed from rendered values. Drop transfer uses hardcoded names on `Statement.command`. HTTP response transfer also uses bracket trim plus whitespace split to recover a head; getter warnings consume direct or reparsed labels similarly. Actual nested collection scanning already consumes `registry_invocation_assistance_with_metadata_context` candidates. | Route direct and assigned/nested invocations through actual retained source binding/candidate owners before consuming selected registry effects or metadata. Preserve source event/body ownership, exact original words/configuration, prior unknown barriers, and existing reachability gates. Add selected/shadowed/aliased qualified-head and dynamic-payload controls. LSP owner confirms this file is unclaimed. | Live semantic migrations required; root notified. No semantic run by auditor. |
| `rust/tcl-lsp-core/examples/tcloo_diag.rs::binding_categories` (195–257) | Diagnostic experiment classifies procedure-return by collecting rendered qualified names and reconstructing a root key from parsed `cmd`; local command knowledge separately recognises constructors and collection-return heads. | Use actual source/CFG selected binding or declared body ID for binding-derived classification, refusing unavailable evidence. A shared string qualifier alone would retain the identity inverse. This is tracked tooling despite its example location. | Additional tooling migration required; root notified. |
| `rust/tcl-registry/src/value_transfer/template.rs::{record,literal_key}` (330,399) | Template source uses actual context grammar but recursive compatibility `decompose_spanned`; array keys join decoded Text through `from_utf8_lossy`. Decoded byte escapes or non-Unicode native units can be opaque even though original template is a Rust String. | Use checked shared geometry/full executable arena or an explicit rejecting depth budget; retain decoded key bytes at an appropriate typed owner or decline exact element identity when unsupported Unicode. Missing representation never becomes an exact replacement-character key. | Additional shared-template migration required; root notified. |
| `rust/tcl-compiler/src/analyser/irules_event_checks.rs::{find_loop_ne_zero,match_dollar_ne_zero,match_zero_ne_dollar}` (1073,1087,1119) | IRULE5003 manually scans `$` and optional braces with ASCII word bytes; the forward matcher accepts a missing braced close, while reversed matching never checks the close. Actual caller has retained analyser context and full lexer configuration, but passes condition text only to the matcher. | Use shared actual configured variable/expression geometry and preserve the rule's deliberate candidate-selection policy. Do not turn malformed or nonmatching reference geometry into a different named variable. `body_decrements` already uses retained metadata/shared bounds command increment owner. | Additional live reference-parser migration required; no new oracle or test execution. |
| `rust/f5-cli/src/commands/split.rs::run_split` (43), `tcl-bigip-io/src/paths.rs::decode` (186), and `ucs.rs::ucs_to_scf` (219) | SCF source decoding may replace opaque bytes before parsing object/rule names and writing partition files. `read_path` exposes strict mode but plain callers such as merge and iRule operations request false; UCS extraction loses original encoding before that flag can help. | Audit genuine mutation/inference callers separately from preview/report contracts. Preserve original source bytes/provenance or checked refusal before authority, including UCS member decoding. Retain valid U+FFFD as a distinct supported source. Do not assume that switching a downstream strict flag repairs already lossy archive conversion. | F5 source-ingress migration candidates, full call graph and existing contracts still unfinished. |
| `rust/tcl-compiler/src/codegen/structured.rs::walk_stmt` (165) | Inside loops, written `Call.command` equal to break/continue emits a structured jump before the backend's typed-statement guard. `WalkSource` holds image/configuration but no selected command/completion proof; script native admission is a distinct compiler-provider obligation. | Ask the existing shared selected completion/invocation owner before emitting structural control transfer. Preserve guarded eval fallback and actual selected/renamed/aliased command cases for WASM, including wrong-arity break/continue calls. The existing backend per-span leaf invocation plan already retains selected metadata; do not create a competing resolver in the driver. | Live codegen migration required; root, VM and Compiler notified. |
| `rust/tcl-compiler/src/codegen/emitter/loop_blocks.rs::detect_foreach` (111,132) | Loop kind and collection mode are recovered from compiler synthetic Call labels plus block shape. Actual producer `cfg_builder/cfg_lower.rs::foreach_binding_statement` already knows typed Foreach booleans and attaches `SyntheticMarker::IterationBindings`. | Transport actual operation/collection kind through this existing synthetic owner. Do not re-resolve original command using rewritten list-only argv: synthetic iteration bindings deliberately do not dispatch or reevaluate arguments. | Synthetic operation transport migration required; no semantic defect asserted solely from labels. |
| `rust/tcl-compiler/src/inline_uplevel.rs::statement_has_completion_escape` (398) and `shimmer/thunking.rs::destructure_foreach_blocks` (131) | Passthrough erasure recognises Break/Continue by Call/Barrier.command text; type heuristic recognises a single Call named break. Frame-reach metadata is a separate semantic purpose. | Consume actual selected completion/source ownership; selected aliases and unknown effects cannot become no-escape evidence from a different label. Preserve rejection direction and genuine compiler loop provenance. | Remaining completion consumers required; root/Compiler notified. |
| `rust/tcl-mcp/src/datagroup.rs::normalise_dg_name` (204) and `rust/tcl-lsp-core/src/refactor/datagroup.rs::normalise_dg_name` (134) | Identical ASCII lowercase/nonword replacement/underscore trimming/fallback loops generate new suggested data-group names. MCP suggestions use whitelist/map descriptors; Core's `resolve_dg_name` applies generation only when caller supplied no name. | Reuse/extend shared naming generation architecture with an explicit data-group suggestion purpose and retain caller collision obligations. Preserve supplied names verbatim and original generation behaviour, including underscores. This never resolves an existing Tcl/F5 identity. | Duplicate naming-policy migration required; root notified. |
| `rust/tcl-compiler/src/taint.rs::{emit_branch_condition_nested_command_warnings,emit_statement_warnings}` (6954,4750) | Nested condition command text is reconstructed/reparsed, and supplied-source admission checks registry availability through `resolve_spec_in_realm` only. `SinkCall` receives no tokens/effective words/source read owner; `operand_read_taint` then selects SSA symbols by rendered variable name. Assigned-substitution fallback similarly reparses `AssignValue.value` with no child tokens, while `resolve_taint_command_heads` can recover the parent set tokens and select the wrong invocation. | Transport original nested expression invocation and actual conditional source warning-read owner, preserving full source/configuration/context. Reuse direct-source warning selection; availability alone grants no source invocation, physical variable cell or SSA read. A parent assignment binding cannot issue a nested child command identity. Preserve valid branch sink controls and explicit standalone utility separately. | Additional live source/SSA naming migration required; root notified. |
| `rust/tcl-cli-support/src/input.rs::combine_sources` (458), consumed by transform.rs297 and compile.rs91/122 | `InputDocument` retains actual DecodeReport while text combination drops it. `decode_source` intentionally supports best-effort W107 analysis; `requires_abstention` covers W109/non-text only. | Audit transformation and compilation authority when malformed name bytes became U+FFFD, separately from existing best-effort diagnostic policy. A blanket W107 analysis refusal would change that original contract. Retain original byte/decode provenance or explicitly refuse unsupported semantic actions. | Actual combining path inspected; edit/compile consequence and existing controls still need classification. |

Studio's original `decodes_non_utf8_bytes_lossily_rather_than_losing_the_file`
control requires retaining one entry when malformed bytes occur in its comment.
Keep that fixture/assertion. An unsupported-encoding source authority boundary
must distinguish retained preview data from admitted source facts, or use an
authentic byte source path. Silently dropping every malformed member would
regress the original control. The current JSON source snapshots carry only
text; any provenance extension must be followed through the versions importer
and browser consumer, not merely added as an unused warning field.

Root's subsequent four-file Studio implementation passes bounded source peer:
`Extraction.unsupported_encodings` retains unsupported-source labels separately
from preview entries. Browser staging preserves the marker; release validation
refuses inference, and `snapshotsJson` carries the marker to Rust. Rust snapshot
parsing rejects nonempty or malformed markers before any `SourceFile` is
constructed, preserving unknown source rather than deriving missing commands or
version bounds. Existing explicit-Unicode snapshots retain their API shape;
new controls distinguish malformed-name previews from valid U+FFFD. Rust and
web execution remain root-owned and independently reported.

Root subsequently reports all 123 Studio web tests and both complete TypeScript
configuration checks passing with pinned npm12.2. Rust execution remains
separate. The full typecheck required the existing `variableReceivers` callback
signature correction; original test assertions were retained.

| Studio peer-reviewed current path | SHA-256 |
| --- | --- |
| `rust/tcl-spec-studio-wasm/src/archive.rs` | `2d6abb7a1ae4602486c693220d6d849f2312457cf56a6125d5729096d7bbfdaf` |
| `rust/tcl-spec-studio-wasm/src/snapshots.rs` | `a845e6c63d3c4eea769acc360a3597e67558477e812630cc8b67acf877eca385` |
| `rust/tcl-spec-studio/web/src/releases.ts` | `5c864159630744e25114e2212fca84c5ba0c6886288f0d12115c378ae040a72f` |
| `rust/tcl-spec-studio/web/src/importReleases.ts` | `e44f447ce6c4f2538c8431839f3eb8e5c97725393215d000bc0fd8ec91c60925` |

Source peer review of root's subsequent `reference_body` draft, SHA-256
`80e5abc10a27218fc6ccdd02ff206f563ff2bd6d9f871998b90acfd7f4547290`,
finds retained `Lexed.config`, `scan_var_ref` and raw-index `decompose` with
explicit recursive part checking appropriate for the bounded change. The
actual raw-index braces remain literal while nested substitutions are
inspected; ParseError and unsupported expression parts decline. Consumer depth
32 rejects nested parts before the compatibility scanner's depth64 truncation.
No blocking source finding remains for that hunk. This supplies no executed
oracle or Rust acceptance.

Final source peer of Compiler's four-file batch finds separate actual Installed
snapshot/prepared-CFG selection preserves declaration-only API purposes,
complete inventories, issuer/currentness and exact source/allocation mapping.
Param traits gather genuine command-body spans and sort by original position
within each word, preserving array-index-before-root alias state and actual
argv word order. The arena does not separately parse Command body descendants,
so recursive script visits do not duplicate retained command entries. Complete
configuration, checked source base and contiguous original image remain required.
No blocking source finding remains for this bounded batch. Acceptance is pending
root's fresh build/execution.

| Peer-reviewed current path | SHA-256 |
| --- | --- |
| `rust/tcl-compiler/src/analyser/param_traits.rs` | `48ee716dc2ebeb41cfd85ed04446035976de8a30801cba4b535165327c2e77b2` |
| `rust/tcl-compiler/src/body_analysis_capsule/readonly_authored_coverage.rs` | `26b86b9b3eb016f65b7078259d90ccdc15cb233fbc8248a8960f07d513ae32ed` |
| `rust/tcl-compiler/src/command_binding/retained_source.rs` | `0670fb1b68d422c0904eef4b32d2364206bf0dd366232109e4fedfdbb88fe588` |
| `rust/tcl-compiler/src/compilation_unit.rs` | `6842a85e28ef4fc9b5a96795fedcc4c44dcf1cb1ffd8a50d56723c50217fe2da` |

Compiler's second Logical source-scope batch also passes bounded architecture
peer after adding explicit actual AuthoredSimulation+C recipe admission at the
issuer (`logical_variable_scope.rs` SHA-256
`ea8c251a32a59c65de1cf8b3740f95b58341b324adbfc65e9feba886effd5cfc`).
It retains genuine tape/current input/full configuration/original word geometry,
the exact scope-body allocation and declaration card. Native alias selection
still precedes Logical advice; known local aliases refuse, namespace fallback
alternatives decline, and earlier unknown effects prevent rename coverage.
Source declaration frames supply no entered activation or physical cell.
Formal/whitespace transport and missing alias VarDefs remain explicit unfinished
producer work; no execution result follows from this peer.

The subsequent command-mutation/pattern migration now passes bounded source
peer after the compositional colon-boundary correction. Genuine configured
original words and shared executable components replace private scanners;
actual source schema and effective operand ordinals replace `subject_word`
String inverses. Dynamic fixed fragments trim colon edge constraints before
using the shared canonical owner, so only interiors bounded by retained
non-colon bytes can exclude a target. Static-only patterns canonicalise the
whole value once. This avoids the false exclusion where `a:::$x:b`, with empty
`x`, reaches constructed `::a::b`. Missing, malformed or backslash-bearing
geometry remains possible overlap. The existing Logical authored-C issuer
purpose remains explicit. No blocking source finding remains for this bounded
change; no tests were run by the auditor.

| Command-pattern peer-reviewed current path | SHA-256 |
| --- | --- |
| `rust/tcl-compiler/src/dynamic_names.rs` | `2a918e5962be42443f6a57cc00fe0f712ea5ba25af62f7534da461f20d862d2d` |
| `rust/tcl-compiler/src/analyser/handlers.rs` | `c19cfed54d7f4765e8065772970a3250d4fc8884931ab85369a9d0539b915720` |

Additional evaluated array-name geometry remains duplicated in
`value_transfer/inputs.rs::PlaceRef::{is_element,base}` in Registry, and in
Compiler's `compilation_unit.rs` connection-scoped root insertion,
`value_transfer.rs::existence_step`,
`sccp/original_value_facts.rs::array_element_base`,
`existence_query.rs::computed_element_base`, and
`analyser/handlers.rs::names_static_variable`. These use first-open plus
terminal-close geometry already owned by `split_element_ref`; their narrower
nonempty, ASCII or substitution policy must remain separate. `PlaceRef::scalar`
currently also accepts encoded element-name fallbacks from existing adapters;
centralisation must preserve those callers rather than trust `kind` alone.
These are migration candidates, not claimed semantic defects or completed
consumer coverage. Root was notified; no source edit or execution by auditor.

Follow-up current-source inspection confirms VM WASM ingress now transports
original counted bytes into `SourceImage::from_bytes` and
`Vm::try_eval_source_image`, retaining host refusal separately from guest
completion bytes. Runtime's `resolve_fqn` and `which_command` both delegate
the selected home/simple slot to `command_fqn`; generations remain separate.
VM Jim canonical no-argument returns its actual current holder after currentness
validation, and named forms delegate `original_jim_canonical_namespace_object`.
These are bounded source observations, not replay acceptance.

| Follow-up source path | SHA-256 |
| --- | --- |
| `rust/tcl-vm-wasm/src/lib.rs` | `4b5bfee56e971801b1cc03d318c6f915c8dbdae55be147a4fea1f384eb2158cb` |
| `runtime/rust/src/namespace.rs` | `6d0d2f0fcb03490ef4a3af5040e87a4350a21b80e515ff98742d676622681337` |
| `rust/tcl-vm/src/cmd_namespace.rs` | `f7af30ede847a0f1e32d2233ac715152ea1fd9e3a40a85389b8553d424bed51d` |

Compiler's subsequent `substitute_bound_words` migration passes bounded source
peer at `handlers.rs` SHA-256
`c802e7ec7fa89dd3c41ea33cba3a241c1d24b5326c1b8e2e15e30763e15e3f87`.
Actual full file configuration now reaches the shared executable arena; genuine
scalar name spans distinguish `$a:b` from `${a:b}` and Tcl8/Tcl9 close rules.
Array references, commands, expressions, malformed parts, escapes and
unresolved parameter values still refuse. Inserted values remain completed
data rather than being parsed again. This closes the bounded private reference
scanner; `static_word_value`'s earlier trim/quote/script reconstruction and
whole static-result provenance remain distinct audit obligations. No execution
acceptance follows from this peer.

Compiler's subsequent seven-file Logical formal topology batch passes bounded
source peer. The actual original formal word feeds the existing topology owner;
optional topology failure keeps its procedure/header card. Formal occurrences
retain exact card/current source/input/full configuration/C issuer and actual
list-field span/input, then reuse the original parameter rename topology recipe.
Coverage treats missing topology/fields and duplicate or unsupported parameter
names as unavailable. Real Native/Jim inputs retain the existing declaration
inventory path. Cursor topology and alias-cell VarDef issuance remain separate
unfinished producers; no execution result follows from this peer.

| Formal-topology peer-reviewed path | SHA-256 |
| --- | --- |
| `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_body.rs` | `728c8da82dbf7e30f19dd6238384ea938f8869d139a25db29bbcb837c5325a68` |
| `rust/tcl-compiler/src/command_binding/logical_variable_scope.rs` | `e6b4d47990785c6c00809cea8935a0f877ae45199d94b4f01fef71abb5e7f07c` |
| `rust/tcl-compiler/src/realm/source_transition_advice.rs` | `325c775a04c93f2acebf959c2da28d5e3927c93ea0f31763db009aba129200aa` |
| `rust/tcl-compiler/src/signature_scan/variable_symbol.rs` | `88f242bf1087934b80b75b372fcfdadc4a1cb7f124e58023e40a82533aacf516` |
| `rust/tcl-compiler/src/analyser/scope.rs` | `6eccbad409e8eaaef76c86786b22fedeb14d62244ca7eeee7a25a8c89068c400` |
| `rust/tcl-compiler/src/analyser/types.rs` | `afac494d6926d37acac7512b45b50976ea6abd98828d10c11abf705a6e7a7c66` |
| `rust/tcl-compiler/src/analyser/variable_source_declarations.rs` | `0a9d52527dcbfdb3822dac78af5e5502731441138ce5c02210d8c1c1d9b2f62f` |

Compiler's following Logical cursor-frame transport passes bounded source
peer. Genuine scope regions are retained at script entry, including empty
bodies. Selection considers the smallest represented region before conflict or
frame filtering, so a namespace/conflicting scope cannot reopen an enclosing
procedure. Shared command-substitution bodies preserve same-frame source
geometry; inert unselected braced operands and malformed/depth-limited regions
refuse. Tuple-key receipt merging preserves existing terminal conflicts, exact
card/body/context/image/configuration and authored-C frame issuance. Native
cursor selection remains independent and unchanged. Alias VarDef transport is
still unfinished; no build/replay follows from this review.

| Cursor-frame peer-reviewed path | SHA-256 |
| --- | --- |
| `rust/tcl-compiler/src/command_binding/source_transition_advice.rs` | `86af8b00753ff7a9360a33dbd48e8372fabb16edb6c7b6d4fbbdf1c04b46047f` |
| `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_body.rs` | `9ee90bd6de4fc50ff013ff1a657df04b1cd39bf19a93bc3d6e3c4f01bfd03b0d` |
| `rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs` | `ee4781537e7220e3f69f2ffe1dfad792c0ecb4557d14fb5675f1b05a755cb603` |
| `rust/tcl-compiler/src/command_binding/declaration_layout.rs` | `be386d662e04402e99abf14abc72f43e24ea2e6857be3b5dbe7d2f8280d4aab5` |
| `rust/tcl-compiler/src/realm/source_transition_advice.rs` | `7c37976335f96ce5dbe75a7803219f3a97b3ff1042214990b5ce98b6a21afc23` |
| `rust/tcl-compiler/src/analyser/types.rs` | `222c846b3e489b6d5f6149fdc5489a792ce172c550a49b632a3b9a72aa97424c` |
| `rust/tcl-compiler/src/analyser/variable_source_declarations.rs` | `a1e8000d6b74222259455a7b3214ed7c3282cca1ca8347f4716e1ce3adabc52b` |

Root's five-verb CLI faithful-source boundary passes bounded source peer.
`InputDocument::require_faithful_source` consumes the actual DecodeReport, and
format/opt/minify/dis/compwasm check every document before combined text,
transformation, compilation or output/sidecar writes. Literal U+FFFD remains
supported; existing W107 best-effort diagnostics and W109 analysis abstention
remain unchanged. Graph/explore/minimize and other CLI consumers are separate
unfinished classification, not closed by these five entry points. No executed
CLI acceptance follows from this review.

| CLI boundary peer-reviewed path | SHA-256 |
| --- | --- |
| `rust/tcl-cli-support/src/input.rs` | `bd351643e11cc75f6bbcd6ba0d49847f0007375c5a39c65cd08beef9b78286cc` |
| `rust/tcl-cli/src/commands/transform.rs` | `e51f152d9c1d6f4386b7932c9ec7643679e83240745e5a75ed7ead02d77f4588` |
| `rust/tcl-cli/src/commands/compile.rs` | `6b64a8291673d38c36fe1fc98fbbc9e89663eca37fe5923cda99303079cf931e` |
| `rust/tcl-cli/tests/cli.rs` | `ee915d252f0072ffe109de8e2848407f90dd585039999117c75aa372bba0194f` |

LSP collection migration has a bounded source-peer concern outstanding:
missing invocation assistance currently returns a successful empty collection
contribution before an original side-switch/body refusal can be seen. The
existing corrupted-clientside-argument withdrawal control requires unavailable
whole pairing state, not absence. LSP owner was asked to distinguish missing
representation from genuine admitted uninteresting calls. Nominal mutation
name hazards remain separate withdrawal-only evidence alongside unknown
transitions; no registry availability, execution or tombstone follows.

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
| `rust/xtask/src/gen_tmlanguage_keywords.rs::lexical_regexes` | Shared `tcl_syntax::naming::textmate_variable_name_body` supplies variable colon-run syntax; selected shared number/escape owners supply other lexical bodies to VS Code/JetBrains/Sublime outputs. | Static fallback colouring is a coarse grammar projection and supplies no name binding; `[:]{2,}` and generated-copy tests exist. Drift and full editor tests remain unexecuted. |
| `rust/xtask/src/gen_zed_queries.rs` | Existing ingress context/Registry and static language roster select per-file-type command metadata; structural tree-sitter nodes and pinned grammar symbols stay separate. | Union fallback highlighting cannot establish per-document dialect or actual command identity. Actual semantic tokens remain server-owned. |

Additional bounded tool/EDA review:

| Adapter | Inspected route | Remaining evidence |
| --- | --- | --- |
| `tcl-diagram/src/data.rs::command_completion` | Actual supplied metadata path resolves original tokens, realm and schema before selected completion; explicit standalone lexical declaration advice remains separate. | Existing original structural diagram controls required; no new execution here. |
| `tcl-diagram/src/graph.rs::irule_flowchart_graph` | Renderer consumes already classified diagram data and allocates monotonic node IDs; procedure/event labels do not key graph identity. Completion rendering consumes the separate supplied completion field. | No additional naming resolver found in this bounded renderer read. |
| `tcl-explorer/src/serialise.rs::{serialise_unit_scope,serialise_interproc}` | Procedure/call evidence joins use issued BodyIds through `get_for_body` and typed maps; output carries body IDs/keys separately from qualified display labels. | Labels in output are presentation, not an identity reconstruction. Source inspection only. |
| `tcl-spectcl/tests/{eda_environment_packs,eda_loadables}.rs` and shipped `specs/eda_*.tclspec` ingress | EDA environment IDs/aliases/packages resolve through environment registry plus discovery/parse/merge/install; vendor command collision filtering is explicitly tested, and generic source consumers remain responsible for actual command binding. | Existing environment snapshot, generated seed drift and per-vendor command/package controls remain unexecuted here. This is metadata-route review, not appliance/tool oracle evidence. |

The Tcl iRule test harness uses original interpreter `namespace which`,
`namespace qualifiers`, `namespace tail`, `namespace eval` and held original
command aliases in `tmm_shim.tcl`/`command_mocks.tcl`. The reviewed
`_private_command_alias` constructs new private names from the fixed generated
post84 catalogue; it does not demonstrate arbitrary command-token identity.
Framework namespace access restrictions are explicit host policy. Native C
namespace forwarding does not establish real TMM semantics, and this bounded
harness read cannot admit missing appliance operation variants.

VSCode `iruleSkeleton.ts::normaliseEventName` trims/uppercases new requested
event labels before deduplication and template generation, alongside the
shared generated event catalogue. This is generation policy rather than
existing Tcl identity; its own casing/suggestion policy still requires
reconciliation with the central generated-name architecture. The broader
186-file editor lexical roster is not a completed editor consumer census.

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
A second exact upstream HEAD read at 2026-10-15 10:48:03 UTC still resolves
`5bac7c99ad65864c87da513e22e2f01703fa4e03`. VM oracle owner was informed.

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

## Complete issue metadata snapshot

Fresh complete exact-read snapshot at 2026-10-15 10:20:39 UTC: all 252 IDs in
`issue_snapshot_mapping` plus #2286/#2287 were fetched independently, with
254 successful responses, 59 OPEN, 195 CLOSED and no fetch errors. The 251
prior rows containing an explicit parsable OPEN/CLOSED state have no changed
states. The three additional/status-mixed rows remain independently captured
below. These are issue metadata only, never implementation acceptance.

OPEN: #1712, #1755, #1778, #1796, #1797, #1946, #1956, #2093, #2193, #2210, #2217, #2233, #2246, #2247, #2253, #2254, #2259, #2260, #2261, #2263, #2266, #2269, #2270, #2273, #2275, #2276, #2286, #2290, #2292, #2297, #2305, #2306, #2308, #2311, #2323, #2335, #2342, #2343, #2356, #2367, #2370, #2371, #2372, #2374, #2376, #2379, #2385, #2398, #2404, #2408, #2410, #2412, #2413, #2414, #2415, #2423, #2424, #2426, #2441.

CLOSED: #23, #40, #42, #61, #62, #82, #87, #189, #246, #268, #276, #280, #281, #310, #327, #637, #756, #776, #790, #923, #934, #936, #945, #946, #958, #973, #974, #979, #980, #981, #988, #990, #991, #994, #999, #1002, #1006, #1007, #1009, #1010, #1013, #1018, #1022, #1025, #1026, #1027, #1031, #1049, #1050, #1051, #1056, #1058, #1064, #1070, #1077, #1078, #1079, #1081, #1088, #1093, #1096, #1098, #1101, #1103, #1104, #1108, #1113, #1114, #1116, #1123, #1133, #1137, #1138, #1139, #1142, #1165, #1185, #1192, #1193, #1194, #1198, #1218, #1237, #1245, #1246, #1247, #1252, #1260, #1261, #1262, #1263, #1275, #1277, #1281, #1296, #1301, #1302, #1305, #1312, #1328, #1331, #1337, #1379, #1388, #1400, #1403, #1408, #1412, #1421, #1427, #1442, #1446, #1447, #1448, #1450, #1451, #1453, #1454, #1455, #1456, #1457, #1458, #1459, #1460, #1463, #1467, #1468, #1469, #1473, #1484, #1492, #1493, #1497, #1575, #1577, #1578, #1582, #1584, #1585, #1588, #1593, #1594, #1602, #1604, #1605, #1606, #1610, #1611, #1612, #1613, #1616, #1617, #1625, #1648, #1660, #1703, #1704, #1705, #1706, #1707, #1708, #1713, #1729, #1737, #1745, #1748, #1751, #1752, #1753, #1757, #1759, #1763, #1764, #1772, #1784, #1788, #1789, #1798, #1928, #1933, #1935, #1951, #2065, #2072, #2134, #2142, #2159, #2164, #2166, #2168, #2173, #2185, #2287, #2300, #2450.

Additional search exclusions inspected: VSCode `tkPreviewPanelHtml.ts`'s
`normaliseCssType` creates a CSS selector suffix for widget presentation and
provides no Tcl identity; BIG-IP topology splits IPv6 double-colon syntax;
proof scripts and `xtask/owner_resolution` split Rust module/test selector
paths; Studio `render_spectcl.rs::{variant_of,surface_members}` splits rendered
Rust enum/constant expressions. These byte similarities do not justify
replacing their domain grammars with a Tcl resolver. Compiler
`cfg_builder::{is_block_terminating_command,is_tailcall_command}` and
`signature_scan::ctx::authored_publication_key` colon matches are test-only;
`diagnostics/dataflow::case_list_command` capitalises a diagnostic label and
does not select semantics.

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

Runtime owner reports that original Jim Import already shares canonical,
enumeration/hash-key, alias/equality and publication owners, while actual
namespace/frame checks across every stage and mutable helper replacement or
deletion remain unfinished (`SDK297` question). This is not permission to
replace the finite alias/cycle/cache/lifetime walk with a generic helper call.

Do not remove this report or the temporary inventory before remaining
consumers, supported oracle variants, repository documentation cleanup and
required gates are complete. Useful current contracts belong in durable
design documentation; this review history belongs in Git after completion.

## Additional unresolved-source bindings

The following hashes bind the later actionable-source reads. They establish
reviewed source identity only and do not certify a completed migration.

| Path | SHA-256 at inspection |
| --- | --- |
| `rust/tcl-registry/src/value_transfer/template.rs` | `2ec9f8a985d7786a9920335a5dade9f1097200c71c31a3edf50bb5c9b87bd999` |
| `rust/tcl-compiler/src/analyser/irules_event_checks.rs` | `f0756fd7c26c1e01102022f95d26f9caed4439f8408a529f59934048510e142d` |
| `rust/tcl-compiler/src/codegen/structured.rs` | `10fd5afee5420a8030e4fd6151e80ef745a1fcc61fe920ba13583cd4cfa50e15` |
| `rust/tcl-compiler/src/codegen/emitter/loop_blocks.rs` | `df87e9ea8b4f460513edd2a8a02155b2194278a5f4cb5effc67459a46a150363` |
| `rust/tcl-compiler/src/shimmer/thunking.rs` | `b9dec180ca3730018a76f1827c35a5e92a37b1797ab64eae19bd7cce60b6c3dd` |
| `rust/tcl-compiler/src/inline_uplevel.rs` | `6ab342312f0826e17d0ef51c51bd45bf84cd6a2d4c6520d56b28875a999c371d` |
| `rust/tcl-compiler/src/irules_checks.rs` | `6fb4cf3a957fd5d9627f0d448baa6461de3991f7ba5c7ef6b19b0858211396e7` |
| `rust/tcl-lsp-core/examples/tcloo_diag.rs` | `84efb01133854502f9a3390f319e48e53f8cf04dd7b6963077cebffbd9548a07` |
| `rust/tcl-registry/src/value_transfer/inputs.rs` | `62535c738502de8351b478be84423cc4a5a3a773e60c59f1b6dba93880daddd4` |
| `rust/f5-cli/src/commands/split.rs` | `d45f702d334fd7a8100372f5b2f836efc10b7d1f5676413a54d6006420ad590d` |
| `rust/tcl-bigip-io/src/paths.rs` | `2d7679f4dcf1ff513a8a888f3b3613ed0772b257038c75fcb85d368cce5159e5` |
| `rust/tcl-bigip-io/src/ucs.rs` | `4d6c8cc50d16504a44aca530089973c59a66b8e277808b9ef1f1948f88e37ccb` |

Additional suggestion-policy source bindings:

| Path | SHA-256 |
| --- | --- |
| `rust/tcl-mcp/src/datagroup.rs` | `f874df1b787b30ebc57c11e96851f611f0542e47bf7a080b44c086d7cd675512` |
| `rust/tcl-lsp-core/src/refactor/datagroup.rs` | `82c1fb758615b6b171c1fc3d6282765f6c517770e992dd5a73efbfd870414d2e` |

Nested-condition taint source binding: `rust/tcl-compiler/src/taint.rs`, SHA-256
`9add81b6b62072fc47609d6d27fcd2773bc46ae609d5905b7ae0369dde5b759f`.

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
| `rust/tcl-compiler/src/analyser/handlers.rs` | `6f235d6b6a041b78392b6c6b6b8777caa7d35a9ddc420fb3dd4c838385cc2071` |
| `rust/tcl-compiler/src/dynamic_names.rs` | `fe84491ac3027317854b3e3a95b69a6e9511e43a8842385c88dfb038b695664e` |
| `runtime/rust/src/namespace.rs` | `7075e4032aef9cb56848e395d87f6596d7fa57ac72f3d2a679e830bf9d01f7b1` |
| `runtime/rust/src/interp.rs` | `ba167632dffced825ae356e179929e02728e7e3f0bc6dc88dc66a75bc4576b99` |
| `rust/xtask/src/gen_tmlanguage_keywords.rs` | `c202618b9c772c83bd748461cca73aec25d9024926e941531bf6fb4ff52f3abc` |
| `rust/xtask/src/gen_zed_queries.rs` | `51de8631aa4a68045e7232179c7a1b1da75875346258dc038203da2a42100121` |
| `rust/tcl-spec-studio/web/src/releases.ts` | `c7e664ca3ac101a94737d7cbd32cea76a541c9b5c4cb3f11a16f30d9a8fc91f9` |
| `rust/tcl-spec-studio/web/src/importReleases.ts` | `7ddb91e0d1d45d11333ef6e99080cf0ec7555c787f37e1c2b282debd9c39856b` |
| `rust/tcl-spec-studio-wasm/src/snapshots.rs` | `8d7250e535bafd5f1897e2ac32b6eb1a411638fba1c9d23789a1b8beeff675bb` |
