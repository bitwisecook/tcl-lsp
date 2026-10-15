# Original Jim canonical namespace holder

Question: does no-argument `namespace canonical` return its actual current
namespace object without obtaining that object's String? The competing result
is a copied or rendered namespace value. Absolute-name, relative-name and
explicit-prefix operations distinguish which original getter is reached.

The [SDK input](probe.c) enters each real namespace frame with the same original
integer namespace and command-head objects in its COLD/REUSE pair. Only the
namespace String is invalidated before REUSE. Each phase constructs fresh
original explicit-prefix and name objects. Header and pointer observations
precede the result String getter. The final getter can materialise the retained
caller after its observation.

The [80 observed rows](probe.stdout) show that CURRENT_REUSE retains the exact
caller object and its String remains absent. ABSOLUTE_REUSE and EXPLICIT_REUSE
also leave the caller String absent. RELATIVE_REUSE materialises that actual
caller. Cold command lookup independently materialises the caller in all four
cases; the primitive's no-getter property does not describe that lookup stage.
Explicit prefix `84` produces `84::child`, separately from the caller `42`.

The latest upstream HEAD check at 2026-10-15 10:12:21 UTC selected
`5bac7c99ad65864c87da513e22e2f01703fa4e03`. The capture at 10:14:05 UTC compiled
and exited successfully, with empty compiler/probe stderr and unchanged selected
provider hashes. The [local verification](provider-verification.json) records
the same clean source revision, actual configure invocation, compiler flags and
enabled Jim configuration. [manifest.json](manifest.json) records source,
configuration, library, interpreter, probe and executable hashes, invocation,
process exit and raw-channel hashes. The executable is deliberately excluded.

The original `tests/namespace.test` suite ran through the same interpreter's
unchanged `tests/testing.tcl` and `tcltest.tcl`: 79 passed, no skipped or failed
tests. [namespace-suite.stdout](namespace-suite.stdout) retains the original
summary; the manifest binds all three original harness/source files. This SDK
probe does not replace or rewrite that harness.

Reproduce with the provider built by `scripts/dev/ensure-jim-oracle.sh`, recheck
upstream HEAD, then run `python3 capture.py --provider /path/to/jimtcl` in a fresh
capture directory. Preserve earlier observations when collecting another run.

These are exact original SDK and namespace-suite results. Selected source and
configuration hashes are not a complete system toolchain closure. They do not
establish Runtime/VM execution, C Tcl/F5 results, universal cache behaviour,
namespace existence, lifetime or callback traversal.
