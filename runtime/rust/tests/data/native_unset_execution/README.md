Original native unset execution
===============================

Six cases on Tcl 8.4, 8.5, 8.6, 9.0, 9.1 and Jim. TSV columns are case, original script bytes (hex), completion code, and original result bytes (hex). Each row executes in a fresh native interpreter. Version-selected trace grammar is part of the original source; Jim trace unavailability is an explicit measured error.

The observations cover undefined traced unset, ignored callback error, quiet missing namespaces, sequential compiled targets, and abort before a later array index. They do not claim object refcounts, cache identity or return-option ownership. The probe and provenance retain the original commands, SHA hashes and 60-second budget.
