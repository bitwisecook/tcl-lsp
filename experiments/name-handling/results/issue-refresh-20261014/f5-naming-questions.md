# Original F5 naming questions

Run only in a context that actually supports the requested commands. Return
answers to this branch with the exact original input, full stdout/stderr,
exit/completion status, reached command, actual product/build/Tcl version and
execution context. Keep unavailable commands, definition/setup failures and
unreached lookups explicit. Preserve an original operand's distinction from
a substituted value or reporting label. Sanitize appliance addresses and
credentials with an explicit transformation record; do not publish secrets.

These questions contain no F5 acceptance or expected appliance answers.
Portable C/Jim results in the companion captures are independent references.
Do not provide command shims, fake successful namespace/child creation or
copied C Native receipts when a native F5 operation is unavailable.

| Question | Exact retained input | Required observation |
| --- | --- | --- |
| F5-NAME-2417 | `2417-root-body-collision.tcl` | Is a procedure named `top` distinct from the enclosing/root execution body in each supported stock/hosted context? Record actual definition and read stages. |
| F5-NAME-2296 | `2296-computed-array-key-read.tcl` | Does the original `$a($k)` read the selected key variable and element in the actual current frame? |
| F5-NAME-2268 | `2268-unknown-and-distinct-array-key.tcl` | Record the actual element existence answers for caller-supplied x/y keys, including context's array support. |
| F5-NAME-2209 | `2209-catch-array-result.tcl`, separately `2209-catch-array-options.tcl` | Does each result/options operand name a scalar/array/qualified cell through native catch? Is a third argument actually supported in this context? |
| F5-NAME-2235 | `2235-braced-incr-amount.tcl` | Capture the real error wording/code from a braced literal `$n` amount; do not substitute it manually. |
| F5-NAME-2255 | `2255-array-index-outer-mask.tcl` | Which substitutions execute inside the array index under each original outer subst mask? |
| F5-NAME-2218 | `2218-dynamic-proc-head-rebinding.tcl` | Where rename/proc are supported, record actual command-generation changes and the original shadowed expression's argument/result. Otherwise record refusal and last reached stage. |
| F5-NAME-2289 | `2289-child-created-in-procedure.tcl` | Is original child-interpreter creation/evaluation available in the actual context? If so, record creation inside New, later actual path evaluation, deletion and post-deletion refusal. |
| F5-NAME-2332 | `2332-external-global-reads.tcl` | Compare original substitution, command-form and declared-global reads of the same changed cell; specify whether the cell is process, connection, event or Tcl interpreter scoped. |
| F5-NAME-2339 | `2339-assignment-name-evaluation.tcl` | Does the original command substitution in the destination array index run once before the store and change i? |
| F5-NAME-2399 | `2399-global-read-store-before-unset.tcl` | Capture both observed global reads through the called procedure before the eventual unset. |
| F5-NAME-2419 | `2419-tcllib-variable-writers.tcl` | Record native package availability and actual package version before any attempted mutator. If unavailable, do not emulate struct::set/list. |
| F5-NAME-2143 | `2143-materialised-factory.tcl` | Where proc/subst are available, record actual factory-created command existence, invocation and value. |
| F5-NAME-2324 | `2324-original-tk-computed-head.tcl` | Record whether native Tk/ttk is available. If unavailable, this cannot answer the Tk receiver-classification question. Any unrelated host command test must carry its own source/descriptor/receiver provenance. |

Each filename refers to the exact input retained in the C8.6 directory of
the companion `oracles-02` capsule. Use separate stock Tcl, control-plane,
RULE_INIT and traffic-event attempts only where those contexts exist and
are authorised. Record procedure/namespace/callback creation failures rather
than claiming the subsequent naming lookup was reached.
