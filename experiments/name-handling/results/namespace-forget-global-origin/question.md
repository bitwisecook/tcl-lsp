# Namespace forget global origin: #2300

The fetched issue's exact five-line witness is retained in `issue-witness.tcl`.
Current Runtime `cmd_namespace.rs::ns_forget` still appends `::` plus the tail
to `qualified_name(src_ns)`; namespace storage explicitly reports global as
`::`. Therefore this path constructs `::::g` by source inspection. A current
Runtime execution has not been measured here. `namespace_pattern_input` selects
the CString extent but does not repair the later consumer join.

VM `interp.rs::forget_imports` avoids that join: it resolves a source namespace
and compares the ultimate imported origin's namespace token and native tail.
Core `namespace.rs` exposes import-pattern parsing, not a shared forget worker.
The existing syntax `namespace_pattern_parts` selects source components/tail
under the original `NamespaceForgetPattern` purpose and actual current context;
its Jim forget purpose is explicitly unavailable. No Jim API substitution follows.

Correct bounded delegation: use that parts owner with Runtime storage's actual
`native_context_path(dest)`; resolve C components directly to a live namespace
token, without constructing/reparsing a display name. For qualified patterns,
compare the ultimate actual imported generation's namespace token and original
tail under `ForgetOriginFilter`; unqualified patterns use imported own names
under `ForgetOwnSearch`. Preserve actual command retirement and currency checks.

Runtime `imported_in` currently returns source display bytes (or ensemble names).
However the actual imported command also retains its immediate source generation,
and `native_command_slot_at_node` already checks generation membership before
returning the actual holder/tail. A storage helper must follow that authentic
chain and expose the typed origin/namespace lookup, not reparse its display.
Unavailable/deleted/foreign origins must refuse; current node access does not
settle #1778's replacement/lifetime gap. No drop-in text-join patch is proposed.

The native question runs unchanged original setup and operation on each pinned
C5/Jim shell. Fixed adverse controls ask unknown-namespace refusal, foreign-import
retention, local-command preservation and source-command retention. Package/API
refusals or unreached operations remain answers to this original question.
Results establish only those native controls; no current Rust consumer run,
source-to-build equivalence, broader lifecycle or native F5 claim is made.
