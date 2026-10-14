# Source-internal owner registration

The separate source manifest registers exact owner and consumer files, crate,
modules, declarations, declaration visibility and actual source routes. The
existing Runtime internal receiver mode retains its original direct-file,
no-argument method and typed std::cell::Ref getter checks without alteration.

Only successfully validated source rows cover a restricted entry in the public
owner map. An unregistered or unsupported restricted declaration still fails.
No private API is exposed or removed from validation. A source registration
never provides Native frame, receiver, handler, provider or observer authority.

Nested modules follow unconditional lib.rs and file module declarations. The
gate rejects path overrides, missing/duplicate routes, wrong files/crates and
conditional owner/consumer declarations. It resolves actual direct imports and
reexports; exact import/reexport aliases preserve canonical identity; type aliases,
unresolved aliases, binding macros, untyped receivers and unavailable routes do not
provide nominal owner identity. Supported routes are static, type, self,
parameter:<binding>, producer:<canonical function>, field:self.<field>,
producer-field:<Type::method>:<field> and parameter-field:<binding>:<field>.
Option/Vec shape, original constructor return signatures and actual struct
fields retain their independent source identities. Only bounded iter/skip/zip
and map/any closure shapes propagate the validated item type and route.

The audited 13 mappings remain in the separate docs packet from
fixtures_docs_next. Broader private-looking inventory rows remain triage until
registered under a supported actual route; no lexical inventory is acceptance.

Validation: scratch Rustfmt and Main patch applicability pass. Fifteen fixed gate
controls cover genuine nested access, typed parameter/field/producer/reexport
routes and explicit scope, shadow, alias, macro, conditional, file, module,
crate and duplicate-declaration refusals. Cargo is unexecuted. Existing Runtime
receiver implementation and controls remain unchanged. No Main source was edited.

Independent refusal controls clear every pattern binder in its actual lexical
scope, reject enclosing type/const generic receiver ambiguity, verify direct
intermediate file-module visibility, and reject statement/item macros that can
create bindings. Genuine reexports retain their own declaring-module access;
unrelated binders do not withdraw an unaffected original typed parameter.

Source declarations accept only explicitly named passive doc/lint/must_use/
inline/cold attributes. Availability, path, cfg_attr and custom rewriting
attributes decline. Type/const generic parameters and nominal type arguments
are unsupported; lifetime-only owners and bounded original Option/Vec shapes
remain distinct.
