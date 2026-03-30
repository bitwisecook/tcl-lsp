#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/analysis/analysis_result.hpp"
#include "tcl_lsp/analysis/auxiliary_types.hpp"
#include "tcl_lsp/analysis/proc_arg_traits.hpp"
#include "tcl_lsp/analysis/semantic_types.hpp"
#include "tcl_lsp/registry/native_registry_adapter.hpp"

#include <pybind11/pybind11.h>
#include <pybind11/stl.h>

#include <memory>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <vector>

namespace py = pybind11;
using namespace tcl_lsp;

// Convert ProcArgTraits bitmask to a Python frozenset of trait names.
static auto traits_to_frozenset(ProcArgTraits traits) -> py::object {
    py::set result;
    if (has_trait(traits, ProcArgTrait::EVAL))
        result.add(py::str("EVAL"));
    if (has_trait(traits, ProcArgTrait::BODY))
        result.add(py::str("BODY"));
    if (has_trait(traits, ProcArgTrait::VAR_WRITE))
        result.add(py::str("VAR_WRITE"));
    if (has_trait(traits, ProcArgTrait::VAR_READ))
        result.add(py::str("VAR_READ"));
    if (has_trait(traits, ProcArgTrait::EXPR))
        result.add(py::str("EXPR"));
    if (has_trait(traits, ProcArgTrait::LOOP_LIST))
        result.add(py::str("LOOP_LIST"));
    return py::frozenset(result);
}

