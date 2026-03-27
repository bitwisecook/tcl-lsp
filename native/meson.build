# Core C++ library (no Python dependency).
tcl_lsp_inc = include_directories('include')

tcl_lsp_lib = static_library('tcl_lsp',
  'src/core/source_position.cpp',
  'src/core/range.cpp',
  'src/core/document_buffer.cpp',
  include_directories: tcl_lsp_inc,
)

tcl_lsp_dep = declare_dependency(
  link_with: tcl_lsp_lib,
  include_directories: tcl_lsp_inc,
)

# pybind11 extension module installed into core/ package.
py.extension_module('_tcl_lsp_native',
  'bindings/core_types.cpp',
  dependencies: [pybind11_dep, tcl_lsp_dep],
  subdir: 'core',
  install: true,
)

if get_option('tests')
  subdir('tests')
endif
