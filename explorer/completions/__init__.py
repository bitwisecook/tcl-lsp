"""Shell-completion script bundles for the ``tcl`` and ``f5`` CLIs.

Each ``<cli>.<shell>`` file in this package is plain text — bash, fish, or
zsh completion script — distributed alongside the Python sources so the
zipapp can ``importlib.resources``-read them when the user runs
``tcl completion <shell>`` or ``f5 completion <shell>``.
"""
