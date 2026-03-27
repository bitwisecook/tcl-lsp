catch2_dep = dependency('catch2-with-main')

test_sources = {
  'test_source_position': 'test_source_position.cpp',
  'test_range': 'test_range.cpp',
  'test_document_buffer': 'test_document_buffer.cpp',
  'test_token': 'test_token.cpp',
  'test_lexer': 'test_lexer.cpp',
}

foreach name, src : test_sources
  exe = executable(name, src,
    dependencies: [catch2_dep, tcl_lsp_dep],
  )
  test(name, exe, suite: 'tcl-lsp')
endforeach
