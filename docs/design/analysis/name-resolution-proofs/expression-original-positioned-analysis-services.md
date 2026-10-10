# naming.expression.original-positioned-analysis-services

Kind: `implementation-contract`

## Problem statement

An expression service can receive only detached nested text or lose full lexer configuration and byte positions through an input wrapper. Equal AST/text alone cannot identify the actual original operand, child extent or source lookup point.

## Question

How do shared expression services retain complete lexer configuration and positioned callbacks while authentic source consumers join the same original literal operand, AST, parent and child extent without borrowing Native completion or frame authority?

## Conclusion

AnalysisInputs retains the producer's complete source_lexer_config and separates prepare_expression_source from nested_expression_at. PinnedInputs forwards those fields and callbacks unchanged. ExprServices consumes the existing Syntax string_bytes_at/command_bytes_at coordinates and checks string interior byte geometry before original decomposition. Actual source producers retain the current parent statement or terminator and expression base. SourceSummaryContext::expression joins the selected original literal Expr operand, complete AST and optional authentic base to its current parent record; only a retained producer base permits descent through that parent's genuine child inventory. OriginalSummaryExpression preserves that parent and base, and expression_substitution maps inclusive child extents with checked arithmetic to the original source span. Cooked/captured operands, foreign or missing parents, detached equal trees/text and known source shadows cannot create that receipt. These source services preserve conditional Logical summary obligations independently of Native execution.

## Scope

