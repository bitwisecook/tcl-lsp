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

//! tcllib `math::*` sub-package commands.
//!
//! Numeric sub-packages (geometry, linearalgebra, bignum, special, trig, calculus, decimal, figurate, …), each attributed to its namespace package.

use crate::prelude::*;

/// A row in a package command table: name, arity, synopsis, and summary.
type Row = (&'static str, Arity, &'static [&'static str], &'static str);

/// Build command specs for a package from its table.
fn rows(pkg: &'static str, table: &'static [Row]) -> Vec<CommandSpec> {
    table
        .iter()
        .map(|&(name, arity, synopsis, summary)| CommandSpec {
            name,
            arity,
            hover: Some(HoverSnippet::brief(summary, synopsis, "tcllib package")),
            tcllib_package: Some(pkg),
            required_package: Some(pkg),
            ..CommandSpec::DEFAULT
        })
        .collect()
}

/// The `math` package.
const MATH_CMDS: &[Row] = &[
    (
        "math::ln_Gamma",
        Arity::exact(1),
        &["math::ln_Gamma z"],
        "Returns the natural logarithm of the Gamma function for the argument z.",
    ),
    (
        "math::factorial",
        Arity::exact(1),
        &["math::factorial x"],
        "Returns the factorial of the argument x.",
    ),
    (
        "math::choose",
        Arity::exact(1),
        &["math::choose n k"],
        "Returns the binomial coefficient C(n, k) [example { C(n,k) = n!.",
    ),
    (
        "math::Beta",
        Arity::exact(1),
        &["math::Beta z w"],
        "Returns the Beta function of the parameters z and w.",
    ),
];

/// The `math::PCA` package.
const MATH__PCA_CMDS: &[Row] = &[(
    "math::PCA::createPCA",
    Arity::at_least(1),
    &["math::PCA::createPCA data args"],
    "Create a new object, based on the data that are passed via the data argument.",
)];

/// The `math::bignum` package.
const MATH__BIGNUM_CMDS: &[Row] = &[
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

/// The `math::calculus` package.
const MATH__CALCULUS_CMDS: &[Row] = &[
    (
        "math::calculus::integral",
        Arity::exact(4),
        &["math::calculus::integral begin end nosteps func"],
        "Determine the integral of the given function using the Simpson rule.",
    ),
    (
        "math::calculus::integralExpr",
        Arity::exact(4),
        &["math::calculus::integralExpr begin end nosteps expression"],
        "Similar to the previous proc, this one determines the integral of the given expression using the Simpson rule.",
    ),
    (
        "math::calculus::integral2D",
        Arity::exact(3),
        &["math::calculus::integral2D xinterval yinterval func"],
        "tcllib command.",
    ),
    (
        "math::calculus::integral2D_accurate",
        Arity::exact(3),
        &["math::calculus::integral2D_accurate xinterval yinterval func"],
        "The commands integral2D and integral2D_accurate calculate the integral of a function of two variables over the re",
    ),
    (
        "math::calculus::integral3D",
        Arity::exact(4),
        &["math::calculus::integral3D xinterval yinterval zinterval func"],
        "tcllib command.",
    ),
    (
        "math::calculus::integral3D_accurate",
        Arity::exact(4),
        &["math::calculus::integral3D_accurate xinterval yinterval zinterval func"],
        "The commands integral3D and integral3D_accurate are the three-dimensional equivalent of integral2D and integral3D",
    ),
    (
        "math::calculus::qk15",
        Arity::exact(4),
        &["math::calculus::qk15 xstart xend func nosteps"],
        "Determine the integral of the given function using the Gauss-Kronrod 15 points quadrature rule.",
    ),
    (
        "math::calculus::qk15_detailed",
        Arity::exact(4),
        &["math::calculus::qk15_detailed xstart xend func nosteps"],
        "Determine the integral of the given function using the Gauss-Kronrod 15 points quadrature rule.",
    ),
    (
        "math::calculus::eulerStep",
        Arity::exact(4),
        &["math::calculus::eulerStep t tstep xvec func"],
        "Set a single step in the numerical integration of a system of differential equations.",
    ),
    (
        "math::calculus::heunStep",
        Arity::exact(4),
        &["math::calculus::heunStep t tstep xvec func"],
        "Set a single step in the numerical integration of a system of differential equations.",
    ),
    (
        "math::calculus::rungeKuttaStep",
        Arity::exact(4),
        &["math::calculus::rungeKuttaStep t tstep xvec func"],
        "Set a single step in the numerical integration of a system of differential equations.",
    ),
    (
        "math::calculus::boundaryValueSecondOrder",
        Arity::exact(5),
        &["math::calculus::boundaryValueSecondOrder coeff_func force_func leftbnd rightbnd nostep"],
        "Solve a second order linear differential equation with boundary values at two sides.",
    ),
    (
        "math::calculus::solveTriDiagonal",
        Arity::exact(4),
        &["math::calculus::solveTriDiagonal acoeff bcoeff ccoeff dvalue"],
        "Solve a system of linear equations Ax = b with A a tridiagonal matrix.",
    ),
    (
        "math::calculus::newtonRaphson",
        Arity::exact(3),
        &["math::calculus::newtonRaphson func deriv initval"],
        "Determine the root of an equation given by func(x) = 0 using the method of Newton-Raphson.",
    ),
    (
        "math::calculus::newtonRaphsonParameters",
        Arity::exact(2),
        &["math::calculus::newtonRaphsonParameters maxiter tolerance"],
        "Set the numerical parameters for the Newton-Raphson method: Maximum number of iteration steps (defaults to 20) Re",
    ),
    (
        "math::calculus::regula_falsi",
        Arity::exact(4),
        &["math::calculus::regula_falsi f xb xe eps"],
        "Return an estimate of the zero or one of the zeros of the function contained in the interval .",
    ),
    (
        "math::calculus::root_bisection",
        Arity::exact(4),
        &["math::calculus::root_bisection f xb xe eps"],
        "Return an estimate of the zero or one of the zeros of the function contained in the interval .",
    ),
    (
        "math::calculus::root_secant",
        Arity::exact(4),
        &["math::calculus::root_secant f xb xe eps"],
        "Return an estimate of the zero or one of the zeros of the function contained in the interval .",
    ),
    (
        "math::calculus::root_brent",
        Arity::exact(4),
        &["math::calculus::root_brent f xb xe eps"],
        "Return an estimate of the zero or one of the zeros of the function contained in the interval .",
    ),
    (
        "math::calculus::root_chandrupatla",
        Arity::exact(4),
        &["math::calculus::root_chandrupatla f xb xe eps"],
        "Return an estimate of the zero or one of the zeros of the function contained in the interval .",
    ),
];

/// The `math::combinatorics` package.
const MATH__COMBINATORICS_CMDS: &[Row] = &[
    (
        "math::combinatorics::permutations",
        Arity::exact(1),
        &["math::combinatorics::permutations n"],
        "Return the number of permutations of n items.",
    ),
    (
        "math::combinatorics::variations",
        Arity::exact(2),
        &["math::combinatorics::variations n k"],
        "Return the number of variations k items selected from the total of n items.",
    ),
    (
        "math::combinatorics::combinations",
        Arity::exact(2),
        &["math::combinatorics::combinations n k"],
        "Return the number of combinations of k items selected from the total of n items.",
    ),
    (
        "math::combinatorics::derangements",
        Arity::exact(1),
        &["math::combinatorics::derangements n"],
        "Return the number of derangements of n items.",
    ),
    (
        "math::combinatorics::catalan",
        Arity::exact(1),
        &["math::combinatorics::catalan n"],
        "Return the n'th Catalan number.",
    ),
    (
        "math::combinatorics::firstStirling",
        Arity::exact(2),
        &["math::combinatorics::firstStirling n m"],
        "Calculate a Stirling number of the first kind (signed version, m cycles in a permutation of n items) Number of it",
    ),
    (
        "math::combinatorics::secondStirling",
        Arity::exact(2),
        &["math::combinatorics::secondStirling n m"],
        "Calculate a Stirling number of the second kind (m non-empty subsets from n items) Number of items Number of subse",
    ),
    (
        "math::combinatorics::partitionP",
        Arity::exact(1),
        &["math::combinatorics::partitionP n"],
        "Calculate the number of ways an integer n can be written as the sum of positive integers.",
    ),
];

/// The `math::complexnumbers` package.
const MATH__COMPLEXNUMBERS_CMDS: &[Row] = &[
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
const MATH__DECIMAL_CMDS: &[Row] = &[
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
const MATH__EXACT_CMDS: &[Row] = &[(
    "math::exact::exactexpr",
    Arity::exact(1),
    &["math::exact::exactexpr expr"],
    "Accepts a mathematical expression in Tcl syntax, and returns an object that represents the program to calculate s",
)];

/// The `math::figurate` package.
const MATH__FIGURATE_CMDS: &[Row] = &[
    (
        "math::figurate::sum_sequence",
        Arity::exact(1),
        &["math::figurate::sum_sequence n"],
        "Return the sum of integers 1, 2, ..., n.",
    ),
    (
        "math::figurate::sum_squares",
        Arity::exact(1),
        &["math::figurate::sum_squares n"],
        "Return the sum of squares 1**2, 2**2, ..., n**2.",
    ),
    (
        "math::figurate::sum_cubes",
        Arity::exact(1),
        &["math::figurate::sum_cubes n"],
        "Return the sum of cubes 1**3, 2**3, ..., n**3.",
    ),
    (
        "math::figurate::sum_4th_power",
        Arity::exact(1),
        &["math::figurate::sum_4th_power n"],
        "Return the sum of 4th powers 1**4, 2**4, ..., n**4.",
    ),
    (
        "math::figurate::sum_5th_power",
        Arity::exact(1),
        &["math::figurate::sum_5th_power n"],
        "Return the sum of 5th powers 1**5, 2**5, ..., n**5.",
    ),
    (
        "math::figurate::sum_6th_power",
        Arity::exact(1),
        &["math::figurate::sum_6th_power n"],
        "Return the sum of 6th powers 1**6, 2**6, ..., n**6.",
    ),
    (
        "math::figurate::sum_7th_power",
        Arity::exact(1),
        &["math::figurate::sum_7th_power n"],
        "Return the sum of 7th powers 1**7, 2**7, ..., n**7.",
    ),
    (
        "math::figurate::sum_8th_power",
        Arity::exact(1),
        &["math::figurate::sum_8th_power n"],
        "Return the sum of 8th powers 1**8, 2**8, ..., n**8.",
    ),
    (
        "math::figurate::sum_9th_power",
        Arity::exact(1),
        &["math::figurate::sum_9th_power n"],
        "Return the sum of 9th powers 1**9, 2**9, ..., n**9.",
    ),
    (
        "math::figurate::sum_10th_power",
        Arity::exact(1),
        &["math::figurate::sum_10th_power n"],
        "Return the sum of 10th powers 1**10, 2**10, ..., n**10.",
    ),
    (
        "math::figurate::sum_sequence_odd",
        Arity::exact(1),
        &["math::figurate::sum_sequence_odd n"],
        "Return the sum of odd integers 1, 3, ..., 2n-1 Highest integer in the sum.",
    ),
    (
        "math::figurate::sum_squares_odd",
        Arity::exact(1),
        &["math::figurate::sum_squares_odd n"],
        "Return the sum of odd squares 1**2, 3**2, ..., (2n-1)**2.",
    ),
    (
        "math::figurate::sum_cubes_odd",
        Arity::exact(1),
        &["math::figurate::sum_cubes_odd n"],
        "Return the sum of odd cubes 1**3, 3**3, ..., (2n-1)**3.",
    ),
    (
        "math::figurate::sum_4th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_4th_power_odd n"],
        "Return the sum of odd 4th powers 1**4, 2**4, ..., (2n-1)**4.",
    ),
    (
        "math::figurate::sum_5th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_5th_power_odd n"],
        "Return the sum of odd 5th powers 1**5, 2**5, ..., (2n-1)**5.",
    ),
    (
        "math::figurate::sum_6th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_6th_power_odd n"],
        "Return the sum of odd 6th powers 1**6, 2**6, ..., (2n-1)**6.",
    ),
    (
        "math::figurate::sum_7th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_7th_power_odd n"],
        "Return the sum of odd 7th powers 1**7, 2**7, ..., (2n-1)**7.",
    ),
    (
        "math::figurate::sum_8th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_8th_power_odd n"],
        "Return the sum of odd 8th powers 1**8, 2**8, ..., (2n-1)**8.",
    ),
    (
        "math::figurate::sum_9th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_9th_power_odd n"],
        "Return the sum of odd 9th powers 1**9, 2**9, ..., (2n-1)**9.",
    ),
    (
        "math::figurate::sum_10th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_10th_power_odd n"],
        "Return the sum of odd 10th powers 1**10, 2**10, ..., (2n-1)**10.",
    ),
    (
        "math::figurate::oblong",
        Arity::exact(1),
        &["math::figurate::oblong n"],
        "Return the nth oblong number (twice the nth triangular number) Required index.",
    ),
    (
        "math::figurate::pronic",
        Arity::exact(1),
        &["math::figurate::pronic n"],
        "Return the nth pronic number (synonym for oblong) Required index.",
    ),
    (
        "math::figurate::triangular",
        Arity::exact(1),
        &["math::figurate::triangular n"],
        "Return the nth triangular number Required index.",
    ),
    (
        "math::figurate::square",
        Arity::exact(1),
        &["math::figurate::square n"],
        "Return the nth square number Required index.",
    ),
    (
        "math::figurate::cubic",
        Arity::exact(1),
        &["math::figurate::cubic n"],
        "Return the nth cubic number Required index.",
    ),
    (
        "math::figurate::biquadratic",
        Arity::exact(1),
        &["math::figurate::biquadratic n"],
        "Return the nth biquaratic number (i.e.",
    ),
    (
        "math::figurate::centeredTriangular",
        Arity::exact(1),
        &["math::figurate::centeredTriangular n"],
        "Return the nth centered triangular number (items arranged in concentric squares) Required index.",
    ),
    (
        "math::figurate::centeredSquare",
        Arity::exact(1),
        &["math::figurate::centeredSquare n"],
        "Return the nth centered square number (items arranged in concentric squares) Required index.",
    ),
    (
        "math::figurate::centeredPentagonal",
        Arity::exact(1),
        &["math::figurate::centeredPentagonal n"],
        "Return the nth centered pentagonal number (items arranged in concentric pentagons) Required index.",
    ),
    (
        "math::figurate::centeredHexagonal",
        Arity::exact(1),
        &["math::figurate::centeredHexagonal n"],
        "Return the nth centered hexagonal number (items arranged in concentric hexagons) Required index.",
    ),
    (
        "math::figurate::centeredCube",
        Arity::exact(1),
        &["math::figurate::centeredCube n"],
        "Return the nth centered cube number (items arranged in concentric cubes) Required index.",
    ),
    (
        "math::figurate::decagonal",
        Arity::exact(1),
        &["math::figurate::decagonal n"],
        "Return the nth decagonal number (items arranged in decagons with one common vertex) Required index.",
    ),
    (
        "math::figurate::heptagonal",
        Arity::exact(1),
        &["math::figurate::heptagonal n"],
        "Return the nth heptagonal number (items arranged in heptagons with one common vertex) Required index.",
    ),
    (
        "math::figurate::hexagonal",
        Arity::exact(1),
        &["math::figurate::hexagonal n"],
        "Return the nth hexagonal number (items arranged in hexagons with one common vertex) Required index.",
    ),
    (
        "math::figurate::octagonal",
        Arity::exact(1),
        &["math::figurate::octagonal n"],
        "Return the nth octagonal number (items arranged in octagons with one common vertex) Required index.",
    ),
    (
        "math::figurate::octahedral",
        Arity::exact(1),
        &["math::figurate::octahedral n"],
        "Return the nth octahedral number (items arranged in octahedrons with a common centre) Required index.",
    ),
    (
        "math::figurate::pentagonal",
        Arity::exact(1),
        &["math::figurate::pentagonal n"],
        "Return the nth pentagonal number (items arranged in pentagons with one common vertex) Required index.",
    ),
    (
        "math::figurate::squarePyramidral",
        Arity::exact(1),
        &["math::figurate::squarePyramidral n"],
        "Return the nth square pyramidral number (items arranged in a square pyramid) Required index.",
    ),
    (
        "math::figurate::tetrahedral",
        Arity::exact(1),
        &["math::figurate::tetrahedral n"],
        "Return the nth tetrahedral number (items arranged in a triangular pyramid) Required index.",
    ),
    (
        "math::figurate::pentatope",
        Arity::exact(1),
        &["math::figurate::pentatope n"],
        "Return the nth pentatope number (items arranged in the four-dimensional analogue of a triangular pyramid) Require",
    ),
];

