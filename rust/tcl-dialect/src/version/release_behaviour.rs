// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The behaviour each release line has: regex, `lset`, level grammars, expression errors, error codes and bytecode rules.

use super::TclVersion;
use super::character_model::ByteStringEncoding;
use super::engine_policies::{FrameLevelPresence, ListSetBounds};
use super::package_versions::PackageDiagnostic;

impl TclVersion {
    /// Whether this core retains Tcl 8's namespace-scope lookup fallback.
    ///
    /// At namespace scope, an unqualified variable name first probes the
    /// current namespace. Through Tcl 8.6 it then probes the global namespace;
    /// Tcl 9.0 removed that second probe (TIP 278). This belongs to the
    /// version-profile vocabulary so runtimes do not each encode a release
    /// comparison beside their variable resolver.
    #[must_use]
    pub const fn namespace_var_global_fallback(self) -> bool {
        self.release_policy().namespace_var_global_fallback
    }

    /// Whether variable traces recover an array element the access spelling
    /// does not name.
    ///
    /// Tcl 9.0 added it in three places at once — `TclVarFindHiddenArray`
    /// (`tclInt.h` 9.0.4:866), the `part2` recovery in `TclCallVarTraces`
    /// (`tclTrace.c` 9.0.4:2560-2565) and the same recovery in
    /// `UnsetVarStruct` (`tclVar.c` 9.0.4:2638-2642); 8.4/8.5/8.6 have none of
    /// them. Two consequences are visible from script: an alias into an element
    /// (`upvar #0 a(k) e; set e 5`) fires the containing array's traces too and
    /// reports `name2 = k`, and `unset a(k)` reports `name1 = a(k)` rather than
    /// the split-off base. Measured on tclsh 8.4.20, 8.5.19, 8.6.16 (off) and
    /// 9.0.4, 9.1b0 (on). This belongs to the version-profile vocabulary so
    /// runtimes do not each encode a release comparison beside their trace
    /// dispatcher.
    #[must_use]
    pub const fn traces_recover_linked_array_element(self) -> bool {
        matches!(self, Self::V9_0 | Self::V9_1)
    }

    /// Whether a leading `~` in a file name names a home directory (TIP 602).
    /// Tcl 9.0 reads `~/x` as an ordinary relative name and keeps the
    /// expansion in `file tildeexpand`; tclsh 8.4.20 through 8.6.18 expand it
    /// in every file-name argument.
    #[must_use]
    pub const fn tilde_names_home(self) -> bool {
        !matches!(self, Self::V9_0 | Self::V9_1)
    }

