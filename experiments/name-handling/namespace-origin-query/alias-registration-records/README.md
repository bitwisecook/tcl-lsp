# C alias registrations

C aliases retain registration coordinates independently of their command's current namespace or hidden placement. Queries, target selection, deletion and inventory select the original registration token and then use the backend-owned generation. A reported command name cannot replace that generation.

C8.4 retires an existing alias registered at the same original C-string coordinate. Later C releases construct distinct registration tokens by repeatedly prepending `::` to the complete original token object. Query misses produce the original empty successful result. Inventory retains each actual registration object as a List member, without reaching its String getter.

The shared registration table records every actual entry insertion and removal in `NativeEntryLedger`; `NativeHashOrder` supplies physical iteration and retained table capacity. C `aliasTable` uses `TCL_STRING_KEYS`: its recipe combines plain-char promotion with unsigned-int width through C8.6 and size_t width in C9. Actual backend ABI issuance belongs to `tcl-runtime-api::native_hash_abi`. Jim's command-object alias query and its independently retained namespace holder remain separate.

`input.tcl` has 26 observations for each original C interpreter: record relocation, query/target/deletion, unique-token allocation, hidden placement, unsorted bucket growth, removal, reinsertion and collisions. Raw outputs and before/after binary/library seals are retained under each provider. The fixed Unicode and ASCII names exercise shared table ordering without sorting the five order observations. `initial-input.tcl` and `membership-input.tcl` retain their separate 14-row and 21-row captures.

`abi.c` is compiled with each original interpreter's configured compiler and flags, matching headers and stub library, and loaded by that interpreter. All five measured `CHAR_MIN`, unsigned-int size and size_t size as `-128 4 8`. Compilation commands, loaded module hashes and original results are retained in each `abi` directory; ELF modules remain external artifacts. The source excerpts and complete-file hashes are indexed in `source-provenance.json`, with original Tcl attribution retained.

The Rust consumer selectors compare all 26 rows against their own provider's real output. These selectors, the shared table changes and the Runtime adapters have not yet been compiled or executed. The results above certify the original interpreters, not Rust conformance.
