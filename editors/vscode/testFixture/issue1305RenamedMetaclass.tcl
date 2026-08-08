# Fixture for issue #1305 — a `rename`d metaclass command manufactures
# nothing. Line numbers are load-bearing — the companion test
# (issue1305RenamedMetaclass.test.ts) asserts on them.
namespace eval ::R {}
oo::class create ::R::M { superclass oo::class }
rename ::R::M ::R::Mk
::R::Mk create ::R::W { method go {} { return went } }
set w [::R::W new]
puts [$w go]
