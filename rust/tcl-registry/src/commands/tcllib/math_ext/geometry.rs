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

//! The `math::geometry` package.

use super::{Arity, Row};

/// The `math::geometry` package.
pub(super) const MATH__GEOMETRY_CMDS: &[Row] = &[
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
