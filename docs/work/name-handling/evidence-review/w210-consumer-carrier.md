# W210 consumer carrier (temporary source review)

Read-only review of `analyser/diagnostics/dataflow.rs` and two direct APIs in
`command_binding/declaration_flow.rs` / `script_binds.rs`; no build or code fix.
The three immutable `results/w210-*refusal/` experiments remain unchanged.
Latest execution: C8.4 W210 0/expected1, warning reports contain `absent`,
child frame/entry Procedure/Body matches caller, and warning getter calls are 0.

At `dataflow.rs:1575`, W210 obtains `declared_flow` solely through
`function_declaration_flow(..., &self.profile_registry())`.
That helper at 4122 searches SSA statements/terminators: source tokens,
token source binding, declaration report, then original-procedure ownership.
Any missing or rejected carrier becomes the same `None`.
The already-built sealed original-source report is not another input here.
At 894, conditional store diagnostics instead query
`head_identities.source_bindings().original_procedure_declaration_flow`
before the SSA route, using `self.registry` or `analysis_context().commands()`.
This is a definite difference in report carrier and registry argument; their
actual registry semantic-key equality was not recorded and must not be assumed.

Zero warning-getter calls does not identify which pre-getter guard failed.
W210 also requires `authored_procedure_read_advice` and `declared_flow`
(`dataflow.rs:1584–1595`, or 1639–1645), then report ownership at 1833.
Thus source cannot honestly label missing SSA carrier the executed first cause.

## Existing owner and smallest conditional implementation

`declaration_flow.rs:477` selects original declaration observations from the
exact Authored image; `original_procedure_entry_matches` at 187 requires Body
entry, declaration allocation offset, body offset/text and exact formal names.
`declaration_flow_report` at 411 requires unanimous original entries/config/
dialect plus the retained inventory; its cache key includes registry semantic
key. The returned report repeats original-procedure ownership at 498.
Do not add name-based lookup, another resolver or display-name recovery.

`script_binds.rs:393` already requires body config, complete resolved input,
current retained realm matching input/image/config, original recorded command
and the existing declaration recipe. It checks original image/body offset/text,
raw/formal parameter equality and actual image bytes; advice owns the read span.
Retain these guards and the per-read aliases/suppression/known-defined/array/
existence checks at `dataflow.rs:1840`; two warning candidates are not two
authorised emissions. Normal/Native entry, physical reads/stores and edits do
not follow from this conditional diagnostic report.

If absence is established, route W210 to the SAME original-procedure report API
and SAME selected registry as the store consumer, joined to current input/realm,
image, body, formals and frame. Keep SSA physical analysis separate. Before an
absence fallback, preserve a tagged distinction between no original SSA carrier
and a present carrier refused by original/reached-owner checks: the helper's
current `Option` cannot express it. A refused retained owner remains terminal.
This is a consumer proposal, not a proved patch or permission to loosen guards.

If needed, trace only W210 entry for `exercise`: IR procedure present, original
advice Some, SSA token/binding/report counts, first report-owner guard result,
selected registry-key equality and declared_flow Some. Stop at the first refusal;
there is no reason to rebuild metadata, lifecycle or child graph speculation.
Validate one existing direct missing-read control (exactly absent, not prepared)
and the existing populated setup/body/cleanup positive control (zero W210).
Also retain current invalid reached-owner refusal and dynamic/data-only controls;
do not infer their success from the new direct control.
