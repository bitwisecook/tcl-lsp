# naming.source.original-immutable-image-geometry

Kind: `implementation-contract`

## Problem statement

Repeated exact source-position queries rebuild whole-image line geometry and copy/hash the complete source per original command.

## Question

How can readonly original source geometry reuse immutable work while preserving whole source, channel, full grammar and context correspondence?

## Conclusion

SourceImage shares one immutable derived byte-digest/line-index payload across clones, computed on first use, so an image that is only lexed never builds line positions. SourceMap::from_image uses that exact byte/channel geometry. Full byte/channel Eq/Hash/Ord semantics exclude the cache. Shared source structure reuses the retained realm image only after complete Document bytes/full configuration match, retaining independent full input/context/Registry/vector checks.

## Scope

Immutable byte-position geometry and storage reuse only. No role, name key, availability, command selection, Native compiler admission, frame, Normal or edit authority. Source position limits and original bytes/channels remain unchanged.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This immutable source byte/channel/digest/line-position cache contract supplies no native interpreter, source evaluation, compiler admission, object/header/frame or result observation.

## Consumer bindings

- [rust/tcl-lexer/src/source_map.rs](../../../../rust/tcl-lexer/src/source_map.rs), `SourceImage`: Retain immutable bytes/channel and a derived shared geometry payload, computed on first use and excluded from identity.
- [rust/tcl-lexer/src/source_map.rs](../../../../rust/tcl-lexer/src/source_map.rs), `SourceMap::from_image`: Reuse exact immutable image line positions while preserving bytes/channel.
- [rust/tcl-compiler/src/registry_invocation/source_structure.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_structure.rs), `retained_document_image`: Authenticate complete source/channel/full configuration before cloning the same original realm image.
- [rust/tcl-compiler/src/registry_invocation/source_structure.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_structure.rs), `matching_document_image`: Require complete Document source bytes before reusing any original image.
- [rust/tcl-lexer/src/source_map.rs](../../../../rust/tcl-lexer/src/source_map.rs), `source_map::immutable_image_geometry_tests::original_image_positions_reuse_immutable_geometry_without_changing_identity` (linked): Finite Unicode/CRLF/NUL positions match across clones; independent cache, exact equality/hash/ordering and changed source/channel remain distinct.
- [rust/tcl-lexer/src/source_map.rs](../../../../rust/tcl-lexer/src/source_map.rs), `source_map::immutable_image_geometry_tests::original_native_byte_positions_need_no_unicode_or_document_channel` (linked): Opaque Native bytes retain their exact positions/channel without a Unicode view.
- [rust/tcl-compiler/src/registry_invocation/source_structure.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_structure.rs), `registry_invocation::source_structure::retained_image_geometry_tests::source_schema_geometry_reuses_the_whole_retained_image_and_keeps_currency` (linked): Genuine Logical schema words reuse the whole immutable byte allocation; stale source/configuration and NativeValue channel decline.