/// The `math::filters` package.
const MATH__FILTERS_CMDS: &[Row] = &[
    (
        "math::filters::filterButterworth",
        Arity::exact(4),
        &["math::filters::filterButterworth lowpass order samplefreq cutofffreq"],
        "Determine the coefficients for a Butterworth filter of given order.",
    ),
    (
        "math::filters::filter",
        Arity::exact(2),
        &["math::filters::filter coeffs data"],
        "Filter the entire data series based on the filter coefficients.",
    ),
];

/// The `math::fourier` package.
const MATH__FOURIER_CMDS: &[Row] = &[
    (
        "math::fourier::dft",
        Arity::exact(1),
        &["math::fourier::dft in_data"],
        "Determine the Fourier transform of the given list of complex numbers.",
    ),
    (
        "math::fourier::inverse_dft",
        Arity::exact(1),
        &["math::fourier::inverse_dft in_data"],
        "Determine the inverse Fourier transform of the given list of complex numbers (interpreted as amplitudes).",
    ),
    (
        "math::fourier::lowpass",
        Arity::exact(2),
        &["math::fourier::lowpass cutoff in_data"],
        "Filter the (complex) amplitudes so that high-frequency components are suppressed.",
    ),
    (
        "math::fourier::highpass",
        Arity::exact(2),
        &["math::fourier::highpass cutoff in_data"],
        "Filter the (complex) amplitudes so that low-frequency components are suppressed.",
    ),
];

