# Uplevel variable source scope

The same written variable name occurs in a root, namespace, procedure or method table. An absolute-zero uplevel reads the root table; relative zero reads the calling frame; a nonzero relative level reads the actual caller, whose identity may be unavailable to a source-only consumer. A nested declared procedure introduces its own local source boundary even inside an absolute-zero body.

The exact runnable probes and actual C8.6.18/C9.0.4 output are retained here with source-built executable hashes, matching library paths and byte-source hashes. Both providers returned `namespace-proc {ROOT PROCEDURE}`, `direct-namespace ROOT`, `method {ROOT METHOD}`, `actual-caller {CALLER INNER}`; the nested-procedure probe returned `INNER`. Exit codes are0 with no stderr. No tcltest substitute is involved.

`AnalysisResult::logical_variable_scope_advice` requires the independently retained Logical domain and the exact whole consumer image/channel/configuration/cursor. Its source table eligibility cannot provide an actual Native frame, completed alias or writable original symbol. Native/hosted consumers retain their own original frame/symbol owners. The scoped public Compiler/Core controls are separate from these interpreter value observations; an unexecuted or failed Rust control establishes no conformance.
