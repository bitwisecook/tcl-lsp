Compiler body identity interface

`ProcedureId` is an opaque `(module scope, allocation ordinal)` value from the shared core-types owner. `Module::retain_procedure` and `Module::retain_body_unit` issue IDs using the module's clone-shared issuer. Cloning a module preserves existing IDs and shares future allocation state; an ID from a separately built module cannot join by the same ordinal. The ordinal is suitable for a within-module presentation field only.

`ProcedureInventory::iter()` yields `(id, procedure)` and keeps every source body, including repeated definitions and declarations whose printed names coincide. `get(&id)` selects an exact retained row. `Procedure.qualified_name` and `.name` remain presentation fields. `Procedure.publication` carries authentic original naming geometry without Native admission. `Procedure.declaration_site` carries the original allocating instruction; `.body_kind` separates named, event, lambda, namespace-script, installed and original-declaration bodies.

`LatticeRequest.body_id` carries the actual body ID into neutral-memo result joins; it is not a requirement to put that ID into the neutral cache key. `TaintCascadeCallback` takes `(ProcedureId, &str, &InterproceduralAnalysis)`.

`FunctionUnit.body_id` retains the exact ID after fresh or memoised builds. CommonAOT caller/SSA keys include that ID, and declaration identities retain the ID independently of their display and source extent.

The primary joins use the same ID through `CfgModule.procedures`, `CompilationUnit.procedures/body_units`, `ModuleAsm.procedures/procedure_provenance`, `NativeOp::DefineProc.entry_key`, and WASM function/table plans. An implementation target joins through `Module::source_procedure_for_target` after the selected implementation allocation, exact original source site and source image agree. An original callback producer exposes `original_ir_procedure_id`. Source-body recovery and Native compiler/handler/table admission remain independent checks.

Adapters should use:

    for (id, procedure) in module.procedures.iter() {
        let unit = compilation.procedures.get(id);
        // Report id.ordinal() and procedure.qualified_name as separate fields.
    }

`unique_display`, `get_unique_display` and `unique_displays` are explicit conservative adapters for legacy readonly analytical summaries. They refuse every repeated display, including equal bodies; they do not resolve written Tcl names. Original selected calls must use their allocation/source owner. `display_entries` traverses all duplicate presentation rows and must not be collected into a name-keyed body map.

Native-tier report rows use `NativeFunctionId::TopLevel` or `NativeFunctionId::Procedure(id)`; each row retains its display name separately. `function_by_id` is the exact join. The display helper `function(name)` refuses duplicate names.

Current implementation status: the primary slice passed scoped `cargo check --offline -p tcl-compiler --lib` in retained typecheck-06 (zero errors,12 warnings). Compiler tests and Runtime/WASM/native-entry acceptance remain pending. Legacy interprocedural/call-site/frame-summary maps and reporting adapters still need migration or explicit conservative refusal. These interfaces are not evidence that any Native entry or table is admitted.
