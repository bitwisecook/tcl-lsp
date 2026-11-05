# naming.return-sole-result-option-spellings

Kind: `native-observation`

## Problem statement

A sole value such as -code or -level can be mistaken for the start of an option pair. Treating it as metadata instead of result changes completion and value.

## Question

Is one return argument an ordinary result even when its value resembles an option?

## Conclusion

Every retained C and Jim run returns the sole VALUE/-code/-level/-bad/-1 argument as its result through the procedure boundary. This does not describe multiword option parsing or raw Return code before the boundary.

## Scope

Exact source.tcl, six shell providers; caught procedure completion and result text only.

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

- [rust/tcl-registry/tests/data/native_return_single_result/manifest.json](../../../../rust/tcl-registry/tests/data/native_return_single_result/manifest.json)
- [rust/tcl-registry/tests/data/native_return_single_result/option_pair_source.tcl](../../../../rust/tcl-registry/tests/data/native_return_single_result/option_pair_source.tcl)
- [rust/tcl-registry/tests/data/native_return_single_result/source.tcl](../../../../rust/tcl-registry/tests/data/native_return_single_result/source.tcl)
