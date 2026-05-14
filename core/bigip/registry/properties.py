"""New-shape property / object specs that own a :class:`ValueSpec`.

These are the Phase 1 scaffolding for the registry rearchitecture
laid out in ``docs/design/bigip-property-registry-rearchitecture.md``.
The old :class:`BigipObjectSpec` / :class:`BigipPropertySpec` shapes
in :mod:`.models` stay in place — every existing spec file under
``specs/`` still uses them — so Phase 1 introduces these new types
*alongside* the old ones rather than replacing them.

Migration is driven by individual spec files declaring a
:class:`PropertySpec` (new) for any property whose value spec wants
the structured behaviour; the registry's helper-loader assembles a
compatibility-shimmed :class:`BigipPropertySpec` from the new shape
so the existing parser / projection paths keep working unchanged.

Phase 2+ then move the parser, projection, edit planner, and graph
layers onto the new shape one stage at a time, eventually retiring
the compat shim.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, field

from .value_specs import ValueSpec


@dataclass(frozen=True, slots=True)
class ObjectKeySpec:
    """How an object kind names its instances.

    Most BIG-IP objects key off ``full_path``.  A few sub-objects
    (pool members, policy rules, attachment lists) are keyed by a
    composite, and ``ObjectKeySpec`` carries the shape so the
    registry can synthesise containers without each consumer
    re-deriving the key form.
    """

    name: str = "full_path"
    composite: tuple[str, ...] = ()

    @classmethod
    def full_path(cls) -> "ObjectKeySpec":
        return cls()


@dataclass(frozen=True, slots=True)
class PropertySpec:
    """A registry-owned property contract.

    The new shape carries:

    - ``attr`` — the dataclass attribute name on the model
      (snake_case; the TMSH key is typically ``attr.replace("_", "-")``
      but some properties spell differently and ``tmsh_name`` lets
      callers be explicit).
    - ``value`` — the :class:`ValueSpec` that handles parse /
      project / render / references for this property.  The single
      source of truth that replaces the old split between parser
      knowledge, projection knowledge, and edit knowledge.
    - ``writable`` — does an ``f5 query`` edit expression target
      this property?  Phase 3 wires the edit planner through this.
    - ``repeated`` / ``default`` / ``tmsh_name`` / ``docs`` — small
      metadata fields the docs generator and LSP completion will
      consume.
    """

    attr: str
    value: ValueSpec
    writable: bool = False
    repeated: bool = False
    default: object = ""
    tmsh_name: str = ""
    docs: str = ""

    @property
    def name(self) -> str:
        """The TMSH-spelt property name (with hyphens)."""
        return self.tmsh_name or self.attr.replace("_", "-")

    # Compatibility properties for the old projection layer ----------------
    #
    # The projection's ``FieldSpec`` historically exposed three flags
    # — ``typed`` / ``ref_kind`` / ``list_ref`` — that picked a
    # branch in ``_project_field``.  Phase 1 keeps those same
    # branches working by deriving the flags from the new value
    # spec.  Phase 2 replaces the branches with ``spec.value.project()``
    # and these helpers can retire.

    @property
    def typed(self) -> bool:
        return self.value.is_structured

    @property
    def ref_kind(self) -> str:
        from .value_specs import ObjectRefSpec

        if isinstance(self.value, ObjectRefSpec):
            return self.value.kind
        return ""

    @property
    def list_ref(self) -> bool:
        from .value_specs import ListSpec, ObjectRefSpec

        return isinstance(self.value, ListSpec) and isinstance(self.value.item, ObjectRefSpec)


@dataclass(frozen=True, slots=True)
class ObjectSpec:
    """The Phase 1 replacement for :class:`BigipObjectSpec`.

    Distinct class so the migration can run incrementally: spec
    files that have been migrated to the new shape live in
    ``specs/`` alongside the legacy ones, and a small loader bridge
    converts each to the legacy registry data structures so the rest
    of the codebase keeps working until Phase 2 turns it on.

    *kind* / *model* / *config_attr* match the existing
    :class:`BigipObjectKindSpec` fields; *key* spells out how
    instances are keyed; *properties* maps TMSH-spelt property names
    to :class:`PropertySpec` entries.
    """

    kind: str
    model: type
    config_attr: str
    key: ObjectKeySpec = field(default_factory=ObjectKeySpec.full_path)
    properties: Mapping[str, PropertySpec] = field(default_factory=dict)
    aliases: tuple[str, ...] = ()
    header_types: tuple[tuple[str, str], ...] = ()


# Public re-exports.
__all__ = [
    "ObjectKeySpec",
    "ObjectSpec",
    "PropertySpec",
]
