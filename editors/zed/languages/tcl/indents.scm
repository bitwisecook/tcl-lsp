; Indentation rules for Tcl in Zed
; Based on upstream tree-sitter-tcl indents.scm

[
  (braced_word_simple)
  (namespace)
  (command)
  (if)
  (else)
  (elseif)
  (foreach)
  (while)
  (try)
  (procedure)
  (command_substitution)
] @indent

"}" @outdent
"]" @outdent
