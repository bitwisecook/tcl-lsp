Original Jim Try leading-switch selection
========================================

Question: how does the actual Try leading scan select completion names, numeric
mask bits and original String getters before body/handler execution?

Capture01 rechecked upstream at 17:06 UTC on 2026-10-15 against clean local
5bac7c99ad65864c87da513e22e2f01703fa4e03. The exact pre-hashed C driver compiled
and executed against the original headers/libjim.a, both exit0 without stderr.
All 247 selected original source/configuration/library/executable/harness hashes
were unchanged. The original namespace suite passed 79/79 with its same-version
testing.tcl/tcltest.tcl. Manifests retain compiler/linker inputs, executable and
linked-library hashes, actual invocations, timestamps, channels and completion.

The 113 raw rows contain 92 execution cells: 23 original selectors crossed with
actual body codes 0,1,63,64. The inputs include named and signed decimal switches,
code64/code128, the highest signed-wide value, overflow failures, embedded-NUL
suffixes, actual custom selector/body updaters and an unselected handler updater.
cases.json is extracted directly from the captured C input declarations; no
consumer implementation produces a selector or expected answer.

This capture reports actual wide/pointer widths64/64. In this original build,
-no64 and -no128 suppress the code0 handler just as -no0 does. -no63 and the
highest accepted positive signed-wide selector suppress code63; code64 itself
still reaches its handler. Signed numeric +1 and leading-space1 are accepted;
negative, base-prefixed and overflowing selectors fail before the body. Raw NUL
suffixes retain CString matching for -no0 and --.

The actual custom body header is read during the leading scan after -no0 at
native count2. After -- it is first read at the later body evaluation stage at
count3. A rejected custom -bad selector stops before the body and leaves the
custom handler selector stringless. Traces add no observation references.

These observations are bounded to the captured original executable, compiler,
configuration and platform. The source expression shifting a signed mask by an
unbounded selector does not establish portable C semantics; other builds need
their own observation before reusing the measured low-six-bit selection. No
Runtime/VM test ran. Unexecuted owner controls consume all 92 cells while keeping
the source mask separate from the bounded Native recipe. Runtime controls check
bounded completion/body/handler/getter stages, valid results and message stores;
they exclude full invalid-input diagnostics and native reference/type parity.
The out-of-width original positives remain explicitly unimplemented Native
cases: architecture controls require refusal before body execution, including a
separate Catch control preventing source wrapping from granting Native work.
Those refusals are not original conformance successes. The rebuildable probe is
excluded; its captured hash is checked when the binary exists.
