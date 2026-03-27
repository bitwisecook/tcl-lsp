# Core C++ library (no Python dependency).
tcl_lsp_inc = include_directories('include')

tcl_lsp_lib = static_library('tcl_lsp',
  'src/core/source_position.cpp',
  'src/core/range.cpp',
  'src/core/document_buffer.cpp',
  'src/core/memory_stats.cpp',
  'src/parsing/token.cpp',
  'src/parsing/lexer.cpp',
  include_directories: tcl_lsp_inc,
)

tcl_lsp_dep = declare_dependency(
  link_with: tcl_lsp_lib,
  include_directories: tcl_lsp_inc,
)

# pybind11 extension module installed into core/ package.
# The bindings are temporary shim code — relax warnings that fire inside
# pybind11 headers (especially under GCC, which is more aggressive about
# -Warray-bounds in template-heavy code).  Our own C++ code in the core
# library and tests still builds with full -Werror.
binding_args = []
if meson.get_compiler('cpp').get_id() == 'gcc'
  # GCC produces false positives in pybind11 template-heavy code involving
  # std::vector and std::initializer_list operations.  These are well-known
  # upstream issues (pybind11#4412, GCC bugzilla#106247).
  binding_args += [
    '-Wno-array-bounds',
    '-Wno-stringop-overflow',
    '-Wno-stringop-overread',
  ]
endif

py.extension_module('_tcl_lsp_native',
  'bindings/core_types.cpp',
  dependencies: [pybind11_dep, tcl_lsp_dep],
  cpp_args: binding_args,
  subdir: 'core',
  install: true,
)

if get_option('tests')
  subdir('tests')
endif
