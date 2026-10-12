# Native naming controls

These controls retain 40 fixed Tcl sources in each of C Tcl 8.4.20, 8.5.19,
8.6.18, 9.0.4 and 9.1.0, and JimTcl 0.84 (`0.84-9-g5bac7c9`, source commit
`5bac7c99ad65864c87da513e22e2f01703fa4e03`). Each source has its original
stdout, stderr, process status, duration and SHA-256 in `manifest.json`.
`version-inventory.json` records the interpreter identities and binary hashes.
File references are relative to this directory; recorded invocation paths
retain the original measurement coordinates.

The controls exercise command and procedure publication, lookup and rename;
namespace addressing, search paths, imports and exports; package keys and
loaders; scalar, array and alias identity; global, upvar and uplevel; variable,
command and execution traces; and TclOO object and method naming. Sources
include dynamically produced NUL bytes and partial colon separators.

A successful process capture can contain a guest error. Each source records
the guest completion code and result bytes independently of the process exit
status. Unsupported dialect operations retain their actual errors. A C result
does not supply an expectation for Jim or BIG-IP.

This evidence establishes native completion codes and result bytes. It does
not establish backend equivalence, physical object headers, reference counts,
return options, compiler selection or BIG-IP event behaviour. Implementation
comparisons use the naming owner and consumer tests, including the separate
`jim_name_resolution_conformance` target and the C command, variable and namespace
conformance targets. Each comparison retains its own supported engine and
operation domain.
