#!/usr/bin/env bash
# Replay the inline-test hoist on the current tree, then the three manual fixes.
set -eu
cd "$(git rev-parse --show-toplevel)"
source scripts/dev/agent-build-env.sh >/dev/null 2>&1 || true
python3 docs/work/file-split/tools/hoist_inline_tests.py . > /dev/null
cargo fmt --all
python3 - <<'PY'
import re
p='rust/tcl-lsp-server/src/tests.rs'
s=open(p).read()
a='[list source [file join $dir package.tcl]]\\n",),'
if a in s:
    s=s.replace(a,'[list source [file join $dir package.tcl]]\\n"),'); open(p,'w').write(s)
p='rust/tcl-compiler/src/side_effects/tests.rs'
s=open(p).read()
i=s.index("fn native_variable_scope_uses_geometry_and_actual_storage_domain()")
j=s.index("\n}\n", i)
body=s[i:j].replace("crate::var_resolve::VariableCellKey::Namespace","VariableCellKey::Namespace")
a=body.index("        if namespace == static_namespace {\n")
b=body.rindex("\n        }\n    }")
inner=body[a+len("        if namespace == static_namespace {\n"):b+1]
helper_body="\n".join(l[4:] if l.startswith("    ") else l for l in inner.split("\n")).replace("&registry","registry").replace("None, &context)","None, context)")
body=body[:a]+"        if namespace == static_namespace {\n            assert_static_policy_scope(&mut context, &namespace, &registry);\n        }\n    }"
t=s.rfind("#[test]",0,i)
helper=('/// The static storage policy applies to its own namespace incarnation only.\n'
        'fn assert_static_policy_scope(\n    context: &mut crate::var_resolve::ResolveContext,\n    namespace: &crate::command_binding::SourceNamespaceKey,\n    registry: &CommandRegistry,\n) {\n'+helper_body+"}\n\n")
s=s[:t]+helper+s[t:i]+body+s[j:]
k=s.index("use super::*;")
s=s[:k]+"use super::*;\nuse crate::var_resolve::VariableCellKey;"+s[k+len("use super::*;"):]
open(p,'w').write(s)
p='rust/tcl-compiler/src/analyser/interp_visibility/tests.rs'
s=open(p).read()
i=s.index("fn logical_lambda_namespace_retains_original_fields_and_scope_applicability()")
j=s.index("\n}\n", i)
body=s[i:j]
m=re.search(r"    fn retained\(.*?\n    \}\n", body, re.S)
nested=m.group(0)
body=body.replace(nested,"")
body=body.replace("crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(","OriginalRegistrySource::SourceTransitions(").replace("crate::command_binding::SourceCommandTransitionObligation::ExplicitLogicalSourceApplicability","SourceCommandTransitionObligation::ExplicitLogicalSourceApplicability")
t=s.rfind("#[test]",0,i)
lifted="\n".join(l[4:] if l.startswith("    ") else l for l in nested.split("\n"))
s=s[:t]+lifted+"\n"+s[t:i]+body+s[j:]
k=s.index("use super::*;")
s=s[:k]+"use super::*;\nuse crate::command_binding::SourceCommandTransitionObligation;\nuse crate::registry_invocation::source_structure::OriginalRegistrySource;"+s[k+len("use super::*;"):]
open(p,'w').write(s)
PY
cargo fmt --all
