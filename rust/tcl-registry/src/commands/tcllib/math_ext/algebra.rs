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

//! Algebra: `math::linearalgebra`, `math::polynomials` and `math::rationalfunctions`.

use super::{Arity, Row};

/// The `math::linearalgebra` package.
pub(super) const MATH__LINEARALGEBRA_CMDS: &[Row] = &[
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

/// The `math::polynomials` package.
pub(super) const MATH__POLYNOMIALS_CMDS: &[Row] = &[
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

/// The `math::rationalfunctions` package.
pub(super) const MATH__RATIONALFUNCTIONS_CMDS: &[Row] = &[
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
