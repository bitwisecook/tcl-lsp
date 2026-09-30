# KCS: Why was a declaration dropped from my dependency's pack?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors

## Question

A package I depend on ships a `.tclspec` pack beside its `tclpkg.tcl`. The
pack file shows a warning that an `alias_of` or `runtime_backing` row was
refused. Why, and what still works?

## Answer

Those two rows tell the compiler which built-in command a pack command is,
and how that command runs. A wrong claim about either is a wrong program. So
tcl-lsp trusts them by how close the package sits to your project:

- **Your own package** may say both.
- **A direct dependency** may say both. That is a package your `tclpkg.tcl`
  lists with `require`.
- **A dependency of a dependency** may say neither. So may a package you list
  only with `dev-require`.

tcl-lsp works the distance out from the `tclpkg.lock` beside your project's
`tclpkg.tcl`. It never takes it from what the package's own manifest says. A
project with no lockfile, or a package the lockfile does not list, gets no
limit.

When a row is refused, the server drops it and puts one warning on the
command's line:

```text
`alias_of lassign` refused for `dep::unpack`: a transitive dependency's pack
may not declare `alias_of`; only the workspace's own package and its direct
dependencies may
```

Only that row goes. The command keeps its arity, argument roles, hover text,
and hook bodies, so completion, hover, and diagnostics work as before.

To keep the row, make the package a direct dependency. Add a `require` line
for it to your `tclpkg.tcl`, then run `tcl pkg install` again.

## Related

- [Why was the codegen hook in my pack refused?](kcs-qa-why-was-my-pack-codegen-hook-refused.md)
- [How do I write a SpecTcl pack?](kcs-howto-write-a-tclspec-pack.md)
- The design: [what a pack still cannot say](../design/registry/spec-packs.md#what-a-pack-still-cannot-say)
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md#dependency-tier-and-codegen-capability)
