from pathlib import Path

from direct_imports import leaves, rewrite


def test_leaves_of_a_nested_tree() -> None:
    assert leaves("a::{b, c::{self, D as E}, f::*}") == [
        (("a", "b"), "b"),
        (("a", "c", "self"), "c"),
        (("a", "c", "D"), "E"),
        (("a", "f", "*"), "*"),
    ]


def test_a_child_imports_from_where_a_name_is_defined(tmp_path: Path) -> None:
    parent = tmp_path / "walk.rs"
    parent.write_text(
        """\
use std::collections::{HashMap, HashSet};
use crate::lexer::Token as Tok;
use self::legend::Kind;
use super::shared::Limit;
#[cfg(test)]
use std::fmt::Debug;

mod legend;
mod steps;

struct Walker;
"""
    )
    (tmp_path / "walk").mkdir()
    child = tmp_path / "walk" / "steps.rs"
    child.write_text("//! Steps.\n\nuse super::{Debug, HashMap, Kind, Limit, Tok, Walker};\n\nfn step() {}\n")
    assert rewrite(child) == 4
    text = child.read_text()
    assert "use std::collections::HashMap;\n" in text
    assert "use super::legend::Kind;\n" in text
    assert "use super::super::shared::Limit;\n" in text
    assert "use crate::lexer::Token as Tok;\n" in text
    assert "use super::{Debug, Walker};\n" in text
    assert text.startswith("//! Steps.\n\nuse ") and text.count("//! Steps.") == 1
    assert text.endswith("\nfn step() {}\n")


def test_a_module_only_import_is_written_back_as_one(tmp_path: Path) -> None:
    parent = tmp_path / "resolve.rs"
    parent.write_text("use std::fmt;\n\nmod cell_key;\nmod frames;\n")
    (tmp_path / "resolve").mkdir()
    child = tmp_path / "resolve" / "frames.rs"
    child.write_text("use super::cell_key::{self};\nuse super::{cell_key::cell_key, fmt};\n\nfn f() {}\n")
    rewrite(child)
    text = child.read_text()
    assert "use super::cell_key::{self};\n" in text
    assert "use std::fmt;\n" in text
