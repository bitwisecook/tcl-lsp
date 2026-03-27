project('tcl-lsp', 'cpp',
  version: '1.2.4',
  default_options: [
    'cpp_std=c++23',
    'buildtype=debugoptimized',
    'warning_level=3',
    'werror=true',
  ],
)

py = import('python').find_installation(pure: false)
pybind11_dep = dependency('pybind11')

subdir('native')