    /// The encoding names this release no longer accepts (TIP 656): `binary`
    /// and `identity` were channel and conversion pseudo-encodings through
    /// 8.6 and raise `unknown encoding` from 9.0, where `-translation binary`
    /// or `-encoding iso8859-1` replaces them.
    #[must_use]
    pub const fn removed_encoding_names(self) -> &'static [&'static str] {
        match self {
            Self::V9_0 | Self::V9_1 => &["binary", "identity"],
            _ => &[],
        }
    }

    /// Whether an ARE accepts `\z` as a synonym for the `\Z` end-of-string
    /// anchor. Tcl 9.1.0 added it (`regc_lex.c`, ticket fbc56b259e); tclsh
    /// 8.4.20 through 9.0.4 and the 9.1b0 beta reject it as `invalid escape \
    /// sequence`.
    #[must_use]
    pub const fn regex_z_anchor(self) -> bool {
        self.release_policy().regex_z_anchor
    }

    /// The prefix `CompileRegexp` (`tclRegexp.c`) puts before a regex compile
    /// error's detail. Tcl 9.0 reworded it: tclsh 8.4.20, 8.5.19 and 8.6.18
    /// say `couldn't compile …`, 9.0.4 and 9.1.0 say `cannot compile …`.
    #[must_use]
    pub const fn regex_compile_error_prefix(self) -> &'static str {
        match self {
            Self::V8_4 | Self::V8_5 | Self::V8_6 => "couldn't compile regular expression pattern: ",
            Self::V9_0 | Self::V9_1 => "cannot compile regular expression pattern: ",
        }
    }

    /// The release-defined conversion used when a string is consumed as raw
    /// binary data. See [`ByteStringEncoding`] for the Tcl 8/Tcl 9 split.
    #[must_use]
    pub const fn byte_string_encoding(self) -> ByteStringEncoding {
        match self {
            Self::V8_4 | Self::V8_5 | Self::V8_6 => ByteStringEncoding::LegacyTruncate,
            Self::V9_0 | Self::V9_1 => ByteStringEncoding::CheckedLatin1,
        }
    }

    /// Whether package commands accept TIP 268 requirement lists, ranges and
    /// alpha/beta versions. Tcl 8.4 accepts one numeric dotted version only.
    #[must_use]
    pub const fn has_package_requirements(self) -> bool {
        !matches!(self, Self::V8_4)
    }

    /// Bounds accepted by `lset`, including the one-past-end append case.
    #[must_use]
    pub const fn list_set_bounds(self) -> ListSetBounds {
        match self {
            Self::V8_4 | Self::V8_5 => ListSetBounds::ExistingElement,
            Self::V8_6 | Self::V9_0 | Self::V9_1 => ListSetBounds::AppendAtEnd,
        }
    }

    /// The optional-level presence grammar of `upvar`.
    #[must_use]
    pub const fn upvar_level_presence(self) -> FrameLevelPresence {
        self.release_policy().upvar_level_presence
    }

    /// The optional-level presence grammar of `uplevel`.
    #[must_use]
    pub const fn uplevel_level_presence(self) -> FrameLevelPresence {
        self.release_policy().uplevel_level_presence
    }

    /// Whether `upvar` chooses its optional level by argument-count parity.
    /// Tcl 8.4 and 8.5 instead probe the leading word before checking pairs.
    #[must_use]
    pub const fn upvar_uses_argument_parity(self) -> bool {
        matches!(
            self.release_policy().upvar_level_presence,
            FrameLevelPresence::ArgumentParity
        )
    }

    /// Whether this release accepts a `+` suffix on package versions.
    ///
    /// Tcl 9 stops package-version conversion at the first `+`, while Tcl 8
    /// rejects the suffix as part of the version. Keep this release fact on
    /// the version vocabulary so package commands and shared version helpers
    /// cannot drift apart.
    #[must_use]
    pub const fn allows_package_version_suffix(self) -> bool {
        matches!(self, Self::V9_0 | Self::V9_1)
    }

    /// Whether `lset` appends when an index equals the length of the list it
    /// addresses, rather than raising `list index out of range`.
    ///
    /// Tcl 8.6 reads the index against `elemCount` inclusive (`TclLsetFlat`,
    /// `tclListObj.c` 8.6.18:1444, `index > elemCount`); 8.4 and 8.5 reject it
    /// (8.4.20:1175 and 8.5.19:1373, `index >= elemCount`). Measured on
    /// tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0: `set x {a b c}; lset x 3
    /// Z` raises on the first two and gives `a b c Z` on the rest, and so does
    /// the empty list's `lset x 0 Z`. The rule holds at every level of a
    /// nested index path, and for the `end+1` spelling where the release reads
    /// it as an index at all (8.4 calls it a bad index).
    #[must_use]
    pub const fn lset_appends_at_length(self) -> bool {
        !matches!(self, Self::V8_4 | Self::V8_5)
    }

    /// [`Self::lset_appends_at_length`] for the release a consumer targets,
    /// or — when it names none — the answer **every** release gives, `None`
    /// where they disagree.
    ///
    /// Unanimity is over [`Self::ALL`], so a document that selects no release
    /// is never told that an `lset` at the length raises (8.4/8.5) or that it
    /// appends (8.6 and later): the boundary index is the one place the
    /// releases split.
    #[must_use]
    pub fn lset_appends_at_length_for(release: Option<Self>) -> Option<bool> {
        if let Some(release) = release {
            return Some(release.lset_appends_at_length());
        }
        let mut answers = Self::ALL.into_iter().map(Self::lset_appends_at_length);
        let first = answers.next()?;
        answers.all(|answer| answer == first).then_some(first)
    }

    /// Whether the string rep this release generates for a computed
    /// `INT64_MIN` is `-8` rather than `-9223372036854775808`.
    ///
    /// `TclFormatInt` (`tclUtil.c`, 8.4.20 and 8.5.19) guards the most
    /// negative `long` with `n == -n || intVal == n`, both signed overflow, so
    /// the compiler may drop the test, and the digit loop over the negated
    /// (still negative) value emits one digit and stops at `intVal > 0`.
    /// `UpdateStringOfInt` spells every 64-bit integer through it on LP64.
    /// 8.6 negates through `unsigned long` instead. Measured: `expr {-9223372036854775807 - 1}`,
    /// `expr {~9223372036854775807}` and `expr {int(-9223372036854775808.0)}`
    /// print `-8` (`string length` 2) on tclsh 8.4.20 and 8.5.19, and the
    /// digits on 8.6.18, 9.0.4, 9.1.0 and jimsh 0.84; a literal
    /// `-9223372036854775808` keeps its own spelling everywhere.
    #[must_use]
    pub const fn garbles_wide_min(self) -> bool {
        matches!(self, Self::V8_4 | Self::V8_5)
    }

    /// Whether `format %d`/`%i` spells `INT64_MIN` through the same garbled
    /// updater ([`Self::garbles_wide_min`]): its digits become `8`, and the
    /// sign, width and precision then apply around them. 8.5's
    /// `Tcl_AppendFormatToObj` takes the digits from a new integer object's
    /// string rep; 8.4's `sprintf` path is unaffected. Measured on tclsh
    /// 8.5.19: `format %d`, `%ld`, `%lld` and `%i` give `-8`, `%05d` gives
    /// `-0008` and `%.3d` gives `-008`; 8.4.20 and 8.6 print the full digits.
    #[must_use]
    pub const fn format_garbles_wide_min(self) -> bool {
        matches!(self, Self::V8_5)
    }

    /// Whether the core implements `namespace path` (introduced in Tcl 8.5).
    #[must_use]
    pub const fn has_namespace_path(self) -> bool {
        !matches!(self, Self::V8_4)
    }

    /// Whether `return` keeps its options in a dictionary (TIP 90, Tcl 8.5):
    /// `-level`, `-options` and any other pair are accepted. 8.4 reads its
    /// words as `-code`/`-errorinfo`/`-errorcode` pairs while two remain and
    /// rejects any other name.
    #[must_use]
    pub const fn has_return_options_dictionary(self) -> bool {
        !matches!(self, Self::V8_4)
    }

    /// Whether an `expr` operator subexpression whose operands are all
    /// literals is folded at compile time, so its error is a fold failure:
    /// raised where the subexpression is evaluated, with `errorInfo` already
    /// holding the message (`invoked from within` the command). Measured on
    /// tclsh 8.4.20 (no folding) and 8.5.19 onward.
    #[must_use]
    pub const fn raises_folded_expr_errors(self) -> bool {
        !matches!(self, Self::V8_4)
    }

    /// Whether an error leaving a command substitution in an expression the
    /// release evaluates as it runs (`Tcl_ExprObj`) stays logged
    /// (`ERR_ALREADY_LOGGED`) through every command enclosing the
    /// expression, so none of them logs and `errorLine` keeps the line the
    /// command inside logged. 8.4's `Tcl_ExprObj` runs the expression's
    /// bytecode without the clear `Tcl_EvalObjEx` ends with
    /// (`tclExecute.c`); a procedure boundary still logs its call. Measured
    /// on tclsh 8.4.20 and 8.5.19.
    #[must_use]
    pub const fn keeps_expression_substitution_errors_logged(self) -> bool {
        matches!(self, Self::V8_4)
    }

    /// Whether the error a completion code becomes where it cannot be taken
    /// (`invoked "break" outside of a loop`, `command returned bad code`)
    /// sets an errorCode: `TCL RESULT UNEXPECTED` at a procedure boundary,
    /// `TCL UNEXPECTED_RESULT_CODE n` at top level. Measured on tclsh 8.5.19
    /// (`NONE`) and 8.6.18.
    #[must_use]
    pub const fn has_unexpected_result_error_codes(self) -> bool {
        !matches!(self, Self::V8_4 | Self::V8_5)
    }

    /// Whether an error from an `enter` or `leave` execution trace adds an
    /// `(enter trace on "cmd")` frame in place of logging the traced command
    /// (`TEOV_RunEnterTraces`, 8.6). Measured on tclsh 8.5.19 and 8.6.18.
    #[must_use]
    pub const fn has_execution_trace_error_frames(self) -> bool {
        !matches!(self, Self::V8_4 | Self::V8_5)
    }

    /// How a math function called with too few arguments says so: `too
    /// few` through 8.5, `not enough` from 8.6.
    #[must_use]
    pub const fn math_arity_shortfall(self) -> &'static str {
        if matches!(self, Self::V8_4 | Self::V8_5) {
            "too few arguments"
        } else {
            "not enough arguments"
        }
    }

    /// Whether `srand` of a non-integer reports the conversion error. 9.0's
    /// `ExprSrandFunc` converts with a NULL interpreter, so the error is
    /// empty with errorCode `NONE`. Measured on tclsh 8.6.18 and 9.0.4.
    #[must_use]
    pub const fn srand_reports_conversion_error(self) -> bool {
        matches!(self, Self::V8_4 | Self::V8_5 | Self::V8_6)
    }

    /// Whether builtins set the errorCode `TCL <family> <detail> …`. 8.4's
    /// sources set no `TCL` code; 8.5's set `TCL LOOKUP` for the kinds
    /// [`Self::has_lookup_error_code`] lists, `TCL VALUE NUMBER`, `TCL
    /// IDELETE` and `TCL API_ABUSE`, and none of `TCL WRONGARGS`, `TCL PARSE`,
    /// `TCL UPVAR` or the other `TCL VALUE` codes, which 8.6 adds. Measured:
    /// `catch set`, `expr {1 +}`, `lindex {a} x` and an `upvar` onto an
    /// existing local leave `NONE` on tclsh 8.4.20 and 8.5.19.
    #[must_use]
    pub fn has_tcl_error_code(self, family: &str, detail: Option<&str>) -> bool {
        match self {
            Self::V8_4 => false,
            Self::V8_5 => match (family, detail) {
                ("LOOKUP", Some(kind)) => self.has_lookup_error_code(kind),
                ("VALUE", Some("NUMBER")) | ("IDELETE" | "API_ABUSE", _) => true,
                _ => false,
            },
            Self::V8_6 | Self::V9_0 | Self::V9_1 => true,
        }
    }

    /// Whether a failed lookup of `kind` carries `TCL LOOKUP <kind> …`. 8.5
    /// sets it for an ensemble, index, namespace, alias, package, channel,
    /// interpreter, encoding, array search or variable lookup, and not for a
    /// command, dictionary key or variable name, which 8.6 adds everywhere
    /// (8.5 already sets the first two for `namespace origin` and a nested
    /// dictionary path, which this does not single out). Measured:
    /// `dict get {a 1} b` and `upvar 0 nosuch::x y` leave `NONE` on tclsh
    /// 8.5.19 and set `TCL LOOKUP DICT b` and `TCL LOOKUP VARNAME
    /// nosuch::x` on 8.6.18.
    #[must_use]
    pub fn has_lookup_error_code(self, kind: &str) -> bool {
        match self {
            Self::V8_4 => false,
            Self::V8_5 => matches!(
                kind,
                "SUBCOMMAND"
                    | "INDEX"
                    | "NAMESPACE"
                    | "ALIAS"
                    | "PACKAGE"
                    | "ENSEMBLE"
                    | "CHANNEL"
                    | "VARIABLE"
                    | "INTERP"
                    | "ENCODING"
                    | "ELEMENT"
                    | "ARRAYSEARCH"
            ),
            Self::V8_6 | Self::V9_0 | Self::V9_1 => true,
        }
    }

    /// Whether an operand `expr` cannot use carries `ARITH DOMAIN <desc>`
    /// (8.5). Measured: `expr {"x" + 1}` leaves `NONE` on tclsh 8.4.20.
    #[must_use]
    pub const fn has_illegal_operand_error_codes(self) -> bool {
        !matches!(self, Self::V8_4)
    }

    /// Whether a floating-point `expr` result that is infinite or NaN is an
    /// error, as is a division by a zero double (8.4's
    /// `TclExprFloatError`); later releases return `Inf`. Measured: `expr
    /// {1e308 * 10}` is `floating-point value too large to represent` and
    /// `expr {1.0 / 0}` `divide by zero` on tclsh 8.4.20, `Inf` from 8.5.19.
    #[must_use]
    pub const fn raises_float_range_errors(self) -> bool {
        matches!(self, Self::V8_4)
    }

    /// Whether a failed variable trace keeps its callback's errorInfo and
    /// adds a `(write trace on "x")` frame (`TclCallVarTraces`, 8.5). 8.4's
    /// `CallVarTraces` resets the result to `can't set "x": <message>`, so
    /// errorInfo starts again from that message. Measured on tclsh 8.4.20
    /// and 8.5.19.
    #[must_use]
    pub const fn has_var_trace_error_frames(self) -> bool {
        !matches!(self, Self::V8_4)
    }

    /// Whether the bytecode compiler emits only the wide (4-byte operand)
    /// instruction forms, which then drop their width suffix (`invokeStk`,
    /// `loadScalar`). Measured in `-errorstack` on tclsh 9.1.0.
    #[must_use]
    pub const fn emits_wide_instructions_only(self) -> bool {
        matches!(self, Self::V9_1)
    }

    /// Whether an error's `(file "…" line N)` frame marks a path cut at 150
    /// bytes with `...`. 8.4 cuts it silently (`%.150s`, `tclIOUtil.c`).
    #[must_use]
    pub const fn marks_truncated_file_path(self) -> bool {
        !matches!(self, Self::V8_4)
    }

    /// Whether `return`'s own option errors (`bad completion code`, `bad -level
    /// value`, an `-options` value that is not a dictionary) set `TCL RESULT
    /// ILLEGAL_…` errorCode words. Measured on tclsh 8.5.19 and 8.6.18: they
    /// arrive at 8.6; earlier releases leave `NONE`.
    #[must_use]
    pub const fn has_return_option_error_codes(self) -> bool {
        !matches!(self, Self::V8_4 | Self::V8_5)
    }

    /// Whether `return -options` rejects an odd-sized list. 8.5 merges its
    /// complete pairs and ignores the last word; 8.6 rejects it.
    #[must_use]
    pub const fn rejects_odd_return_options(self) -> bool {
        !matches!(self, Self::V8_4 | Self::V8_5)
    }

    /// Whether `package` requirements are TIP 268 ranges (`min-max`, `min-`)
    /// and `-exact` is spelled `exactly V` in diagnostics. 8.4 reads a
    /// requirement as one bare version, rejecting `1-2` as a bad version.
    #[must_use]
    pub const fn has_package_requirement_ranges(self) -> bool {
        !matches!(self, Self::V8_4)
    }

    /// The `errorCode` words a `package` diagnostic sets on this release, or
    /// `None` where C leaves `errorCode` at `NONE`. Measured on tclsh 8.4.20
    /// through 9.1.0: the `TCL PACKAGE` and `TCL VALUE VERSION…` families
    /// arrive at 8.6, `package present`'s lookup code at 8.5.
    #[must_use]
    pub const fn package_error_code(
        self,
        diagnostic: PackageDiagnostic,
    ) -> Option<&'static [&'static str]> {
        let structured = match diagnostic {
            PackageDiagnostic::NotPresent => !matches!(self, Self::V8_4),
            _ => !matches!(self, Self::V8_4 | Self::V8_5),
        };
        if structured {
            Some(diagnostic.error_code_words())
        } else {
            None
        }
    }

    /// Whether `rename` sets a structured `errorCode` for a missing source
    /// (`TCL LOOKUP COMMAND <name>`) and a taken destination
    /// (`TCL OPERATION RENAME TARGET_EXISTS`). Measured on tclsh 8.4.20
    /// through 9.1.0: both arrive at 8.6, earlier releases leave `NONE`.
    #[must_use]
    pub fn has_rename_error_codes(self) -> bool {
        self.has_tcl_error_code("LOOKUP", Some("COMMAND"))
    }

    /// Whether this release exposes TIP 348 structured error stacks.
    ///
    /// Tcl 8.6 introduced both `info errorstack` and the `-errorstack` return
    /// option. Keeping the release boundary here lets every runtime and
    /// completion adapter consume the same dialect fact.
    #[must_use]
    pub fn has_error_stack(self) -> bool {
        matches!(self, Self::V8_6 | Self::V9_0 | Self::V9_1)
    }

    /// Whether this release tags a `wrong # args` usage error
    /// (`Tcl_WrongNumArgs`) and a malformed list (`TclFindElement`) with a
    /// `TCL …` error code. 8.4 and 8.5 set none, so `$errorCode` is `NONE`.
    #[must_use]
    pub fn tags_usage_error_codes(self) -> bool {
        self.has_tcl_error_code("WRONGARGS", None)
    }

    /// Whether a builtin raising errorCode `code` leaves it under this
    /// release: any code but a `TCL …` one the release does not set yet
    /// ([`Self::has_tcl_error_code`]).
    #[must_use]
    pub fn sets_error_code(self, code: &[u8]) -> bool {
        let mut words = code
            .split(u8::is_ascii_whitespace)
            .filter(|w| !w.is_empty());
        if words.next() != Some(b"TCL") {
            return true;
        }
        let word = |w: &[u8]| std::str::from_utf8(w).ok().map(str::to_owned);
        match words.next().and_then(word) {
            Some(family) => {
                self.has_tcl_error_code(&family, words.next().and_then(word).as_deref())
            }
            None => true,
        }
    }

    /// The `-errorcode` a builtin raising `code` leaves under this release:
    /// `NONE` for a code this release does not set yet
    /// ([`Self::sets_error_code`]), and `code` otherwise.
    #[must_use]
    pub fn release_error_code(self, code: &[u8]) -> &[u8] {
        if self.sets_error_code(code) {
            code
        } else {
            b"NONE"
        }
    }

    /// Whether this release tags an expression's parse error with a
    /// `TCL PARSE EXPR …` error code. `ParseExpr` sets one from 8.6; 8.4's
    /// and 8.5's parsers set none, so `$errorCode` is `NONE`.
    #[must_use]
    pub fn tags_expression_parse_errors(self) -> bool {
        self.has_tcl_error_code("PARSE", Some("EXPR"))
    }

    /// Whether a script this release compiles fails whole on a compile
    /// procedure's error or a parse error, so that none of it runs (8.4's
    /// `TclCompileScript`). From 8.5 a compile procedure that cannot compile
    /// a command leaves it to run, and a parse error is raised where it lies.
    #[must_use]
    pub fn compile_errors_fail_scripts(self) -> bool {
        matches!(self, Self::V8_4)
    }

    /// Whether this release parses an expression with its own
    /// recursive-descent parser (`tclParseExpr.c`), whose syntax errors read
    /// `syntax error in expression "…": <reason>` and carry no `(parsing
    /// expression …)` frame. 8.5 replaced it.
    #[must_use]
    pub fn parses_expressions_as_8_4(self) -> bool {
        matches!(self, Self::V8_4)
    }

    /// Does this release revalidate bytecode that is already running? From
    /// 8.5 every compiled command starts with `INST_START_CMD`, which checks
    /// the compile epoch and evaluates the command's source afresh when a
    /// command-table change outdated it. 8.4 checks the epoch only when
    /// bytecode is entered (`TclCompEvalObj`), so running bytecode keeps the
    /// commands it compiled until its next entry.
    #[must_use]
    pub fn revalidates_running_bytecode(self) -> bool {
        !matches!(self, Self::V8_4)
    }
}