/// The `math::geometry` package.
const MATH__GEOMETRY_CMDS: &[Row] = &[
    (
        "math::geometry::p",
        Arity::exact(2),
        &["math::geometry::p x y"],
        "Construct a point from its coordinates and return it as the result of the command.",
    ),
    (
        "math::geometry::distance",
        Arity::exact(2),
        &["math::geometry::distance point1 point2"],
        "Compute the distance between the two points and return it as the result of the command.",
    ),
    (
        "math::geometry::length",
        Arity::exact(1),
        &["math::geometry::length point"],
        "Compute the length of the vector and return it as the result of the command.",
    ),
    (
        "math::geometry::direction",
        Arity::exact(1),
        &["math::geometry::direction angle"],
        "Given the angle in degrees this command computes and returns the unit vector pointing into this direction.",
    ),
    (
        "math::geometry::h",
        Arity::exact(1),
        &["math::geometry::h length"],
        "Returns a horizontal vector on the X-axis of the specified length.",
    ),
    (
        "math::geometry::v",
        Arity::exact(1),
        &["math::geometry::v length"],
        "Returns a vertical vector on the Y-axis of the specified length.",
    ),
    (
        "math::geometry::between",
        Arity::exact(3),
        &["math::geometry::between point1 point2 s"],
        "Compute the point which is at relative distance s between the two points and return it as the result of the comma",
    ),
    (
        "math::geometry::octant",
        Arity::exact(1),
        &["math::geometry::octant point"],
        "Compute the octant of the circle the point is in and return it as the result of the command.",
    ),
    (
        "math::geometry::rect",
        Arity::exact(2),
        &["math::geometry::rect nw se"],
        "Construct a rectangle from its northwest and southeast corners and return it as the result of the command.",
    ),
    (
        "math::geometry::nwse",
        Arity::exact(1),
        &["math::geometry::nwse rect"],
        "Extract the northwest and southeast corners of the rectangle and return them as the result of the command (a 2-el",
    ),
    (
        "math::geometry::angle",
        Arity::exact(1),
        &["math::geometry::angle line"],
        "Calculate the angle from the positive x-axis to a given line (in two dimensions only).",
    ),
    (
        "math::geometry::angleBetween",
        Arity::exact(2),
        &["math::geometry::angleBetween vector1 vector2"],
        "Calculate the angle between two vectors (in degrees) First vector Second vector.",
    ),
    (
        "math::geometry::inproduct",
        Arity::exact(2),
        &["math::geometry::inproduct vector1 vector2"],
        "Calculate the inner product of two vectors First vector Second vector.",
    ),
    (
        "math::geometry::areaParallellogram",
        Arity::exact(2),
        &["math::geometry::areaParallellogram vector1 vector2"],
        "Calculate the area of the parallellogram with the two vectors as its sides First vector Second vector.",
    ),
    (
        "math::geometry::calculateDistanceToLine",
        Arity::exact(2),
        &["math::geometry::calculateDistanceToLine P line"],
        "Calculate the distance of point P to the (infinite) line and return the result List of two numbers, the coordinat",
    ),
    (
        "math::geometry::calculateDistanceToLineSegment",
        Arity::exact(2),
        &["math::geometry::calculateDistanceToLineSegment P linesegment"],
        "Calculate the distance of point P to the (finite) line segment and return the result.",
    ),
    (
        "math::geometry::calculateDistanceToPolyline",
        Arity::exact(2),
        &["math::geometry::calculateDistanceToPolyline P polyline"],
        "Calculate the distance of point P to the polyline and return the result.",
    ),
    (
        "math::geometry::calculateDistanceToPolygon",
        Arity::exact(2),
        &["math::geometry::calculateDistanceToPolygon P polygon"],
        "Calculate the distance of point P to the polygon and return the result.",
    ),
    (
        "math::geometry::findClosestPointOnLine",
        Arity::exact(2),
        &["math::geometry::findClosestPointOnLine P line"],
        "Return the point on a line which is closest to a given point.",
    ),
    (
        "math::geometry::findClosestPointOnLineSegment",
        Arity::exact(2),
        &["math::geometry::findClosestPointOnLineSegment P linesegment"],
        "Return the point on a line segment which is closest to a given point.",
    ),
    (
        "math::geometry::findClosestPointOnPolyline",
        Arity::exact(2),
        &["math::geometry::findClosestPointOnPolyline P polyline"],
        "Return the point on a polyline which is closest to a given point.",
    ),
    (
        "math::geometry::lengthOfPolyline",
        Arity::exact(1),
        &["math::geometry::lengthOfPolyline polyline"],
        "Return the length of the polyline (note: it not regarded as a polygon) List of numbers, the vertices of the polyl",
    ),
    (
        "math::geometry::movePointInDirection",
        Arity::exact(3),
        &["math::geometry::movePointInDirection P direction dist"],
        "Move a point over a given distance in a given direction and return the new coordinates (in two dimensions only).",
    ),
    (
        "math::geometry::lineSegmentsIntersect",
        Arity::exact(2),
        &["math::geometry::lineSegmentsIntersect linesegment1 linesegment2"],
        "Check if two line segments intersect or coincide.",
    ),
    (
        "math::geometry::findLineSegmentIntersection",
        Arity::exact(2),
        &["math::geometry::findLineSegmentIntersection linesegment1 linesegment2"],
        "Find the intersection point of two line segments.",
    ),
    (
        "math::geometry::findLineIntersection",
        Arity::exact(2),
        &["math::geometry::findLineIntersection line1 line2"],
        "Find the intersection point of two (infinite) lines.",
    ),
    (
        "math::geometry::polylinesIntersect",
        Arity::exact(2),
        &["math::geometry::polylinesIntersect polyline1 polyline2"],
        "Check if two polylines intersect or not (in two dimensions only).",
    ),
    (
        "math::geometry::polylinesBoundingIntersect",
        Arity::exact(3),
        &["math::geometry::polylinesBoundingIntersect polyline1 polyline2 granularity"],
        "Check whether two polylines intersect, but reduce the correctness of the result to the given granularity.",
    ),
    (
        "math::geometry::intervalsOverlap",
        Arity::exact(5),
        &["math::geometry::intervalsOverlap y1 y2 y3 y4 strict"],
        "Check if two intervals overlap.",
    ),
    (
        "math::geometry::rectanglesOverlap",
        Arity::exact(5),
        &["math::geometry::rectanglesOverlap P1 P2 Q1 Q2 strict"],
        "Check if two rectangles overlap.",
    ),
    (
        "math::geometry::bbox",
        Arity::exact(1),
        &["math::geometry::bbox polyline"],
        "Calculate the bounding box of a polyline.",
    ),
    (
        "math::geometry::overlapBBox",
        Arity::at_least(2),
        &["math::geometry::overlapBBox polyline1 polyline2 strict"],
        "Check if the bounding boxes of two polylines overlap or not.",
    ),
    (
        "math::geometry::pointInsideBBox",
        Arity::exact(2),
        &["math::geometry::pointInsideBBox bbox point"],
        "Check if the point is inside or on the bounding box or not.",
    ),
    (
        "math::geometry::cathetusPoint",
        Arity::at_least(3),
        &["math::geometry::cathetusPoint pa pb cathetusLength location"],
        "Return the third point of the rectangular triangle defined by the two given end points of the hypothenusa.",
    ),
    (
        "math::geometry::parallel",
        Arity::at_least(2),
        &["math::geometry::parallel line offset orient"],
        "Return a line parallel to the given line, with a distance offset.",
    ),
    (
        "math::geometry::unitVector",
        Arity::exact(1),
        &["math::geometry::unitVector line"],
        "Return a unit vector from the given line or direction, if the line argument is a single point (then a line throug",
    ),
    (
        "math::geometry::pointInsidePolygon",
        Arity::exact(2),
        &["math::geometry::pointInsidePolygon P polyline"],
        "Determine if a point is completely inside a polygon.",
    ),
    (
        "math::geometry::pointInsidePolygonAlt",
        Arity::exact(2),
        &["math::geometry::pointInsidePolygonAlt P polyline"],
        "Determine if a point is completely inside a polygon.",
    ),
    (
        "math::geometry::rectangleInsidePolygon",
        Arity::exact(3),
        &["math::geometry::rectangleInsidePolygon P1 P2 polyline"],
        "Determine if a rectangle is completely inside a polygon.",
    ),
    (
        "math::geometry::areaPolygon",
        Arity::exact(1),
        &["math::geometry::areaPolygon polygon"],
        "Calculate the area of a polygon.",
    ),
    (
        "math::geometry::translate",
        Arity::exact(2),
        &["math::geometry::translate vector polyline"],
        "Translate a polyline over a given vector Translation vector The polyline to be translated.",
    ),
    (
        "math::geometry::rotate",
        Arity::exact(2),
        &["math::geometry::rotate angle polyline"],
        "Rotate a polyline over a given angle (degrees) around the origin Angle over which to rotate the polyline (degrees",
    ),
    (
        "math::geometry::rotateAbout",
        Arity::exact(3),
        &["math::geometry::rotateAbout p angle polyline"],
        "Rotate a polyline around a given point p and return the new polyline.",
    ),
    (
        "math::geometry::reflect",
        Arity::exact(2),
        &["math::geometry::reflect angle polyline"],
        "Reflect a polyline in a line through the origin at a given angle (degrees) to the x-axis Angle of the line of ref",
    ),
    (
        "math::geometry::degToRad",
        Arity::exact(1),
        &["math::geometry::degToRad angle"],
        "Convert from degrees to radians Angle in degrees.",
    ),
    (
        "math::geometry::radToDeg",
        Arity::exact(1),
        &["math::geometry::radToDeg angle"],
        "Convert from radians to degrees Angle in radians.",
    ),
    (
        "math::geometry::circle",
        Arity::exact(2),
        &["math::geometry::circle centre radius"],
        "Convenience procedure to create a circle from a point and a radius.",
    ),
    (
        "math::geometry::circleTwoPoints",
        Arity::exact(2),
        &["math::geometry::circleTwoPoints point1 point2"],
        "Convenience procedure to create a circle from two points on its circumference The centre is the point between the",
    ),
    (
        "math::geometry::pointInsideCircle",
        Arity::exact(2),
        &["math::geometry::pointInsideCircle point circle"],
        "Determine if the given point is inside the circle or on the circumference (1) or outside (0).",
    ),
    (
        "math::geometry::lineIntersectsCircle",
        Arity::exact(2),
        &["math::geometry::lineIntersectsCircle line circle"],
        "Determine if the given line intersects the circle or touches it (1) or does not (0).",
    ),
    (
        "math::geometry::lineSegmentIntersectsCircle",
        Arity::exact(2),
        &["math::geometry::lineSegmentIntersectsCircle segment circle"],
        "Determine if the given line segment intersects the circle or touches it (1) or does not (0).",
    ),
    (
        "math::geometry::intersectionLineWithCircle",
        Arity::exact(2),
        &["math::geometry::intersectionLineWithCircle line circle"],
        "Determine the points at which the given line intersects the circle.",
    ),
    (
        "math::geometry::intersectionCircleWithCircle",
        Arity::exact(2),
        &["math::geometry::intersectionCircleWithCircle circle1 circle2"],
        "Determine the points at which the given two circles intersect.",
    ),
    (
        "math::geometry::tangentLinesToCircle",
        Arity::exact(2),
        &["math::geometry::tangentLinesToCircle point circle"],
        "Determine the tangent lines from the given point to the circle.",
    ),
    (
        "math::geometry::intersectionPolylines",
        Arity::at_least(2),
        &["math::geometry::intersectionPolylines polyline1 polyline2 mode granularity"],
        "Return the first point or all points where the two polylines intersect.",
    ),
    (
        "math::geometry::intersectionPolylineCircle",
        Arity::at_least(2),
        &["math::geometry::intersectionPolylineCircle polyline circle mode granularity"],
        "Return the first point or all points where the polyline intersects the circle.",
    ),
    (
        "math::geometry::polylineCutOrigin",
        Arity::at_least(2),
        &["math::geometry::polylineCutOrigin polyline1 polyline2 granularity"],
        "Return the part of the first polyline from the origin up to the first intersection with the second.",
    ),
    (
        "math::geometry::polylineCutEnd",
        Arity::at_least(2),
        &["math::geometry::polylineCutEnd polyline1 polyline2 granularity"],
        "Return the part of the first polyline from the last intersection point with the second to the end.",
    ),
    (
        "math::geometry::splitPolyline",
        Arity::exact(2),
        &["math::geometry::splitPolyline polyline numberVertex"],
        "Split the poyline into a set of polylines where each separate polyline holds numberVertex vertices between the tw",
    ),
    (
        "math::geometry::enrichPolyline",
        Arity::exact(2),
        &["math::geometry::enrichPolyline polyline accuracy"],
        "Split up each segment of a polyline into a number of smaller segments and return the result.",
    ),
    (
        "math::geometry::cleanupPolyline",
        Arity::exact(1),
        &["math::geometry::cleanupPolyline polyline"],
        "Remove duplicate neighbouring vertices and return the result.",
    ),
];

