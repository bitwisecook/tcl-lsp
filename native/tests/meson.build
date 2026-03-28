catch2_dep = dependency('catch2-with-main')

# LSP server tests (depend on lsp-framework).
lsp_test_sources = {
  'test_lsp_document_store': 'test_lsp_document_store.cpp',
}

foreach name, src : lsp_test_sources
  exe = executable(name, src,
    dependencies: [catch2_dep, tcl_lsp_server_dep],
  )
  test(name, exe, suite: 'tcl-lsp')
endforeach

# Core library tests (no lsp-framework dependency).
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
  'test_recovery_ghost': 'test_recovery_ghost.cpp',
  'test_upstream_parse': 'test_upstream_parse.cpp',
  'test_semantic_types': 'test_semantic_types.cpp',
  'test_analysis_result': 'test_analysis_result.cpp',
  'test_param_list_parser': 'test_param_list_parser.cpp',
  'test_stub_parser': 'test_stub_parser.cpp',
  'test_analyser_proc': 'test_analyser_proc.cpp',
  'test_analyser_variable': 'test_analyser_variable.cpp',
  'test_analyser_control_flow': 'test_analyser_control_flow.cpp',
  'test_analyser_regex': 'test_analyser_regex.cpp',
  'test_analyser_package': 'test_analyser_package.cpp',
  'test_analyser_alias': 'test_analyser_alias.cpp',
  'test_analyser_w123': 'test_analyser_w123.cpp',
  'test_analyser_diagnostics': 'test_analyser_diagnostics.cpp',
}

foreach name, src : test_sources
  exe = executable(name, src,
    dependencies: [catch2_dep, tcl_lsp_dep],
  )
  test(name, exe, suite: 'tcl-lsp')
endforeach
