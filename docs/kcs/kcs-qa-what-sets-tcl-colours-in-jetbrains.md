# KCS: What sets the colours of a Tcl file in a JetBrains IDE?

> **Audience:** User
> **Type:** Q&A

## Applies to

JetBrains

## Question

What decides how a Tcl file is coloured in a JetBrains IDE, and where do I
change those colours?

## Answer

Two layers colour a Tcl file, and both draw on your own editor colour
scheme. The plugin ships no colours of its own.

The **bundled TextMate grammar** colours the file the moment it opens. It
reaches keywords, strings, numbers, comments, and `proc` names.

**Semantic tokens** from the Tcl Language Server are painted over the top.
The server knows what each word means, so this layer reaches built-in
commands, your own procs, variables, parameters, operators, iRule events,
regular expressions, and `format` specifiers. Switch it off under **Settings
→ Tools → Tcl Language Server → Features → Semantic tokens**.

Every colour comes from **Settings → Editor → Color Scheme → Language
Defaults**. Change an entry there and every Tcl token that uses it follows.

| Tcl token | Language Defaults entry |
|---|---|
| built-in command, your own proc, iRule event handler | **Identifiers → Function declaration** |
| variable, proc parameter | **Classes → Instance field** |
| `proc`, `if`, `foreach`, and the other control commands | **Keyword** |
| string | **String → String text** |
| number, BIG-IP port, route domain | **Number** |
| comment | **Comments → Line comment** |
| backslash escape, `format` and `clock` specifier | **String → Escape sequence → Valid** |
| namespace, BIG-IP pool, monitor, profile | **Classes → Class reference** |

Some entries are plain in a stock scheme, because the IDE leaves the same
tokens plain in its own languages: an operator such as `+` uses **Braces and
Operators → Operation sign**, and a TclOO class name uses **Classes → Class
name**. On the **Classic Light** scheme, command names are plain as well —
that scheme colours no function name in any language, Java included. Give any
of those entries a foreground on the same page if you want it coloured.

With **Semantic tokens** switched off, only the grammar layer remains.
Built-in command names and variables then stay in the plain text colour.
The IDE's TextMate support maps those scopes to scheme entries the stock
schemes leave unpainted, and no plugin can change that table.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [feature — Semantic tokens](features/kcs-feature-semantic-tokens.md)
