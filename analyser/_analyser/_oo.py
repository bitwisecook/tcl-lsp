# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from analyser.compiler_checks import iter_ir_statements
from compiler.ir import (
    IRBarrier,
    IRCall,
    IRSwitch,
)
from compiler.parsing.recovery import segment_with_recovery
from shared.naming import normalise_qualified_name
from shared.ranges import range_from_token
from shared.tokens import Token

from ..semantic_model import (
    ClassDef,
    MethodDef,
    ParamDef,
    PropertyDef,
    Range,
    Scope,
    UnknownProcInfo,
    VarDef,
)
from ._utils import parse_param_list

log = logging.getLogger(__name__)


def _class_ref_ranges(names: list[str], tokens: list[Token]) -> list[tuple[str, Range]]:
    """Pair each wanted class *name* with its token range (skips ``-flags``)."""
    wanted = set(names)
    return [(tok.text, range_from_token(tok)) for tok in tokens if tok.text in wanted]


class _AnalyserOOMixin(_Base):
    """TclOO class/method analysis."""

    if TYPE_CHECKING:
        # Class constants defined in _AnalyserProcMixin
        _OO_METACLASSES: frozenset[str]
        _OO_DEFINE_SUBCMDS: frozenset[str]

        # From _AnalyserBase (also declared here for clarity via inherited _Base)
        # From _AnalyserCommandsMixin
        def _analyse_body(self, *a: Any, **kw: Any) -> None: ...
        def _handle_proc_command(self, *a: Any, **kw: Any) -> bool: ...
        # From _AnalyserScopeMixin
        def _namespace_from_scope(self, *a: Any, **kw: Any) -> str: ...

    def _qualify_oo_name(self, name: str, scope: Scope) -> str:
        """Fully-qualified name for an OO/snit class/type definition.

        Mirrors tclsh 9.0.3 ``create`` semantics: an *absolute* name (``::Foo``)
        lives at the global root regardless of the enclosing namespace, while a
        *relative* name resolves against the current namespace (``::ns::Foo``
        inside ``namespace eval ns``, recursively for nested evals).
        ``normalise_qualified_name`` collapses any doubled ``::`` (so the
        top-level ``::`` joined with ``Baz`` yields ``::Baz``, not ``::::Baz``).
        """
        if name.startswith("::"):
            return normalise_qualified_name(name)
        ns = self._namespace_from_scope(scope)
        return normalise_qualified_name(f"{ns}::{name}")

    def _handle_oo_class_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> bool:
        """Handle ``oo::class create Name { body }`` and similar metaclass commands."""
        if cmd_name not in self._OO_METACLASSES:
            return False
        # Expect: create Name ?body?  OR  new ?body?
        if len(args) < 2:
            return False
        subcommand = args[0]
        if subcommand not in ("create", "new", "createWithNamespace"):
            return False
        if subcommand == "create" and len(args) < 2:
            return False

        class_name = args[1]
        body = args[2] if len(args) > 2 else ""
        name_range = range_from_token(arg_tokens[1]) if len(arg_tokens) > 1 else Range.zero()
        body_range = range_from_token(arg_tokens[2]) if len(arg_tokens) > 2 else name_range

        # Determine qualified name (absolute names ignore the caller scope;
        # relative names resolve against the enclosing namespace).
        qualified = self._qualify_oo_name(class_name, scope)

        preceding_doc = self._last_comment
        self._last_comment = ""

        class_def = ClassDef(
            name=class_name,
            qualified_name=qualified,
            name_range=name_range,
            body_range=body_range,
            metaclass=cmd_name,
            doc=preceding_doc,
        )

        # Register the class before analysing its body so self-references — a
        # method calling a sibling via ``my`` (#957), or ``set x [Class new]``
        # inside the class — resolve against the class being defined.  The
        # ``ClassDef`` is mutable; ``_parse_oo_definition_body`` adds methods to
        # the same object the map already holds.
        scope.classes[class_name] = class_def
        self.result.all_classes[qualified] = class_def

        if body:
            self._parse_oo_definition_body(
                body, arg_tokens[2] if len(arg_tokens) > 2 else None, class_def, scope
            )

        return True

    def _handle_oo_define_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> bool:
        """Handle ``oo::define ClassName { body }`` and inline forms."""
        if cmd_name != "oo::define":
            return False
        if len(args) < 2:
            return False

        class_name = args[0]
        # Determine qualified name (absolute names ignore the caller scope;
        # relative names resolve against the enclosing namespace).
        qualified = self._qualify_oo_name(class_name, scope)

        # Look up or create partial ClassDef
        class_def = self.result.all_classes.get(qualified)
        if class_def is None:
            name_range = range_from_token(arg_tokens[0]) if arg_tokens else Range.zero()
            class_def = ClassDef(
                name=class_name,
                qualified_name=qualified,
                name_range=name_range,
                body_range=name_range,
            )
            scope.classes[class_name] = class_def
            self.result.all_classes[qualified] = class_def

        # Distinguish body form from inline form.
        # Inline: oo::define ClassName method name args body
        # Body:   oo::define ClassName { method name args body; ... }
        if len(args) >= 2:
            if args[1] in self._OO_DEFINE_SUBCMDS:
                # Inline form
                self._parse_oo_define_inline(
                    args[1:], arg_tokens[1:] if len(arg_tokens) > 1 else [], class_def, scope
                )
            else:
                # Body form
                body_tok = arg_tokens[1] if len(arg_tokens) > 1 else None
                self._parse_oo_definition_body(args[1], body_tok, class_def, scope)
        return True

    def _parse_oo_definition_body(
        self,
        body: str,
        body_token: Token | None,
        class_def: ClassDef,
        scope: Scope,
    ) -> None:
        """Parse the body of an oo::class create or oo::define block."""
        commands, _ = segment_with_recovery(body, body_token)
        for cmd in commands:
            if not cmd.texts:
                continue
            subcmd = cmd.texts[0]
            sub_args = cmd.texts[1:]
            sub_tokens = cmd.argv[1:] if len(cmd.argv) > 1 else []

            match subcmd:
                case "superclass":
                    class_def.superclasses = list(sub_args)
                    class_def.superclass_refs.extend(_class_ref_ranges(sub_args, sub_tokens))
                case "mixin":
                    # Skip -append flag if present
                    mixins = [a for a in sub_args if not a.startswith("-")]
                    class_def.mixins = mixins
                    class_def.mixin_refs.extend(_class_ref_ranges(mixins, sub_tokens))
                case "variable":
                    class_def.variables = list(sub_args)
                case "method":
                    self._extract_method_def(sub_args, sub_tokens, class_def, scope, kind="method")
                case "classmethod":
                    self._extract_method_def(
                        sub_args, sub_tokens, class_def, scope, kind="classmethod"
                    )
                case "constructor":
                    self._extract_method_def(
                        sub_args,
                        sub_tokens,
                        class_def,
                        scope,
                        kind="constructor",
                        synthetic_name="<constructor>",
                    )
                case "destructor":
                    self._extract_method_def(
                        sub_args,
                        sub_tokens,
                        class_def,
                        scope,
                        kind="destructor",
                        synthetic_name="<destructor>",
                    )
                case "forward":
                    if sub_args:
                        fwd_name = sub_args[0]
                        fwd_range = range_from_token(sub_tokens[0]) if sub_tokens else Range.zero()
                        fwd_def = MethodDef(
                            name=fwd_name,
                            params=(),
                            name_range=fwd_range,
                            body_range=fwd_range,
                            kind="forward",
                        )
                        class_def.methods[fwd_name] = fwd_def
                case "filter":
                    class_def.filters = list(sub_args)
                case "export":
                    class_def.exports.update(sub_args)
                case "unexport":
                    class_def.unexports.update(sub_args)
                case "property":
                    self._extract_property_defs(sub_args, sub_tokens, class_def, scope)
                case "private":
                    # private wraps another definition subcommand
                    if sub_args and sub_tokens:
                        inner_subcmd = sub_args[0]
                        inner_args = sub_args[1:]
                        inner_tokens = sub_tokens[1:]
                        if inner_subcmd in ("method", "classmethod"):
                            self._extract_method_def(
                                inner_args,
                                inner_tokens,
                                class_def,
                                scope,
                                kind=inner_subcmd,
                                visibility="private",
                            )
                case "initialise" | "initialize":
                    # initialise { body } — class-level initialisation script
                    if sub_args:
                        init_tok = sub_tokens[0] if sub_tokens else None
                        self._analyse_body(sub_args[0], scope, body_token=init_tok)

            # Also recurse into method/constructor/destructor bodies for
            # variable tracking (handled inside _extract_method_def)

    def _parse_oo_define_inline(
        self,
        args: list[str],
        arg_tokens: list[Token],
        class_def: ClassDef,
        scope: Scope,
    ) -> None:
        """Handle inline oo::define form: ``oo::define Class method name args body``."""
        if not args:
            return
        subcmd = args[0]
        sub_args = args[1:]
        sub_tokens = arg_tokens[1:] if len(arg_tokens) > 1 else []

        match subcmd:
            case "method" | "classmethod":
                self._extract_method_def(sub_args, sub_tokens, class_def, scope, kind=subcmd)
            case "superclass":
                class_def.superclasses = list(sub_args)
                class_def.superclass_refs.extend(_class_ref_ranges(sub_args, sub_tokens))
            case "mixin":
                mixins = [a for a in sub_args if not a.startswith("-")]
                class_def.mixins = mixins
                class_def.mixin_refs.extend(_class_ref_ranges(mixins, sub_tokens))
            case "variable":
                class_def.variables = list(sub_args)
            case "constructor":
                self._extract_method_def(
                    sub_args,
                    sub_tokens,
                    class_def,
                    scope,
                    kind="constructor",
                    synthetic_name="<constructor>",
                )
            case "destructor":
                self._extract_method_def(
                    sub_args,
                    sub_tokens,
                    class_def,
                    scope,
                    kind="destructor",
                    synthetic_name="<destructor>",
                )
            case "filter":
                class_def.filters = list(sub_args)
            case "export":
                class_def.exports.update(sub_args)
            case "unexport":
                class_def.unexports.update(sub_args)
            case "property":
                self._extract_property_defs(sub_args, sub_tokens, class_def, scope)

    def _extract_method_def(
        self,
        args: list[str],
        arg_tokens: list[Token],
        class_def: ClassDef,
        scope: Scope,
        *,
        kind: str = "method",
        visibility: str = "public",
        synthetic_name: str = "",
    ) -> None:
        """Extract a method/constructor/destructor definition from definition args."""
        if kind in ("constructor", "destructor"):
            # constructor argList body  /  destructor body
            if kind == "constructor" and len(args) >= 2:
                param_str = args[0]
                body = args[1]
                name_range = Range.zero()
                body_range = (
                    range_from_token(arg_tokens[1]) if len(arg_tokens) > 1 else Range.zero()
                )
                params = parse_param_list(param_str)
            elif kind == "destructor" and len(args) >= 1:
                param_str = ""
                body = args[0]
                name_range = Range.zero()
                body_range = range_from_token(arg_tokens[0]) if arg_tokens else Range.zero()
                params = []
            else:
                return
            method_name = synthetic_name or kind
        else:
            # method name argList body  /  classmethod name argList body
            if len(args) < 3:
                return
            method_name = args[0]
            param_str = args[1]
            body = args[2]
            name_range = range_from_token(arg_tokens[0]) if arg_tokens else Range.zero()
            body_range = range_from_token(arg_tokens[2]) if len(arg_tokens) > 2 else Range.zero()
            params = parse_param_list(param_str)

        method_def = MethodDef(
            name=method_name,
            params=tuple(params),
            name_range=name_range,
            body_range=body_range,
            visibility=visibility,
            kind=kind,
        )

        match kind:
            case "constructor":
                class_def.constructors.append(method_def)
            case "destructor":
                class_def.destructor = method_def
            case "classmethod":
                class_def.class_methods[method_name] = method_def
            case _:
                class_def.methods[method_name] = method_def

        # Recurse into the method body for variable analysis
        method_scope = Scope(
            kind="method",
            name=f"{class_def.name}::{method_name}",
            parent=scope,
            body_range=body_range,
        )
        scope.children.append(method_scope)

        for p in params:
            method_scope.variables[p.name] = VarDef(
                name=p.name,
                definition_range=name_range,
                warn_if_unused=False,
            )

        # Instance variables declared via class-level 'variable' are
        # available in all methods
        for var_name in class_def.variables:
            if var_name not in method_scope.variables:
                method_scope.variables[var_name] = VarDef(
                    name=var_name,
                    definition_range=class_def.name_range,
                    warn_if_unused=False,
                )

        saved_comment = self._last_comment
        body_tok = (
            arg_tokens[2]
            if len(arg_tokens) > 2 and kind not in ("constructor", "destructor")
            else None
        )
        if kind == "constructor" and len(arg_tokens) > 1:
            body_tok = arg_tokens[1]
        elif kind == "destructor" and arg_tokens:
            body_tok = arg_tokens[0]
        self._analyse_body(body, method_scope, body_token=body_tok)
        self._last_comment = saved_comment

    def _extract_property_defs(
        self,
        args: list[str],
        arg_tokens: list[Token],
        class_def: ClassDef,
        scope: Scope | None = None,
    ) -> None:
        """Extract property definitions from a ``property`` subcommand.

        The property command accepts: ``property name ?name ...? ?-get body?
        ?-set body? ?-kind readable|readwrite|writable?``.  All three options
        take a value; there are no flag-only options.
        """
        # Collect property names first, then apply trailing options.
        names: list[tuple[str, int]] = []
        options: dict[str, str] = {}
        option_tokens: dict[str, Token | None] = {}
        i = 0
        while i < len(args):
            arg = args[i]
            if arg.startswith("-"):
                # All property options (-get, -set, -kind) take a value.
                if i + 1 < len(args):
                    options[arg] = args[i + 1]
                    option_tokens[arg] = arg_tokens[i + 1] if i + 1 < len(arg_tokens) else None
                i += 2
                continue
            names.append((arg, i))
            i += 1

        kind = options.get("-kind", "readwrite")
        has_getter = "-get" in options
        has_setter = "-set" in options
        for name, idx in names:
            prop_range = (
                range_from_token(arg_tokens[idx]) if idx < len(arg_tokens) else Range.zero()
            )
            class_def.properties[name] = PropertyDef(
                name=name,
                name_range=prop_range,
                kind=kind,
                has_getter=has_getter,
                has_setter=has_setter,
            )
        # Recurse into accessor bodies.
        if scope is not None:
            for opt in ("-set", "-get"):
                if opt in options:
                    body = options[opt]
                    body_tok = option_tokens.get(opt)
                    prop_scope = Scope(
                        kind="accessor",
                        name=f"{class_def.name}::<{opt[1:]}>",
                        parent=scope,
                    )
                    scope.children.append(prop_scope)
                    for var_name in class_def.variables:
                        if var_name not in prop_scope.variables:
                            prop_scope.variables[var_name] = VarDef(
                                name=var_name,
                                definition_range=class_def.name_range,
                                warn_if_unused=False,
                            )
                    self._analyse_body(body, prop_scope, body_token=body_tok)

    # ------------------------------------------------------------------
    # snit (tcllib) type/widget definitions
    # ------------------------------------------------------------------

    # ``snit::type``/``snit::widget``/``snit::widgetadaptor`` (and their
    # ``::``-qualified forms) reinterpret their body as a class description,
    # exactly like an ``oo::class`` body.  Modelling them as real ClassDefs
    # with method scopes means object dispatch inside method bodies
    # (``$self foo``, ``$component bar``) is recognised as in-method dispatch
    # (no false W307) and snit's implicit instance variables don't surface as
    # read-before-set / unused.
    _SNIT_DEFINERS = frozenset(
        {
            "snit::type",
            "snit::widget",
            "snit::widgetadaptor",
            "::snit::type",
            "::snit::widget",
            "::snit::widgetadaptor",
        }
    )

    def _handle_snit_type_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> bool:
        """Handle ``snit::type Name { body }`` (and widget/widgetadaptor)."""
        if cmd_name not in self._SNIT_DEFINERS:
            return False
        if len(args) < 2:
            return False
        type_name = args[0]
        body = args[1]
        name_range = range_from_token(arg_tokens[0]) if arg_tokens else Range.zero()
        body_range = range_from_token(arg_tokens[1]) if len(arg_tokens) > 1 else name_range
        is_widget = cmd_name.endswith(("widget", "widgetadaptor"))

        simple = type_name.rsplit("::", 1)[-1]
        # Absolute names ignore the caller scope; relative names resolve against
        # the enclosing namespace (handles nested ``namespace eval``, which the
        # former ``scope.name`` concat got wrong for depth > 1).
        qualified = self._qualify_oo_name(type_name, scope)

        preceding_doc = self._last_comment
        self._last_comment = ""

        class_def = ClassDef(
            name=simple,
            qualified_name=qualified,
            name_range=name_range,
            body_range=body_range,
            metaclass=cmd_name,
            doc=preceding_doc,
        )

        if body:
            self._parse_snit_definition_body(
                body,
                arg_tokens[1] if len(arg_tokens) > 1 else None,
                class_def,
                scope,
                is_widget=is_widget,
            )

        scope.classes[simple] = class_def
        self.result.all_classes[qualified] = class_def
        return True

    # snit method bodies have these implicit variables (snit injects them).
    _SNIT_INSTANCE_IMPLICIT = ("self", "selfns", "type", "options")
    _SNIT_TYPE_IMPLICIT = ("type",)

    def _parse_snit_definition_body(
        self,
        body: str,
        body_token: Token | None,
        class_def: ClassDef,
        scope: Scope,
        *,
        is_widget: bool,
    ) -> None:
        """Parse a snit type/widget body into methods + variable declarations.

        Two passes: first collect instance/type variable + component + option
        names (snit lets a method reference any of them regardless of
        declaration order), then analyse method/constructor/etc. bodies in
        proper method scopes seeded with those names plus snit's implicit
        variables.
        """
        commands, _ = segment_with_recovery(body, body_token)

        instance_vars: list[str] = list(self._SNIT_INSTANCE_IMPLICIT)
        if is_widget:
            instance_vars.append("win")
            instance_vars.append("hull")
        type_vars: list[str] = list(self._SNIT_TYPE_IMPLICIT)

        # First pass: collect declared names.
        for cmd in commands:
            if not cmd.texts:
                continue
            subcmd = cmd.texts[0]
            sub_args = cmd.texts[1:]
            match subcmd:
                case "variable":
                    if sub_args:
                        instance_vars.append(sub_args[0])
                case "typevariable":
                    if sub_args:
                        type_vars.append(sub_args[0])
                case "component":
                    if sub_args:
                        instance_vars.append(sub_args[0])
                case "typecomponent":
                    if sub_args:
                        type_vars.append(sub_args[0])

        # Record instance variables AND type variables on the class so
        # method-scope seeding, any future hover/refs work, and the W307
        # suppression in ``_diag_var_command.py`` (which treats class
        # variables as legitimate dispatch sources inside method bodies)
        # can see them.  Type variables are similarly long-lived /
        # externally-bound and must not blanket-fire W307 on
        # ``$typevar method`` dispatch from inside a typemethod.
        # (PR #498/#499 follow-up F4 closure -- the dispatch suppression
        # used to come from a method-body blanket; now it requires
        # positive evidence, of which class-var membership is one.)
        explicit_instance = [v for v in instance_vars if v not in self._SNIT_INSTANCE_IMPLICIT]
        explicit_type = [v for v in type_vars if v not in self._SNIT_TYPE_IMPLICIT]
        class_def.variables = explicit_instance + explicit_type

        # Second pass: analyse method-bearing declarations.
        for cmd in commands:
            if not cmd.texts:
                continue
            subcmd = cmd.texts[0]
            sub_args = cmd.texts[1:]
            sub_tokens = cmd.argv[1:] if len(cmd.argv) > 1 else []

            match subcmd:
                case "proc":
                    # snit allows type-private ``proc name args body`` inside
                    # the body; analyse it as a normal proc in the enclosing
                    # scope (matching the pre-model generic-recursion path).
                    self._handle_proc_command(subcmd, sub_args, sub_tokens, scope)
                case "method":
                    self._extract_snit_method(
                        sub_args, sub_tokens, class_def, scope, instance_vars, kind="method"
                    )
                case "typemethod":
                    self._extract_snit_method(
                        sub_args,
                        sub_tokens,
                        class_def,
                        scope,
                        type_vars,
                        kind="classmethod",
                    )
                case "constructor":
                    self._extract_snit_method(
                        sub_args,
                        sub_tokens,
                        class_def,
                        scope,
                        instance_vars,
                        kind="constructor",
                        no_arglist=False,
                        synthetic_name="<constructor>",
                    )
                case "destructor":
                    self._extract_snit_method(
                        sub_args,
                        sub_tokens,
                        class_def,
                        scope,
                        instance_vars,
                        kind="destructor",
                        no_arglist=True,
                        synthetic_name="<destructor>",
                    )
                case "typeconstructor":
                    self._extract_snit_method(
                        sub_args,
                        sub_tokens,
                        class_def,
                        scope,
                        type_vars,
                        kind="classmethod",
                        no_arglist=True,
                        synthetic_name="<typeconstructor>",
                    )
                case "onconfigure":
                    # ``onconfigure -opt valuevar { body }`` (snit 1.x).
                    self._extract_snit_method(
                        sub_args[1:],
                        sub_tokens[1:],
                        class_def,
                        scope,
                        instance_vars,
                        kind="method",
                        synthetic_name=f"<onconfigure {sub_args[0]}>" if sub_args else "",
                    )
                case "oncget":
                    # ``oncget -opt { body }`` (snit 1.x) — no value arg.
                    self._extract_snit_method(
                        sub_args[1:],
                        sub_tokens[1:],
                        class_def,
                        scope,
                        instance_vars,
                        kind="method",
                        no_arglist=True,
                        synthetic_name=f"<oncget {sub_args[0]}>" if sub_args else "",
                    )

    def _extract_snit_method(
        self,
        args: list[str],
        arg_tokens: list[Token],
        class_def: ClassDef,
        scope: Scope,
        seed_vars: list[str],
        *,
        kind: str = "method",
        no_arglist: bool = False,
        synthetic_name: str = "",
    ) -> None:
        """Analyse one snit method/constructor/etc. body in a method scope.

        *seed_vars* are pre-seeded into the scope as never-warn variables
        (snit's implicit ``self``/``type``/``options`` plus declared instance
        or type variables) so they never surface as read-before-set/unused.
        *no_arglist* is for declarations whose body immediately follows the
        keyword (``destructor``/``typeconstructor``/``oncget``).
        """
        if no_arglist:
            if not args:
                return
            method_name = synthetic_name or "<body>"
            params: list[ParamDef] = []
            body = args[0]
            name_range = range_from_token(arg_tokens[0]) if arg_tokens else Range.zero()
            body_range = name_range
            body_tok = arg_tokens[0] if arg_tokens else None
        else:
            if synthetic_name:
                # constructor/onconfigure: ``ARGLIST BODY`` (name is synthetic).
                if len(args) < 2:
                    return
                method_name = synthetic_name
                params = parse_param_list(args[0])
                body = args[1]
                name_range = range_from_token(arg_tokens[0]) if arg_tokens else Range.zero()
                body_range = range_from_token(arg_tokens[1]) if len(arg_tokens) > 1 else name_range
                body_tok = arg_tokens[1] if len(arg_tokens) > 1 else None
            else:
                # method/typemethod: ``NAME ARGLIST BODY``.
                if len(args) < 3:
                    return
                method_name = args[0]
                params = parse_param_list(args[1])
                body = args[2]
                name_range = range_from_token(arg_tokens[0]) if arg_tokens else Range.zero()
                body_range = range_from_token(arg_tokens[2]) if len(arg_tokens) > 2 else name_range
                body_tok = arg_tokens[2] if len(arg_tokens) > 2 else None

        method_def = MethodDef(
            name=method_name,
            params=tuple(params),
            name_range=name_range,
            body_range=body_range,
            kind=kind,
        )
        if kind == "constructor":
            class_def.constructors.append(method_def)
        elif kind == "destructor":
            class_def.destructor = method_def
        elif kind == "classmethod":
            class_def.class_methods[method_name] = method_def
        else:
            class_def.methods[method_name] = method_def

        method_scope = Scope(
            kind="method",
            name=f"{class_def.name}::{method_name}",
            parent=scope,
            body_range=body_range,
        )
        scope.children.append(method_scope)

        for p in params:
            method_scope.variables[p.name] = VarDef(
                name=p.name,
                definition_range=name_range,
                warn_if_unused=False,
            )
        for var_name in seed_vars:
            if var_name not in method_scope.variables:
                method_scope.variables[var_name] = VarDef(
                    name=var_name,
                    definition_range=class_def.name_range,
                    warn_if_unused=False,
                )

        saved_comment = self._last_comment
        self._analyse_body(body, method_scope, body_token=body_tok)
        self._last_comment = saved_comment

    # ------------------------------------------------------------------
    # Unknown proc body analysis
    # ------------------------------------------------------------------

    _CHAIN_TARGETS = frozenset(
        {
            "_original_unknown",
            "_orig_unknown",
            "::tcl::unknown",
            "tcl::unknown",
            "original_unknown",
        }
    )

    def _extract_unknown_proc_info(
        self,
        body: str,
        params: list[ParamDef],
    ) -> None:
        """Analyse the body of a user-defined ``unknown`` proc.

        Extracts dispatch targets (switch arm labels), detects chaining
        to the original handler, auto_load, exec, and case-insensitive
        dispatch patterns.  The result is stored as
        ``self.result.unknown_proc_info``.
        """
        if not body or not body.strip():
            self.result.unknown_proc_info = UnknownProcInfo(empty_stub=True)
            return

        first_param = params[0].name if params else "cmd"

        try:
            from compiler.lowering import lower_to_ir

            ir_module = lower_to_ir(body)
        except Exception:
            # If lowering fails, be conservative — assume unknown can
            # resolve anything. Treat the handler as fully dynamic/opaque
            # so downstream diagnostics (e.g. W123) are suppressed.
            log.exception(
                "Failed to lower 'unknown' proc body to IR; assuming fully dynamic dispatch"
            )
            self.result.unknown_proc_info = UnknownProcInfo(
                dispatch_targets=frozenset(),
                chains_original=True,
                empty_stub=False,
                case_insensitive=True,
                has_pattern_dispatch=True,
                has_exec=True,
                has_auto_load=True,
            )
            return

        dispatch_targets: set[str] = set()
        chains_original = False
        has_exec = False
        has_auto_load = False
        case_insensitive = False
        has_pattern_dispatch = False

        for stmt in iter_ir_statements(ir_module.top_level):
            if isinstance(stmt, IRSwitch):
                subj = stmt.subject
                if f"${first_param}" in subj or "${" + first_param + "}" in subj:
                    if "string tolower" in subj or "string toupper" in subj:
                        case_insensitive = True
                    if stmt.mode == "exact":
                        for arm in stmt.arms:
                            if arm.pattern != "default":
                                dispatch_targets.add(arm.pattern)
                    else:
                        has_pattern_dispatch = True

            elif isinstance(stmt, IRCall):
                cmd = stmt.command
                if cmd in self._CHAIN_TARGETS:
                    chains_original = True
                elif cmd == "exec":
                    has_exec = True
                elif cmd == "auto_load":
                    has_auto_load = True

            elif isinstance(stmt, IRBarrier):
                # ``exec $cmd {*}$args`` is lowered to IRBarrier (because
                # argument expansion defeats specialised lowering), so
                # recognise exec / auto_load / chain targets here too.
                cmd = stmt.command
                if cmd in self._CHAIN_TARGETS:
                    chains_original = True
                elif cmd == "exec":
                    has_exec = True
                elif cmd == "auto_load":
                    has_auto_load = True

        self.result.unknown_proc_info = UnknownProcInfo(
            dispatch_targets=frozenset(dispatch_targets),
            chains_original=chains_original,
            empty_stub=False,
            case_insensitive=case_insensitive,
            has_pattern_dispatch=has_pattern_dispatch,
            has_exec=has_exec,
            has_auto_load=has_auto_load,
        )

    # ------------------------------------------------------------------
    # W123 — unresolved command (post-analysis pass)
    # ------------------------------------------------------------------
