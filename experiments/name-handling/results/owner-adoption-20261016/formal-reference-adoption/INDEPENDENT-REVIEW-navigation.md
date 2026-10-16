# Independent review of formal observations V3

Reviewed SHA256 `7cc7df805ad7547116860189325c2dd7f7d34ccede49cfe38a34205f2abaebf1`. All three manifest beforeimages match retained base and current live source; all edit afterimages match their manifest hashes. The narrow registry export hunk retains root-integrated prefix621.

Verdict: no concrete correctness/authority blocker found in V3. Approved for root integration/build review, with tests still unexecuted by this reviewer.

## Correction to my V1 review

The V1 report's proposed AST-only mutation defect was not established. I missed `OriginalAuthoredProcedureBody::matches_procedure`'s existing `self.lowered_body.as_deref() == Some(&procedure.body)` structural Script equality in a truncated source read. `Module::original_source_procedure_bodies` already requires that card check before exposing an Authored body, and the parameter owner repeats it. Therefore the original AST-only mutation leaves the source inventory before incidence; the V1 report must not be cited as proof of an actual authority gap. Preserve that historical review with this explicit correction rather than silently treating the defect as proven.

V3 usefully makes the guard local and independently testable: parameter ownership retains the actual selected BodyId; Authored expression config rechecks that exact issued body/card and immutable Script, then requires the expression to be a pointer-equal actual condition/ExprEval/AssignExpr/Return member with matching base. It separately reauthenticates the actual source command/operand/configuration and affine original content. The same-length first→other mutation adversary preserves CFG/source/tokens/base and asserts both card/inventory refusal while retaining the original unmodified condition positive.

## Retained boundaries

A supplied CompilationUnit must be the actual source Module and match complete original source bytes, owned body/function IDs, actual procedure pointer, prepared original declaration CFG and nested full lexer configuration. FunctionUnit source metadata input now must equal the Module input or its exact existing nested form; mismatched/absent whole input is terminal. The new adversary changes only that input while preserving CFG and expects incomplete observations.

The selected formal/card/topology owner remains genuine; caller-link formals do not become direct source parameters. Installed inventory is preserved and excluded from this declaration-only observation purpose. Default/missing/stale owners cannot supply parameter observations or Unused.

The Module original-word helper checks retained positioned Module ownership and actual analysis attachment before reborrowing complete original words. Present stronger layout/Native/Logical branches return their refusal terminally. Native syntax, Logical source transitions and Authored body source remain separate actual producers. Captured prefixes do not become newly written variables.

Incidence uses executable variable components under the original full grammar, including nested command source spans and literal scalar names. Escaped sigils and braced data remain inert. Unknown roles/calls, unsupported expression leaves, array-index expression coverage, changed frames, raw detached scans and traversal exhaustion mark observations incomplete. Unused still requires complete owned observations plus no reference or trait flag; missing data does not become absence.

The separate Native expression/ReturnKind route retains original metadata and full expression/base syntax equality, without borrowing the Authored incidence purpose. Source Passthrough/UsedInCondition observations grant no physical variable cell, read, Native entry, Normal result or return-fold admission. This patch does not close command-trace observer applicability or the project SourceHeader unknown-mutation failure.

No tests, builds, source/main edits or Git operations were performed. Author-recorded rustfmt/dry-run success is not candidate Rust acceptance; root must run the exact source-bound ordinary validation.
