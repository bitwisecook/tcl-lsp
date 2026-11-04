# KCS: Does a class in the outline prove which method will run?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors

## Question

Does seeing a class and method in the outline guarantee a method jump or rename?

## Answer

The outline lists written declarations. A method jump also needs the receiving
object and the method selected at that call. Rename needs enough information
to identify and update every affected occurrence safely.

Use the outline to open the written declaration when that call remains unknown.
Loading a library in another shell does not establish the editor's call context.

Clay can generate methods and rewrite constructor bodies. Its ordinary method
declarations therefore provide only part of the class's possible behaviour.
The HTTPD `my connect` jump remains a separate open investigation in
[#1956](https://github.com/bitwisecook/tcl-lsp/issues/1956).

## Related

- [Source class inventory](../GLOSSARY.md#source-class-inventory)
- [TclOO implementation](../design/contracts/tcloo-implementation.md#clay-source-inventory)
- [KCS index](README.md)
