"""Registry-backed BIG-IP object kinds and property-reference metadata."""

from __future__ import annotations

from dataclasses import dataclass
from functools import lru_cache

from ..analysis.semantic_model import Range
from .model import BigipConfig
from .registry.data import (
    HEADER_KIND_MAP,
    OBJECT_KIND_SPECS,
    PROPERTY_REFERENCE_SPECS,
)
from .registry.models import BigipObjectKindSpec, BigipPropertySpec


@dataclass(slots=True)
class BigipObjectRegistry:
    """Lookup facade over BIG-IP object and reference metadata."""

    object_kind_specs: dict[str, BigipObjectKindSpec]
    property_reference_specs: dict[
        tuple[str, str],
        dict[str, tuple[BigipPropertySpec, ...]],
    ]
    header_kind_map: dict[tuple[str, str], str]

    def _property_specs_for_context(
        self,
        property_name: str,
        *,
        container_module: str | None,
        container_object_type: str | None,
    ) -> tuple[BigipPropertySpec, ...]:
        if container_module is None or container_object_type is None:
            return ()
        by_name = self.property_reference_specs.get((container_module, container_object_type), {})
        return by_name.get(property_name, ())

    @staticmethod
    def _kinds_from_properties(
        property_specs: tuple[BigipPropertySpec, ...],
        *,
        section: str | None,
    ) -> tuple[str, ...]:
        kinds: list[str] = []
        matching = [prop for prop in property_specs if prop.matches_section(section)]
        # Prioritise section-constrained properties over generic ones.
        matching.sort(key=lambda prop: 0 if prop.in_sections else 1)
        for prop in matching:
            for kind in prop.references:
                if kind not in kinds:
                    kinds.append(kind)
        return tuple(kinds)

    def candidate_kinds_for_key(
        self,
        key: str,
        *,
        section: str | None,
        container_module: str | None,
        container_object_type: str | None,
    ) -> tuple[str, ...]:
        property_specs = self._property_specs_for_context(
            key,
            container_module=container_module,
            container_object_type=container_object_type,
        )
        return self._kinds_from_properties(property_specs, section=section)

    def candidate_kinds_for_section_item(
        self,
        section: str,
        *,
        container_module: str | None,
        container_object_type: str | None,
    ) -> tuple[str, ...]:
        property_specs = self._property_specs_for_context(
            section,
            container_module=container_module,
            container_object_type=container_object_type,
        )
        return self._kinds_from_properties(property_specs, section=section)

    def kind_for_header(self, module: str, object_type: str) -> str | None:
        kind = self.header_kind_map.get((module, object_type))
        if kind is not None:
            return kind
        for spec in self.object_kind_specs.values():
            if spec.module == module and object_type in spec.object_types:
                return spec.kind
        return None

    def resolve_kind_in_configs(
        self,
        kind: str,
        ref: str,
        configs: dict[str, BigipConfig],
        *,
        preferred_module: str | None = None,
    ) -> tuple[str, Range] | None:
        """Resolve ``ref`` by *kind* across configs and return ``(uri, range)``."""
        if not ref:
            return None
        clean = ref.strip("{}\"'[]")
        if not clean:
            return None

        spec = self.object_kind_specs.get(kind)
        if spec is None:
            return None

        if spec.table_name is not None:
            for cfg_uri, cfg in configs.items():
                table = getattr(cfg, spec.table_name)

                resolved: str | None
                if clean in table:
                    resolved = clean
                elif kind in {"node", "monitor", "virtual"}:
                    resolved = cfg.resolve_name(clean, table)
                elif spec.resolver_name is not None:
                    resolver = getattr(cfg, spec.resolver_name)
                    resolved = resolver(clean)
                else:
                    resolved = None

                if resolved and resolved in table:
                    obj = table[resolved]
                    obj_module = getattr(obj, "module", None)
                    if spec.module is not None and obj_module is not None:
                        if obj_module != spec.module:
                            continue
                    if preferred_module is not None and obj_module is not None:
                        if obj_module != preferred_module:
                            continue
                    if obj.range is not None:
                        return (cfg_uri, obj.range)
            return None

        for cfg_uri, cfg in configs.items():
            module = spec.module
            if preferred_module is not None and module is None and kind == "pool":
                module = preferred_module
            key = cfg.resolve_generic_object(
                clean,
                module=module,
                object_types=spec.object_types,
            )
            if key is None:
                continue
            obj = cfg.generic_objects.get(key)
            if obj and obj.range is not None:
                return (cfg_uri, obj.range)
        return None


def build_bigip_object_registry() -> BigipObjectRegistry:
    """Build BIG-IP object registry from curated catalogue files."""
    return BigipObjectRegistry(
        object_kind_specs=dict(OBJECT_KIND_SPECS),
        property_reference_specs=dict(PROPERTY_REFERENCE_SPECS),
        header_kind_map=dict(HEADER_KIND_MAP),
    )


@lru_cache(maxsize=1)
def get_default_bigip_object_registry() -> BigipObjectRegistry:
    """Return default BIG-IP object registry (cached for process lifetime)."""
    return build_bigip_object_registry()


def candidate_kinds_for_key(
    key: str,
    *,
    section: str | None,
    container_module: str | None,
    container_object_type: str | None,
) -> tuple[str, ...]:
    return get_default_bigip_object_registry().candidate_kinds_for_key(
        key,
        section=section,
        container_module=container_module,
        container_object_type=container_object_type,
    )


def candidate_kinds_for_section_item(
    section: str,
    *,
    container_module: str | None,
    container_object_type: str | None,
) -> tuple[str, ...]:
    return get_default_bigip_object_registry().candidate_kinds_for_section_item(
        section,
        container_module=container_module,
        container_object_type=container_object_type,
    )


def kind_for_header(module: str, object_type: str) -> str | None:
    return get_default_bigip_object_registry().kind_for_header(module, object_type)


def resolve_kind_in_configs(
    kind: str,
    ref: str,
    configs: dict[str, BigipConfig],
    *,
    preferred_module: str | None = None,
) -> tuple[str, Range] | None:
    return get_default_bigip_object_registry().resolve_kind_in_configs(
        kind,
        ref,
        configs,
        preferred_module=preferred_module,
    )
