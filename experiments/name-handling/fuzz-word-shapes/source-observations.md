# Source-reported word-shape observations

These excerpts retain the generator's existing campaign and buggy-result
reports. Provider identities, seeds and raw campaign output were not recorded
in these comments. They are source-reported observations, not newly executed
oracles or conformance acceptance. Deterministic fixtures require independently
captured original provider results.

Source: `rust/tcl-fuzz/src/generator.rs`, SHA256 `ecb8416a74335bc32850824bbb0a9c20d47aba9e991ee8c34615d71d19a3b917`.
The excerpts below preserve the original wording and concrete word inputs.

```rust
//! Two productions exist because the generator was *structurally* unable to
//! reach a bug class, rather than reaching it rarely — a distinction worth
//! keeping in mind before reading anything into a clean campaign. A
//! deliberately [`Malformed`] expression, because every generated expression
//! was well-formed by construction; and a [`WordShape`], because every
//! generated word came from [`WORDS`], where the spelling *is* the value. The
//! second was added after a 1500-script campaign returned 0 findings against a
//! build that answered `string index "{}" 0` as the empty string.
//!
    /// How often a generated word is given a **delimiter-shaped value** — a
    /// value that looks like a braced word without being one (`"{}"`, `"{$x}"`,
    /// `\{\}`) — in parts per thousand.
    ///
    /// Until this existed the generator drew its words from [`WORDS`], none of
    /// which contains a brace, bracket, quote or backslash. It was therefore
    /// structurally incapable of producing a word whose *value* is delimiter
    /// shaped, and the whole word-value class was invisible: a 1500-script
    /// campaign found 0 findings against a build that provably answered
    /// `string index "{}" 0` as the empty string and `set v "{}"` as 0
    /// characters. Same failure mode as `malformed_expr_permille`, same
    /// remedy — see [`WordShape`] for what the shapes are and what each one
    /// caught.
    ///
    /// This class is much cheaper than a malformed expression: every shape is
    /// well-formed, so it does not abort the script and the statements after
    /// it still run. The rate is correspondingly higher — but only to the
    /// point where it still leaves most of the corpus to the rest of the
    /// grammar. At 60‰ a third of scripts carry a shape; 120‰ put it over half,
    /// which is more of the budget than one class should own.
    ///
    /// `0` opts out entirely, and consumes no random draw, so a rate-`0`
    /// campaign reproduces the pre-existing generator stream exactly and
    /// historical findings still replay from their recorded seeds.
/// Each group's variants say what they caught. Where a shape is a control
/// rather than a catch, it says that too.
///
/// **Every shape is a single well-formed Tcl word**, so it drops into any word
/// position without changing the shape of the surrounding script — the same
/// discipline [`Malformed`] keeps for the expression grammar. Shapes that are
/// not brace-balanced (`"{"`, `"}"`) are additionally kept out of the contexts
/// that wrap the word in braces; see [`Self::brace_balanced`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordShape {
    /// `"{}"` — the value `{}`. `string index "{}" 0` answered the empty
    /// string where both oracles say `{`, and `set v "{}"` stored 0 characters
    /// where both say 2.
    QuotedEmptyBraces,
    /// `"{abc}"` — the value `{abc}`. `string length` answered 3 for 5.
    QuotedBracedWord,
    /// `"{$x}"` — braced-shaped *and* still carrying a live variable. Pushed
    /// raw, the VM stripped the braces and handed back the inside
    /// unsubstituted: `puts "{$x}"` printed `${x}`.
    QuotedBracedVar,
    /// `"{[…]}"` — the command-substitution half of the same shape.
    QuotedBracedCmd,
    /// `"{}$x"` — a composite word whose *literal fragment* is brace-shaped.
    /// The fragment is already decoded, so re-substituting it stripped the
    /// braces off it: `string length "{}$x"` answered 1 for 3.
    QuotedFragmentBraces,
    /// `"{} {}"` — brace-shaped but *not* a whole braced word, so the VM would
    /// never have stripped it. The negative control for every shape above.
    QuotedBraceRun,
    /// `"{"` — an unbalanced single brace as a value.
    QuotedOpenBrace,
    /// `"}"` — the same, closing.
    QuotedCloseBrace,
    /// `\{\}` — braces reached by escape rather than by quoting: the same word
    /// one step later, finished once its escapes are decoded. `[format %s \{\}]`
    /// froze the 4-byte source spelling where both oracles give 2.
    EscapedBraces,
    /// `"\{\}"` — both routes at once.
    QuotedEscapedBraces,
    /// `{{}}` — a *genuinely* braced word, which must still lose exactly one
    /// layer. The positive control: the fix for every shape above must not
    /// disturb this one.
    BracedDoubleGroup,
    /// `{a[…]}` — a braced word whose brackets are data. A engine that treats
    /// the de-braced value as source *runs* the command, so the shape names a
    /// command that does not exist and the divergence is loud.
    BracedCmdIsData,
    /// `{a${…}}` — the variable half of the same control, naming a variable
    /// that does not exist.
    BracedVarIsData,
    /// `"{a}b{c}"` — two brace groups in one value, so the first `{` does not
    /// match the last `}`. Distinguishes a real balance walk from a two-byte
    /// `starts_with('{') && ends_with('}')` test, which is the bug that made
    /// `switch -- "{}$x" …` refuse a script both oracles run.
    QuotedTwoGroups,

    // -- escape-bearing --
    /// `a\ b` — one word whose value carries a space, because the backslash
    /// escapes the separator. The word *boundary* is the thing under test:
    /// `string length a\ b` answers `wrong # args` where both oracles say 3,
    /// so the escaped space is being read as a word break.
    EscapedSpace,
    /// `\x41` — a hex escape; the value is one character, the spelling four.
    /// Hex escape width is a release axis.
    EscapedHex,
    /// `\101` — the octal spelling of the same character, whose width is a
    /// release axis of its own.
    EscapedOctal,
    /// `\\` — an escaped backslash: the value is one byte the *next* decode
    /// would consume if a path decoded twice.
    EscapedBackslash,
    /// `a\tb` — an escape in the middle of a word, so the value carries a
    /// control character no spelling of it contains.
    EscapedTab,

    // -- non-ASCII and whitespace-bearing --
    /// `"café"` — a value whose character count (4) is not its byte count (5).
    /// `[format %s "café"]` answered 5 characters and printed mojibake, having
    /// walked the quoted word a byte at a time.
    NonAscii,
    /// `"{café}"` — non-ASCII crossed with the brace shape, so a byte-wise
    /// walk and a brace walk have to be right at the same time.
    NonAsciiBraced,
    /// `"a b"` — a value carrying a space, which is what list quoting exists
    /// for: `[list "a b"]` is `{a b}`, and that fold has been wrong before.
    QuotedSpace,
    /// `" "` — nothing but a separator.
    QuotedSpaceOnly,
    /// `""` — the empty value, which a list or dict element can collapse.
    QuotedEmpty,
    /// `"#"` — the character list quoting brace-wraps at element position 0
    /// and nowhere else (`[list #]` is `{#}`, `[list a #]` is `a #`), a rule
    /// the `dict create` fold's own documentation calls out.
    QuotedHash,
}

```
