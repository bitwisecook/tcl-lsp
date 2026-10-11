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

## Original C String units and constructors

`string::native_c` is the shared C String index/range producer for command and compiled consumers. `index_c_string_storage` reaches the original String representation and its count; it bypasses character-length shortcuts only when the actual unit/range getter requires String conversion. `index_character_count` retains its independent short-string and ByteArray shortcuts. A snapshot describes already reached storage and grants no conversion authority.

C8.4/8.5 count conversion eagerly retains Unicode for a multibyte extent. C8.6 and C9 retain a count before reaching Unicode. One-byte-per-unit `string index` promotes the selected byte through the local backend's actual C ABI for C8.4/8.5 and uses unsigned bytes in later supported builds. Multibyte units use the existing checked original Unicode getter. The result uses the native unit encoder, including modified NUL and individual C8 surrogate units. The ABI issuer describes this process and does not attest an external library's `TCL_UTF_MAX` or a BIG-IP build.

Range conversion has a separate raw-byte branch. C8.4/8.5 copy resident bytes when the original count equals their byte length; C8.6 and C9 use that branch before Unicode is present. This preserves invalid high bytes and the C8.4/8.5 raw four-byte slicing behavior. Other ranges use a fresh native Unicode constructor. An entire C range remains a fresh result; an entire Jim range can borrow its subject.

C8.4's binary command branch captures an allocation-only original primary receipt before index conversion and checks it before reading its captured storage. Runtime's primary epoch rejects replacement and change-and-restore. VM's Weak immutable backing receipt rejects withdrawal or replacement without keeping the backing alive; an independent actual shallow owner can keep that same backing allocation valid. This is a backing-lifetime check and does not claim a general VM primary epoch. C8.5 rereads the original binary header after the index getters through the same checked byte getter/cache worker used by its physical adapter. C8.6 and C9 select binary eligibility in their later unit/range getters, independently of the preceding character-length shortcut.

C9 eligibility uses the proper ByteArray object type even when a String is resident. The public `PureProperByteArray` enum member therefore tests the proper backing predicate; resident bytes do not determine that predicate. C8.6 character length accepts any ByteArray, while its later unit/range binary shortcut requires absent resident bytes. These purposes must not share an eligibility guess.

The [original index/range proof](../analysis/name-resolution-proofs/string-original-counted-index-range-storage-and-results.md) retains 116 actual command outcomes and four explicit Jim constructor omissions. Software controls compare original public type names, result residency, deterministic bytes and supported alias predicates. Direct software owners supply their own references; the controls do not infer the native `EvalObjv` temporary reference schedule, private C cache contents or callback lifetime from those public rows. Separate source-backed software controls exercise eager/lazy cache admission, actual extension callbacks and original storage retirement.

To extend this interface, identify the selected getter and constructor first. Preserve whether it reads the original bytes, a native Unicode unit, a proper binary payload or a counted raw slice. Use the checked physical String and byte-cache siblings at reached conversions, retain the full original failure before mandatory fresh-result cleanup, and carry actual constructor/result roles. A release's replacement observation does not establish its index/range behavior.


## Search and sort option paths

`NativeIndexOps::original_c_option_path` prepares original `-index` operands
through `native_option_index::prepare_original`. Actual C adapters select this
worker; explicit neutral adapters retain lexical paths. Jim keeps its own
original index/path consumer. `PreparedPath` stores either neutral spellings or
pure `NativeCompiledListIndex` coordinates. A prepared native path never parses
its original header again while selecting keys or building subindices.

C8.4 `lsort` has one scalar index, so it does not first convert that original
operand to a List. Its `-1` coordinate selects the entire key. C8.4 `lsearch`
uses its native option table without `-index`. C8.5 converts each original path
member with the native biased end value `-2`. C8.6 selects its distinct Int32,
end-offset and limited arithmetic encoding. C9 uses the original wide-index
producer and original offset-primary sign, with the actual local backend's
`Tcl_Size` width and distinct `LIST_MAX` allocation bound controlling encoding.
`NativeOptionIndexLayout` derives `ListStore` slot alignment and pointer width
from the local C target. Fixed source controls cover both standard 32-bit and
64-bit geometry and the exact numeric/relative encoding boundaries. The 192
command observations do not measure these large bounds. This descriptive local layout
cannot authenticate a foreign C library or BIG-IP build.

Every `lsort -index` occurrence is validated when reached. C8.6 and C9 then
quietly prepare only the last original operand after all options and before
input List conversion. The quiet pass uses the same owner and actual original
headers. Neither an empty input nor a later replacement option bypasses the
reached validation. C8.6's CString suffix integer getter has a separate local
temporary with type NULL, resident CString bytes and native reference count
one. It uses the selected Int32 worker and mandatory cleanup. The original
option/member header gains no temporary reference or donated numeric cache.
A local allocated temporary does not claim the provider's C stack location.

The option member view retains allocation safety, selected original header
identity and backing generation. It adds no original header or child native
references. Each reached coordinate checks the original view again before
using the next member. Runtime's immutable C9 arithmetic-series cache uses its
own reached array and child allocation leases; it does not convert through a
textual path. Withdrawn original backing cannot donate still-live old children
to another operation. `retain_index_failure` uses the actual interpreter's
unconditional first-cause latch before mandatory temporary, view or key cleanup.
All neutral implementations explicitly use a no-op latch.

`Path::drill` uses the shared `IndexedKey` roles and reaches selected Length
before Member. End-relative encoding stays in `NativeCompiledListIndex`.
Consumers must use `Path::decode` with their actual meaning of `end`, rather
than decoding a path through a current Unicode spelling. Guest error metadata
is structured: native index error codes, option-item propagation, CString
message extents and the independently selected String result producer travel
in `CmdError` without message reparsing.

The original command/child-cache observations and their bounds are recorded in
[list-original-option-index-path-preparation-and-results.md](../analysis/name-resolution-proofs/list-original-option-index-path-preparation-and-results.md).
Software controls compare exact success/error result bytes and original child
cache types from all 192 measured C/Jim cases. They compare child residency only
on successful commands, because native EvalObjv error propagation can separately
format argv. Provider temporary reference counts, private caches, error-stack
frames and callback chronology are not inferred from those public predicates.
The extension callback control independently checks actual software backing
withdrawal, preservation of the first cause and absence of a protective hold.


C search paths and shared sort key paths use `index::checked_operand_bytes`,
which borrows the actual selected original String owner and retains a first Host
cause before the caller releases its key or temporary. Allocation-only key
transport does not provide a native reference. Neutral adapters declare their
neutral byte implementation explicitly. Jim search keeps its selected original
String owner and its separate command and Enum schedule.

Stride arguments use `index::original_list_stride` and the independent
`NativeNumericCommandPurpose` scalar owner. They never enter Index expressions.
Tcl 8.6 sort uses Int; Tcl 9 search and sort use Wide; Jim search and sort use
Long with the selected primitive target ABI. Tcl 8.4/8.5 reject stride in their
actual option tables, and Tcl 8.6 search rejects it. Search requires a minimum
of one and sort a minimum of two after successful conversion. Primitive cache
changes remain on the original argument when a command minimum or primitive
range failure is returned. A target that the shared Long primitive does not
support refuses through that owner instead of borrowing a Wide result.

Implementers must retain a checked String or scalar failure before dropping
any key/view/result transport, preserve the original full `CmdError`, and use
the same captured operation for later checks. Add a scalar command purpose only
when the selected source uses that primitive; neutral textual parsing has no
native cache or header authority. Public-row controls compare the exact recorded
predicates, while software callback/reference controls remain separate.
