# Original namespace origin queries

`input.tcl` is executed unchanged by C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4,
9.1.0 and pinned latest Jim (`5bac7c99ad65864c87da513e22e2f01703fa4e03`). Each
provider records 37 observations: 26 origin queries, nine setup results and two
namespace reports. C aliases retain their own origin names; Jim follows only
canonical aliases through the complete original `info alias` prefix object.
The prefix remains one recursive name, including its arguments and list quoting.

`helper-root-input.tcl` adds five observations per provider: two alias setups,
two origin queries and one command query. A doubled namespace contains a competing
alias target. Jim reports the first target, establishing the helper's root lookup
basis independently of caller namespace canonicalisation.

Jim's exact helper source is retained in
[the command-query source receipt](../namespace-command-query/jim-source/).
`nshelper.tcl` lines 17–30 select canonical alias, canonical command and original
command in that order. Both `exists -alias` and `info alias` use the same canonical
object. The global helper uses the actual root namespace for these getters;
`uplevel 1` canonicalisation retains the actual caller namespace object.

`alias-cycle-input.tcl` records a separate bounded attempt. C rejects the second
alias registration as a loop. Jim accepts both registrations, then the origin
query reaches the one-second process timeout. There is no Jim guest result to use
as a cycle error oracle; an adapter unable to finish that query must report its
unsupported operation separately.

Each original input, binary hash, matching C source/library authority, exact
command, raw stdout/stderr and exit or timeout status is retained. These are ASCII
query/report and setup observations. Reported strings never reconstruct namespace
or command identities. There is no raw NUL, F5 or Native C allocation claim.

Records seal the measurement-time executable, input and raw outputs and link
the original provisioning/reference integrity records. They do not establish a
new per-run before/after library inventory. Rust adapter capture and acceptance
remain separate from these original oracle observations.
