# Public stage and word-effect baseline controls

These bounded read-only public-API probes link the exact first-attempt product
library (`9c61225962f50fc60cafb24c8fd6ef10b92c7033005d290fcff5570ac8a2a91a`),
not the pending typed-stage repair. Their receipts retain the first attempt's
base/patch attribution, exact commands and all 138 emitted rlib hashes. Every
library stays byte-identical through both links and executions. The source and
output files are exact retained bytes; implementation observations are separate
from the native Tcl execution records.

The two nested substitution controls already emit no W210: an unknown operation
may initialise the variable, and a retained by-name writer actually writes it.
They are preservation controls, not new coverage being demanded of this fix.

The loop-only source contributes `x` to the opaque-callee inventory when the
lookup callback declines all targets, but contributes no `x` when the callback
accepts all targets. The real `operation x` case contributes `x` under both
callbacks: its genuine producer emits an UnseenCall marker. This establishes
both the argument-stage failure and the actual unseen-call control without
fabricating source tokens, registry targets or execution receipts. The pending
IR-stage repair must remove the loop-only contribution while retaining the
real opaque/unseen call and both nested word effects.
