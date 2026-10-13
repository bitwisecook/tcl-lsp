# Original command inventory observations

Fresh `shared-family-*` captures retain executable inputs, stdout/stderr, linked-library and C library seals, provider environments and input digests. `validation.json` checks 132 presence observations, six ordinary package outputs, and six provider-specific ambiguous-namespace outputs. C 8.4/8.5/8.6/9.0/9.1 and the selected latest Jim executable each returned zero, produced no stderr, and retained all sampled binary/library bytes.

The ordinary original package load preserves root `:lost` alongside pre-existing `lost`, literal namespace `N[0]`, simple wildcard `p*`, sibling namespace `N0`, and helper-prefix sibling `__spec_probeOther`. C returns checked root-address operands `::N0::pZ`, `::N[0]::p*`, `:lost`, and `__spec_probeOther`; Jim returns its original flat keys `:lost`, `N0::pZ`, `N[0]::p*`, and `__spec_probeOther` in its actual sorted order. C refuses the whole snapshot after a package introduces namespace `N:`. Jim retains callable flat key `N:::x`.

Earlier failed Jim captures used a fixture containing the unavailable `package ifneeded` subcommand; their original exit/stderr remain in `ordinary/jim` and `ambiguous/jim`. Fresh package-file controls use Jim's actual `auto_path` loader. No capture replaces an earlier observation or renews a historical source digest.

These are original executable script-form observations, without tcltest. They do not establish compiled CLI/Studio assertions, Native command authority, F5 answers, Runtime/WASM acceptance, or any other provider's behavior. The authored Rust selectors remain pending a separately authorised fresh build/capture:

- `original_c_package_inventory_keeps_root_colons_and_literal_namespace_words`
- `original_c_package_inventory_refuses_ambiguous_namespace_root_addresses`
- `original_jim_package_inventory_retains_flat_root_keys_without_namespace_children`
- `literal_registration_inputs_remain_unresolved_and_do_not_fold_root_colons`
- `matching_original_operands_share_a_draft_without_root_reinterpretation`

`current-contract.md` describes the current intended API contract. `main-safe-consumers.patch` uses only owned consumer files plus additive Syntax modules/exports; its sampled Main preimages must still match at integration. No staging/commit/push occurred here.

The lookup owner reports strict06 Compiler library typechecking passed with the fresh source closure including these Syntax helpers. This establishes dependency ingress only; no CLI/Studio or Runtime inventory selector executed in that check.
