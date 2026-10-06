# Original Jim command and variable lookup caches

The pinned native C harness retains original counted name objects and reads their header-defined caches without inspecting guest error globals. It records success, primary class, actual name-object references, frame identity, procedure epoch, namespace holder references, and cached command liveness. Stale command pointers are never dereferenced after a procedure-epoch mismatch.

Controls include replacement without epoch change, rename, duplicate headers, failed lookup preserving old primary, unrelated variable unset, frame changes, absolute and counted NUL/opaque names. The active-deletion control distinguishes the same cached original head from a fresh equal-byte name: the original remains a hit while its actual command is executing, and becomes a miss when that invocation retires. No cache owns the command worker.

Rows distinguish observation-only before states from reached lookup outcomes. This fixture attests native behavior; type-name text in a fixture is not a production object-identity or cache issuer.
