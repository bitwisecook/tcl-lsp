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

//! Number representations: `math::bignum`, `math::complexnumbers`, `math::decimal` and `math::exact`.

use super::{Arity, Row};

/// The `math::bignum` package.
pub(super) const MATH__BIGNUM_CMDS: &[Row] = &[
    (
        "math::bignum::sign",
        Arity::exact(1),
        &["math::bignum::sign bignum"],
        "Return the sign of the bignum.",
    ),
    (
        "math::bignum::abs",
        Arity::exact(1),
        &["math::bignum::abs bignum"],
        "Return the absolute value of the bignum.",
    ),
    (
        "math::bignum::cmp",
        Arity::exact(2),
        &["math::bignum::cmp a b"],
        "Compare the two bignums a and b, returning 0 if a == b, 1 if a > b, and -1 if a < b.",
    ),
    (
        "math::bignum::iszero",
        Arity::exact(1),
        &["math::bignum::iszero bignum"],
        "Return true if bignum value is zero, otherwise false is returned.",
    ),
    (
        "math::bignum::lt",
        Arity::exact(2),
        &["math::bignum::lt a b"],
        "Return true if a < b, otherwise false is returned.",
    ),
    (
        "math::bignum::le",
        Arity::exact(2),
        &["math::bignum::le a b"],
        "Return true if a <= b, otherwise false is returned.",
    ),
    (
        "math::bignum::gt",
        Arity::exact(2),
        &["math::bignum::gt a b"],
        "Return true if a > b, otherwise false is returned.",
    ),
    (
        "math::bignum::ge",
        Arity::exact(2),
        &["math::bignum::ge a b"],
        "Return true if a >= b, otherwise false is returned.",
    ),
    (
        "math::bignum::eq",
        Arity::exact(2),
        &["math::bignum::eq a b"],
        "Return true if a == b, otherwise false is returned.",
    ),
    (
        "math::bignum::ne",
        Arity::exact(2),
        &["math::bignum::ne a b"],
        "Return true if a != b, otherwise false is returned.",
    ),
    (
        "math::bignum::isodd",
        Arity::exact(1),
        &["math::bignum::isodd bignum"],
        "Return true if bignum is odd.",
    ),
    (
        "math::bignum::iseven",
        Arity::exact(1),
        &["math::bignum::iseven bignum"],
        "Return true if bignum is even.",
    ),
    (
        "math::bignum::add",
        Arity::exact(2),
        &["math::bignum::add a b"],
        "Return the sum of the two bignums a and b.",
    ),
    (
        "math::bignum::sub",
        Arity::exact(2),
        &["math::bignum::sub a b"],
        "Return the difference of the two bignums a and b.",
    ),
    (
        "math::bignum::mul",
        Arity::exact(2),
        &["math::bignum::mul a b"],
        "Return the product of the two bignums a and b.",
    ),
    (
        "math::bignum::divqr",
        Arity::exact(2),
        &["math::bignum::divqr a b"],
        "Return a two-elements list containing as first element the quotient of the division between the two bignums a and",
    ),
    (
        "math::bignum::div",
        Arity::exact(2),
        &["math::bignum::div a b"],
        "Return the quotient of the division between the two bignums a and b.",
    ),
    (
        "math::bignum::rem",
        Arity::exact(2),
        &["math::bignum::rem a b"],
        "Return the remainder of the division between the two bignums a and b.",
    ),
    (
        "math::bignum::mod",
        Arity::exact(2),
        &["math::bignum::mod n m"],
        "Return n modulo m.",
    ),
    (
        "math::bignum::pow",
        Arity::exact(2),
        &["math::bignum::pow base exp"],
        "Return base raised to the exponent exp.",
    ),
    (
        "math::bignum::powm",
        Arity::exact(3),
        &["math::bignum::powm base exp m"],
        "Return base raised to the exponent exp, modulo m.",
    ),
    (
        "math::bignum::sqrt",
        Arity::exact(1),
        &["math::bignum::sqrt bignum"],
        "Return the integer part of the square root of bignum.",
    ),
    (
        "math::bignum::rand",
        Arity::exact(1),
        &["math::bignum::rand bits"],
        "Return a random number of at most bits bits.",
    ),
    (
        "math::bignum::lshift",
        Arity::exact(2),
        &["math::bignum::lshift bignum bits"],
        "Return the result of left shifting bignum's binary representation of bits positions on the left.",
    ),
    (
        "math::bignum::rshift",
        Arity::exact(2),
        &["math::bignum::rshift bignum bits"],
        "Return the result of right shifting bignum's binary representation of bits positions on the right.",
    ),
    (
        "math::bignum::bitand",
        Arity::exact(2),
        &["math::bignum::bitand a b"],
        "Return the result of doing a bitwise AND operation on a and b.",
    ),
    (
        "math::bignum::bitor",
        Arity::exact(2),
        &["math::bignum::bitor a b"],
        "Return the result of doing a bitwise OR operation on a and b.",
    ),
    (
        "math::bignum::bitxor",
        Arity::exact(2),
        &["math::bignum::bitxor a b"],
        "Return the result of doing a bitwise XOR operation on a and b.",
    ),
    (
        "math::bignum::setbit",
        Arity::exact(2),
        &["math::bignum::setbit bignumVar bit"],
        "Set the bit at bit position to 1 in the bignum stored in the variable bignumVar.",
    ),
    (
        "math::bignum::clearbit",
        Arity::exact(2),
        &["math::bignum::clearbit bignumVar bit"],
        "Set the bit at bit position to 0 in the bignum stored in the variable bignumVar.",
    ),
    (
        "math::bignum::testbit",
        Arity::exact(2),
        &["math::bignum::testbit bignum bit"],
        "Return true if the bit at the bit position of bignum is on, otherwise false is returned.",
    ),
    (
        "math::bignum::bits",
        Arity::exact(1),
        &["math::bignum::bits bignum"],
        "Return the number of bits needed to represent bignum in radix 2.",
    ),
];

