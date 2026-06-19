"""Canonical (de)serialisation bridge for the Rust BIG-IP parser port.

The Rust ``tcl-bigip`` crate emits a parsed ``BigipConfig`` as a canonical
JSON document; :func:`rebuild` turns that document back into the existing
Python dataclasses so the ``f5`` command (and every other consumer) keeps
its typed-attribute contract. :func:`canonical` is the inverse — it
serialises a Python ``BigipConfig`` into the same shape, used both to
pin the schema and to self-test the round-trip (``rebuild(canonical(c))
== c``) before the Rust side is wired in.

Canonical value tags (compact, JSON-friendly):

- ``None``/``bool``/``int``/``str`` — encoded verbatim.
- ``{"r": [sl, sc, so, el, ec, eo]}`` — a :class:`Range`.
- ``{"e": "NAME"}`` — an enum member, by name.
- ``{"s": "..."}`` — a typed scalar value (``Address`` / ``Destination``
  / ``Network`` / …), by its canonical ``str()``.
- ``{"t": [...]}`` — a tuple/list of canonical values.
- ``{"m": {k: ...}}`` — a string-keyed dict of canonical values.
- ``{"d": {field: ...}}`` — a nested dataclass.
- ``{"L": {"syntax", "operator", "items": [{"v", "k", "b"}]}}`` — a
  :class:`BigipList`.
"""

from __future__ import annotations

import dataclasses as dc
import typing
from enum import Enum
from typing import Any

from dialects.f5.bigip import types as bigip_types
from dialects.f5.bigip.model import BigipConfig
from dialects.f5.bigip.types import (
    FQDN,
    BigipList,
    Destination,
    IPAddress,
    IPRange,
    ListItem,
    Network,
    ObjectPath,
    Partition,
    Port,
    PortRange,
    PortSet,
    RouteDomain,
    parse_address,
)
from dialects.f5.bigip.types._bigip_list import SourceSpan
from dialects.f5.bigip.types._folder import Folder
from shared.diagnostic import Range
from shared.tokens import SourcePosition


def _span(s: SourceSpan | None) -> Any:
    return None if s is None else {"S": [s.start, s.end]}


def _unspan(v: Any) -> SourceSpan | None:
    return None if v is None else SourceSpan(start=v["S"][0], end=v["S"][1])


# Scalar value types that round-trip cleanly through ``str()`` / a
# dedicated parser — serialised as ``{"s": ...}`` and rebuilt by parser.
_SCALAR_VALUE_TYPES = (
    IPAddress,
    FQDN,
    Network,
    Destination,
    Port,
    PortRange,
    Partition,
    RouteDomain,
    Folder,
    ObjectPath,
    IPRange,
    PortSet,
)


def _canonical(value: Any) -> Any:
    if value is None or isinstance(value, (bool, int, str)):
        return value
    if isinstance(value, Range):
        s, e = value.start, value.end
        return {"r": [s.line, s.character, s.offset, e.line, e.character, e.offset]}
    if isinstance(value, Enum):
        return {"e": value.name}
    if isinstance(value, _SCALAR_VALUE_TYPES):
        return {"s": str(value)}
    if isinstance(value, BigipList):
        return {
            "L": {
                "syntax": value.syntax,
                "operator": value.operator,
                "raw": value.raw,
                "range": _span(value.range),
                "items": [
                    {
                        "v": _canonical(it.value),
                        "k": it.key,
                        "b": it.body,
                        "raw": it.raw,
                        "range": _span(it.range),
                        "key_range": _span(it.key_range),
                        "body_range": _span(it.body_range),
                    }
                    for it in value.items
                ],
            }
        }
    if dc.is_dataclass(value) and not isinstance(value, type):
        return {
            "__t": type(value).__name__,
            "d": {f.name: _canonical(getattr(value, f.name)) for f in dc.fields(value)},
        }
    if isinstance(value, (tuple, list)):
        return {"t": [_canonical(v) for v in value]}
    if isinstance(value, dict):
        return {"m": {k: _canonical(v) for k, v in value.items()}}
    # Any other typed scalar value.
    return {"s": str(value)}


def canonical(config: BigipConfig) -> dict[str, Any]:
    """Serialise a Python ``BigipConfig`` to the canonical document."""
    out: dict[str, Any] = {"default_partition": config.default_partition}
    generic: list[Any] = []
    objects: list[Any] = []
    for f in dc.fields(config):
        if f.name == "default_partition":
            continue
        table = getattr(config, f.name)
        if not isinstance(table, dict) or not table:
            continue
        if f.name == "generic_objects":
            for key, obj in table.items():
                generic.append([key, _canonical(obj)["d"]])
            continue
        for full_path, obj in table.items():
            objects.append(
                {"table": f.name, "full_path": full_path, "fields": _canonical(obj)["d"]}
            )
    out["generic_objects"] = generic
    out["objects"] = objects
    return out


# Reconstruction ---------------------------------------------------------

# Typed-scalar value parsers, keyed by the dataclass field's annotated type.
_VALUE_PARSERS = {
    "Destination": lambda s: Destination.parse(s),
    "Network": lambda s: Network.parse(s),
    "IPAddress": lambda s: IPAddress.parse(s),
    "FQDN": lambda s: FQDN(s) if not isinstance(s, FQDN) else s,
    "Address": parse_address,
}