/// The `math::interpolate` package.
const MATH__INTERPOLATE_CMDS: &[Row] = &[
    (
        "math::interpolate::defineTable",
        Arity::exact(3),
        &["math::interpolate::defineTable name colnames values"],
        "Define a table with one or two independent variables (the distinction is implicit in the data).",
    ),
    (
        "math::interpolate::neville",
        Arity::exact(3),
        &["math::interpolate::neville xlist ylist x"],
        "Interpolates between the tabulated values of a function whose abscissae are xlist and whose ordinates are ylist t",
    ),
];

/// The `math::linearalgebra` package.
const MATH__LINEARALGEBRA_CMDS: &[Row] = &[
    (
        "math::linearalgebra::mkVector",
        Arity::exact(2),
        &["math::linearalgebra::mkVector ndim value"],
        "Create a vector with ndim elements, each with the value value.",
    ),
    (
        "math::linearalgebra::mkUnitVector",
        Arity::exact(2),
        &["math::linearalgebra::mkUnitVector ndim ndir"],
        "Create a unit vector in ndim-dimensional space, along the ndir-th direction.",
    ),
    (
        "math::linearalgebra::mkMatrix",
        Arity::exact(3),
        &["math::linearalgebra::mkMatrix nrows ncols value"],
        "Create a matrix with nrows rows and ncols columns.",
    ),
    (
        "math::linearalgebra::getrow",
        Arity::at_least(2),
        &["math::linearalgebra::getrow matrix row imin imax"],
        "Returns a single row of a matrix as a list Matrix in question Index of the row to return Minimum index of the col",
    ),
    (
        "math::linearalgebra::setrow",
        Arity::at_least(3),
        &["math::linearalgebra::setrow matrix row newvalues imin imax"],
        "Set a single row of a matrix to new values (this list must have the same number of elements as the number of colu",
    ),
    (
        "math::linearalgebra::getcol",
        Arity::at_least(2),
        &["math::linearalgebra::getcol matrix col imin imax"],
        "Returns a single column of a matrix as a list Matrix in question Index of the column to return Minimum index of t",
    ),
    (
        "math::linearalgebra::setcol",
        Arity::at_least(3),
        &["math::linearalgebra::setcol matrix col newvalues imin imax"],
        "Set a single column of a matrix to new values (this list must have the same number of elements as the number of r",
    ),
    (
        "math::linearalgebra::getelem",
        Arity::exact(3),
        &["math::linearalgebra::getelem matrix row col"],
        "Returns a single element of a matrix/vector Matrix or vector in question Row of the element Column of the element",
    ),
    (
        "math::linearalgebra::setelem",
        Arity::at_least(3),
        &["math::linearalgebra::setelem matrix row col newvalue"],
        "Set a single element of a matrix (or vector) to a new value name of the matrix in question Row of the element Col",
    ),
    (
        "math::linearalgebra::swaprows",
        Arity::at_least(3),
        &["math::linearalgebra::swaprows matrix irow1 irow2 imin imax"],
        "Swap two rows in a matrix completely or only a selected part name of the matrix in question Index of first row In",
    ),
    (
        "math::linearalgebra::swapcols",
        Arity::at_least(3),
        &["math::linearalgebra::swapcols matrix icol1 icol2 imin imax"],
        "Swap two columns in a matrix completely or only a selected part name of the matrix in question Index of first col",
    ),
    (
        "math::linearalgebra::show",
        Arity::at_least(1),
        &["math::linearalgebra::show obj format rowsep colsep"],
        "Return a string representing the vector or matrix, for easy printing.",
    ),
    (
        "math::linearalgebra::dim",
        Arity::exact(1),
        &["math::linearalgebra::dim obj"],
        "Returns the number of dimensions for the object (either 0 for a scalar, 1 for a vector and 2 for a matrix) Scalar",
    ),
    (
        "math::linearalgebra::shape",
        Arity::exact(1),
        &["math::linearalgebra::shape obj"],
        "Returns the number of elements in each dimension for the object (either an empty list for a scalar, a single numb",
    ),
    (
        "math::linearalgebra::conforming",
        Arity::exact(3),
        &["math::linearalgebra::conforming type obj1 obj2"],
        "Checks if two objects (vector or matrix) have conforming shapes, that is if they can be applied in an operation l",
    ),
    (
        "math::linearalgebra::symmetric",
        Arity::at_least(1),
        &["math::linearalgebra::symmetric matrix eps"],
        "Checks if the given (square) matrix is symmetric.",
    ),
    (
        "math::linearalgebra::norm",
        Arity::exact(2),
        &["math::linearalgebra::norm vector type"],
        "Returns the norm of the given vector.",
    ),
    (
        "math::linearalgebra::norm_one",
        Arity::exact(1),
        &["math::linearalgebra::norm_one vector"],
        "Returns the L1 norm of the given vector, the sum of absolute values Vector, list of coefficients.",
    ),
    (
        "math::linearalgebra::norm_two",
        Arity::exact(1),
        &["math::linearalgebra::norm_two vector"],
        "Returns the L2 norm of the given vector, the ordinary Euclidean norm Vector, list of coefficients.",
    ),
    (
        "math::linearalgebra::norm_max",
        Arity::at_least(1),
        &["math::linearalgebra::norm_max vector index"],
        "Returns the Linf norm of the given vector, the maximum absolute coefficient Vector, list of coefficients (optiona",
    ),
    (
        "math::linearalgebra::normMatrix",
        Arity::exact(2),
        &["math::linearalgebra::normMatrix matrix type"],
        "Returns the norm of the given matrix.",
    ),
    (
        "math::linearalgebra::dotproduct",
        Arity::exact(2),
        &["math::linearalgebra::dotproduct vect1 vect2"],
        "Determine the inproduct or dot product of two vectors.",
    ),
    (
        "math::linearalgebra::unitLengthVector",
        Arity::exact(1),
        &["math::linearalgebra::unitLengthVector vector"],
        "Return a vector in the same direction with length 1.",
    ),
    (
        "math::linearalgebra::normalizeStat",
        Arity::exact(1),
        &["math::linearalgebra::normalizeStat mv"],
        "Normalize the matrix or vector in a statistical sense: the mean of the elements of the columns of the result is z",
    ),
    (
        "math::linearalgebra::axpy",
        Arity::exact(3),
        &["math::linearalgebra::axpy scale mv1 mv2"],
        "Return a vector or matrix that results from a daxpy operation, that is: compute a*x+y (a a scalar and x and y bot",
    ),
    (
        "math::linearalgebra::add",
        Arity::exact(2),
        &["math::linearalgebra::add mv1 mv2"],
        "Return a vector or matrix that is the sum of the two arguments (x+y) Specialised variants are: add_vect and add_m",
    ),
    (
        "math::linearalgebra::sub",
        Arity::exact(2),
        &["math::linearalgebra::sub mv1 mv2"],
        "Return a vector or matrix that is the difference of the two arguments (x-y) Specialised variants are: sub_vect an",
    ),
    (
        "math::linearalgebra::scale",
        Arity::exact(2),
        &["math::linearalgebra::scale scale mv"],
        "Scale a vector or matrix and return the result, that is: compute a*x.",
    ),
    (
        "math::linearalgebra::rotate",
        Arity::exact(4),
        &["math::linearalgebra::rotate c s vect1 vect2"],
        "Apply a planar rotation to two vectors and return the result as a list of two vectors: c*x-s*y and s*x+c*y.",
    ),
    (
        "math::linearalgebra::transpose",
        Arity::exact(1),
        &["math::linearalgebra::transpose matrix"],
        "Transpose a matrix Matrix to be transposed.",
    ),
    (
        "math::linearalgebra::matmul",
        Arity::exact(2),
        &["math::linearalgebra::matmul mv1 mv2"],
        "Multiply a vector/matrix with another vector/matrix.",
    ),
    (
        "math::linearalgebra::angle",
        Arity::exact(2),
        &["math::linearalgebra::angle vect1 vect2"],
        "Compute the angle between two vectors (in radians) First vector Second vector.",
    ),
    (
        "math::linearalgebra::crossproduct",
        Arity::exact(2),
        &["math::linearalgebra::crossproduct vect1 vect2"],
        "Compute the cross product of two (three-dimensional) vectors First vector Second vector.",
    ),
    (
        "math::linearalgebra::mkIdentity",
        Arity::exact(1),
        &["math::linearalgebra::mkIdentity size"],
        "Create an identity matrix of dimension size.",
    ),
    (
        "math::linearalgebra::mkDiagonal",
        Arity::exact(1),
        &["math::linearalgebra::mkDiagonal diag"],
        "Create a diagonal matrix whose diagonal elements are the elements of the vector diag.",
    ),
    (
        "math::linearalgebra::mkRandom",
        Arity::exact(1),
        &["math::linearalgebra::mkRandom size"],
        "Create a square matrix whose elements are uniformly distributed random numbers between 0 and 1 of dimension size.",
    ),
    (
        "math::linearalgebra::mkTriangular",
        Arity::at_least(1),
        &["math::linearalgebra::mkTriangular size uplo value"],
        "Create a triangular matrix with non-zero elements in the upper or lower part, depending on argument uplo.",
    ),
    (
        "math::linearalgebra::mkHilbert",
        Arity::exact(1),
        &["math::linearalgebra::mkHilbert size"],
        "Create a Hilbert matrix of dimension size.",
    ),
    (
        "math::linearalgebra::mkDingdong",
        Arity::exact(1),
        &["math::linearalgebra::mkDingdong size"],
        "Create a dingdong matrix of dimension size.",
    ),
    (
        "math::linearalgebra::mkOnes",
        Arity::exact(1),
        &["math::linearalgebra::mkOnes size"],
        "Create a square matrix of dimension size whose entries are all 1.",
    ),
    (
        "math::linearalgebra::mkMoler",
        Arity::exact(1),
        &["math::linearalgebra::mkMoler size"],
        "Create a Moler matrix of size size.",
    ),
    (
        "math::linearalgebra::mkFrank",
        Arity::exact(1),
        &["math::linearalgebra::mkFrank size"],
        "Create a Frank matrix of size size.",
    ),
    (
        "math::linearalgebra::mkBorder",
        Arity::exact(1),
        &["math::linearalgebra::mkBorder size"],
        "Create a bordered matrix of size size.",
    ),
    (
        "math::linearalgebra::solveGauss",
        Arity::exact(2),
        &["math::linearalgebra::solveGauss matrix bvect"],
        "Solve a system of linear equations (Ax=b) using Gauss elimination.",
    ),
    (
        "math::linearalgebra::solvePGauss",
        Arity::exact(2),
        &["math::linearalgebra::solvePGauss matrix bvect"],
        "Solve a system of linear equations (Ax=b) using Gauss elimination with partial pivoting.",
    ),
    (
        "math::linearalgebra::solveTriangular",
        Arity::at_least(2),
        &["math::linearalgebra::solveTriangular matrix bvect uplo"],
        "Solve a system of linear equations (Ax=b) by backward substitution.",
    ),
    (
        "math::linearalgebra::solveGaussBand",
        Arity::exact(2),
        &["math::linearalgebra::solveGaussBand matrix bvect"],
        "Solve a system of linear equations (Ax=b) using Gauss elimination, where the matrix is stored as a band matrix (c",
    ),
    (
        "math::linearalgebra::solveTriangularBand",
        Arity::exact(2),
        &["math::linearalgebra::solveTriangularBand matrix bvect"],
        "Solve a system of linear equations (Ax=b) by backward substitution.",
    ),
    (
        "math::linearalgebra::determineSVD",
        Arity::exact(2),
        &["math::linearalgebra::determineSVD A eps"],
        "Determines the Singular Value Decomposition of a matrix: A = U S Vtrans.",
    ),
    (
        "math::linearalgebra::eigenvectorsSVD",
        Arity::exact(2),
        &["math::linearalgebra::eigenvectorsSVD A eps"],
        "Determines the eigenvectors and eigenvalues of a real symmetric matrix, using SVD.",
    ),
    (
        "math::linearalgebra::leastSquaresSVD",
        Arity::exact(4),
        &["math::linearalgebra::leastSquaresSVD A y qmin eps"],
        "Determines the solution to a least-sqaures problem Ax ~ y via singular value decomposition.",
    ),
    (
        "math::linearalgebra::choleski",
        Arity::exact(1),
        &["math::linearalgebra::choleski matrix"],
        "Determine the Choleski decomposition of a symmetric positive semidefinite matrix (this condition is not checked!)",
    ),
    (
        "math::linearalgebra::orthonormalizeColumns",
        Arity::exact(1),
        &["math::linearalgebra::orthonormalizeColumns matrix"],
        "Use the modified Gram-Schmidt method to orthogonalize and normalize the columns of the given matrix and return th",
    ),
    (
        "math::linearalgebra::orthonormalizeRows",
        Arity::exact(1),
        &["math::linearalgebra::orthonormalizeRows matrix"],
        "Use the modified Gram-Schmidt method to orthogonalize and normalize the rows of the given matrix and return the r",
    ),
    (
        "math::linearalgebra::dger",
        Arity::at_least(4),
        &["math::linearalgebra::dger matrix alpha x y scope"],
        "Perform the rank 1 operation A + alpha*x*y' inline (that is: the matrix A is adjusted).",
    ),
    (
        "math::linearalgebra::dgetrf",
        Arity::exact(1),
        &["math::linearalgebra::dgetrf matrix"],
        "Computes an LU factorization of a general matrix, using partial, pivoting with row interchanges.",
    ),
    (
        "math::linearalgebra::det",
        Arity::exact(1),
        &["math::linearalgebra::det matrix"],
        "Returns the determinant of the given matrix, based on PA=LU decomposition, i.e.",
    ),
    (
        "math::linearalgebra::largesteigen",
        Arity::exact(3),
        &["math::linearalgebra::largesteigen matrix tolerance maxiter"],
        "Returns a list made of the largest eigenvalue (in magnitude) and associated eigenvector.",
    ),
    (
        "math::linearalgebra::to_LA",
        Arity::exact(1),
        &["math::linearalgebra::to_LA mv"],
        "Transforms a vector or matrix into the format used by the original LA package.",
    ),
    (
        "math::linearalgebra::from_LA",
        Arity::exact(1),
        &["math::linearalgebra::from_LA mv"],
        "Transforms a vector or matrix from the format used by the original LA package into the format used by the present",
    ),
];

