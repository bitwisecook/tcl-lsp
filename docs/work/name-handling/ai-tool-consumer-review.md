# Bounded MCP and AI consumer review

Temporary coordination note; remove before merge. Read-only review on `work`
at `d72ab9471`; no tests, generators or oracle experiments were executed.
Owners inspected: MCP `tools.rs`, Core `procedure_symbol.rs`, and AI
`gen_ai.rs`, with their immediate contracts, templates and context seam.
This does not establish coverage of every AI/tool consumer.

## Findings

1. **Literal catalogue request is changed before central lookup.**
   `rust/tcl-mcp/src/tools.rs:1094` applies `.trim()` to the API's
   `command_name`; `:1101` then calls `ResolvedContext::resolve_spec`.
   The tool schema at `:2003` calls this a command name and documents no
   whitespace coercion. Thus `" HTTP::uri "` requests metadata for
   `"HTTP::uri"`, rather than its exact value. Preserve the incoming name
   for the central query, or explicitly define a separate search query
   contract. Test the ordinary key against leading/trailing space, tab,
   embedded NUL and missing-key variants. This is catalogue metadata,
   not a runtime command-resolution claim.

2. **Procedure documentation delegates to current original declarations.**
   `rust/tcl-mcp/src/tools.rs:1505` uses
   `procedure_symbol::for_literal_name`; `:1509` distinguishes one match,
   no match and multiple declarations. `:1534` uses the shared declaration
   inventory and `:1559` transports original source spelling separately
   from display names. `rust/tcl-lsp-core/src/procedure_symbol.rs:17`
   validates the full source image/configuration; `:60` selects literal
   values through the publication recipe, with an independent hosted
   header-only branch. It does not claim a live target or resolve aliases.
   Existing unexecuted-in-this-review tests at MCP `:3583`, `:3592`, `:3602`
   cover qualification, missing namespace scope, opaque names and repeated
   declarations. A useful additional finite control is a dynamic
   `proc $name ...` followed by `rename`: documentation must describe only
   admitted source declarations rather than infer current dispatch from
   the final displayed name.

3. **Graph tools retain their analysis's actual context.**
   `rust/tcl-mcp/src/tools.rs:579` (`supplied_source_graph`) takes
   `analysis.resolved_input`, refuses an absent input and passes its own
   `context.commands()` plus that input to the Core graph builder. The
   call/symbol/dataflow tools at `:591`, `:595`, `:599` share this seam.
   No local name splitting or alias reconstruction occurs there. Next
   finite consumer comparison: the same document's literal list-built
   callback alias before and after an unknown command writer; compare MCP
   graph output with the same Core builder and retain unknown target
   alternatives rather than treating a matching report label as identity.

4. **Cursor tools adapt Core naming results rather than reconstruct names.**
   MCP definitions at `rust/tcl-mcp/src/tools.rs:1225`, references at
   `:1242` and rename at `:1261` delegate to the respective Core providers
   with the same source and analysis. Rename returns null when Core
   supplies no edits (`:1270`). This verifies transport at these call
   sites, not the correctness/completeness of their providers. Use a
   namespace-imported procedure, rename it between two calls, and add an
   unknown intervening writer as three finite comparison controls; MCP
   locations/edits should agree with Core, including refusals.

5. **Generated AI catalogue prose uses the shared context seam.**
   `rust/xtask/src/gen_ai.rs:304` obtains the iRules command store and
   `:306` calls its environment's `vendor_command_surface`; missing
   metadata yields no invented summary. The output at `:318` calls the
   inventory "modelled commands". Templates
   `ai/prompts/tcl_system.md.j2:20` and
   `ai/prompts/irules_system.md.j2:36` explicitly distinguish conditional
   candidates, missing/stale/unsupported inputs and exact oracle contexts.
   Generator drift and all generated copies remain unverified here.
   During broad comment cleanup, shorten `gen_ai.rs:298`'s account of
   retired `ProfileQueries` to the present context/store invariant; no
   runtime behavior change is implied.

Suggested controls are proposals only. Bind any conformance assertions to
the retained C Tcl/Jim/native-context F5 results for those exact questions.
No Tcltest execution in F5 TMM/MCPD is proposed.

## Exact-name repair prepared outside the repository

Scratch source: `/tmp/ai-tool-command-info-tools.rs`. It removes only the
command-name trim, shortens the adjacent obsolete API history, and adds
`tools::dialect_param_tests::command_info_preserves_exact_literal_names`
through public tool dispatch. The original ordinary request must find its
real registry row; seven exact adverse names must remain unchanged and
not found. BIG-IP version trimming remains unchanged. Source is not applied
and the test is unexecuted while shared Rust inputs are frozen.

`/tmp/ai-tool-command-info-plan.json` records the source hashes, exact
`cargo test -p tcl-mcp --bin tcl-mcp ... -- --exact --nocapture` command
and conservative 39-package local path-dependency graph. This graph
includes `tcl-vm`, excludes standalone Runtime, includes the root's dev
dependencies and all manifest target tables, and is not a compiled graph.
Full offline Cargo metadata failed on absent `addr2line` 0.25.1; an
explicit Linux-host filtered query failed on absent 0.26.1. No fetch or
build was performed. Resolve the actual selected target graph before
attesting compilation or leasing its cache.
