"""Typed BIG-IP data-group record value.

Data-group records have a keyed-block shape where each record is a
key (the lookup name) with an optional ``data`` value:

    records {
        host1.example.com { data 10.0.0.1 }
        host2.example.com { data 10.0.0.2 }
        prod-allowed { }
    }

The flat ``tuple[str, ...]`` representation the legacy parser
captured drops the data side, making queries like "what address
does this hostname map to?" impossible without re-parsing the
stanza body.  This typed value preserves the structure so the
DSL can ask

    .ltm["data-group"]["/Common/hosts"].records[]
      | select(.key == "host1.example.com")
      | .data

without losing fidelity.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class DataGroupRecord:
    """One record inside a data-group's records list.

    ``key`` is the lookup name (a string for type-string groups,
    a CIDR for type-ip groups, an integer for type-integer
    groups; we keep it as a string so the typed value works for
    every group type).  ``data`` is the per-record value (often
    an IP, label, or arbitrary string); empty when the record is
    bare (key only).
    """

    key: str = ""
    data: str = ""
    raw: str = ""

    @classmethod
    def from_raw(cls, key: str, body: str) -> "DataGroupRecord":
        """Build a record from the keyed-block parser output.

        Recognises the ``data <value>`` property inside the body
        and stores its value on ``.data``; bare records (with an
        empty or whitespace-only body) get an empty data field.
        """
        data = ""
        tokens = body.split()
        for i, tok in enumerate(tokens):
            if tok == "data" and i + 1 < len(tokens):
                # ``data`` may be followed by a quoted string or a
                # bare token; we keep whatever the user wrote so
                # the rewrite round trip preserves spelling.
                data = tokens[i + 1]
        return cls(key=key, data=data, raw=body.strip())

    def __str__(self) -> str:
        if self.raw:
            return f"{self.key} {{ {self.raw} }}" if self.raw else f"{self.key} {{ }}"
        if self.data:
            return f"{self.key} {{ data {self.data} }}"
        return f"{self.key} {{ }}"


__all__ = ["DataGroupRecord"]
