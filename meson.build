project('tcl-lsp', 'cpp',
  version: '1.2.4',
  default_options: [
    'cpp_std=c++23',
    'buildtype=debugoptimized',
    'warning_level=3',
    'werror=true',
  ],
)

cc = meson.get_compiler('cpp')

# Use libc++ with Clang for full C++23 support (libstdc++-13 lacks
# __cpp_concepts >= 202002L which gates <expected> and other C++23 headers).
if cc.get_id() == 'clang' and cc.has_argument('-stdlib=libc++')
  add_global_arguments('-stdlib=libc++', language: 'cpp')
  add_global_link_arguments('-stdlib=libc++', language: 'cpp')
endif

# Hardening flags — enabled unconditionally for maximum safety during development.
# These can be relaxed for production release builds later.
hardening_args = []

# Stack buffer overflow detection.
if cc.has_argument('-fstack-protector-strong')
  hardening_args += '-fstack-protector-strong'
endif

# Fortified libc functions (buffer overflow checks in memcpy, strcpy, etc.).
# Requires optimisation (debugoptimized satisfies this).
if cc.has_argument('-D_FORTIFY_SOURCE=2')
  hardening_args += '-D_FORTIFY_SOURCE=2'
endif

# Standard library debug assertions: bounds checking on operator[], iterator validity.
# _GLIBCXX_ASSERTIONS for libstdc++, _LIBCPP_HARDENING_MODE for libc++.
if cc.get_id() == 'clang' and cc.has_argument('-stdlib=libc++')
  hardening_args += '-D_LIBCPP_HARDENING_MODE=_LIBCPP_HARDENING_MODE_EXTENSIVE'
else
  hardening_args += '-D_GLIBCXX_ASSERTIONS'
endif

# Catch implicit fallthrough in switch statements.
if cc.has_argument('-Wimplicit-fallthrough')
  hardening_args += '-Wimplicit-fallthrough'
endif

# Warn on conversions that may lose data.
if cc.has_argument('-Wconversion')
  hardening_args += '-Wconversion'
endif

# Warn on sign comparison mismatches.
if cc.has_argument('-Wsign-compare')
  hardening_args += '-Wsign-compare'
endif

# Warn if a variable-length array is used (not safe in C++).
if cc.has_argument('-Wvla')
  hardening_args += '-Wvla'
endif

# Warn on double promotions (float → double implicitly).
if cc.has_argument('-Wdouble-promotion')
  hardening_args += '-Wdouble-promotion'
endif

# Warn on missing field initialisers in aggregate init.
if cc.has_argument('-Wmissing-field-initializers')
  hardening_args += '-Wmissing-field-initializers'
endif

# Null dereference is UB — warn on code paths that assume it isn't.
if cc.has_argument('-Wnull-dereference')
  hardening_args += '-Wnull-dereference'
endif

# Format string security.
if cc.has_argument('-Wformat=2')
  hardening_args += '-Wformat=2'
endif

# Warn on unused results (supplements [[nodiscard]]).
if cc.has_argument('-Wunused-result')
  hardening_args += '-Wunused-result'
endif

add_project_arguments(hardening_args, language: 'cpp')

# Control Flow Integrity (CFI): validates indirect call targets match expected
# type signatures.  Catches vtable corruption, type confusion, and ROP gadgets.
# Requires clang + LTO + compiler-rt.
if get_option('cfi')
  cfi_args = ['-fsanitize=cfi', '-fvisibility=hidden']
  cfi_link_args = ['-fsanitize=cfi', '-flto']
  add_project_arguments(cfi_args, language: 'cpp')
  add_project_link_arguments(cfi_link_args, language: 'cpp')
endif

py = import('python').find_installation(pure: false)
pybind11_dep = dependency('pybind11', required: false)
lsp_framework_proj = subproject('lsp-framework')
lsp_framework_dep = lsp_framework_proj.get_variable('lsp_dep')

subdir('native')
