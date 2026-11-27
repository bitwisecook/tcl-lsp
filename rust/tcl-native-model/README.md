# tcl-native-model

The interpreter model that the Tcl engines (`tcl-vm`, `runtime/rust`) and
the registry's compile recipes share: `InvocationDialect`, the execution
policies an invocation is selected under, and the `native_*` protocols,
tables and recipes that model C Tcl and Jim internals. It holds no command
spec. `tcl-registry` depends on it; it never depends on `tcl-registry`.
