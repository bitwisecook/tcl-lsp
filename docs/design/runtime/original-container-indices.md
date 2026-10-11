# Original container indices

Container commands, regular expressions and compiled instructions use `tcl_cmd_core::index::NativeIndexOps` to convert an original index object. `ValueOps` remains the general value interface. The index interface adds complete command errors and actual operation, header and constructor ownership; it does not introduce a dependency from Syntax on CmdCore.

`resolve_value` accepts the original object. Physical C adapters invoke the existing original `NativeListIndexOps` conversion. Physical Jim adapters invoke the shared `native_jim_index::original_index` worker, including original Int/Index shortcuts, counted safe-expression input and CString end-suffix input. The Jim expression worker uses the actual interpreter's existing native Wide-expression producer. `resolve_for_ops` is the independently selected textual API: its physical adapter constructs a fresh original String with its own actual String producer. A diagnostic or Unicode projection cannot reconstruct an original index header.

Neutral analysis adapters explicitly use `IndexOperation = ()` and their own value-shaped key owner. Their lexical answers grant no native cache, interpreter, callback or reference ownership.

## Operation and getter order

Every public indexed command enters `run_index_result`, `run_created_index_result` or `run_index_output` before its first operand getter or length query. Physical implementations retain the same actual entered operation through getters, old-primary retirement, result construction and mandatory cleanup. A first Host refusal retains its complete `NativeExecutionError`, including its original cause; command adapters transport that structure through `CmdError` or `Completion`. A message string cannot recover that structure.

The conversion order is command specific. Jim extracts all flat `lindex` coordinates before converting any List; it converts both `lrange` coordinates before List length; `string index` converts its index before the subject String, while `string range` and `string replace` obtain the subject's count first. Physical C `lindex` and `lrange` use their existing original List owners and C release-specific copying/getter order, also when called through the public shared command interface. The selected range owner reuses checked List conversion, copy and backing-retirement kernels before its next mutation or result publication.

Runtime's `OriginalArguments::check_for` validates the actual interpreter allocation, captured currency and world. `check_original_for` also validates the exact captured argv/header slot. Equal runtime-context values or equal spelling cannot substitute another receiver or object. VM's `NativeOperationScope` and `OriginalNumericOps` provide the corresponding actual receiver and original-header checks. Allocation leases leave native reference counts unchanged and cannot revive retired native headers.

Old-primary retirement and incoming cache adoption share Runtime's `change_type_with_stage_check` and VM's checked primary replacement worker. Runtime's primary epoch rejects a reentrant replacement or retired selected slot. An incoming backing remains caller owned on refusal; successful adoption transfers exactly that backing. The original cause is checked before classifying any legacy `ValueError` projection.

## Selected keys and result ownership

`IndexedKey` retains the selected header and any genuine constructor ancestors. Its physical `IndexKeyOwner` is cloneable allocation transport: copying a sort key or merge-sort record adds no native reference. An arithmetic-series Index hook can produce a genuine fresh-zero scalar; ordinary List selection borrows an existing member. That distinction comes from the reached producer, rather than a pointer or refcount guess. Key selection preserves abstract Length/Index shortcuts and does not request GetElements merely to manufacture a key.

`IndexResult` records four actual roles:

| Role | Meaning | Cleanup |
| --- | --- | --- |
| `Borrowed` | Original argument or existing member, with its original owner still live | Release only the software transport |
| `Created` | Result of the reached fresh constructor | The physical backend's genuine fresh-result cleanup |
| `Detached` | Selected member transferred through a genuine result hold before a created ancestor retires | The selected result's actual returned ownership transport |
| `ReturnedOwnedTransport` | Existing C ListIndex/Range returned owner transferred by its actual owner API | The actual returned transport; no reference is added during original getters |

`key_result` first holds a borrowed selected child when a created ancestor owns it, retires that ancestor, then transfers the same held child. Runtime uses `Owned::into_native_unowned`; VM transfers the real result reference. A guard cannot donate an invented protective reference during getters. `discard_owned_index_result` accepts only a reached constructor, detached child or existing returned-owner transport. Runtime cleanup uses the native Bounce door and preserves an independent reference already published by a reached callback.

`lsearch -start` retains its original argv slot until index conversion. C search/sort option paths use the textual path interface; Jim sort paths retain their original member headers. `lsearch` and `lsort` keep these key guards through comparison, comparator callbacks, merge copies and error cleanup. Inline and deferred regular-expression captures also keep explicit constructor guards through later matches/getters and output List construction. Replacing a deferred `-all` capture retires its previous fresh capture. Variable-assignment callbacks retain their existing actual publication/completion contract.

## Original String replacement

