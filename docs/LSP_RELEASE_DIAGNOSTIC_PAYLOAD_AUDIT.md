# LSP diagnostic payload audit: #2426

This scoped source audit was taken on 4 November 2026 from `work` at
`a121a9ab80a087d381a8d3d9169b3b9642628c97`. The issue was fetched again and
remained open. This is an implementation inventory, not a claim that #2426's
repository-wide acceptance criteria pass.

## Scope and existing owners

The review covers compiler/analyser, Core, database, native and WASM servers,
and editor adapters. It searches message, description, replacement and action
title reads, including aliased message variables, then separates tests,
display, edit validation, aggregation and semantic recovery. VM, runtime
implementation, BIG-IP/F5-specific consumers, CLI tooling and Spec Studio are
outside this release lane. Those omitted surfaces still require their own
audit before the issue can close.

The existing owners are:

- `analyser/types.rs::Diagnostic` with `DiagnosticSubject` from
  `analyser/diagnostic_subject.rs`. Source subjects retain the original name,
  invocation, naming policy and lookup context; unsupported subjects remain
  absent.
- `tcl-lsp-core/src/diagnostic_policy.rs::Finding` retains the original
  analyser/compiler diagnostic or issued optimiser report. Its
  `structured_data` projection uses the existing diagnostic-subject or
  context-diagnostic owner.
- `tcl-lsp-core/src/diagnostic_subject.rs::DiagnosticSubjectData` serialises
  version 1 and checks the version, code and supported subject when decoding.
  `diagnostic_subject_data` preserves an absent subject.
- `tcl-lexer::ParseCut` retains `ParseIssueKind`, exact native message and
  original offsets. Parse kind already belongs to the lexer; recovery needs
  no diagnostic wrapper or new parser.

## Residual semantic consumers

Locations below refer to the source cut above. Later edits may move lines.

| Consumer | Current decision | Smallest safe candidate |
|---|---|---|
| `command_binding/incomplete_body_metadata.rs:85` | Matches `fatal.cut.message` to select a quote or brace and recover an incomplete procedure body. | Match existing `ParseCut.kind`; preserve independently checked source geometry and header words. Candidate prepared in this lane. |
| `compile_service.rs:1124`, called at `1152` | Matches the guest error message to decide which opener a retained malformed-tail term must point at. | Keep the existing structured cut through both text and byte paths, and validate the term using its kind. Preserve message bytes in `FatalTail`. Candidate prepared in this lane. |
| `analyser/diagnostics/proven.rs:640–645` | Deduplicates written/proven index diagnostics using code, span and message, ignoring their retained subjects. | Define same-access identity from the existing registry diagnostic subject and relevant index operands; unsupported subjects must not be guessed from text. |
| `optimiser/manager.rs:98–117`, then `optimiser/helpers/select.rs:50–80` | Sorts by message before stable overlap arbitration. Otherwise tied span/priority/length candidates inherit message order, which can select a different replacement. | Define a stable rewrite identity/order from the issued rewrite facts before presentation; retain group and overlap rules. Add a tied-candidate wording-change control. |
| `optimiser/mod.rs:158–174` | `Optimisation` equality and hashing include message. | Separate semantic rewrite identity from whole rendered-row equality where consumers deduplicate; audit each equality use before changing the public comparison. |
| `tcl-lsp-core/src/code_actions.rs:2399–2410` | Deduplicates context actions by title and replacement strings, omitting edit ranges. | Retain complete structured edits and stable action intent; title changes must not affect identity. Parent owns this independent fix. |
| `editors/vscode/src/extension.ts:593` and `extensionBrowser.ts:1236` | The generate-docstring command chooses an action by its English title prefix before checking its server command. | Carry a stable docstring action intent through the existing action adapter and select it by that intent. Generic `source` kind also includes other source actions. |
| `tcl-lsp-core/src/refactor/extract_proc.rs:172–201` | Exported legacy helper derives a rename target by scanning `definition.new_text` for `proc ` and whitespace. | Retire/restrict the compatibility helper or require the existing current-analysis entry point. The live Core action path at `code_actions.rs:1985` already uses `extract_proc_rename_command_for_analysis`, with parsed source/name ownership. No in-repository caller of the legacy helper was found. |
| `analyser/class_hierarchy.rs:976–1001` | Hierarchy error aggregation uses rendered strings, including `e.message`, as duplicate identity. | Retain the MRO owner's structured error kind and actual class subject before formatting. This is error-list aggregation, not name lookup from a message. |

