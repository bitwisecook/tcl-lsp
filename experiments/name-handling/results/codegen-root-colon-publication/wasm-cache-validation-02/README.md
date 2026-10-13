# Strict WASM execution with a writable cache

The same real-link test executable and frozen source were run with
TCL_REQUIRE_WASM_LINK=1 and XDG_CACHE_HOME set inside this workspace. The original
cache refusals remain in compiler-wasm-validation-01. No source, admission guard,
ABI proof or native callback was changed.

Six genuine Runtime-linked WASM controls pass: evaluated body once, inert braced
body, evaluated name once, unknown-name abort, replaced definer through general
emission, and replaced definer through the actual native tier. The last control
proves the conservative generic route under that tier; it is not a compiled
procedure-entry positive.

Two native publication/table positives still fail their required shared-table
assertion before actual linking. Thus the additive original-argv native entry
path and foreign prior-fingerprint rejection have not been accepted through
genuine WASM. API and scoped Runtime software checks are separate facts.

All preexisting captured inputs and the test executable are unchanged. Exact
selected binary/Runtime/tool hashes, argv/environment/status and raw outputs are
in validation.json. Newly generated/third-party inputs and broad cached dep-info
coverage are classified separately; no full suite or every-consumer acceptance
is claimed. No Tcltest or F5 execution was used.