Jim replacement uses `RawString::jim084_replacement_parts`: its prefix traverses original UTF boundaries, and its after-part copies the character-derived native byte count. A Rust Unicode slice would change the measured Jim result. Independent segment counts preserve the constructed Jim String cache, including truncated multibyte suffixes and counted NUL.

C replacement uses original native Unicode storage units and release-specific constructor/append rules. `NativeTclUtf::decode_storage_units` is separate from the public UTF decoder. The physical backend supplies `NativeCPlainChar` through `native_c_string_abi`; this records the local target's actual plain-char promotion, not an arbitrary external library, appliance build or `TCL_UTF_MAX` configuration. Legacy signed-char storage conversion does not change public UTF decoding.

`replace_c_parsed_range` is the original C9 parsed-range helper. `string replace` uses that same implementation after converting its original indices. Direct callers supply their actual in-place flag, parsed range and input owners. Omitted replacement with zero deletion returns the subject when the in-place flag permits it and otherwise invokes the original checked duplicate constructor. Proper C9 ByteArray backing remains eligible when a String representation is resident. Equal binary splices reuse an unshared subject only when the actual flag permits it; shared or non-in-place splices construct a fresh ByteArray; whole binary replacement returns the exact original replacement before reaching its getter. Entered EvalObjv reference schedules are a separate producer and cannot be inferred from direct helper inputs.

Private Unicode-prefix append reuses the shared checked native append worker. Original source duplication captures source context and the receiver primary before its actual dup callback, checks the original source, receiver and incoming backing before admission, and retains refusal before mandatory unused-result cleanup. Checked Runtime duplication requires non-exhausted primary epochs and rejects a receiver change followed by restoration before receiver retirement. Checked VM Dictionary duplication captures its original backing identity and epoch, uses allocation leases for original keys, and admits target key/value references after each reached key getter and insertion. Same-backing mutation refuses before the next member or target adoption. Its receiver cache adoption uses the same checked primary owner. It has no copy-on-write or protective native reference around the fresh private receiver.

Some reached native producers have no safe deterministic software representation. C9 partial binary deletion creates a nonzero-length NULL-source ByteArray whose initial payload is not initialised; the shared implementation returns a typed unavailable-producer result rather than using observed allocator bytes. Whole binary deletion of length zero is available. C8.4 replacement can retain an original Unicode backing pointer across index conversion; retirement of that exact backing returns a typed unavailable result rather than reading retired storage. A native observation does not turn unspecified or retired storage into a fallback recipe.

## Consumer interfaces

| Consumer | Shared interface |
| --- | --- |
| Runtime and VM List/String command adapters | Original value arguments to shared command workers; physical C List owners stay selected |
| Container edit and nested selection | `resolve_value`, checked List length/member APIs and explicit result roles |
| Search and sort | `drill`, `drill_original_jim`, `IndexedKey`, `key_value` and `key_result` |
| Original regular expressions | `original_jim_index`, original count/Unicode getters, complete operation and capture guards |
| Compiled index instructions | `resolve_value` on the actual original `Value`; full `CmdError` completion and checked transport width |
| Registry constant folding and transfer | Explicit neutral `NativeIndexOps` implementation, without native ownership claims |

## Implementer guidance

Pass original object handles down to the shared owner. Convert to Unicode only when the reached operation actually requests that projection. Preserve counted bytes separately from CString extent. Sample sharing from the actual original before acquiring any working reference. Retain full command failures until the consuming command publishes or returns them; never flatten an index failure to `Option`, suppress it with `.ok()`, or reconstruct Host refusal from a diagnostic.

Add an independently named producer purpose when another operation needs different native conversion, ownership or getter order. Primitive Int/Wide operands, parsed helper ranges, original index objects and compiled immediate coordinates remain distinct. Extend the shared lowest-level worker and its physical adapters before adding consumer-specific recipes.

Tests must name their question and canonical proof record. Native command snapshots, direct helper observations, pure geometry/ABI controls and software extension callbacks answer different questions. Compare only the fields the original driver actually measured, keep before-result-getter snapshots separate from materialised results, and preserve unspecified fields as unspecified. A coverage binding does not assert that a test executed.

Relevant original evidence is recorded in:

- [Jim original Index conversion](../analysis/name-resolution-proofs/jim-original-index-conversion-and-result-object.md).
- [Original counted replacement and index getter order](../analysis/name-resolution-proofs/string-original-counted-replacement-and-index-stages.md).
- [Indexed binary replacement and original caller roles](../analysis/name-resolution-proofs/string-original-binary-replacement-sharing-and-result-roles.md).
- [Direct C9 binary helper sharing](../analysis/name-resolution-proofs/string-original-binary-replacement-direct-helper-sharing.md).
- [Original arithmetic-series List conversion](../analysis/name-resolution-proofs/list-arithseries-original-multiple-index-conversion.md).
- [Original Host publication and fact transport](../analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md).
