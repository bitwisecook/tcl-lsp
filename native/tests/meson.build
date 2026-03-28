catch2_dep = dependency('catch2-with-main')

test_sources = {
  'test_source_position': 'test_source_position.cpp',
  'test_range': 'test_range.cpp',
  'test_document_buffer': 'test_document_buffer.cpp',
  'test_token': 'test_token.cpp',
  'test_lexer': 'test_lexer.cpp',
  'test_lexer_edge_cases_1': 'test_lexer_edge_cases_1.cpp',
  'test_lexer_edge_cases_2': 'test_lexer_edge_cases_2.cpp',
  'test_lexer_edge_cases_3': 'test_lexer_edge_cases_3.cpp',
  'test_segmenter': 'test_segmenter.cpp',
  'test_segmenter_recovery': 'test_segmenter_recovery.cpp',
  'test_segmenter_chunks': 'test_segmenter_chunks.cpp',
  'test_recovery_e201': 'test_recovery_e201.cpp',
  'test_recovery_e202': 'test_recovery_e202.cpp',
  'test_recovery_e203': 'test_recovery_e203.cpp',
  'test_recovery_virtual': 'test_recovery_virtual.cpp',
  'test_upstream_parse': 'test_upstream_parse.cpp',
}

foreach name, src : test_sources
  exe = executable(name, src,
    dependencies: [catch2_dep, tcl_lsp_dep],
  )
  test(name, exe, suite: 'tcl-lsp')
endforeach
