# naming.return-option-pairs-release-shape

Kind: `native-observation`

## Problem statement

Pair count, release-specific option availability and the procedure boundary determine Return semantics. A sole dynamic value cannot be donated to an option-selector role.

## Question

How do the original return -code and -level pairs differ from a sole dynamic option-looking result?

## Conclusion

C8.4 rejects the -level pair in the captured sources while later C and Jim accept it; a sole dynamic -code remains the ordinary result on all six. Exact boundary-settled results remain distinct from private raw completion storage.

## Scope

Five original option-pair/sole-dynamic sources from option_pair_source.tcl, six captured providers.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: jim Jim0.84.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_return_single_result/option_pair_manifest.json](../../../../rust/tcl-registry/tests/data/native_return_single_result/option_pair_manifest.json)
- [rust/tcl-registry/tests/data/native_return_single_result/option_pair_source.tcl](../../../../rust/tcl-registry/tests/data/native_return_single_result/option_pair_source.tcl)
- [rust/tcl-registry/tests/data/native_return_single_result/source.tcl](../../../../rust/tcl-registry/tests/data/native_return_single_result/source.tcl)
