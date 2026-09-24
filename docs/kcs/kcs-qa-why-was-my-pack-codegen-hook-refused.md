# KCS: Why was the codegen hook in my pack refused?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors

## Question

My `.tclspec` pack has a `codegen_hook`, `inline_codegen_hook`, or
`semantic_operation {Intrinsic …}` row, and the pack file shows a warning
that it was refused. Why, and what still works?

## Answer

Those three rows do not describe a command to the editor. They tell the
compiler to emit a built-in command's own fast code for your command, and
wrong fast code is a wrong program. So tcl-lsp keeps such a row only when
both of these are true:

- The pack ships with tcl-lsp (the bundled tier). A pack in your
  workspace, in your user folder, or in a Spec Studio session cannot keep
  one.
- The command says which built-in it is, with `alias_of NAME`, and that
  built-in has the same row. `codegen_hook -native Lassign` is `lassign`'s
  own, so it can only sit on a command that says `alias_of lassign`.

Otherwise the server drops the row and puts one warning on the command's
line:

```text
`codegen_hook Lassign` refused for `vendor::unpack`: a trusted workspace
pack may not name a codegen catalogue member; the stamp would have to sit
on `alias_of lassign`
```

Only that row goes. The command keeps its arity, argument roles, hover
text, and hook bodies, so completion, hover, and diagnostics work as
before. Calls to it compile to ordinary command dispatch, which is always
correct.

In your own pack there is nothing to fix: delete the row to clear the
warning. The `spectcl_check` MCP tool lists the same refusals under
`stamp_refusals` before you save.

## Related

- [How do I write a SpecTcl pack?](kcs-howto-write-a-tclspec-pack.md)
- [Why is my pack hook dormant?](kcs-qa-why-is-my-pack-hook-dormant.md)
- The design: [what a pack still cannot say](../design/registry/spec-packs.md#what-a-pack-still-cannot-say)
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