/// The `math::optimize` package.
const MATH__OPTIMIZE_CMDS: &[Row] = &[
    (
        "math::optimize::minimum",
        Arity::exact(4),
        &["math::optimize::minimum begin end func maxerr"],
        "Minimize the given (continuous) function by examining the values in the given interval.",
    ),
    (
        "math::optimize::maximum",
        Arity::exact(4),
        &["math::optimize::maximum begin end func maxerr"],
        "Maximize the given (continuous) function by examining the values in the given interval.",
    ),
    (
        "math::optimize::solveLinearProgram",
        Arity::exact(2),
        &["math::optimize::solveLinearProgram objective constraints"],
        "Solve a linear program in standard form using a straightforward implementation of the Simplex algorithm.",
    ),
    (
        "math::optimize::linearProgramMaximum",
        Arity::exact(2),
        &["math::optimize::linearProgramMaximum objective result"],
        "Convenience function to return the maximum for the solution found by the solveLinearProgram procedure.",
    ),
];

/// The `math::polynomials` package.
const MATH__POLYNOMIALS_CMDS: &[Row] = &[
    (
        "math::polynomials::polynomial",
        Arity::exact(1),
        &["math::polynomials::polynomial coeffs"],
        "Return an (encoded) list that defines the polynomial.",
    ),
    (
        "math::polynomials::polynCmd",
        Arity::exact(1),
        &["math::polynomials::polynCmd coeffs"],
        "Create a new procedure that evaluates the polynomial.",
    ),
    (
        "math::polynomials::evalPolyn",
        Arity::exact(2),
        &["math::polynomials::evalPolyn polynomial x"],
        "Evaluate the polynomial at x.",
    ),
    (
        "math::polynomials::addPolyn",
        Arity::exact(2),
        &["math::polynomials::addPolyn polyn1 polyn2"],
        "Return a new polynomial which is the sum of the two others.",
    ),
    (
        "math::polynomials::subPolyn",
        Arity::exact(2),
        &["math::polynomials::subPolyn polyn1 polyn2"],
        "Return a new polynomial which is the difference of the two others.",
    ),
    (
        "math::polynomials::multPolyn",
        Arity::exact(2),
        &["math::polynomials::multPolyn polyn1 polyn2"],
        "Return a new polynomial which is the product of the two others.",
    ),
    (
        "math::polynomials::divPolyn",
        Arity::exact(2),
        &["math::polynomials::divPolyn polyn1 polyn2"],
        "Divide the first polynomial by the second polynomial and return the result.",
    ),
    (
        "math::polynomials::remainderPolyn",
        Arity::exact(2),
        &["math::polynomials::remainderPolyn polyn1 polyn2"],
        "Divide the first polynomial by the second polynomial and return the remainder.",
    ),
    (
        "math::polynomials::derivPolyn",
        Arity::exact(1),
        &["math::polynomials::derivPolyn polyn"],
        "Differentiate the polynomial and return the result.",
    ),
    (
        "math::polynomials::primitivePolyn",
        Arity::exact(1),
        &["math::polynomials::primitivePolyn polyn"],
        "Integrate the polynomial and return the result.",
    ),
    (
        "math::polynomials::degreePolyn",
        Arity::exact(1),
        &["math::polynomials::degreePolyn polyn"],
        "Return the degree of the polynomial.",
    ),
    (
        "math::polynomials::coeffPolyn",
        Arity::exact(2),
        &["math::polynomials::coeffPolyn polyn index"],
        "Return the coefficient of the term of the index'th degree of the polynomial.",
    ),
    (
        "math::polynomials::allCoeffsPolyn",
        Arity::exact(1),
        &["math::polynomials::allCoeffsPolyn polyn"],
        "Return the coefficients of the polynomial (in ascending order).",
    ),
];

