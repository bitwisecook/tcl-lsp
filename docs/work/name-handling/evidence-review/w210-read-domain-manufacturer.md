# W210 read-domain manufacturer (temporary source review)

No build, trace or compiler change. Reviewed three owners: declaration_preview,
original_name_value and ir. Prior consumer execution establishes recipe None,
while SSA report/source report ownership and registry-key equality succeed.

`original_name_value.rs:962` rebuilds `original_recorded_command` tokens with
`CommandTokens::from_segmented`; it checks the complete original vector but
does not reattach the receiver binding. `ir.rs:628–680` explicitly constructs
`source_binding: None`. This producer intentionally returns syntax only.
The inspected `script_binds::authored_procedure_read_advice` call passes those
tokens directly to the factory. Thus its input binding is source-proven absent.
`declaration_preview.rs:433` requires that binding and its invocation site.
The preceding `InvocationMetadataContext::for_analysis_input` at 430 remains
untraced: if it succeeds, the absent-binding guard necessarily returns None.
Do not claim that its metadata guard executed successfully from source alone.

## Remaining guards in execution order

After metadata and binding/site: exact source image/config; checked source
purpose. Logical purpose requires the existing logical declaration plus raw
parameter text. Otherwise: original row set; config and resegmented vector;
operand-layout advice; selected input dialect; registry-backed targets;
`native_procedure_read_recipe`; unanimous resulting recipes (`422–489`).
The native recipe (`1507–1580`) requires effective argv, metadata-aware registry
resolution, valid definition shape, parameter operand OriginalWord, original
formal topology, written body ordinal, literal body and same source origin.
`original_declaration_operand_input:1350` requires the row's execution policy
to retain a native recipe. Prepended operands require same-policy/current input;
written operands require original native-word geometry; expanded elements need
complete original-list correspondence. No later guard verdict is observed yet.

Keep the syntax-only producer unchanged. At the explicit authored-read consumer,
retain the existing realm-issued binding and attach that SAME binding to the
rebuilt, already-equal complete vector before calling the existing factory.
This forwards its source/site/config/metadata owner; it creates no point, frame,
handler, Normal grant or name-derived identity. The current realm/input/image
checks stay in place, and invalid retained-owner refusal remains terminal inside
the checked factory. Do not replace None with an earlier guessed declaration.
Before changing semantics, the smallest next trace records only factory metadata
Some and incoming token binding/site presence for `exercise` at offset61.
If a reviewed carrier-forwarding patch is tried, run the single existing direct
missing-read and populated setup/body/cleanup controls; do not bypass later
formal/source-purpose guards if another refusal becomes reachable.
