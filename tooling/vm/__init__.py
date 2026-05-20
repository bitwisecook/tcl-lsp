"""Tcl bytecode virtual machine.

Executes Tcl code using the full compiler pipeline from ``compiler/``:
lexer -> segmenter -> lowering -> IR -> CFG -> codegen -> bytecode.
Provides a curses-based REPL (with readline fallback) similar to tclsh.
"""