The incomplete-body candidate extracts a private helper so controls can pass a
real parser-issued cut with only its presentation changed. The malformed-tail
candidate returns the existing `segmenter::SegmentedScript` from the lowering
helper instead of discarding kind in a `(start, message, term)` tuple. The
four consumers were checked: compiler service, Tcl 8.4 compile adapter, inline
body emitter and empty-script codegen query. The two emitters still copy the
same native message bytes; the empty-script query needs no text.

## Migrated consumers and presentation reads

The original issue's examples have already moved to typed owners:

- Server `w120_required_package` (`lib.rs:40790`) calls
  `required_package_key`; package refinement does not inspect fix text.
- Server `refine_original_w123_diagnostics` (`lib.rs:39947`) uses the
  unresolved-command subject's actual name input and invocation. An absent
  subject preserves the unknown diagnostic rather than parsing a quote.
- Database `project_source_diagnostics_for_analysis` (`lib.rs:2194`) and
  `retained_project_diagnostics` (`lib.rs:2163`) use retained unresolved and
  arity sites with source currency, not message extraction.

Other inspected reads have distinct purposes:

| Surface | Classification |
|---|---|
| Core `lift_fixes`, `rewrite_actions`, server `lift_report_with_payloads` | Render titles/messages and copy independently issued edits or typed payloads. Fix descriptions do not recover a subject. |
| Core newline normalisation, refactoring proposal application and owner-backed proposed-source checks | Materialise or independently validate actual edits. They do not derive diagnostic identity from prose. |
| Analyser unresolved diagnostic punctuation and taint callback label replacement | Presentation formatting; names and callback facts are supplied separately. |
| Native parse/codegen error message transport | Preserve Tcl/Jim guest error bytes. Transport alone supplies no tooling identity. |
| `compiler_checks.rs::sort_diagnostics` | Deterministic rendered diagnostic order; no overlap winner or recovered name was found here. |
| Core `minify.rs::unminify_error` and `remap_line_references` | Heuristic display annotation of an external error with a symbol map. Its delimiter scans must not become source/name authority. No diagnostic resolution or action selection was found. |
| VS Code chat context and compiler-explorer error rendering | Display/search/prompt text. The field-name adapter in `clientCore.ts` accepts camelCase/snake_case edits without recovering identity from their contents. |
| Zed, JetBrains, Neovim, Emacs, Helix and Sublime adapters | No production diagnostic-message parser was found in the reviewed adapter sources. This does not cover excluded tooling or establish runtime payload round trips. |

`analyser/authenticated_report/field_comparison.rs:390` additionally compares
messages and fixes **after** typed-subject correspondence. It is a full report
integrity comparison, not a name extractor. Its use as an acceptance gate needs
separate consideration under #2426's presentation-invariance requirement;
removing a global equality field without reviewing its authority boundary is
not a safe migration.

## Remaining acceptance gaps and verification

There are still separate analyser, compiler-check, Core and wire envelopes.
`compiler_checks::Diagnostic` has optional source context and taint subjects,
but no declared typed contract for every family. On the wire,
`lift_report_with_payloads` prefers issued rewrite data over
`Finding::structured_data`, and emits no related information. These are
coverage/composition gaps to assess against the complete issue contract;
they are not evidence of another text parser.

No maintained repository-wide presentation-parsing drift gate was identified
in this scoped review. The inventory above does not substitute for that gate
or the required serializer, opaque-name and family coverage tests.

The two compiler candidates have fixed-input metamorphic controls for unchanged
kind with changed/empty/quote-containing messages, and changed kind with
unchanged messages. Existing native C8.4 parse-context fixtures are retained and
now exercise the actual byte-plan entry point, including exact native text.
The controls are prepared but **not compiled or run** at this checkpoint;
the shared compiler queue is reserved for higher-priority work. Formatting and
`git diff --check` pass. Full push gates remain unrun. No issue comment or
closure was made by this lane.