Five marked software/API assertion definitions cover the fused top-level parent, authentic literal operand/child extent, cooked/foreign refusal, captured/shadowed refusal and complete configuration plus inclusive Unicode byte-position forwarding. Actual source inputs require the complete matching Module/input/configuration and per-point metadata owner. The explicitly unpositioned compatibility callbacks remain separate: their missing source config/positions retain no original-source receipt and cannot be substituted for withheld actual source inputs. Selected numeral/expression policies remain independent of source lexer configuration. Positions and AST equality provide no Native object/header, executed child/body, physical frame/cell/read, Handler/Normal completion, compiler instruction or edit-equivalence authority. No matching Rust assertion receipt is attached by this source binding; no mutable implementation pin is refreshed.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No original external provider process answers this positioned expression-service contract. The linked controls describe software source/configuration/coordinate transport and original ancestry; Native completion, frame and execution remain independent.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No original external provider process answers this positioned expression-service contract. The linked controls describe software source/configuration/coordinate transport and original ancestry; Native completion, frame and execution remain independent.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No original external provider process answers this positioned expression-service contract. The linked controls describe software source/configuration/coordinate transport and original ancestry; Native completion, frame and execution remain independent.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No original external provider process answers this positioned expression-service contract. The linked controls describe software source/configuration/coordinate transport and original ancestry; Native completion, frame and execution remain independent.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No original external provider process answers this positioned expression-service contract. The linked controls describe software source/configuration/coordinate transport and original ancestry; Native completion, frame and execution remain independent.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No original external provider process answers this positioned expression-service contract. The linked controls describe software source/configuration/coordinate transport and original ancestry; Native completion, frame and execution remain independent.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No original external provider process answers this positioned expression-service contract. The linked controls describe software source/configuration/coordinate transport and original ancestry; Native completion, frame and execution remain independent.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/value_transfer/inputs.rs](../../../../rust/tcl-registry/src/value_transfer/inputs.rs), `AnalysisInputs::source_lexer_config`: Retain the actual producer's full lexer axes independently of numeric context; explicit unpositioned compatibility returns no original-source authority.
- [rust/tcl-registry/src/value_transfer/inputs.rs](../../../../rust/tcl-registry/src/value_transfer/inputs.rs), `AnalysisInputs::prepare_expression_source`: Prepare an original-expression receipt only after the actual parent, selected whole operand and complete tree are validated.
- [rust/tcl-registry/src/value_transfer/inputs.rs](../../../../rust/tcl-registry/src/value_transfer/inputs.rs), `AnalysisInputs::nested_expression_at`: Carry original expression byte extents to the owning nested source consumer; actual missing parents/receipts refuse independently of unpositioned compatibility.
- [rust/tcl-registry/src/value_transfer/lift.rs](../../../../rust/tcl-registry/src/value_transfer/lift.rs), `PinnedInputs`: Forward retained full config, source preparation and positioned nested callbacks through value pinning without rebuilding source origins.
- [rust/tcl-compiler/src/tcl_expr_eval.rs](../../../../rust/tcl-compiler/src/tcl_expr_eval.rs), `ExprServices::string_bytes_at`: Validate exact interior start/end and byte length before positioned original string decomposition, preserving all retained lexer axes.
- [rust/tcl-compiler/src/tcl_expr_eval.rs](../../../../rust/tcl-compiler/src/tcl_expr_eval.rs), `ExprServices::command_bytes_at`: Forward actual Syntax AST command start/end through nested_expression_at rather than discarding the source position.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `SourceSummaryContext::expression`: Join genuine selected original literal Expr operand, tree, retained parent and optional actual producer base; detached/cooked/captured or conflicting producers refuse.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `OriginalSummaryExpression`: Retain the same current original parent and expression base without donating native completion or physical execution.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `SourceSummaryContext::expression_substitution`: Map inclusive original child extents with checked arithmetic to the same parent's genuine substitution inventory.
- [rust/tcl-compiler/src/value_transfer.rs](../../../../rust/tcl-compiler/src/value_transfer.rs), `LatticeDriver::original_expression_parent`: Retrieve the actual Module-owned statement carrier rather than a name or detached reconstructed source.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `interprocedural::transfer::source_context::tests::original_expression_fused_top_level_retains_its_existing_parent_record` (linked): Software/API source-ancestry, complete lexer configuration and positioned expression service transport. No Native Normal/frame/compiler/execution/object proof. No Cargo/native assertions executed by this agent. Independently selected numeric policy, explicit compatibility input and actual original-source receipts remain separate; no matching executed assertion result is inferred.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `interprocedural::transfer::source_context::tests::original_expression_receipts_join_actual_literal_operand_and_child_extent` (linked): Software/API source-ancestry, complete lexer configuration and positioned expression service transport. No Native Normal/frame/compiler/execution/object proof. No Cargo/native assertions executed by this agent. Independently selected numeric policy, explicit compatibility input and actual original-source receipts remain separate; no matching executed assertion result is inferred.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `interprocedural::transfer::source_context::tests::original_expression_receipts_refuse_cooked_and_foreign_producers` (linked): Software/API source-ancestry, complete lexer configuration and positioned expression service transport. No Native Normal/frame/compiler/execution/object proof. No Cargo/native assertions executed by this agent. Independently selected numeric policy, explicit compatibility input and actual original-source receipts remain separate; no matching executed assertion result is inferred.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `interprocedural::transfer::source_context::tests::original_expression_receipts_refuse_captured_and_shadowed_producers` (linked): Software/API source-ancestry, complete lexer configuration and positioned expression service transport. No Native Normal/frame/compiler/execution/object proof. No Cargo/native assertions executed by this agent. Independently selected numeric policy, explicit compatibility input and actual original-source receipts remain separate; no matching executed assertion result is inferred.
- [rust/tcl-compiler/src/value_transfer.rs](../../../../rust/tcl-compiler/src/value_transfer.rs), `value_transfer::tests::expression_services_forward_full_config_and_inclusive_unicode_child_positions` (linked): Software/API source-ancestry, complete lexer configuration and positioned expression service transport. No Native Normal/frame/compiler/execution/object proof. No Cargo/native assertions executed by this agent. Independently selected numeric policy, explicit compatibility input and actual original-source receipts remain separate; no matching executed assertion result is inferred.

A named test is a coverage binding, not a claim that it executed.

## Replay

Source-defined controls and current concrete interfaces are linked. Assertion coverage requires an independently compiled, listed and executed image matching those definitions. No Native process/private header/frame or mutable implementation digest is supplied by this binding.
