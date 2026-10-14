# Hook-context reference observations

The original C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0
interpreters read `$ctxé` as `ctx` followed by literal `é`; the supplied
latest Jim interpreter reads the distinct variable `ctxé`. Braced `${ctxé}`
selects that complete variable on all six providers. `$ctxfoo` stays a
distinct variable, and `${ctx}` selects `ctx`. `probe.tcl`, raw stdout/stderr
and the executable/input hashes reproduce these observations. No Tcltest
or F5 appliance was used.

MCP hook context reporting uses the selected target profile's full lexer
configuration and shared `scan_var_ref`. This is bounded textual source
advice; hook execution, cacheability declarations and Native authority
retain their independent owners. `mcp-exact-plan.json` lists the consumer
controls that must be replayed after integration.