/// The `math::probopt` package.
const MATH__PROBOPT_CMDS: &[Row] = &[
    (
        "math::probopt::pso",
        Arity::at_least(3),
        &["math::probopt::pso function bounds args"],
        "The particle swarm optimisation algorithm uses the idea that the candidate optimum points should swarm around the",
    ),
    (
        "math::probopt::sce",
        Arity::at_least(3),
        &["math::probopt::sce function bounds args"],
        "The shuffled complex evolution algorithm is an extension of the Nelder-Mead algorithm that uses multiple complexe",
    ),
    (
        "math::probopt::diffev",
        Arity::at_least(3),
        &["math::probopt::diffev function bounds args"],
        "The differential evolution algorithm uses a number of initial points that are then updated using randomly selecte",
    ),
    (
        "math::probopt::lipoMax",
        Arity::at_least(3),
        &["math::probopt::lipoMax function bounds args"],
        "The Lipschitz optimisation algorithm uses the Lipschitz property of the given function to find a maximum in the g",
    ),
    (
        "math::probopt::adaLipoMax",
        Arity::at_least(3),
        &["math::probopt::adaLipoMax function bounds args"],
        "The adaptive Lipschitz optimisation algorithm uses the Lipschitz property of the given function to find a maximum",
    ),
];

/// The `math::rationalfunctions` package.
const MATH__RATIONALFUNCTIONS_CMDS: &[Row] = &[
    (
        "math::rationalfunctions::rationalFunction",
        Arity::exact(2),
        &["math::rationalfunctions::rationalFunction num den"],
        "Return an (encoded) list that defines the rational function.",
    ),
    (
        "math::rationalfunctions::ratioCmd",
        Arity::exact(2),
        &["math::rationalfunctions::ratioCmd num den"],
        "Create a new procedure that evaluates the rational function.",
    ),
    (
        "math::rationalfunctions::evalRatio",
        Arity::exact(2),
        &["math::rationalfunctions::evalRatio rational x"],
        "Evaluate the rational function at x.",
    ),
    (
        "math::rationalfunctions::addRatio",
        Arity::exact(2),
        &["math::rationalfunctions::addRatio ratio1 ratio2"],
        "Return a new rational function which is the sum of the two others.",
    ),
    (
        "math::rationalfunctions::subRatio",
        Arity::exact(2),
        &["math::rationalfunctions::subRatio ratio1 ratio2"],
        "Return a new rational function which is the difference of the two others.",
    ),
    (
        "math::rationalfunctions::multRatio",
        Arity::exact(2),
        &["math::rationalfunctions::multRatio ratio1 ratio2"],
        "Return a new rational function which is the product of the two others.",
    ),
    (
        "math::rationalfunctions::divRatio",
        Arity::exact(2),
        &["math::rationalfunctions::divRatio ratio1 ratio2"],
        "Divide the first rational function by the second rational function and return the result.",
    ),
    (
        "math::rationalfunctions::derivPolyn",
        Arity::exact(1),
        &["math::rationalfunctions::derivPolyn ratio"],
        "Differentiate the rational function and return the result.",
    ),
    (
        "math::rationalfunctions::coeffsNumerator",
        Arity::exact(1),
        &["math::rationalfunctions::coeffsNumerator ratio"],
        "Return the coefficients of the numerator of the rational function.",
    ),
    (
        "math::rationalfunctions::coeffsDenominator",
        Arity::exact(1),
        &["math::rationalfunctions::coeffsDenominator ratio"],
        "Return the coefficients of the denominator of the rational function.",
    ),
];

