# TclOO implicit command path

The controlled probe runs the original C Tcl 8.4–9.1 and pinned Jim interpreters with each C interpreter’s original init.tcl and library. It uses no tcltest. C Tcl 8.6, 9.0 and 9.1 report `::oo::Helpers` as the method command path, resolve the helper installed there, and reject the command installed directly in `::oo`. Both bare commands fail at the top level. These C 8.4/8.5 and Jim builds have no TclOO class command.

Each manifest retains the original executable/init hashes before and after, selected library and complete output. The initial probe leaves the unsupported `::oo` call uncaught; its stderr records that failure despite the shell’s zero exit status. The controlled probe catches each call separately so its output distinguishes lookup results.

The reference consumer delegates its existing lexical scope path query to `implicit_command_namespace_path_at`, whose path is Registry data. The existing positive helper-reference and negative top-level-reference tests cover that behaviour. These shell observations establish neither an LSP test result nor F5 behaviour.
