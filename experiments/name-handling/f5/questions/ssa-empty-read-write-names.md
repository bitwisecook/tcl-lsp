# F5 literal and empty variable read/write names

Status: unanswered. C/Jim observations are not F5/TMM observations. Return actual answers on the same `work` branch under experiments/name-handling/f5/answers/ssa-empty-read-write-names/. Do not replace an unavailable iRule operation with tmsh or a Linux tclsh result.

Run the unchanged [finite source](../../results/ssa-empty-read-write-names/probe.tcl) in an isolated supported TMM/iRule context through the actual platform harness. Its procedure helper uses uplevel only to keep each captured operation in the supplied scope. If that helper, command or declaration is unsupported, retain the complete setup/refusal output and the unreached cases. A separately adapted iRule must retain its full actual source and context; it is a different input.

Questions:

1. Can set/incr access the empty variable name, and does the captured result equal5?
2. Does foreach with a literal empty name bind the value read by ${}?
3. Does dict with preserve the literal dollar name {$name}, and separately the empty name?
4. Does foreach preserve the literal dollar name {$name}, distinct from name?
5. Does any answer differ between supported event/procedure contexts or interpreted versus compiled iRule entry? Measure only contexts actually available; do not infer an absent one.

Preserve appliance product/version/build, actual interpreter/context/channel/event, harness revision, full source bytes and SHA256, exact invocation, compile/setup status, per-case catch status/result, complete stdout/logs/stderr and cleanup outcome. Keep unsupported, unavailable and unreached stages distinct. Record allocation/trace/generation facts only when the harness actually exposes them. No Native compilation/frame/cell conclusion follows from a name result alone.
