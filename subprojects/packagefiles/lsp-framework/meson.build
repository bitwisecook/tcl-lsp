# lsp-framework — Meson build for the tcl-lsp fork.
#
# Uses pre-generated LSP types in generated/lsp/ (run lspgen manually
# when updating the metaModel.json).

project('lsp-framework', 'cpp',
  version: '0.1.0',
  default_options: [
    'cpp_std=c++20',
    'warning_level=3',
    'werror=false',
  ],
)

cc = meson.get_compiler('cpp')

lsp_src = files(
  'lsp/connection.cpp',
  'lsp/fileuri.cpp',
  'lsp/messagehandler.cpp',
  'lsp/process.cpp',
  'lsp/threadpool.cpp',
  'lsp/uri.cpp',
  'lsp/io/socket.cpp',
  'lsp/io/standardio.cpp',
  'lsp/json/json.cpp',
  'lsp/jsonrpc/jsonrpc.cpp',
  'generated/lsp/types.cpp',
)

# '.' for <lsp/connection.h> etc., 'generated' for <lsp/types.h> and <lsp/messages.h>.
lsp_inc = include_directories('.', 'generated')

lsp_lib = static_library('lsp',
  lsp_src,
  include_directories: lsp_inc,
)

lsp_dep = declare_dependency(
  link_with: lsp_lib,
  include_directories: lsp_inc,
)
