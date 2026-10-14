# Hosted F5 global frame questions

Run the retained `oracles-isolated/global-source-frame.tcl` and `global-source-trace.tcl` through the BIG-IP Tcl test entry available in the separate chat, adapting only the enclosing invocation/event harness required by that host. Preserve each original tested body, receiver name and frame-selector word. Record host version, actual event/procedure harness, original commands, stdout/stderr and per-case catch status/value/local `x`.

1. Does a selected global body write preserve the suspended caller procedure's local `x`?
2. Do nested absolute `#1` and relative `1` frame selections from that global body fail, and with what error?
3. Does a newly invoked callback's `#1` name that callback's frame rather than the suspended caller?
4. Is a namespace alias referring to the procedure variable refused?
5. Is `trace add variable` available in this host and event? If permitted, does the global variable's callback leave suspended caller-local `x` unchanged?

Return answer documents in this branch with exact harness/source and actual observations. Unavailable commands or inaccessible test entry points remain explicit unanswered cases. C/Jim observations do not establish hosted F5 results or arbitrary external observer closure.