/// The `math::complexnumbers` package.
pub(super) const MATH__COMPLEXNUMBERS_CMDS: &[Row] = &[
    (
        "math::complexnumbers::conj",
        Arity::exact(1),
        &["math::complexnumbers::conj z1"],
        "Return the conjugate of the given complex number Complex number in question.",
    ),
    (
        "math::complexnumbers::real",
        Arity::exact(1),
        &["math::complexnumbers::real z1"],
        "Return the real part of the given complex number Complex number in question.",
    ),
    (
        "math::complexnumbers::imag",
        Arity::exact(1),
        &["math::complexnumbers::imag z1"],
        "Return the imaginary part of the given complex number Complex number in question.",
    ),
    (
        "math::complexnumbers::mod",
        Arity::exact(1),
        &["math::complexnumbers::mod z1"],
        "Return the modulus of the given complex number Complex number in question.",
    ),
    (
        "math::complexnumbers::arg",
        Arity::exact(1),
        &["math::complexnumbers::arg z1"],
        "Return the argument (angle in radians) of the given complex number Complex number in question.",
    ),
    (
        "math::complexnumbers::complex",
        Arity::exact(2),
        &["math::complexnumbers::complex real imag"],
        "Construct the complex number real + imag*i and return it The real part of the new complex number The imaginary pa",
    ),
    (
        "math::complexnumbers::tostring",
        Arity::exact(1),
        &["math::complexnumbers::tostring z1"],
        "Convert the complex number to the form real + imag*i and return the string The complex number to be converted.",
    ),
    (
        "math::complexnumbers::exp",
        Arity::exact(1),
        &["math::complexnumbers::exp z1"],
        "Calculate the exponential for the given complex argument and return the result The complex argument for the funct",
    ),
    (
        "math::complexnumbers::sin",
        Arity::exact(1),
        &["math::complexnumbers::sin z1"],
        "Calculate the sine function for the given complex argument and return the result The complex argument for the fun",
    ),
    (
        "math::complexnumbers::cos",
        Arity::exact(1),
        &["math::complexnumbers::cos z1"],
        "Calculate the cosine function for the given complex argument and return the result The complex argument for the f",
    ),
    (
        "math::complexnumbers::tan",
        Arity::exact(1),
        &["math::complexnumbers::tan z1"],
        "Calculate the tangent function for the given complex argument and return the result The complex argument for the ",
    ),
    (
        "math::complexnumbers::log",
        Arity::exact(1),
        &["math::complexnumbers::log z1"],
        "Calculate the (principle value of the) logarithm for the given complex argument and return the result The complex",
    ),
    (
        "math::complexnumbers::sqrt",
        Arity::exact(1),
        &["math::complexnumbers::sqrt z1"],
        "Calculate the (principle value of the) square root for the given complex argument and return the result The compl",
    ),
    (
        "math::complexnumbers::pow",
        Arity::exact(2),
        &["math::complexnumbers::pow z1 z2"],
        "Calculate z1 to the power of z2 and return the result The complex number to be raised to a power The complex powe",
    ),
];

