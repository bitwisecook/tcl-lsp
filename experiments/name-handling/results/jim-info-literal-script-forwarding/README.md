Original Jim Info literal-script forwarding
===========================================

Question: do info hostname/nameofexecutable invoke actual mutable commands via
their original literal Jim_Eval inputs, retaining namespace and raw completion?

Captures01/02 rechecked upstream on 2026-10-15 at 16:34 and 16:38 UTC. Both
matched clean local 5bac7c99ad65864c87da513e22e2f01703fa4e03. The retained
pre-hashed drivers compiled their exact C API inputs against the original
headers/libjim.a. All four compile/execution stages exited 0 without stderr;
all 247 provider input hashes were unchanged. Manifests retain source, compiler,
executable, linked-library and configuration hashes, invocation, timing and
channels. Both separately executed original namespace.test suites passed 79/79
with the same revision's original testing.tcl and tcltest.tcl.

The main probes register original core commands without importing stdlib.tcl.
Namespace cases explicitly call original Jim_namespaceInit, independently of
scripted helpers. Only the conformance suite uses its normal original provider
bootstrap. Missing helpers therefore remain genuine core-only missing commands.

Each 48-row capture measures both members through core-only absence, a native
helper, a namespaced helper, Error, replacement, rename/removal, Break, a script
replacement and a renamed public info command. The observer records actual argc,
argv0, current namespace, evaluation level/parent, Script primary/count and
source filename/line. Each helper receives one argument, the fixed original
literal command name. Error and Break propagate unchanged. Script objects are
observed as script/count1 with empty filename and line1.

Capture01 replaced a helper with the same native function, and its result tag
came from the probe mode; that alone cannot discriminate stale callee selection.
Capture02 supplies distinct original command private data (321 versus 654), so
the observed result discriminates the selected actual replacement. Capture01
remains unchanged; primary stdout is byte-identical. Rust fixtures use capture02.

Rust controls compare actual completion/result, reached helper argv and root
versus namespace context across the eighteen cases. The captured Script count,
full evaluation-frame geometry and source metadata are retained evidence, not
yet Rust assertions or complete lifecycle acceptance. Actual stdlib provider
installation, signal/unknown hooks, native argument counts and every cache stage
remain independent obligations. Rebuildable probes are excluded; captured hashes
are checked when present. Rust controls remain unexecuted.
