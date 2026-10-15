# Original expression equality and source literals

The retained fixed [probe](probe.tcl) ran against original Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and pinned Jim before the related expression-consumer assertions. All six exited zero. The [original manifest](manifest.json) retains commands, actual source libraries, provider/init hashes before and after, and exact stdout/stderr hashes. Empty stderr files are retained.

All six show that `01`, ` 1 ` and `1e0` compare numerically equal to `1` but differ in textual equality and exact Switch matching. Numeric comparison therefore cannot become exact string matching for an unknown scalar. A quoted literal containing `||` and a braced literal containing `$value` remain data in their expression operands. A whole braced string used as an If condition errors as a non-boolean literal; it is not a container to remove in order to discover a different equality expression.

These are specific original expression/value observations. They supply no F5 runtime, reached command binding, current frame, Rust acceptance, or whole-refactoring/edit equivalence. See the [name-handling contract](../../../docs/design/contracts/name-handling.md), [pattern catalogue](../patterns/README.md) and [result index](../results/README.md).

Original outputs are immutable. `SHA256SUMS` also covers portable replay support. Run `python replay.py --out /tmp/new-expression-equality-capture` for a separate reproduction using the retained provider binaries/init files. The runner checks provider hashes, refuses an existing output directory and never overwrites original outputs; any replay result is separate evidence.
