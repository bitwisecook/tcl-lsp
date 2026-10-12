# Bounded authored-document cleanup

2026-10-12: inspected exactly five authored Markdown documents: the three F5
documents below and `docs/design/registry/dialect-and-package-registry-redesign.md`
and `docs/design/registry/dialect-and-package-registry-centralisation.md`.
The two registry documents were left unchanged to avoid naming-contract overlap.
Only three F5 documents and this review record were edited. No executable source,
generated content, proof manifest, Cargo, Git or issue mutation was performed.

Five concrete correction groups, with current post-edit locations:

1. `docs/design/f5/f5-cli-architecture.md:169–171`, IP-redaction model:
   replaced the incorrect `/20-worth` limit with **20 host bits / 1,048,576
   entries**. `rust/tcl-bigip/src/redact.rs:854–860` defines the bit-width cap
   and `1usize << host_width`; `shuffle_host` at line 880 computes width as
   address width minus prefix length and falls back to direct above the cap.

2. `docs/design/f5/f5-cli-architecture.md:123–127`, edit-plan layering:
   replaced bottom-up splicing with ascending reconstruction and overlap
   rejection. `rust/tcl-bigip-query/src/edit_plan.rs:455–495` sorts spans
   ascending, checks overlap, then appends unchanged slices and replacements.

3. `docs/design/f5/bigip-registry-architecture.md:144–148`, reference dispatch:
   removed migration narration and corrected conditional-fallback wording.
   `rust/tcl-bigip/src/graph.rs:601–619` always runs `pilot_pass`, `key_pass`
   and `section_pass` in that order; their shared accumulator deduplicates edges.
   Modelled extraction shapes and reference-free properties remain documented.

4. `docs/design/f5/f5-query-engine-internals.md:630–641`, diagram/ordering
   explanation at `:650–677`, and apply-order contract at `:869–884`:
   corrected all repeated descending-splice and final-validation-reparse claims.
   `edit_plan.rs::apply_one_uri` at line 232 applies prefix/identity/field edits
   and returns `AppliedSource` at line 301 without calling `parse_bigip_conf`.
   `splice_edits` at line 455 reconstructs text after ascending overlap checks.
   The diagram now flows directly from rewritten bytes to `AppliedSource`.

5. `docs/design/f5/f5-query-engine-internals.md:61–63` and `:128–130`,
   runner overview: removed the nonexistent final output-validation guarantee.
   `rust/tcl-bigip-query/src/runner.rs:164–168` parses when building a root;
   statement loops at `:293–299` and `:569–575` rebuild roots from current text
   before evaluation. This preserves the real sequential-statement semantics.

Validation: all 15 local Markdown links in the three edited documents resolve
(5 CLI, 1 registry, 9 query). No link targets were changed. Contextual reread
retained actual runtime phases, caller procedures, oracle/provenance material
and current extension guidance. Behaviour and test inputs remain unchanged;
these corrections describe the implementation, not newly executed acceptance.
The repository-wide final cleanup audit remains unfinished.
