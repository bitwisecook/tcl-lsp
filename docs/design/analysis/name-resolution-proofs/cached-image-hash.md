# Cached image hash

Proof ID: `naming.source.cached-image-hash`

## Problem statement

Original producer graphs repeatedly hash the same full source buffer. Replacing that cost with an offset, pointer or digest identity would merge different source bytes or Document and NativeValue channels, invalidating name provenance.

## Question

Can SourceImage cache hashing while preserving full byte/channel equality and ordering, including a deliberate digest collision?

## Scope

Implementation identity and cost model only. No timing, native parser acceptance, source correspondence or interpreter authority follows from the digest.

## Provider answers

| Provider | Status | Answer |
| --- | --- | --- |
| tcl8.4 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| tcl8.5 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| tcl8.6 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| tcl9.0 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| tcl9.1 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| jim | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| bigip | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |

## Conclusion

SourceImage computes an immutable byte digest once and hashes that digest with its channel. Equality and ordering still compare complete bytes and channel. A deliberately colliding digest remains two distinct map keys and two distinct ordering values.

## Shared owners and tests

- `rust/tcl-lexer/src/source_map.rs`: `SourceImage`.

- `rust/tcl-lexer/src/source_map.rs`: `source_map::source_image_hash_tests::source_image_cached_hash_preserves_full_equality_order_and_channel`.
- `rust/tcl-lexer/src/source_map.rs`: `source_map::source_image_hash_tests::source_image_digest_collision_does_not_merge_map_or_order_identity`.

The tests assert implementation correspondence and refusal, not a native observation. No actual run or timing result is encoded in this record. Source producer currency is independent of cell-read currency and Normal completion.

## Reconfirmation

Run the exact Rust selectors listed above in the relevant crate with the workspace's maintained test setup. No C/Jim/BIG-IP replay is attached to this implementation question.
