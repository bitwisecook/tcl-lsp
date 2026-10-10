# KCS: Why is my pack hook dormant?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors

## Question

My workspace carries a `.tclspec` pack, and a hook in it does nothing: the
pack file shows a note that the hook "is dormant". Why, and how do I make it
run?

## Answer

A spec pack holds two kinds of thing. **Declarations** say what a command
looks like: its arity, which argument is a script, its hover text. **Hook
bodies** are small Tcl scripts the language server runs while it analyses
your code: a `const_fold { … }` block, an option's `-arity-hook`, an
`evaluate -implementation` body, and the other hooks a pack can write out in
Tcl.

In a workspace your editor does not trust, the server uses the declarations
and does not run the hook bodies. A hook body runs again and again, on words
taken from the code you are editing. In a folder you have not trusted, both
the pack and that code may come from someone else. So the body stays
dormant, and the server puts one information note on the line that declares
it:

```text
`const_fold` is dormant: the workspace is not trusted, so this hook body
does not run and the command keeps its declarative facts
```

Everything the declarations give you keeps working: the commands are known,
hover and completion answer, and diagnostics use the arity and argument
roles. What you lose is what the body computes, such as a folded constant.
A `-native` hook names code tcl-lsp ships, not Tcl the pack supplies, so it
is never dormant.

Trust the workspace to run the bodies. The server reloads its packs as soon
as the editor reports the change. The notes clear, and you do not need to
restart anything.

### VS Code

VS Code shows **Restricted Mode** in the status bar while a folder is not
trusted. Run **Workspaces: Manage Workspace Trust** from the Command Palette
(`Ctrl+Shift+P` or `Cmd+Shift+P`) and choose **Trust**.

### Other editors

Zed, JetBrains, Neovim, Helix, Emacs, and Sublime Text do not tell the
language server whether a folder is trusted. The server treats their
workspaces as trusted, so pack hook bodies always run there.

## Related

- [How do I write a SpecTcl pack?](kcs-howto-write-a-tclspec-pack.md)
- [How do I declare an evaluator for my own pack command?](spectcl/kcs-howto-declare-an-evaluator-for-a-pack-command.md)
- The design: [spec packs and Workspace Trust](../design/registry/spec-packs.md#workspace-trust-the-setting-is-gated-the-workspace-tier-is-not)
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
