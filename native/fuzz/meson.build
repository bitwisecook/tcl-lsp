fuzz_lexer = executable('fuzz_lexer',
  'fuzz_lexer.cpp',
  dependencies: [tcl_lsp_dep],
  link_args: ['-fsanitize=fuzzer'],
)
