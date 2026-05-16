"""Per-package registry for BIG-IP object definitions.

Specs register at import time via :func:`register`.  After every
spec module has been imported, :func:`normalise_registry` runs a
curated post-processing pass — adding ``list_operators`` to
properties the manpage corpus mis-classifies (the ``ltm
virtual.rules`` / ``gtm wideip *.rules`` family), pinning
operator-prefixed edit verbs where the generator drops them, etc.

Keeping the normalisation here means a spec regeneration that
overwrites every ``specs/*.py`` file still produces a
device-valid registry: the post-process layer re-applies the
curated overrides regardless of what the generator emits.
"""

from __future__ import annotations

from dataclasses import replace as _dc_replace

from ..models import BigipObjectSpec, BigipPropertySpec

_REGISTRY: list[BigipObjectSpec] = []


def register(spec_factory):
    """Decorator that registers an object spec at import time."""
    _REGISTRY.append(spec_factory())
    return spec_factory


# (module, object_type, property_name) tuples whose ``list_operators``
# must include ``replace-all-with``.  Generated from the audit in
# ``docs/design/bigip-list-operator-audit.md``; every entry is a
# real TMSH list property whose full-body ``tmsh modify`` rejects
# emission without an operator.  Hand-curated until the spec
# generator gains a per-property override file.
_FORCE_REPLACE_ALL_WITH: frozenset[tuple[str, str, str]] = frozenset(
    {
        ("gtm", "listener", "rules"),
        ("gtm", "listener-doh-proxy", "rules"),
        ("gtm", "listener-doh-server", "rules"),
        ("gtm", "wideip a", "rules"),
        ("gtm", "wideip aaaa", "rules"),
        ("gtm", "wideip cname", "rules"),
        ("gtm", "wideip https", "rules"),
        ("gtm", "wideip mx", "rules"),
        ("gtm", "wideip naptr", "rules"),
        ("gtm", "wideip srv", "rules"),
        ("gtm", "wideip svcb", "rules"),
        ("ltm", "message-routing diameter route", "peers"),
        ("ltm", "message-routing diameter transport-config", "rules"),
        ("ltm", "message-routing generic route", "peers"),
        ("ltm", "message-routing generic router", "routes"),
        ("ltm", "message-routing generic transport-config", "rules"),
        ("ltm", "message-routing mqtt transport-config", "rules"),
        ("ltm", "message-routing sip route", "peers"),
        ("ltm", "message-routing sip transport-config", "rules"),
        ("ltm", "virtual", "rules"),
        ("sys", "cluster", "members"),
    }
)


# Properties the generator mis-classifies (e.g. as ``enum`` when
# they're really lists).  Each entry forces ``value_type="list"``
# and applies the default ``add``/``delete``/``replace-all-with``
# operator set so the renderer treats them correctly.
_FORCE_LIST_VALUE_TYPE: frozenset[tuple[str, str, str]] = frozenset(
    {
        # SNAT pool members: a list of translation IPs but the
        # manpage spec emits ``enum`` because of the ``default`` /
        # ``none`` operator keywords.
        ("ltm", "snatpool", "members"),
    }
)


def _normalise_property(
    module: str, object_type: str, prop: BigipPropertySpec
) -> BigipPropertySpec:
    """Return *prop* with curated overrides applied."""
    key = (module, object_type, prop.name)
    if key in _FORCE_LIST_VALUE_TYPE and prop.value_type != "list":
        prop = _dc_replace(
            prop,
            value_type="list",
            list_operators=frozenset({"add", "delete", "replace-all-with"}),
        )
        return prop
    if prop.value_type != "list":
        return prop
    if prop.list_operators:
        return prop
    if key in _FORCE_REPLACE_ALL_WITH:
        return _dc_replace(
            prop,
            list_operators=frozenset({"add", "delete", "replace-all-with"}),
        )
    return prop


def normalise_registry() -> None:
    """Walk every registered spec and apply curated property overrides.

    Idempotent: a property that already has ``list_operators`` set
    is left alone; only the curated gaps fire.  Runs after every
    spec module has been imported (via the import in
    :mod:`core.bigip.registry.specs.__init__`).
    """
    for idx, spec in enumerate(_REGISTRY):
        modules = {mod for mod, _ in spec.header_types}
        object_types = {obj for _, obj in spec.header_types}
        new_props: list[BigipPropertySpec] = []
        changed = False
        for prop in spec.properties:
            best: BigipPropertySpec = prop
            for module in modules:
                for object_type in object_types:
                    best = _normalise_property(module, object_type, best)
                    if best is not prop:
                        break
                if best is not prop:
                    break
            if best is not prop:
                changed = True
            new_props.append(best)
        if changed:
            _REGISTRY[idx] = _dc_replace(spec, properties=tuple(new_props))
