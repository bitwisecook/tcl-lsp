# ILX variable-handle source relation

The minimum source is:

```tcl
when HTTP_REQUEST {
    set h [ILX::init p e]
    ILX::call $h m
}
```

The corresponding extension source is:

```javascript
var f5 = require('f5-nodejs');
var ilx = new f5.ILXServer();
ilx.addMethod('m', function (req, res) { res.reply('ok'); });
```

Use the `f5-irules` ContextRegistry and
`tcl_irules::ilx::ilx_method_calls(source, context.commands())`. The document
route is `ilx_method_calls_from_analysis` with that document's retained input.

## Cause found by source inspection

On `work`, `ilx_method_calls_from_source_context` always writes `target: None`.
It retains a `source_target` only for an inline constructor; a variable operand
has neither target. This is deliberate, rather than a failed compiler lookup:
`original_ilx_source_cards_do_not_derive_handles_from_cell_or_label_guesses`
explicitly asserts that the example has no target or source target.

The current implementation contract
`docs/design/analysis/name-resolution-proofs/original-ilx-method-source-candidates.md`
forbids deriving evaluated handles from nearby `set` syntax. Core therefore
returns only the queried call site for references and cannot offer a definition
for this variable operand. The server's closed-sibling input fix does not
change that earlier refusal.

At `origin/rust` (`fcb7927f02b5395753247b322fdd7be87ae5c731`), the same module
has a local `HandleScope`; `record_handle_binding` and `handle_target` propagate
the constructor labels. Its existing
`ilx::tests::a_literal_handle_and_method_resolve` expects `(p, e)` (using longer
labels in its fixture). This paragraph is source inspection; execution results
are recorded separately when available.

## Contract needed from the compiler owner

Provide a current, authenticated source relation from the original remote-handle
constructor through its selected setter and later variable operand in the same
eligible source frame. Preserve the complete image, input, configuration,
Registry schema and positioned original words. Reassignment, unknown writes,
observers, frame changes and ambiguous reaching definitions must withdraw or
widen the relation. Use the registry's remote-method and handle-binding data;
do not add command-name matching to a consumer.

This can be conditional source provenance, with explicit applicability and
result obligations. It need not claim that an RPC succeeded or issue a live
runtime handle. The existing `OriginalSourceClassHandleBinding` is an analogous
compiler-owned source relation, but its constructor/class contract cannot be
relabelled as a remote handle. `value_provenance::const_contributors` handles
literals and copies; it deliberately refuses command-substitution results and
cannot supply this relation.

Once that owner is available, BIG-IP can project it into the separate readonly
source-candidate navigation/reference path. Keep `target`'s runtime meaning and
the current negative assertions. Do not restore the old consumer-local scope
walker or join unrelated calls solely by method/plugin text.

## F5 authority

`rust/tcl-registry/src/remote_method.rs` records the fetched F5 documentation
for `ILX::init`, `ILX::call`, `ILX::notify` and `ILXServer.addMethod`.
`commands/irules/ilx__init.rs` supplies the two-label example and the registry
descriptor. C Tcl can adjudicate variable assignment and frame semantics; it
cannot establish ILX execution or a running BIG-IP extension.
