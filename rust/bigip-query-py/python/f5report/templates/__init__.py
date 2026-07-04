"""Jinja templates + CSS/JS assets, loaded via ``importlib.resources.files``.

Kept a regular package (this file), not an implicit namespace package, so
``resources.files("f5report.templates")`` resolves the CSS/JS assets on every
supported CPython — the stdlib only supports namespace-package resources from
3.10 up, and the floor is 3.9. Mirrors the sibling ``vendor`` package.
"""