def _type_name(ann: Any) -> str:
    s = str(ann).replace("typing.", "")
    return s.replace(" | None", "").strip().strip("'")


def _decode(value: Any, ann: Any) -> Any:
    if value is None:
        return None
    if isinstance(value, (bool, int, str)):
        return value
    if "r" in value:
        sl, sc, so, el, ec, eo = value["r"]
        return Range(
            start=SourcePosition(line=sl, character=sc, offset=so),
            end=SourcePosition(line=el, character=ec, offset=eo),
        )
    if "e" in value:
        enum_cls = _resolve_enum(_type_name(ann))
        return enum_cls[value["e"]]
    if "s" in value:
        tn = _type_name(ann)
        parser = _VALUE_PARSERS.get(tn)
        return parser(value["s"]) if parser else value["s"]
    if "t" in value:
        inner = _tuple_inner(ann)
        return tuple(_decode(v, inner) for v in value["t"])
    if "m" in value:
        return {k: _decode(v, None) for k, v in value["m"].items()}
    if "L" in value:
        body = value["L"]
        items = [
            ListItem(
                value=_decode(it["v"], None),
                key=it["k"],
                body=it["b"],
                raw=it["raw"],
                range=_unspan(it["range"]),
                key_range=_unspan(it["key_range"]),
                body_range=_unspan(it["body_range"]),
            )
            for it in body["items"]
        ]
        return BigipList(
            items=items,
            syntax=body["syntax"],
            operator=body["operator"],
            raw=body["raw"],
            range=_unspan(body["range"]),
        )
    if "__t" in value:
        cls = _resolve_struct(value["__t"])
        return _build_dataclass(cls, value["d"])
    raise ValueError(f"unknown canonical value {value!r}")


def _tuple_inner(ann: Any):
    args = typing.get_args(ann)
    if args:
        return args[0]
    return Any


def _resolve_enum(name: str):
    from dialects.f5.bigip.model import _enums

    return getattr(_enums, name)


def _resolve_struct(name: str):
    import dialects.f5.bigip.model as M

    if hasattr(M, name):
        return getattr(M, name)
    return getattr(bigip_types, name)


def _build_dataclass(cls, fields: dict[str, Any]):
    kwargs = {}
    ann = {f.name: f.type for f in dc.fields(cls)}
    for name, raw in fields.items():
        kwargs[name] = _decode(raw, ann.get(name))
    return cls(**kwargs)


def _table_struct_map() -> dict[str, Any]:
    import re

    import dialects.f5.bigip.model as M

    out: dict[str, Any] = {}
    for f in dc.fields(BigipConfig):
        m = re.search(r"dict\[str,\s*([A-Za-z0-9_]+)\]", str(f.type))
        if not m:
            continue
        name = m.group(1)
        if "Minimal" in name:
            name = "BigipMinimalObject"
        out[f.name] = getattr(M, name)
    return out


def rebuild(doc: dict[str, Any]) -> BigipConfig:
    """Reconstruct a Python ``BigipConfig`` from a canonical document."""
    from dialects.f5.bigip.model import BigipGenericObject

    config = BigipConfig(default_partition=doc["default_partition"])
    for key, fields in doc["generic_objects"]:
        config.generic_objects[key] = _build_dataclass(BigipGenericObject, fields)
    tables = _table_struct_map()
    for obj in doc["objects"]:
        cls = tables[obj["table"]]
        getattr(config, obj["table"])[obj["full_path"]] = _build_dataclass(cls, obj["fields"])
    return config


# APL (iApp presentation language) reconstruction --------------------------


def _apl_range(v: Any) -> Any:
    if v is None:
        return None
    sl, sc, so, el, ec, eo = v["r"]
    return Range(
        start=SourcePosition(line=sl, character=sc, offset=so),
        end=SourcePosition(line=el, character=ec, offset=eo),
    )


def _apl_field(d: dict[str, Any]):
    from dialects.f5.bigip.apl_model import AplField

    return AplField(
        name=d["name"],
        qualified_name=d["qualified_name"],
        field_type=d["field_type"],
        is_required=d["is_required"],
        range=_apl_range(d["range"]),
    )


def rebuild_apl(doc: dict[str, Any]):
    """Reconstruct a Python ``AplModel`` from a canonical APL document."""
    from dialects.f5.bigip.apl_model import AplInclude, AplModel, AplSection, AplTable

    model = AplModel()
    for name, sec in doc["sections"]:
        s = AplSection(
            name=sec["name"],
            qualified_name=sec["qualified_name"],
            fields={k: _apl_field(f) for k, f in sec["fields"]},
            range=_apl_range(sec["range"]),
        )
        model.sections[name] = s
    for name, tbl in doc["tables"]:
        t = AplTable(
            name=tbl["name"],
            qualified_name=tbl["qualified_name"],
            columns={k: _apl_field(f) for k, f in tbl["columns"]},
            range=_apl_range(tbl["range"]),
        )
        model.tables[name] = t
    model.defines = {k: v for k, v in doc["defines"]}
    model.includes = [
        AplInclude(path=i["path"], line=i["line"], resolved=i["resolved"]) for i in doc["includes"]
    ]
    model.all_fields = {k: _apl_field(f) for k, f in doc["all_fields"]}
    return model
