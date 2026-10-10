# naming.source.original-document-line-ending-projection

Kind: `implementation-contract`

## Problem statement

LF presentation of CRLF or lone-CR document text changes byte coordinates. Relexing equal presentation text cannot recover a whole original word, and the collapsed CR/LF middle boundary cannot supply a genuine source extent.

## Question

How does a Document-only newline projection retain the complete original image and map checked Unicode offsets and whole spans to LF presentation without inventing collapsed boundaries or accepting Native value channels?

## Conclusion

DocumentLineEndingProjection retains one complete original Document SourceImage and a separate LF presentation. CRLF collapses to one LF; lone CR becomes LF without removing a byte. Bidirectional offsets require valid Unicode byte boundaries and checked u32 arithmetic. The original CR/LF middle boundary is unmappable, so a span using it refuses. Whole original/presentation spans round-trip only through valid endpoints. NativeValue, invalid UTF-8 and unsupported extents have no document projection. The original bytes and channel remain unchanged; this geometric projection supplies no source word, value, grammar, Registry descriptor or execution owner.

Original provider direct/file-channel line-ending observations remain separately recorded in [source-original-crlf-evaluation-channels.md](source-original-crlf-evaluation-channels.md). Actual-analysis layout uses this geometry under the independent [formatting contract](editor-original-source-formatting.md).

## Scope

Two marked Lexer software controls cover mixed CRLF/lone-CR/LF text, whole Unicode words, every valid presentation byte boundary, collapsed/Unicode-interior/out-of-range refusals, empty/end extents and the NativeValue/opaque-byte negative channels. They are current source/API definitions with no completed assertion result or external-provider process attached. The projection is document geometry, independently of lexer grammar, original values, source input currency, current command lookup, entry/frame, Native String/object/cache birth, Normal completion or safe rewrite. Original direct-versus-file CRLF evaluation330 remains a separate measured provider question; it does not admit this software geometry or a Native value channel.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this document geometry contract. Dialect: tcl8.4.

No original C, Jim or BIG-IP process answers the software Document projection contract. Authored Unicode/line-ending tests assert geometry and refusal only; provider input-channel behaviour and Native authority are independent.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this document geometry contract. Dialect: tcl8.5.

No original C, Jim or BIG-IP process answers the software Document projection contract. Authored Unicode/line-ending tests assert geometry and refusal only; provider input-channel behaviour and Native authority are independent.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this document geometry contract. Dialect: tcl8.6.

No original C, Jim or BIG-IP process answers the software Document projection contract. Authored Unicode/line-ending tests assert geometry and refusal only; provider input-channel behaviour and Native authority are independent.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this document geometry contract. Dialect: tcl9.0.

No original C, Jim or BIG-IP process answers the software Document projection contract. Authored Unicode/line-ending tests assert geometry and refusal only; provider input-channel behaviour and Native authority are independent.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this document geometry contract. Dialect: tcl9.1.

No original C, Jim or BIG-IP process answers the software Document projection contract. Authored Unicode/line-ending tests assert geometry and refusal only; provider input-channel behaviour and Native authority are independent.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this document geometry contract. Dialect: jim.

No original C, Jim or BIG-IP process answers the software Document projection contract. Authored Unicode/line-ending tests assert geometry and refusal only; provider input-channel behaviour and Native authority are independent.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this document geometry contract. Dialect: bigip.

No original C, Jim or BIG-IP process answers the software Document projection contract. Authored Unicode/line-ending tests assert geometry and refusal only; provider input-channel behaviour and Native authority are independent.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lexer/src/source_map.rs](../../../../rust/tcl-lexer/src/source_map.rs), `DocumentLineEndingProjection::new`: Accept only a complete valid UTF-8 Document SourceImage within shared offset bounds, retain its identity and record exact CRLF collapse positions.
- [rust/tcl-lexer/src/source_map.rs](../../../../rust/tcl-lexer/src/source_map.rs), `DocumentLineEndingProjection::original_offset`: Map a valid presentation Unicode byte boundary to the complete original document using checked arithmetic.
- [rust/tcl-lexer/src/source_map.rs](../../../../rust/tcl-lexer/src/source_map.rs), `DocumentLineEndingProjection::normalised_offset`: Map a valid original Unicode byte boundary while refusing the removed CR/LF middle boundary.
- [rust/tcl-lexer/src/source_map.rs](../../../../rust/tcl-lexer/src/source_map.rs), `DocumentLineEndingProjection::original_span`: Map a whole valid presentation extent through both original endpoints without inventing words or changing source identity.
- [rust/tcl-lexer/src/source_map.rs](../../../../rust/tcl-lexer/src/source_map.rs), `DocumentLineEndingProjection::normalised_span`: Map a whole original extent only when both endpoints retain genuine presentation correspondence.
- [rust/tcl-lexer/src/source_map.rs](../../../../rust/tcl-lexer/src/source_map.rs), `source_map::document_line_ending_projection_tests::document_projection_round_trips_whole_unicode_words_across_mixed_endings` (linked): Mixed CRLF/lone-CR/LF source keeps complete original bytes and whole Unicode braced-word extents; every valid presentation Unicode boundary round-trips and Unicode interiors refuse.
- [rust/tcl-lexer/src/source_map.rs](../../../../rust/tcl-lexer/src/source_map.rs), `source_map::document_line_ending_projection_tests::document_projection_refuses_collapsed_boundary_native_values_and_non_source_extents` (linked): Collapsed CR/LF middle endpoints, Unicode interiors, out-of-range coordinates, NativeValue and invalid UTF-8 refuse; empty, LF, lone-CR and CRLF whole extents retain checked round-trip geometry.

A named test is a coverage binding, not a claim that it executed.

## Replay

Source/API definitions only; no Rust assertion or external provider operation is attached. Native original-source/file-channel CRLF observations330 retain their independent request, version and input-channel scope.
