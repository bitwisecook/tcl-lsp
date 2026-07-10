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
    "apply" "break" "classvariable" "continue" "coroutine" "global"
    "interp" "my" "next" "nextto" "oo::abstract" "oo::class"
    "oo::configurable" "oo::define" "oo::objdefine" "oo::singleton" "package" "rename"
    "return" "self" "source" "tailcall" "throw" "uplevel"
    "upvar" "variable" "yield" "yieldto"))

; --- generated from tcl-registry: built-in commands ---
(command
  name: (simple_word) @function.builtin
  (#any-of? @function.builtin
    "::tcl::mathop::eq" "::tcl::mathop::in" "::tcl::mathop::ne" "::tcl::mathop::ni" "::tcl::process" "::tcl::unsupported::corotype"
    "after" "append" "array" "auto_execok" "auto_import" "auto_load"
    "auto_load_index" "auto_mkindex" "auto_mkindex_old" "auto_qualify" "auto_reset" "bgerror"
    "binary" "cd" "chan" "clock" "close" "concat"
    "const" "coroinject" "coroprobe" "debug" "dict" "disconnect"
    "divmod" "encoding" "eof" "eq" "error" "eval"
    "exec" "exit" "exp_continue" "exp_internal" "exp_pid" "exp_version"
    "expect" "expect_after" "expect_background" "expect_before" "expect_tty" "expect_user"
    "fblocked" "fconfigure" "fcopy" "file" "fileevent" "filename"
    "flush" "fork" "format" "frexp" "gets" "gettimes"
    "glob" "history" "http" "in" "incr" "info"
    "interact" "join" "lappend" "lassign" "ledit" "lgen"
    "lindex" "linsert" "list" "llength" "load" "log_file"
    "log_user" "lpop" "lrange" "lremove" "lrepeat" "lreplace"
    "lreverse" "lsearch" "lseq" "lset" "lsort" "lstring"
    "match_max" "memory" "modf" "ne" "ni" "noop"
    "oo::copy" "oo::object" "open" "overlay" "parity" "parray"
    "pid" "pkg::create" "pkg_mkIndex" "pkg_mkindex" "puts" "pwd"
    "re_quote" "read" "readFile" "regex::quote" "regex_quote" "regexp"
    "regexp::quote" "registry" "regsub" "remove_nulls" "remquo" "scan"
    "seek" "send" "send_error" "send_log" "send_tty" "send_user"
    "sleep" "socket" "spawn" "split" "strace" "string"
    "stty" "subst" "system" "tcl::mathop" "tcl::mathop::eq" "tcl::mathop::in"
    "tcl::mathop::ne" "tcl::mathop::ni" "tcl::process" "tcl::tm::path" "tcl::tm::roots" "tclLog"
    "tclPkgSetup" "tclPkgUnknown" "tcl_endOfWord" "tcl_findLibrary" "tcl_startOfNextWord" "tcl_startOfPreviousWord"
    "tcl_wordBreakAfter" "tcl_wordBreakBefore" "tell" "testaccessproc" "testalarm" "testapplylambda"
    "testappverifierpresent" "testasync" "testbigdata" "testbignumobj" "testbooleanobj" "testbumpinterpepoch"
    "testbytestring" "testchancreate" "testchannel" "testchannelevent" "testchmod" "testcmdinfo"
    "testcmdobj2" "testcmdtoken" "testcmdtrace" "testconcatobj" "testconvertobj" "testcpuid"
    "testcreatecommand" "testdcall" "testdel" "testdelassocdata" "testdoubledigits" "testdoubleobj"
    "testdstring" "testencoding" "testevalex" "testevalobjv" "testevent" "testexithandler"
    "testexitmainloop" "testexprdouble" "testexprdoubleobj" "testexprlong" "testexprlongobj" "testexprparser"
    "testexprstring" "testfevent" "testfile" "testfilehandler" "testfilelink" "testfilesystem"
    "testfilewait" "testfindexecutable" "testfindfirst" "testfindlast" "testfork" "testfstildeexpand"
    "testgetassocdata" "testgetdefenc" "testgetindexfromobjstruct" "testgetint" "testgetintforindex" "testgetopenfile"
    "testgetplatform" "testgetunichar" "testgetvarfullname" "testgotsig" "testhandlecount" "testhashsystemhash"
    "testindexobj" "testinterpdelete" "testinterpresolver" "testintobj" "testisempty" "testlink"
    "testlinkarray" "testlistapi" "testlistobj" "testlistrep" "testlocale" "testlongsize"
    "testlutil" "testmainthread" "testmsb" "testnrelevels" "testnreunwind" "testnumutfchars"
    "testobj" "testopenfilechannelproc" "testpanic" "testparseargs" "testparser" "testparsevar"
    "testparsevarname" "testpostinit" "testpreferstable" "testprint" "testpurebytesobj" "testregexp"
    "testreturn" "testsaveresult" "testservicemode" "testset2" "testsetassocdata" "testsetbytearraylength"
    "testsetdefenc" "testseterr" "testseterrorcode" "testsetmainloop" "testsetnoerr" "testsetobjerrorcode"
    "testsetplatform" "testsimplefilesystem" "testsize" "testsocket" "teststaticlibrary" "teststaticpkg"
    "teststatproc" "teststringbytes" "teststringobj" "testtranslatefilename" "testuniclass" "testupvar"
    "testutfnext" "testutfprev" "testutftonormalized" "testutftonormalizeddstring" "testwrongnumargs" "time"
    "timer" "timerate" "timestamp" "trace" "trap" "unicode"
    "unknown" "unload" "unset" "update" "vwait" "wait"
    "writeFile" "zlib"))

; Highlight unset / variable arguments as variables
(command
  name: (simple_word) @_kw
  arguments: (word_list) @variable
  (#any-of? @_kw "unset" "variable"))

; Generic command calls
(command
  name: (simple_word) @function)

; Operators
(unpack) @operator

["**" "/" "*" "%" "+" "-"
 "<<" ">>"
 ">" "<" ">=" "<="
 "==" "!="
 "eq" "ne"
 "in" "ni"
 "&" "^" "|"
 "&&" "||"
] @operator

; Punctuation
["{" "}" "[" "]" ";"] @punctuation.bracket
