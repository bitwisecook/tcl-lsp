# naming.source.original-surrogate-versus-replacement-escape-values

Kind: `native-observation`

## Problem statement

Lossy Unicode presentation can replace an unrepresentable logical source value with U+FFFD. If the original source escape is observably distinct from the replacement escape, that presentation cannot supply source identity or equality facts.

## Question

Do the original source escapes \uD800 and \uFFFD have the same public string value, and what do the exact adjacent, self, valid Unicode and braced literal controls report?

## Conclusion

The exact ASCII source distinguishes the quoted numeric escape \uD800 from \uFFFD on all five pinned C releases and current Jim: `string equal` returns `0` and both `string length` results are `1`. Adjacent \uD800 and \uD801 also compare unequal and the latter has length `1`. The replacement escape compares equal to itself; caf\u00e9 differs from cafe with length `4`; the braced literal \uD800 differs from its quoted decoded escape and has length `6`. All six original source processes exit `0` with empty stderr and all five rows; six separate fresh version queries also exit `0` with empty stderr. Therefore replacement presentation cannot stand in for these original source values when equality matters. This conclusion concerns only public string equal/string length outcomes for the exact source and observed distributions. It does not identify numeric storage units, object/getter/cache/header identity, variable/command name slots, wide escapes, surrogate pairing, generic evaluation, Native admission, BIG-IP behavior or any Rust assertion outcome.

## Scope

The retained source is fixed ASCII LF text, with no raw NUL or direct non-ASCII input. Quoted \u escapes are evaluated by each original CLI; braces retain literal escape spelling. Public commands emit ASCII boolean/length rows, without binary scan, object-vector entry, direct getters, private headers or string-byte inspection. Each receipt retains exact source argv, exit, timing, complete stdout/stderr, CLI digest, separately observed version-query argv/source/streams, explicit TCL_LIBRARY overlay for C and exact listed CLI/header/static archive/configuration/source inputs verified before and after. The original build269 input receipt is reused only under its exact recorded digest and listed input pins; no new compile is performed. Compiler version, unlisted inherited environment, unrelated system libraries and a launch program are unrecorded and are not invented. These observations do not determine representation, source edit permission or behavior beyond the five fixed public rows. Source-only lossless Unicode projections and Logical identity consumers require their independent owner contracts and test receipts. Existing Native records remain unchanged; BIG-IP is not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Original fixed ASCII LF file passed to stock CLI; quoted numeric escapes versus braced literal text; public string equal and string length rows. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

Exact five rows: SOURCE_SURROGATE_REPLACEMENT|0|1|1; SOURCE_ADJACENT_SURROGATES|0|1; SOURCE_REPLACEMENT_SELF|1; SOURCE_VALID_UNICODE|0|4; SOURCE_BRACED_ESCAPE|0|6. Whole source and separate version-query processes exit `0` with empty stderr.

### bigip

Status: `not-tested`.

No BIG-IP request or source/string result observation is executed by this proof.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_child_alias_inventory298](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
