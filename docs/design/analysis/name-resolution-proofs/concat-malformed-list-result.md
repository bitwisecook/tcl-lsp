# naming.concat-malformed-list-result

Kind: `native-observation`

## Problem statement

A concatenation result may contain an unmatched brace while still being a successful string result. Treating the result as an already parsed List would suppress a later native list error.

## Question

Does concat preserve a malformed list fragment as text, and what happens when llength subsequently parses that result?

## Conclusion

The captured C and Jim source controls keep concat output separate from the subsequent llength completion and exact error string. Successful concatenation does not confer a successful list parse.

## Scope

Exact malformed.tcl ASCII source and six retained shell outputs; source escape evaluation and later list parsing are distinct.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: jim jim0.84.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_concat](../../../../rust/tcl-cmd-core/tests/data/native_concat)
