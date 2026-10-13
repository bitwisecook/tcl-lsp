# Sealed Compiler source trace

This replays the exact executable from compiler-wasm-validation-01 after the
checkout began other producer edits. Binary identity and original build closure
remain separate from current source. The existing compiled source trace flag
was enabled; no source was edited and no build occurred.

The fixed original root-colon source retains two Call rows with four and one
original words and matching retained words. It still takes GenericInvoke. The
trace does not show the first native admission gate, so it establishes no cause
for missing table attachment and no native positive acceptance.
