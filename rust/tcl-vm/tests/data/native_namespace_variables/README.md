# Namespace-variable query controls

Fixed name objects cross resident strings and pure byte arrays, with absent,
full-name-only, short-name-only, both, declared-undefined and trace-undefined
storage. Each TSV row records both original-name and short-name query results.

C Tcl queries use `Tcl_FindNamespaceVar` and report the selected raw cell;
aliases are not followed. Jim's script helper returns a canonical spelling
without requiring storage. The manifest records exact native inputs and
source, header, library, executable and log hashes. Completion snapshots are
captured before subsequent read observers.
