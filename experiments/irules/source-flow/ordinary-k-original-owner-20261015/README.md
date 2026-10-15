# Original Hosted source owners in ordinary K

Question: why do the unchanged quoted-template getter and four collection
controls withdraw their inventories when the original iRules source is retained?

This diagnostic client uses public APIs from the already built ordinary K
libraries. It does not change the implementation, instrument a test, invoke an
appliance, or provide an oracle for Tcl behavior. The five exact inputs are in
[probe.rs.txt](probe.rs.txt). Each input runs only its corresponding getter or
collection query first; subsequent output inspects original Module, event,
function, CFG, token and source-schema owners. The unexecuted preliminary draft
that queried both warning families is retained separately as
[probe-before-first-query-narrowing.rs.txt](probe-before-first-query-narrowing.rs.txt).

The root coordinator compiled and ran the diagnostic once against the exact
normal K libraries. Compilation and execution both exited 0. Execution took
1.569 seconds, wrote 59,439 stdout bytes and no stderr. The complete retained
source/artifact guards reported no drift. The executable hash before and after
execution is recorded in [run-result.json](run-result.json); the executable and
libraries are deliberately omitted. [dependencies.json](dependencies.json)
retains all five original library paths and hashes. [compile-result.json](compile-result.json)
and [compile-argv.json](compile-argv.json) retain the actual compiler invocation.
The original compile record did not retain a separate compiler executable hash.

[run.stdout](run.stdout) shows that the complete input, Module source/config,
retained bindings, function metadata and original event ownership were present.
Original IR/CFG command vectors and invocation sites were present. The original
Registry-assistance API returned an unknown, nonabsent envelope with no
candidates at the inspected commands. The independently retained Hosted source
inventory selected the actual `set`, `HTTP::uri`, `HTTP::collect` and
`HTTP::release` schemas, including `HTTP::uri`'s available `-normalized` option.
These are different purposes: a conditional source schema is not an entered
handler or evaluated Native argv.

The CFG output also retains genuine `GlobalFrameScript` and `CallerFrameOpaque`
phases without original invocation carriers. This capture does not establish
that those phases have an empty lexical inventory. Missing phase representation,
child completion/reachability, unknown effects and named-mutation barriers remain
independent obligations. The empty warning vectors are diagnostic observations,
not passing acceptance controls or proof that the source contains no collectors.

To replay after obtaining the same normal libraries, adapt only the absolute
source, dependency-search and output paths in the retained compiler argv, compile
`probe.rs.txt` as Rust with the recorded edition, and record a fresh capture.
Do not replace the original capture or treat a different build as ordinary K.
