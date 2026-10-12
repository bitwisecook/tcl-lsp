# Default-frame direct-source diagnostic attempt

This retained attempt removed the older wrapper's frame switch without setting the upstream in-process configuration. Actual C8.6 then failed var-21.0 and four upvar line-number cases because actual tests/all.tcl enables extended frame debugging in single-process mode. C interpreter debug cases passed with the default frame value.

These are wrapper-state diagnostic observations, with no aggregate conformance verdict. No original init, tcltest or suite was edited; all 18 fresh processes completed without timeout, crash or OOM. Final measured evidence uses the upstream-consistent singleProcess=1/singleTestInterp=1 configuration in [20261012T193343Z](../20261012T193343Z/README.md).
