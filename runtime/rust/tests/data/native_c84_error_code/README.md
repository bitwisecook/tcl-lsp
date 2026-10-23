# Tcl 8.4 error-code publication

Tcl 8.4 records its error-code-set flag independently of completion metadata. An absent code publishes fresh `NONE` after the initial errorInfo setter stores the original result. A supplied code uses the original global header and reports `errorCode` to its write callback; the default setter reports `::errorCode`. Successful variable trace chains preserve the three error flags while global values and object headers retain their separate owners.

A nonempty `error` info argument publishes and appends errorInfo before the supplied code setter and final message result. The `nonempty` controls cover absent, explicit `NONE` and structured codes at that frontier.

The controls compare actual Tcl 8.4 callback order, names, value bytes and result bytes. Raw five-release header and reference-count observations capture fields before their observer getters; the callback controls do not assert modern releases' ownership windows.