/// The `math::decimal` package.
pub(super) const MATH__DECIMAL_CMDS: &[Row] = &[
    (
        "math::decimal::fromstr",
        Arity::exact(1),
        &["math::decimal::fromstr string"],
        "Convert string into a decimal.",
    ),
    (
        "math::decimal::tostr",
        Arity::exact(1),
        &["math::decimal::tostr decimal"],
        "Convert decimal into a string representing the number in base 10.",
    ),
    (
        "math::decimal::setVariable",
        Arity::exact(2),
        &["math::decimal::setVariable variable setting"],
        "Sets the variable to setting.",
    ),
    (
        "math::decimal::add",
        Arity::exact(2),
        &["math::decimal::add a b"],
        "tcllib command.",
    ),
    (
        "math::decimal::subtract",
        Arity::exact(2),
        &["math::decimal::subtract a b"],
        "tcllib command.",
    ),
    (
        "math::decimal::multiply",
        Arity::exact(2),
        &["math::decimal::multiply a b"],
        "tcllib command.",
    ),
    (
        "math::decimal::divide",
        Arity::exact(2),
        &["math::decimal::divide a b"],
        "tcllib command.",
    ),
    (
        "math::decimal::divideint",
        Arity::exact(2),
        &["math::decimal::divideint a b"],
        "Return a the integer portion of the quotient of the division between decimals a and b.",
    ),
    (
        "math::decimal::remainder",
        Arity::exact(2),
        &["math::decimal::remainder a b"],
        "Return the remainder of the division between the two decimals a and b.",
    ),
    (
        "math::decimal::abs",
        Arity::exact(1),
        &["math::decimal::abs decimal"],
        "Return the absolute value of the decimal.",
    ),
    (
        "math::decimal::compare",
        Arity::exact(2),
        &["math::decimal::compare a b"],
        "Compare the two decimals a and b, returning 0 if a == b, 1 if a > b, and -1 if a < b.",
    ),
    (
        "math::decimal::max",
        Arity::exact(2),
        &["math::decimal::max a b"],
        "Compare the two decimals a and b, and return a if a >= b, and b if a < b.",
    ),
    (
        "math::decimal::maxmag",
        Arity::exact(2),
        &["math::decimal::maxmag a b"],
        "Compare the two decimals a and b while ignoring their signs, and return a if abs(a) >= abs(b), and b if abs(a) < ",
    ),
    (
        "math::decimal::min",
        Arity::exact(2),
        &["math::decimal::min a b"],
        "Compare the two decimals a and b, and return a if a <= b, and b if a > b.",
    ),
    (
        "math::decimal::minmag",
        Arity::exact(2),
        &["math::decimal::minmag a b"],
        "Compare the two decimals a and b while ignoring their signs, and return a if abs(a) <= abs(b), and b if abs(a) > ",
    ),
    (
        "math::decimal::plus",
        Arity::exact(1),
        &["math::decimal::plus a"],
        "Return the result from ::math::decimal::+ 0 $a.",
    ),
    (
        "math::decimal::minus",
        Arity::exact(1),
        &["math::decimal::minus a"],
        "Return the result from ::math::decimal::- 0 $a.",
    ),
    (
        "math::decimal::copynegate",
        Arity::exact(1),
        &["math::decimal::copynegate a"],
        "Returns a with the sign flipped.",
    ),
    (
        "math::decimal::copysign",
        Arity::exact(2),
        &["math::decimal::copysign a b"],
        "Returns a with the sign set to the sign of the b.",
    ),
    (
        "math::decimal::fma",
        Arity::exact(3),
        &["math::decimal::fma a b c"],
        "Return the result from first multiplying a by b and then adding c.",
    ),
    (
        "math::decimal::round_half_even",
        Arity::exact(2),
        &["math::decimal::round_half_even decimal digits"],
        "Rounds decimal to digits number of decimal points with the following rules: Round to the nearest.",
    ),
    (
        "math::decimal::round_half_up",
        Arity::exact(2),
        &["math::decimal::round_half_up decimal digits"],
        "Rounds decimal to digits number of decimal points with the following rules: Round to the nearest.",
    ),
    (
        "math::decimal::round_half_down",
        Arity::exact(2),
        &["math::decimal::round_half_down decimal digits"],
        "Rounds decimal to digits number of decimal points with the following rules: Round to the nearest.",
    ),
    (
        "math::decimal::round_down",
        Arity::exact(2),
        &["math::decimal::round_down decimal digits"],
        "Rounds decimal to digits number of decimal points with the following rules: Round toward 0.",
    ),
    (
        "math::decimal::round_up",
        Arity::exact(2),
        &["math::decimal::round_up decimal digits"],
        "Rounds decimal to digits number of decimal points with the following rules: Round away from 0.",
    ),
    (
        "math::decimal::round_floor",
        Arity::exact(2),
        &["math::decimal::round_floor decimal digits"],
        "Rounds decimal to digits number of decimal points with the following rules: Round toward -Infinity.",
    ),
    (
        "math::decimal::round_ceiling",
        Arity::exact(2),
        &["math::decimal::round_ceiling decimal digits"],
        "Rounds decimal to digits number of decimal points with the following rules: Round toward Infinity.",
    ),
    (
        "math::decimal::round_05up",
        Arity::exact(2),
        &["math::decimal::round_05up decimal digits"],
        "Rounds decimal to digits number of decimal points with the following rules: Round zero or five away from 0.",
    ),
];

/// The `math::exact` package.
pub(super) const MATH__EXACT_CMDS: &[Row] = &[(
    "math::exact::exactexpr",
    Arity::exact(1),
    &["math::exact::exactexpr expr"],
    "Accepts a mathematical expression in Tcl syntax, and returns an object that represents the program to calculate s",
)];
