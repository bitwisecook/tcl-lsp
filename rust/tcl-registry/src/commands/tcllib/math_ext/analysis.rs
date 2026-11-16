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

//! Analysis: `math::calculus`, `math::filters`, `math::fourier`, `math::interpolate`, `math::optimize`, `math::special` and `math::trig`.

use super::{Arity, Row};

/// The `math::calculus` package.
pub(super) const MATH__CALCULUS_CMDS: &[Row] = &[
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

/// The `math::filters` package.
pub(super) const MATH__FILTERS_CMDS: &[Row] = &[
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
pub(super) const MATH__FOURIER_CMDS: &[Row] = &[
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

/// The `math::interpolate` package.
pub(super) const MATH__INTERPOLATE_CMDS: &[Row] = &[
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

/// The `math::optimize` package.
pub(super) const MATH__OPTIMIZE_CMDS: &[Row] = &[
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

/// The `math::special` package.
pub(super) const MATH__SPECIAL_CMDS: &[Row] = &[
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

/// The `math::trig` package.
pub(super) const MATH__TRIG_CMDS: &[Row] = &[
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
