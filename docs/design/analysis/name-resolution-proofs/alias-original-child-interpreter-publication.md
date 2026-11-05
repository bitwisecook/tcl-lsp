# naming.alias.original-child-interpreter-publication

Kind: `native-observation`

## Problem statement

Parent-to-child alias creation uses provider-specific APIs and can retain different returned values while accepting the same original call spellings. Cross-interpreter ports must preserve each API result instead of inferring shared storage or a single returned-name protocol.

## Question

How do rooted, relative-qualified, unqualified and extra-root alias names publish and resolve for parent-to-child aliases under each provider's actual supported child-interpreter API?

## Conclusion

Each recorded C release uses interp create, interp alias $child $name {} list VALUE and interp eval $child. For the four exact names ::A::a, A::relative, plain and ::::extra, creation is caught with code zero, returns the original alias name, and the child call returns VALUE with code zero. Jim 0.84-9-g5bac7c9 uses interp to create its child handle, $child alias $name list VALUE and $child eval. Its four creation codes are also zero, but each creation result is the empty list field {}; each child call returns VALUE with code zero. Both branches delete their child through the corresponding API and all six whole processes exit zero with empty stderr. The original per-provider rows retain the return-value distinction. Matching public child invocation results establish no private table-key, allocation, cross-interpreter storage, alias header, frame, namespace-token or complete API equivalence. BIG-IP is not tested, and no source-carrier or edit authority follows.

## Scope

One original fixed ASCII LF file per fresh provider process selects an explicit C versus Jim branch by its reported patchlevel, creates one child, tests four original alias names and deletes the child. The aliases target the parent list VALUE prefix; child evaluation is exactly [list $name]. No nonroot child current namespace, arbitrary target, target rename/delete/replacement, alias loop, NUL, Unicode, continuation/control escape, entered receiver frame or interpreter lifetime after deletion is measured. API return values are retained independently and not equated.

## Provider answers

### tcl8.4

Status: `observed`. Channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim. Version: tcl8.4 8.4.20.

Actual C interp alias API 8.4.20 records four exact rows: child_alias ::A::a 0 ::A::a 0 VALUE; child_alias A::relative 0 A::relative 0 VALUE; child_alias plain 0 plain 0 VALUE; child_alias ::::extra 0 ::::extra 0 VALUE. Creation results remain the original alias spellings while each child call returns VALUE with code zero. This is the measured provider-specific API branch; no return-value or private storage/key/frame equivalence is inferred. Whole process exits zero with empty stderr.

### tcl8.5

Status: `observed`. Channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim. Version: tcl8.5 8.5.19.

Actual C interp alias API 8.5.19 records four exact rows: child_alias ::A::a 0 ::A::a 0 VALUE; child_alias A::relative 0 A::relative 0 VALUE; child_alias plain 0 plain 0 VALUE; child_alias ::::extra 0 ::::extra 0 VALUE. Creation results remain the original alias spellings while each child call returns VALUE with code zero. This is the measured provider-specific API branch; no return-value or private storage/key/frame equivalence is inferred. Whole process exits zero with empty stderr.

### tcl8.6

Status: `observed`. Channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim. Version: tcl8.6 8.6.18.

Actual C interp alias API 8.6.18 records four exact rows: child_alias ::A::a 0 ::A::a 0 VALUE; child_alias A::relative 0 A::relative 0 VALUE; child_alias plain 0 plain 0 VALUE; child_alias ::::extra 0 ::::extra 0 VALUE. Creation results remain the original alias spellings while each child call returns VALUE with code zero. This is the measured provider-specific API branch; no return-value or private storage/key/frame equivalence is inferred. Whole process exits zero with empty stderr.

### tcl9.0

Status: `observed`. Channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim. Version: tcl9.0 9.0.4.

Actual C interp alias API 9.0.4 records four exact rows: child_alias ::A::a 0 ::A::a 0 VALUE; child_alias A::relative 0 A::relative 0 VALUE; child_alias plain 0 plain 0 VALUE; child_alias ::::extra 0 ::::extra 0 VALUE. Creation results remain the original alias spellings while each child call returns VALUE with code zero. This is the measured provider-specific API branch; no return-value or private storage/key/frame equivalence is inferred. Whole process exits zero with empty stderr.

### tcl9.1

Status: `observed`. Channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim. Version: tcl9.1 9.1.0.

Actual C interp alias API 9.1.0 records four exact rows: child_alias ::A::a 0 ::A::a 0 VALUE; child_alias A::relative 0 A::relative 0 VALUE; child_alias plain 0 plain 0 VALUE; child_alias ::::extra 0 ::::extra 0 VALUE. Creation results remain the original alias spellings while each child call returns VALUE with code zero. This is the measured provider-specific API branch; no return-value or private storage/key/frame equivalence is inferred. Whole process exits zero with empty stderr.

### jim

Status: `observed`. Channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim. Version: jim 0.84-9-g5bac7c9.

Actual Jim child-handle alias API 0.84-9-g5bac7c9 records four exact rows: child_alias ::A::a 0 {} 0 VALUE; child_alias A::relative 0 {} 0 VALUE; child_alias plain 0 {} 0 VALUE; child_alias ::::extra 0 {} 0 VALUE. Creation results remain empty while each child call returns VALUE with code zero. This is the measured provider-specific API branch; no return-value or private storage/key/frame equivalence is inferred. Whole process exits zero with empty stderr.

### bigip

Status: `not-tested`.

No appliance execution answers this exact API/input question; stock C/Jim public outcomes supply no hosted result.

## Consumer bindings

- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::child_alias_publication_matches_original_supported_provider_apis` (linked): Independent Runtime alias conformance compares the complete original measured public rows after provider-version metadata through the selected supported API; no extra native process, private key/cell or frame identity follows.
- [rust/tcl-vm/tests/native_compilation_conformance.rs](../../../../rust/tcl-vm/tests/native_compilation_conformance.rs), `native_compilation_conformance::child_alias_publication_matches_original_supported_provider_apis` (linked): Full original source and retained provider stdout bind VM public conformance after version metadata; actual assertions require separate validation receipts and do not establish private interpreter storage or API equivalence.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::jim_child_alias_result_is_born_in_the_actual_parent_allocator` (linked): The actual Jim child alias API returns an empty result owned by the genuine parent allocator and object context; the distinct actual child allocator/context refuses that result. Parent and child retain no Host refusal. This software ownership control is separate from the captured public provider rows and supplies no private native allocation observation.

## Evidence

- [rust/tcl-registry/tests/data/native_child_alias_publication199](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199)
- [rust/tcl-registry/tests/data/native_child_alias_publication199/8.4.20](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.4.20)
- [rust/tcl-registry/tests/data/native_child_alias_publication199/8.5.19](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.5.19)
- [rust/tcl-registry/tests/data/native_child_alias_publication199/8.6.18](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.6.18)
- [rust/tcl-registry/tests/data/native_child_alias_publication199/9.0.4](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/9.0.4)
- [rust/tcl-registry/tests/data/native_child_alias_publication199/9.1.0](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/9.1.0)
- [rust/tcl-registry/tests/data/native_child_alias_publication199/jim](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/jim)
