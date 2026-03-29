# Core C++ library (no Python dependency).
tcl_lsp_inc = include_directories('include')

tcl_lsp_lib = static_library('tcl_lsp',
  'src/core/source_position.cpp',
  'src/core/range.cpp',
  'src/core/document_buffer.cpp',
  'src/core/memory_stats.cpp',
  'src/core/diagnostic.cpp',
  'src/parsing/token.cpp',
  'src/parsing/lexer.cpp',
  'src/parsing/segmenter.cpp',
  'src/parsing/recovery.cpp',
  'src/analysis/semantic_types.cpp',
  'src/analysis/auxiliary_types.cpp',
  'src/analysis/analysis_result.cpp',
  'src/analysis/command_interface.cpp',
  'src/analysis/param_list_parser.cpp',
  'src/analysis/stub_parser.cpp',
  'src/analysis/analyser.cpp',
  'src/analysis/analyser_commands.cpp',
  'src/analysis/analyser_helpers.cpp',
  'src/analysis/arg_role_resolver.cpp',
  'src/parsing/expr_lexer.cpp',
  'src/lsp/known_commands.cpp',
  'src/lsp/semantic_token_collector.cpp',
  include_directories: tcl_lsp_inc,
)

tcl_lsp_dep = declare_dependency(
  link_with: tcl_lsp_lib,
  include_directories: tcl_lsp_inc,
)

# LSP server core (document store — no Python dependency).
tcl_lsp_server_core = static_library('tcl_lsp_server_core',
  'src/lsp/document_store.cpp',
  include_directories: tcl_lsp_inc,
  dependencies: [tcl_lsp_dep, lsp_framework_dep],
)

tcl_lsp_server_core_dep = declare_dependency(
  link_with: tcl_lsp_server_core,
  include_directories: tcl_lsp_inc,
  dependencies: [tcl_lsp_dep, lsp_framework_dep],
)

# LSP server with Python bridge (requires pybind11 for embedded interpreter).
# Only built when pybind11 is available.
if pybind11_dep.found()
  # Relax warnings from pybind11 template-heavy headers.
  # Relax GCC warnings that fire in pybind11 template-heavy headers.
  # These are well-known GCC false positives (pybind11#4412, GCC#106247).
  # Our own non-pybind11 code builds with full -Werror.
  bridge_args = []
  if meson.get_compiler('cpp').get_id() == 'gcc'
    bridge_args += [
      '-Wno-array-bounds',
      '-Wno-stringop-overflow',
      '-Wno-stringop-overread',
      '-Wno-attributes',
      '-Wno-null-dereference',
    ]
  endif

  py_embed_dep = py.dependency(embed: true)

  tcl_lsp_server_lib = static_library('tcl_lsp_server',
    'src/lsp/server.cpp',
    'src/lsp/python_bridge.cpp',
    include_directories: tcl_lsp_inc,
    dependencies: [tcl_lsp_server_core_dep, pybind11_dep, py_embed_dep],
    cpp_args: bridge_args,
  )

  tcl_lsp_server_dep = declare_dependency(
    link_with: tcl_lsp_server_lib,
    include_directories: tcl_lsp_inc,
    dependencies: [tcl_lsp_server_core_dep, pybind11_dep, py_embed_dep],
  )

  # LSP server executable.
  executable('tcl-lsp-server',
    'src/lsp/main.cpp',
    dependencies: [tcl_lsp_server_dep],
    cpp_args: bridge_args,
    install: true,
  )

  # pybind11 extension module installed into core/ package.
  binding_args = bridge_args
  py.extension_module('_tcl_lsp_native',
    'bindings/core_types.cpp',
    dependencies: [pybind11_dep, tcl_lsp_dep],
    cpp_args: binding_args,
    subdir: 'core',
    install: true,
  )
endif

if get_option('tests')
  subdir('tests')
endif

if get_option('fuzz')
  subdir('fuzz')
endif