/// The `math::special` package.
const MATH__SPECIAL_CMDS: &[Row] = &[
    (
        "math::special::eulerNumber",
        Arity::exact(1),
        &["math::special::eulerNumber index"],
        "Return the index'th Euler number (note: these are integer values).",
    ),
    (
        "math::special::bernoulliNumber",
        Arity::exact(1),
        &["math::special::bernoulliNumber index"],
        "Return the index'th Bernoulli number.",
    ),
    (
        "math::special::Beta",
        Arity::exact(2),
        &["math::special::Beta x y"],
        "Compute the Beta function for arguments x and y First argument for the Beta function Second argument for the Beta",
    ),
    (
        "math::special::incBeta",
        Arity::exact(3),
        &["math::special::incBeta a b x"],
        "Compute the incomplete Beta function for argument x with parameters a and b First parameter for the incomplete Be",
    ),
    (
        "math::special::regIncBeta",
        Arity::exact(3),
        &["math::special::regIncBeta a b x"],
        "Compute the regularized incomplete Beta function for argument x with parameters a and b First parameter for the i",
    ),
    (
        "math::special::Gamma",
        Arity::exact(1),
        &["math::special::Gamma x"],
        "Compute the Gamma function for argument x Argument for the Gamma function.",
    ),
    (
        "math::special::digamma",
        Arity::exact(1),
        &["math::special::digamma x"],
        "Compute the digamma function (psi) for argument x Argument for the digamma function.",
    ),
    (
        "math::special::erf",
        Arity::exact(1),
        &["math::special::erf x"],
        "Compute the error function for argument x Argument for the error function.",
    ),
    (
        "math::special::erfc",
        Arity::exact(1),
        &["math::special::erfc x"],
        "Compute the complementary error function for argument x Argument for the complementary error function.",
    ),
    (
        "math::special::invnorm",
        Arity::exact(1),
        &["math::special::invnorm p"],
        "Compute the inverse of the normal distribution function for argument p Argument for the inverse normal distributi",
    ),
    (
        "math::special::J0",
        Arity::exact(1),
        &["math::special::J0 x"],
        "Compute the zeroth-order Bessel function of the first kind for the argument x Argument for the Bessel function.",
    ),
    (
        "math::special::J1",
        Arity::exact(1),
        &["math::special::J1 x"],
        "Compute the first-order Bessel function of the first kind for the argument x Argument for the Bessel function.",
    ),
    (
        "math::special::Jn",
        Arity::exact(2),
        &["math::special::Jn n x"],
        "Compute the nth-order Bessel function of the first kind for the argument x Order of the Bessel function Argument ",
    ),
    (
        "math::special::I_n",
        Arity::exact(1),
        &["math::special::I_n x"],
        "Compute the modified Bessel function of the first kind of order n for the argument x Positive integer order of th",
    ),
    (
        "math::special::cn",
        Arity::exact(2),
        &["math::special::cn u k"],
        "Compute the elliptic function cn for the argument u and parameter k.",
    ),
    (
        "math::special::dn",
        Arity::exact(2),
        &["math::special::dn u k"],
        "Compute the elliptic function dn for the argument u and parameter k.",
    ),
    (
        "math::special::sn",
        Arity::exact(2),
        &["math::special::sn u k"],
        "Compute the elliptic function sn for the argument u and parameter k.",
    ),
    (
        "math::special::elliptic_K",
        Arity::exact(1),
        &["math::special::elliptic_K k"],
        "Compute the complete elliptic integral of the first kind for the argument k Argument for the function.",
    ),
    (
        "math::special::elliptic_E",
        Arity::exact(1),
        &["math::special::elliptic_E k"],
        "Compute the complete elliptic integral of the second kind for the argument k Argument for the function.",
    ),
    (
        "math::special::exponential_Ei",
        Arity::exact(1),
        &["math::special::exponential_Ei x"],
        "Compute the exponential integral of the second kind for the argument x Argument for the function (x != 0).",
    ),
    (
        "math::special::exponential_En",
        Arity::exact(2),
        &["math::special::exponential_En n x"],
        "Compute the exponential integral of the first kind for the argument x and order n Order of the integral (n >= 0) ",
    ),
    (
        "math::special::exponential_li",
        Arity::exact(1),
        &["math::special::exponential_li x"],
        "Compute the logarithmic integral for the argument x Argument for the function (x > 0).",
    ),
    (
        "math::special::exponential_Ci",
        Arity::exact(1),
        &["math::special::exponential_Ci x"],
        "Compute the cosine integral for the argument x Argument for the function (x > 0).",
    ),
    (
        "math::special::exponential_Si",
        Arity::exact(1),
        &["math::special::exponential_Si x"],
        "Compute the sine integral for the argument x Argument for the function (x > 0).",
    ),
    (
        "math::special::exponential_Chi",
        Arity::exact(1),
        &["math::special::exponential_Chi x"],
        "Compute the hyperbolic cosine integral for the argument x Argument for the function (x > 0).",
    ),
    (
        "math::special::exponential_Shi",
        Arity::exact(1),
        &["math::special::exponential_Shi x"],
        "Compute the hyperbolic sine integral for the argument x Argument for the function (x > 0).",
    ),
    (
        "math::special::fresnel_C",
        Arity::exact(1),
        &["math::special::fresnel_C x"],
        "Compute the Fresnel cosine integral for real argument x Argument for the function.",
    ),
    (
        "math::special::fresnel_S",
        Arity::exact(1),
        &["math::special::fresnel_S x"],
        "Compute the Fresnel sine integral for real argument x Argument for the function.",
    ),
    (
        "math::special::sinc",
        Arity::exact(1),
        &["math::special::sinc x"],
        "Compute the sinc function for real argument x Argument for the function.",
    ),
    (
        "math::special::legendre",
        Arity::exact(1),
        &["math::special::legendre n"],
        "Return the Legendre polynomial of degree n (see ) Degree of the polynomial.",
    ),
    (
        "math::special::chebyshev",
        Arity::exact(1),
        &["math::special::chebyshev n"],
        "Return the Chebyshev polynomial of degree n (of the first kind) Degree of the polynomial.",
    ),
    (
        "math::special::laguerre",
        Arity::exact(2),
        &["math::special::laguerre alpha n"],
        "Return the Laguerre polynomial of degree n with parameter alpha Parameter of the Laguerre polynomial Degree of th",
    ),
    (
        "math::special::hermite",
        Arity::exact(1),
        &["math::special::hermite n"],
        "Return the Hermite polynomial of degree n Degree of the polynomial.",
    ),
];

