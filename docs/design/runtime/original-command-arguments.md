# Original command arguments

`OriginalArguments` captures the entered Runtime operation and allocation leases for the exact original `Tcl_Obj` headers. The leases preserve allocation safety without adding native references, changing sharing, producing strings, or granting a command-specific getter purpose.

A physical consumer calls `check_for(actual)` before and after a reached effect. The actual receiver must be the same interpreter allocation that captured the arguments; an interpreter clone is accepted, while another interpreter with equal context values is refused. `check_original_for(actual, ordinal, header)` additionally requires the exact captured header for that argument. Equal spelling and an independently retained allocation do not satisfy this relationship.

`original(ordinal)` checks currency and header liveness. `bytes(ordinal)` reaches the existing selected String getter for that one original and checks the operation and header after the getter. Neither method eagerly reads later arguments. Command-specific numeric, List, Unicode, name and result workers select their own purpose and use the receiver/header checks at their actual effect boundaries.

A mismatch is retained on the actual receiver as a structured `NativeExecutionError`. Its complete first cause is returned as `CmdError`; consumers must preserve this object rather than reconstructing a refusal from a message or reducing it to `ValueError`. Mandatory cleanup still occurs after refusal, followed by the original operation check. Cleanup grants no new native argument, result or working-reference role.

The software controls in `interp::native_arguments::original_receiver_tests` cover same-receiver clones, distinct receivers with equal contexts, distinct headers with equal spelling, unchanged native argument reference counts, retained result identity, and an existing first Host cause. These controls test the implementation contract. They do not claim native C/Jim callback or foreign-interpreter behaviour. The native evidence boundaries are documented in the original Host publication proof.

## Compiled procedure definitions

[`NativeOp::DefineProc`](../../../rust/tcl-compiler/src/native_lowering/ir.rs)
retains already evaluated argv objects separately from its internal compiled-body
key. [`tcl_codegen_proc_define_original_argv`](../../../runtime/rust/src/codegen_abi.rs)
borrows that vector, retaining the caller's references through activation,
original procedure handling and checked completion capture. It accepts a
separately proven optional compiled entry; that entry supplies no name,
namespace, original argument or native execution authority.

The direct ABI caller must independently prove that the selected definer is the
live stock procedure handler under the invocation's current lookup context.
Preserving `argv[0]` supplies no such proof: the direct entry bypasses command
head dispatch. A replaced, shadowed or otherwise unproved definer uses generic
dispatch with the same evaluated argv. Compiler declaration/span and operand
admission remain separate from both live definer selection and compiled-entry
authority.

The ABI and the stock command share
[`proc_cmd_with_native_entry`](../../../runtime/rust/src/cmd_proc.rs).
`OriginalArguments` guards the actual operands while the shared handler selects
name publication, current namespace, formal/body order and result semantics.
A display FQN or internal entry key cannot substitute for the original name
object. A substituted name or body is evaluated at its original call site once
through generic dispatch, rather than reconstructed or evaluated in the new
procedure's frame.

The ABI uses the existing activation, borrowed-argv and completion owners. A
Host refusal bypasses output adoption; caller-owned objects and mandatory
cleanup retain their existing lifetime rules. Replacement by a source-only
procedure retires the prior compiled entry while independently retained caller
objects remain valid. The shared descriptor table owns the import signature and
strict artefact/Runtime fingerprint comparison.

## Native C86 source text and split

For independently selected Native C86, [`parse_script_with_original_text`](../../../runtime/rust/src/parse.rs) retains top-level `Esc` token spans and word-part indices in the central `ExecutablePartArena`, alongside the unchanged lexical `WordPart::Text` value. [`Interp::original_c86_word_text`](../../../runtime/rust/src/interp.rs) materialises the original text when that part is reached during substitution. It requires Native source admission, the entered operation's currency, `SourceChannel::NativeValue`, the actual lexer configuration and complete contiguous Text coverage of the retained span. A Logical-capable `source_string_protocol` alone grants no Native admission. Changed or unavailable authority, channel, grammar or geometry refuses rather than reconstructing original bytes from lexical text.

[`tcl_syntax::backslash::native_arena_text`](../../../rust/tcl-syntax/src/backslash.rs) owns source text decoding; [`NativeTclUtf`](../../../rust/tcl-syntax/src/native_tcl_utf.rs) owns selected character-unit decoding. [`plan_c86_split`](../../../rust/tcl-cmd-core/src/list.rs) is a pure byte planner for the empty-input, empty-delimiter, single-byte and general C86 branches. It returns original byte ranges and character reuse keys, without granting physical object, cache or execution authority. Escaped NUL and literal resident zero remain distinct inputs; split does not rewrite one into the other.

[`ValueOps::native_split`](../../../rust/tcl-syntax/src/value.rs) separates that plan from physical storage. The [Runtime adapter](../../../runtime/rust/src/value_ops.rs) admits Native C86 only, reads the delimiter before the input through the actual String getter, and checks original allocation liveness and operation currency after reached getters and native List appends. It starts with a fresh empty result and appends genuine String children; repeated empty-delimiter characters share actual child pointers whose references belong to the List owner. A raw single-byte zero delimiter refuses because the unmeasured terminator-pointer extent is outside the admitted owned-storage contract. The [command entry](../../../runtime/rust/src/cmd_list.rs) delegates to shared split rather than decoding bytes itself.

Other existing Native and Logical routes retain their checked text path. The VM has no C86 physical split override: the trait's default refuses an independently selected Native C86 worker instead of falling back to a reconstructed Unicode value. Original source materialisation here covers top-level `Esc` parts; nested array-index and `subst` provenance is not proved by this integration.

The original producer question, native split reference and implementation validation retain exact inputs, channels, provider observations and source scopes. Their evidence covers the selected Runtime C86 integration; nested-source, VM, F5 TMM/MCPD and full-suite conformance require independent evidence.

## Original Jim procedure namespaces

Runtime uses `Namespaces::retain_unmaterialized_jim_namespace` and the same
original holder through its own frame owner. `Interp::materialize_jim_namespace`
checks the actual interpreter operation, frame birth and holder before reaching
the getter; native pointer-equal cache checks obtain neither namespace getter,
while distinct holders retain cached-before-current comparison order. Jim apply
retains its real `apply lambdaExpr` dummy argv head and supplied argument
objects. `CallMeta::original_frame_arguments` carries that native invocation
tape without reading lambda or argument Strings for metadata. Query, error and
body consumers independently select their reached getters.

VM procedure declarations and call frames retain `NamespaceExecutionContext`.
A resolved namespace owns its actual token. An original Jim lambda namespace
retains its genuine original Value, interpreter, naming policy and world owner.
Preparing and parsing its original Script, checking arity and binding local
arguments do not obtain the namespace String getter or create a namespace path.
Comment-only and invalid bodies can therefore leave an integer namespace
object without resident String bytes, as the original Jim does.

A reached command lookup owns Jim's namespace getter. Cold lookup obtains the
namespace before the command operand, including qualified command heads.
A valid command cache with the same namespace object follows Jim's pointer
comparison without obtaining either namespace String getter. Materialisation
records the coordinate on the same original execution context; it never replaces
that context's identity. Stack transport, frame selection and cleanup retain
its original activation birth independently of any eventual coordinate.
Compiled native entry capture requires an already materialised, authenticated
namespace token and declines while that coordinate is unavailable.

The VM test
[`jim_lambda_unreached_command_lookup_keeps_original_namespace_unmaterialized`](../../../rust/tcl-vm/src/command/native_apply_original_tests.rs)
checks Script preparation and actual invocation separately.
