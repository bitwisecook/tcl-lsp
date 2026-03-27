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

# libstdc++ debug assertions: bounds checking on operator[], iterator validity, etc.
hardening_args += '-D_GLIBCXX_ASSERTIONS'

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

py = import('python').find_installation(pure: false)
pybind11_dep = dependency('pybind11')

subdir('native')