/// The `math::statistics` package.
const MATH__STATISTICS_CMDS: &[Row] = &[
    (
        "math::statistics::bootstrap",
        Arity::at_least(2),
        &["math::statistics::bootstrap data sampleSize numberSamples"],
        "Create a subsample or subsamples from a given list of data.",
    ),
    (
        "math::statistics::tstat",
        Arity::at_least(1),
        &["math::statistics::tstat dof alpha"],
        "Returns the value of the t-distribution t* satisfying [example { P(t*) = 1 - alpha/2 P(-t*) = alpha/2 }] for the ",
    ),
    (
        "math::statistics::incompleteGamma",
        Arity::at_least(2),
        &["math::statistics::incompleteGamma x p tol"],
        "Evaluate the incomplete Gamma integral [example { 1 / x p-1 P(p,x) = -------- | dt exp(-t) * t Gamma(p) / 0 }] - ",
    ),
    (
        "math::statistics::incompleteBeta",
        Arity::at_least(3),
        &["math::statistics::incompleteBeta a b x tol"],
        "Evaluate the incomplete Beta integral - First shape parameter - Second shape parameter - Value of x (limit of the",
    ),
    (
        "math::statistics::subdivide",
        Arity::exact(0),
        &["math::statistics::subdivide"],
        "Routine PM - not implemented yet.",
    ),
];

/// The `math::trig` package.
const MATH__TRIG_CMDS: &[Row] = &[
    (
        "math::trig::radian_reduced",
        Arity::exact(1),
        &["math::trig::radian_reduced angle"],
        "Return the equivalent angle in the interval [0, 2pi).",
    ),
    (
        "math::trig::degree_reduced",
        Arity::exact(1),
        &["math::trig::degree_reduced angle"],
        "Return the equivalent angle in the interval [0, 360).",
    ),
    (
        "math::trig::cosec",
        Arity::exact(1),
        &["math::trig::cosec angle"],
        "Calculate the cosecant of the angle (1/cos(angle)) Angle (in radians).",
    ),
    (
        "math::trig::sec",
        Arity::exact(1),
        &["math::trig::sec angle"],
        "Calculate the secant of the angle (1/sin(angle)) Angle (in radians).",
    ),
    (
        "math::trig::cotan",
        Arity::exact(1),
        &["math::trig::cotan angle"],
        "Calculate the cotangent of the angle (1/tan(angle)) Angle (in radians).",
    ),
    (
        "math::trig::acosec",
        Arity::exact(1),
        &["math::trig::acosec value"],
        "Calculate the arc cosecant of the value Value of the argument.",
    ),
    (
        "math::trig::asec",
        Arity::exact(1),
        &["math::trig::asec value"],
        "Calculate the arc secant of the value Value of the argument.",
    ),
    (
        "math::trig::acotan",
        Arity::exact(1),
        &["math::trig::acotan value"],
        "Calculate the arc cotangent of the value Value of the argument.",
    ),
    (
        "math::trig::cosech",
        Arity::exact(1),
        &["math::trig::cosech value"],
        "Calculate the hyperbolic cosecant of the value (1/sinh(value)) Value of the argument.",
    ),
    (
        "math::trig::sech",
        Arity::exact(1),
        &["math::trig::sech value"],
        "Calculate the hyperbolic secant of the value (1/cosh(value)) Value of the argument.",
    ),
    (
        "math::trig::cotanh",
        Arity::exact(1),
        &["math::trig::cotanh value"],
        "Calculate the hyperbolic cotangent of the value (1/tanh(value)) Value of the argument.",
    ),
    (
        "math::trig::asinh",
        Arity::exact(1),
        &["math::trig::asinh value"],
        "Calculate the arc hyperbolic sine of the value Value of the argument.",
    ),
    (
        "math::trig::acosh",
        Arity::exact(1),
        &["math::trig::acosh value"],
        "Calculate the arc hyperbolic cosine of the value Value of the argument.",
    ),
    (
        "math::trig::atanh",
        Arity::exact(1),
        &["math::trig::atanh value"],
        "Calculate the arc hyperbolic tangent of the value Value of the argument.",
    ),
    (
        "math::trig::acosech",
        Arity::exact(1),
        &["math::trig::acosech value"],
        "Calculate the arc hyperbolic cosecant of the value Value of the argument.",
    ),
    (
        "math::trig::asech",
        Arity::exact(1),
        &["math::trig::asech value"],
        "Calculate the arc hyperbolic secant of the value Value of the argument.",
    ),
    (
        "math::trig::acotanh",
        Arity::exact(1),
        &["math::trig::acotanh value"],
        "Calculate the arc hyperbolic cotangent of the value Value of the argument.",
    ),
    (
        "math::trig::sind",
        Arity::exact(1),
        &["math::trig::sind angle"],
        "Calculate the sine of the angle (in degrees) Angle (in degrees).",
    ),
    (
        "math::trig::cosd",
        Arity::exact(1),
        &["math::trig::cosd angle"],
        "Calculate the cosine of the angle (in degrees) Angle (in radians).",
    ),
    (
        "math::trig::tand",
        Arity::exact(1),
        &["math::trig::tand angle"],
        "Calculate the cotangent of the angle (in degrees) Angle (in degrees).",
    ),
    (
        "math::trig::cosecd",
        Arity::exact(1),
        &["math::trig::cosecd angle"],
        "Calculate the cosecant of the angle (in degrees) Angle (in degrees).",
    ),
    (
        "math::trig::secd",
        Arity::exact(1),
        &["math::trig::secd angle"],
        "Calculate the secant of the angle (in degrees) Angle (in degrees).",
    ),
    (
        "math::trig::cotand",
        Arity::exact(1),
        &["math::trig::cotand angle"],
        "Calculate the cotangent of the angle (in degrees) Angle (in degrees).",
    ),
];

/// (package, command-table) pairs.
const GROUPS: &[(&str, &[Row])] = &[
    ("math", MATH_CMDS),
    ("math::PCA", MATH__PCA_CMDS),
    ("math::bignum", MATH__BIGNUM_CMDS),
    ("math::calculus", MATH__CALCULUS_CMDS),
    ("math::combinatorics", MATH__COMBINATORICS_CMDS),
    ("math::complexnumbers", MATH__COMPLEXNUMBERS_CMDS),
    ("math::decimal", MATH__DECIMAL_CMDS),
    ("math::exact", MATH__EXACT_CMDS),
    ("math::figurate", MATH__FIGURATE_CMDS),
    ("math::filters", MATH__FILTERS_CMDS),
    ("math::fourier", MATH__FOURIER_CMDS),
    ("math::geometry", MATH__GEOMETRY_CMDS),
    ("math::interpolate", MATH__INTERPOLATE_CMDS),
    ("math::linearalgebra", MATH__LINEARALGEBRA_CMDS),
    ("math::optimize", MATH__OPTIMIZE_CMDS),
    ("math::polynomials", MATH__POLYNOMIALS_CMDS),
    ("math::probopt", MATH__PROBOPT_CMDS),
    ("math::rationalfunctions", MATH__RATIONALFUNCTIONS_CMDS),
    ("math::special", MATH__SPECIAL_CMDS),
    ("math::statistics", MATH__STATISTICS_CMDS),
    ("math::trig", MATH__TRIG_CMDS),
];

/// All command specs in this group.
pub fn specs() -> Vec<CommandSpec> {
    GROUPS
        .iter()
        .flat_map(|&(pkg, table)| rows(pkg, table))
        .collect()
}
