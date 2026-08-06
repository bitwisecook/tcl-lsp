; Tcl-family highlights for Zed
;
; DO NOT EDIT — the command-classification `#any-of?` lists below are generated
; from `tcl-registry` (scoped to this language's dialect) by
; `cargo xtask gen-zed-queries`. Structural rules are a static template in that
; generator. Run `make generate` after registry changes.

; Comments
(comment) @comment

; Strings and escapes
(quoted_word) @string
(escaped_character) @string.escape

; Numbers
(number) @number

((simple_word) @number
  (#match? @number "^[+-]?[0-9]+$"))

((simple_word) @boolean
  (#any-of? @boolean "true" "false"))

; Variables
(variable_substitution) @variable

(set (id) @variable)

(argument
  name: (_) @variable.parameter)

; Tcl built-in variables
((simple_word) @variable.builtin
  (#any-of? @variable.builtin
    "argc" "argv" "argv0" "auto_path" "env" "errorCode" "errorInfo"
    "tcl_interactive" "tcl_library" "tcl_nonwordchars" "tcl_patchLevel"
    "tcl_pkgPath" "tcl_platform" "tcl_precision" "tcl_rcFileName"
    "tcl_traceCompile" "tcl_traceExec" "tcl_wordchars" "tcl_version"))

; Procedure definitions
"proc" @keyword.function

(procedure
  name: (_) @function.definition)

; Control-flow keywords parsed as dedicated grammar nodes
["if" "else" "elseif"] @keyword.control
["while" "foreach"] @keyword.control
["catch" "try" "finally"] @keyword.control
["set" "namespace"] @keyword
"expr" @function.builtin

; --- generated from tcl-registry: control-flow commands ---
(command
  name: (simple_word) @keyword.control
  (#any-of? @keyword.control
    "foreachLine" "lfilter" "lmap"))

; --- generated from tcl-registry: language keywords ---
(command
  name: (simple_word) @keyword
  (#any-of? @keyword
    "apply" "break" "callback" "classvariable" "const" "continue"
    "coroutine" "error" "eval" "global" "interp" "link"
    "my" "mymethod" "next" "nextto" "oo::Helpers::callback" "oo::Helpers::classvariable"
    "oo::Helpers::link" "oo::Helpers::mymethod" "oo::Helpers::next" "oo::Helpers::nextto" "oo::Helpers::self" "oo::abstract"
    "oo::class" "oo::configurable" "oo::define" "oo::objdefine" "oo::object" "oo::singleton"
    "package" "rename" "return" "self" "source" "tailcall"
    "throw" "uplevel" "upvar" "variable" "yield" "yieldto"))

; --- generated from tcl-registry: built-in commands ---
(command
  name: (simple_word) @function.builtin
  (#any-of? @function.builtin
    "after" "append" "array" "auto_execok" "auto_import" "auto_load"
    "auto_load_index" "auto_mkindex" "auto_mkindex_old" "auto_qualify" "auto_reset" "bgerror"
    "binary" "cd" "chan" "clock" "close" "concat"
    "coroinject" "coroprobe" "dict" "divmod" "encoding" "eof"
    "eq" "exec" "exit" "fblocked" "fconfigure" "fcopy"
    "file" "fileevent" "filename" "flush" "format" "frexp"
    "ge" "gets" "gettimes" "glob" "gt" "history"
    "http" "in" "incr" "info" "join" "lappend"
    "lassign" "le" "ledit" "lgen" "lindex" "linsert"
    "list" "llength" "load" "lpop" "lrange" "lremove"
    "lrepeat" "lreplace" "lreverse" "lsearch" "lseq" "lset"
    "lsort" "lstring" "lt" "memory" "modf" "ne"
    "ni" "noop" "oo::copy" "open" "parray" "pid"
    "pkg::create" "pkg_mkIndex" "pkg_mkindex" "puts" "pwd" "re_quote"
    "read" "readFile" "regex::quote" "regex_quote" "regexp" "regexp::quote"
    "registry" "regsub" "remquo" "scan" "seek" "socket"
    "split" "string" "subst" "tclLog" "tclPkgSetup" "tclPkgUnknown"
    "tcl_endOfWord" "tcl_findLibrary" "tcl_startOfNextWord" "tcl_startOfPreviousWord" "tcl_wordBreakAfter" "tcl_wordBreakBefore"
    "tell" "time" "timer" "timerate" "trace" "unicode"
    "unknown" "unload" "unset" "update" "vwait" "writeFile"
    "zipfs" "zlib"))

; Highlight unset / variable arguments as variables
(command
  name: (simple_word) @_kw
  arguments: (word_list) @variable
  (#any-of? @_kw "unset" "variable"))

; Generic command calls
(command
  name: (simple_word) @function)

; Operators recognised as dedicated tokens by the vendored tree-sitter-tcl
; grammar
(unpack) @operator

[
    "!" "!=" "%" "&" "&&" "*"
    "**" "+" "-" "/" "<" "<<"
    "<=" "==" ">" ">=" ">>" "^"
    "eq" "in" "ne" "ni" "|" "||"
    "~"
] @operator

; Word-shaped operators (TIP 461 string-ordering, iRules words) the vendored
; grammar parses as a plain word rather than a dedicated token — matched by
; content, like `@boolean` below.
((simple_word) @operator
  (#any-of? @operator
    "ge" "gt" "le" "lt"))

; Punctuation
["{" "}" "[" "]" ";"] @punctuation.bracket