void register_analysis_bindings(py::module_& m) {
    // ScopeKind enum.
    py::enum_<ScopeKind>(m, "NativeScopeKind")
        .value("GLOBAL", ScopeKind::GLOBAL)
        .value("NAMESPACE", ScopeKind::NAMESPACE)
        .value("PROC", ScopeKind::PROC);

    // ProcArgTrait enum.
    py::enum_<ProcArgTrait>(m, "NativeProcArgTrait")
        .value("EVAL", ProcArgTrait::EVAL)
        .value("BODY", ProcArgTrait::BODY)
        .value("VAR_WRITE", ProcArgTrait::VAR_WRITE)
        .value("VAR_READ", ProcArgTrait::VAR_READ)
        .value("EXPR", ProcArgTrait::EXPR)
        .value("LOOP_LIST", ProcArgTrait::LOOP_LIST);

    // Confidence enum.
    py::enum_<Confidence>(m, "NativeConfidence")
        .value("DEFINITE", Confidence::DEFINITE)
        .value("PROBABLE", Confidence::PROBABLE)
        .value("UNKNOWN", Confidence::UNKNOWN);

    // ParamDef — proc parameter with optional default value.
    py::class_<ParamDef>(m, "NativeParamDef")
        .def_readonly("name", &ParamDef::name)
        .def_property_readonly("default_value",
                               [](const ParamDef& p) -> py::object {
                                   if (p.default_value.has_value())
                                       return py::str(*p.default_value);
                                   return py::none();
                               })
        .def("__eq__", [](const ParamDef& a, const ParamDef& b) { return a == b; })
        .def("__repr__", [](const ParamDef& p) {
            auto s = "NativeParamDef(name='" + p.name + "'";
            if (p.default_value)
                s += ", default='" + *p.default_value + "'";
            return s + ")";
        });

    // VarDef — variable definition with references.
    py::class_<VarDef>(m, "NativeVarDef")
        .def_readonly("name", &VarDef::name)
        .def_readonly("definition_range", &VarDef::definition_range)
        .def_readonly("references", &VarDef::references)
        .def_readonly("warn_if_unused", &VarDef::warn_if_unused)
        .def("__repr__", [](const VarDef& v) {
            return "NativeVarDef(name='" + v.name +
                   "', refs=" + std::to_string(v.references.size()) + ")";
        });

    // ProcDef — procedure definition.
    py::class_<ProcDef>(m, "NativeProcDef")
        .def_readonly("name", &ProcDef::name)
        .def_readonly("qualified_name", &ProcDef::qualified_name)
        .def_readonly("params", &ProcDef::params)
        .def_readonly("name_range", &ProcDef::name_range)
        .def_readonly("body_range", &ProcDef::body_range)
        .def_readonly("doc", &ProcDef::doc)
        .def_property_readonly("param_traits",
                               [](const ProcDef& p) {
                                   py::dict result;
                                   for (const auto& [name, traits] : p.param_traits) {
                                       result[py::str(name)] = traits_to_frozenset(traits);
                                   }
                                   return result;
                               })
        .def("__repr__", [](const ProcDef& p) {
            return "NativeProcDef(name='" + p.qualified_name +
                   "', params=" + std::to_string(p.params.size()) + ")";
        });

    // Scope — lexical scope (read-only view).
    py::class_<Scope>(m, "NativeScope")
        .def_property_readonly("kind", [](const Scope& s) { return s.kind; })
        .def_readonly("name", &Scope::name)
        .def_property_readonly("body_range",
                               [](const Scope& s) -> py::object {
                                   if (s.body_range.has_value())
                                       return py::cast(*s.body_range);
                                   return py::none();
                               })
        .def_property_readonly("variables",
                               [](const Scope& s) {
                                   py::dict result;
                                   for (const auto& [name, var] : s.variables) {
                                       result[py::str(name)] =
                                           py::cast(var, py::return_value_policy::reference);
                                   }
                                   return result;
                               })
        .def_property_readonly("procs",
                               [](const Scope& s) {
                                   py::dict result;
                                   for (const auto& [name, proc] : s.procs) {
                                       result[py::str(name)] =
                                           py::cast(proc, py::return_value_policy::reference);
                                   }
                                   return result;
                               })
        .def_property_readonly(
            "children",
            [](const Scope& s) {
                py::list result;
                for (const auto& child : s.children) {
                    result.append(py::cast(child.get(), py::return_value_policy::reference));
                }
                return result;
            })
        .def("__repr__", [](const Scope& s) {
            return "NativeScope(kind=" + to_string(s.kind) + ", name='" + s.name +
                   "', vars=" + std::to_string(s.variables.size()) +
                   ", procs=" + std::to_string(s.procs.size()) +
                   ", children=" + std::to_string(s.children.size()) + ")";
        });

    // RegexPattern.
    py::class_<RegexPattern>(m, "NativeRegexPattern")
        .def_readonly("range", &RegexPattern::range)
        .def_readonly("pattern", &RegexPattern::pattern)
        .def_readonly("command", &RegexPattern::command)
        .def("__eq__", [](const RegexPattern& a, const RegexPattern& b) { return a == b; })
        .def("__repr__", [](const RegexPattern& r) {
            return "NativeRegexPattern(cmd='" + r.command + "', pat='" + r.pattern + "')";
        });

    // CommandInvocation.
    py::class_<CommandInvocation>(m, "NativeCommandInvocation")
        .def_readonly("name", &CommandInvocation::name)
        .def_readonly("range", &CommandInvocation::range)
        .def_readonly("resolved_qualified_name", &CommandInvocation::resolved_qualified_name)
        .def("__eq__",
             [](const CommandInvocation& a, const CommandInvocation& b) { return a == b; });

    // PackageRequire.
    py::class_<PackageRequire>(m, "NativePackageRequire")
        .def_readonly("name", &PackageRequire::name)
        .def_readonly("version", &PackageRequire::version)
        .def_readonly("range", &PackageRequire::range)
        .def_readonly("conditional", &PackageRequire::conditional)
        .def("__eq__", [](const PackageRequire& a, const PackageRequire& b) { return a == b; });

    // PackageProvide.
    py::class_<PackageProvide>(m, "NativePackageProvide")
        .def_readonly("name", &PackageProvide::name)
        .def_readonly("version", &PackageProvide::version)
        .def_readonly("range", &PackageProvide::range)
        .def("__eq__", [](const PackageProvide& a, const PackageProvide& b) { return a == b; });

    // SourceTarget.
    py::class_<SourceTarget>(m, "NativeSourceTarget")
        .def_readonly("raw_path", &SourceTarget::raw_path)
        .def_readonly("range", &SourceTarget::range)
        .def_readonly("is_literal", &SourceTarget::is_literal)
        .def("__eq__", [](const SourceTarget& a, const SourceTarget& b) { return a == b; });

    // PackageContext.
    py::class_<PackageContext>(m, "NativePackageContext")
        .def_readonly("definite", &PackageContext::definite)
        .def_readonly("probable", &PackageContext::probable)
        .def_readonly("provided", &PackageContext::provided)
        .def_readonly("unknown_providers", &PackageContext::unknown_providers)
        .def("all_required", &PackageContext::all_required)
        .def("confidence", &PackageContext::confidence, py::arg("package"));

    // UnknownProcInfo.
    py::class_<UnknownProcInfo>(m, "NativeUnknownProcInfo")
        .def_readonly("dispatch_targets", &UnknownProcInfo::dispatch_targets)
        .def_readonly("chains_original", &UnknownProcInfo::chains_original)
        .def_readonly("empty_stub", &UnknownProcInfo::empty_stub)
        .def_readonly("case_insensitive", &UnknownProcInfo::case_insensitive)
        .def_readonly("has_pattern_dispatch", &UnknownProcInfo::has_pattern_dispatch)
        .def_readonly("has_exec", &UnknownProcInfo::has_exec)
        .def_readonly("has_auto_load", &UnknownProcInfo::has_auto_load)
        .def("__eq__", [](const UnknownProcInfo& a, const UnknownProcInfo& b) { return a == b; });

    // StubArgDef.
    py::class_<StubArgDef>(m, "NativeStubArgDef")
        .def_readonly("name", &StubArgDef::name)
        .def_readonly("role", &StubArgDef::role)
        .def_readonly("optional", &StubArgDef::optional)
        .def("__eq__", [](const StubArgDef& a, const StubArgDef& b) { return a == b; });

    // StubCommandDef.
    py::class_<StubCommandDef>(m, "NativeStubCommandDef")
        .def_readonly("name", &StubCommandDef::name)
        .def_readonly("args", &StubCommandDef::args)
        .def_readonly("range", &StubCommandDef::range)
        .def_readonly("barrier", &StubCommandDef::barrier)
        .def_readonly("loop", &StubCommandDef::loop)
        .def_readonly("pure", &StubCommandDef::pure)
        .def_readonly("mutator", &StubCommandDef::mutator)
        .def_readonly("unsafe", &StubCommandDef::unsafe)
        .def_readonly("scope_alias", &StubCommandDef::scope_alias)
        .def("__eq__", [](const StubCommandDef& a, const StubCommandDef& b) { return a == b; });

    // StubExprDef.
    py::class_<StubExprDef>(m, "NativeStubExprDef")
        .def_readonly("name", &StubExprDef::name)
        .def_readonly("kind", &StubExprDef::kind)
        .def_readonly("arity", &StubExprDef::arity)
        .def_readonly("pure", &StubExprDef::pure)
        .def_readonly("range", &StubExprDef::range)
        .def("__eq__", [](const StubExprDef& a, const StubExprDef& b) { return a == b; });

    // AnalysisResult — exposed via shared_ptr holder for move-only support.
    py::class_<AnalysisResult, std::shared_ptr<AnalysisResult>>(m, "NativeAnalysisResult")
        .def_property_readonly(
            "global_scope",
            [](const AnalysisResult& r) -> const Scope& { return r.global_scope(); },
            py::return_value_policy::reference_internal)
        .def_property_readonly("diagnostics",
                               [](const AnalysisResult& r) { return r.diagnostics(); })
        .def_property_readonly("all_procs",
                               [](const AnalysisResult& r) {
                                   py::dict result;
                                   for (const auto& [name, ptr] : r.all_procs()) {
                                       if (ptr) {
                                           result[py::str(name)] =
                                               py::cast(ptr, py::return_value_policy::reference);
                                       }
                                   }
                                   return result;
                               })
        .def_property_readonly("all_variables",
                               [](const AnalysisResult& r) {
                                   py::dict result;
                                   for (const auto& [name, ptr] : r.all_variables()) {
                                       if (ptr) {
                                           result[py::str(name)] =
                                               py::cast(ptr, py::return_value_policy::reference);
                                       }
                                   }
                                   return result;
                               })
        .def_property_readonly("regex_patterns",
                               [](const AnalysisResult& r) { return r.regex_patterns(); })
        .def_property_readonly("command_invocations",
                               [](const AnalysisResult& r) { return r.command_invocations(); })
        .def_property_readonly("package_requires",
                               [](const AnalysisResult& r) { return r.package_requires(); })
        .def_property_readonly("package_provides",
                               [](const AnalysisResult& r) { return r.package_provides(); })
        .def_property_readonly("has_dynamic_providers",
                               [](const AnalysisResult& r) { return r.has_dynamic_providers(); })
        .def_property_readonly("source_targets",
                               [](const AnalysisResult& r) { return r.source_targets(); })
        .def_property_readonly("stub_commands",
                               [](const AnalysisResult& r) { return r.stub_commands(); })
        .def_property_readonly("stub_expr_defs",
                               [](const AnalysisResult& r) { return r.stub_expr_defs(); })
        .def_property_readonly("command_aliases",
                               [](const AnalysisResult& r) { return r.command_aliases(); })
        .def_property_readonly("unknown_proc_info",
                               [](const AnalysisResult& r) -> py::object {
                                   const auto& info = r.unknown_proc_info();
                                   if (info.has_value())
                                       return py::cast(*info);
                                   return py::none();
                               })
        .def_property_readonly("package_context",
                               [](const AnalysisResult& r) { return r.package_context(); })
        .def(
            "find_proc",
            [](const AnalysisResult& r, const std::string& name) -> py::object {
                const auto* p = r.find_proc(name);
                if (p)
                    return py::cast(p, py::return_value_policy::reference);
                return py::none();
            },
            py::arg("name"))
        .def("active_package_names", &AnalysisResult::active_package_names)
        .def("__repr__", [](const AnalysisResult& r) {
            return "NativeAnalysisResult(diags=" + std::to_string(r.diagnostics().size()) +
                   ", procs=" + std::to_string(r.all_procs().size()) +
                   ", vars=" + std::to_string(r.all_variables().size()) + ")";
        });

    // analyse() — main entry point using native registry.
    m.def(
        "native_analyse",
        [](const std::string& source,
           py::object disabled_diags_obj) -> std::shared_ptr<AnalysisResult> {
            std::unordered_set<std::string> disabled;
            if (!disabled_diags_obj.is_none()) {
                auto seq = disabled_diags_obj.cast<py::sequence>();
                for (auto item : seq) {
                    disabled.insert(item.cast<std::string>());
                }
            }

            const auto& reg = native_registry();
            Analyser analyser(&reg, disabled);
            auto result = analyser.analyse(source);
            return std::make_shared<AnalysisResult>(std::move(result));
        },
        py::arg("source"),
        py::arg("disabled_diagnostics") = py::none(),
        "Analyse Tcl source using the native C++ analyser with native command registry.");

    // infer_param_traits_shallow — shallow proc parameter trait inference.
    m.def(
        "native_infer_param_traits_shallow",
        [](const std::vector<std::string>& params, const std::string& body) {
            auto result = infer_param_traits_shallow(params, body);
            py::dict py_result;
            for (const auto& [name, traits] : result) {
                py_result[py::str(name)] = traits_to_frozenset(traits);
            }
            return py_result;
        },
        py::arg("params"),
        py::arg("body_source"),
        "Infer parameter usage traits from a proc body (shallow, single-pass).");
}
