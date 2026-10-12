The unchanged negative fixture has five statements in lexical order:
`method $name {} {}`; `method \uD800 {} {}`; `method Ω {} {}`;
`method \x00 {} {}`; **`method literal $params {}`**.
The actual previous binary fails `region.script_bodies(command).unwrap()` during collection,
before any row/name assertions. It does not print the failing statement.

Read-only API tracing identifies the fifth statement as the source-layout refusal:
- `NativeCompilerWords::capture_word` returns `Substituted, None` for `$params`
  (native_compiler_words.rs:362); no counted/formal value or logical string is issued.
- `OriginalSourceDefinitionMemberRegion::script_bodies` maps that actual absent value
  to `InvocationWord::Dynamic` before calling `source_member_script_arguments_in`.
- The method descriptor has `TCLOO_METHOD_VISIBILITY_ARGUMENT.position == 1`
  (definer.rs:3743), the argument position occupied by the formal list here.
- `source_member_indices_where` requires `word_at(optional.position)?` (definer.rs:3498).
  `word_at` borrows only literal/native byte facets. `Dynamic` supplies neither,
  so the source-body selector returns `None` before the new row-only ingress runs.

For the preceding four statements, argument 1 is the independently captured empty braced
literal. Unknown/opaque/non-ASCII/zero names are at argument 0 and do not choose this optional
layout. Their `{}` bodies have genuine original braced geometry. This is deterministic
code tracing, not a fresh runtime measurement of each statement or row assertion.

A bounded future diagnostic should name each exact original statement and record
`script_bodies` availability before collecting rows, plus optional-position word kind.
Do not change `$params`, switch profiles, bypass optional ambiguity or infer an arity row.
If measured as traced, a separate explicit source-body refusal assertion for the fifth
statement preserves a stronger negative barrier; no such test change is made here.
All previous exact source, binary and failure captures remain unchanged.
