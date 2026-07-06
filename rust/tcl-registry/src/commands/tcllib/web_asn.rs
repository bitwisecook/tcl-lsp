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

//! tcllib web / protocol packages — `asn` (BER/DER), `ncgi` (CGI), and
//! `htmlparse` (HTML parsing).
//!
//! Public command surface from tcllib `modules/{asn/asn,ncgi/ncgi,
//! htmlparse/htmlparse}.man`.  Command names, arity bounds, synopses, and
//! summaries are derived from those manual pages.  These commands interact
//! with binary encodings, the CGI environment, and callback-driven parsers,
//! so none are marked pure.  Requires Tcl 8.5+.

use crate::prelude::*;

/// A row in a package command table: name, arity, synopsis, and summary.
type Row = (&'static str, Arity, &'static [&'static str], &'static str);

/// Build (non-pure) command specs for a package from its table.
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

/// The `asn` package (BER/DER encode + decode primitives).
const ASN_CMDS: &[Row] = &[
    (
        "asn::asnSequence",
        Arity::at_least(1),
        &["asn::asnSequence evalue..."],
        "Takes zero or more encoded values, packs them into an ASN sequence and returns its encoded binary form.",
    ),
    (
        "asn::asnSequenceFromList",
        Arity::exact(1),
        &["asn::asnSequenceFromList elist"],
        "Takes a list of encoded values, packs them into an ASN sequence and returns its encoded binary form.",
    ),
    (
        "asn::asnSet",
        Arity::at_least(1),
        &["asn::asnSet evalue..."],
        "Takes zero or more encoded values, packs them into an ASN set and returns its encoded binary form.",
    ),
    (
        "asn::asnSetFromList",
        Arity::exact(1),
        &["asn::asnSetFromList elist"],
        "Takes a list of encoded values, packs them into an ASN set and returns its encoded binary form.",
    ),
    (
        "asn::asnApplicationConstr",
        Arity::at_least(2),
        &["asn::asnApplicationConstr appNumber evalue..."],
        "Takes zero or more encoded values, packs them into an ASN application construct and returns its encoded binary.",
    ),
    (
        "asn::asnApplication",
        Arity::exact(2),
        &["asn::asnApplication appNumber data"],
        "Takes a single encoded value data, packs it into an ASN application construct and returns its encoded binary form.",
    ),
    (
        "asn::asnChoice",
        Arity::at_least(2),
        &["asn::asnChoice appNumber evalue..."],
        "Takes zero or more encoded values, packs them into an ASN choice construct and returns its encoded binary form.",
    ),
    (
        "asn::asnChoiceConstr",
        Arity::at_least(2),
        &["asn::asnChoiceConstr appNumber evalue..."],
        "Takes zero or more encoded values, packs them into an ASN choice construct and returns its encoded binary form.",
    ),
    (
        "asn::asnInteger",
        Arity::exact(1),
        &["asn::asnInteger number"],
        "Returns the encoded form of the specified integer number.",
    ),
    (
        "asn::asnEnumeration",
        Arity::exact(1),
        &["asn::asnEnumeration number"],
        "Returns the encoded form of the specified enumeration id number.",
    ),
    (
        "asn::asnBoolean",
        Arity::exact(1),
        &["asn::asnBoolean bool"],
        "Returns the encoded form of the specified boolean value bool.",
    ),
    (
        "asn::asnContext",
        Arity::exact(2),
        &["asn::asnContext context data"],
        "Takes an encoded value and packs it into a constructed value with application tag, the context number.",
    ),
    (
        "asn::asnContextConstr",
        Arity::at_least(2),
        &["asn::asnContextConstr context evalue..."],
        "Takes zero or more encoded values and packs them into a constructed value with application tag, the context number.",
    ),
    (
        "asn::asnObjectIdentifier",
        Arity::exact(1),
        &["asn::asnObjectIdentifier idlist"],
        "Takes a list of at least 2 integers describing an object identifier (OID) value, and returns the encoded value.",
    ),
    (
        "asn::asnUTCTime",
        Arity::exact(1),
        &["asn::asnUTCTime utcstring"],
        "Returns the encoded form of the specified UTC time string.",
    ),
    (
        "asn::asnNull",
        Arity::exact(0),
        &["asn::asnNull"],
        "Returns the NULL encoding.",
    ),
    (
        "asn::asnBitString",
        Arity::exact(1),
        &["asn::asnBitString string"],
        "Returns the encoded form of the specified string.",
    ),
    (
        "asn::asnOctetString",
        Arity::exact(1),
        &["asn::asnOctetString string"],
        "Returns the encoded form of the specified string.",
    ),
    (
        "asn::asnNumericString",
        Arity::exact(1),
        &["asn::asnNumericString string"],
        "Returns the string encoded as ASN.1 NumericString.",
    ),
    (
        "asn::asnPrintableString",
        Arity::exact(1),
        &["asn::asnPrintableString string"],
        "Returns the string encoding as ASN.1 PrintableString.",
    ),
    (
        "asn::asnIA5String",
        Arity::exact(1),
        &["asn::asnIA5String string"],
        "Returns the string encoded as ASN.1 IA5String.",
    ),
    (
        "asn::asnBMPString",
        Arity::exact(1),
        &["asn::asnBMPString string"],
        "Returns the string encoded as ASN.1 Basic Multilingual Plane string (Which is essentialy big-endian UCS2).",
    ),
    (
        "asn::asnUTF8String",
        Arity::exact(1),
        &["asn::asnUTF8String string"],
        "Returns the string encoded as UTF8 String.",
    ),
    (
        "asn::asnString",
        Arity::exact(1),
        &["asn::asnString string"],
        "Returns an encoded form of string, choosing the most restricted ASN.1 string type possible.",
    ),
    (
        "asn::defaultStringType",
        Arity::at_least(0),
        &["asn::defaultStringType"],
        "Selects the string type to use for the encoding of non-ASCII strings.",
    ),
    (
        "asn::asnPeekByte",
        Arity::exact(2),
        &["asn::asnPeekByte data_var byte_var"],
        "Retrieve the first byte of the data, without modifing data_var.",
    ),
    (
        "asn::asnGetLength",
        Arity::exact(2),
        &["asn::asnGetLength data_var length_var"],
        "Decode the length information for a block of BER data.",
    ),
    (
        "asn::asnGetResponse",
        Arity::exact(2),
        &["asn::asnGetResponse chan data_var"],
        "Reads an encoded ASN sequence from the channel chan and stores it into the variable named by data_var.",
    ),
    (
        "asn::asnGetInteger",
        Arity::exact(2),
        &["asn::asnGetInteger data_var int_var"],
        "Assumes that an encoded integer value is at the front of the data stored in the variable named data_var, extracts.",
    ),
    (
        "asn::asnGetEnumeration",
        Arity::exact(2),
        &["asn::asnGetEnumeration data_var enum_var"],
        "Assumes that an enumeration id is at the front of the data stored in the variable named data_var, and stores it.",
    ),
    (
        "asn::asnGetOctetString",
        Arity::exact(2),
        &["asn::asnGetOctetString data_var string_var"],
        "Assumes that a string is at the front of the data stored in the variable named data_var, and stores it into the.",
    ),
    (
        "asn::asnGetString",
        Arity::at_least(2),
        &["asn::asnGetString data_var string_var"],
        "Decodes a user-readable string.",
    ),
    (
        "asn::asnGetNumericString",
        Arity::exact(2),
        &["asn::asnGetNumericString data_var string_var"],
        "Assumes that a numeric string value is at the front of the data stored in the variable named data_var, and stores.",
    ),
    (
        "asn::asnGetPrintableString",
        Arity::exact(2),
        &["asn::asnGetPrintableString data_var string_var"],
        "Assumes that a printable string value is at the front of the data stored in the variable named data_var, and.",
    ),
    (
        "asn::asnGetIA5String",
        Arity::exact(2),
        &["asn::asnGetIA5String data_var string_var"],
        "Assumes that a IA5 (ASCII) string value is at the front of the data stored in the variable named data_var, and.",
    ),
    (
        "asn::asnGetBMPString",
        Arity::exact(2),
        &["asn::asnGetBMPString data_var string_var"],
        "Assumes that a BMP (two-byte unicode) string value is at the front of the data stored in the variable named.",
    ),
    (
        "asn::asnGetUTF8String",
        Arity::exact(2),
        &["asn::asnGetUTF8String data_var string_var"],
        "Assumes that a UTF8 string value is at the front of the data stored in the variable named data_var, and stores it.",
    ),
    (
        "asn::asnGetUTCTime",
        Arity::exact(2),
        &["asn::asnGetUTCTime data_var utc_var"],
        "Assumes that a UTC time value is at the front of the data stored in the variable named data_var, and stores it.",
    ),
    (
        "asn::asnGetBitString",
        Arity::exact(2),
        &["asn::asnGetBitString data_var bits_var"],
        "Assumes that a bit string value is at the front of the data stored in the variable named data_var, and stores it.",
    ),
    (
        "asn::asnGetObjectIdentifier",
        Arity::exact(2),
        &["asn::asnGetObjectIdentifier data_var oid_var"],
        "Assumes that a object identifier (OID) value is at the front of the data stored in the variable named data_var.",
    ),
    (
        "asn::asnGetBoolean",
        Arity::exact(2),
        &["asn::asnGetBoolean data_var bool_var"],
        "Assumes that a boolean value is at the front of the data stored in the variable named data_var, and stores it.",
    ),
    (
        "asn::asnGetNull",
        Arity::exact(1),
        &["asn::asnGetNull data_var"],
        "Assumes that a NULL value is at the front of the data stored in the variable named data_var and removes the bytes.",
    ),
    (
        "asn::asnGetSequence",
        Arity::exact(2),
        &["asn::asnGetSequence data_var sequence_var"],
        "Assumes that an ASN sequence is at the front of the data stored in the variable named data_var, and stores it.",
    ),
    (
        "asn::asnGetSet",
        Arity::exact(2),
        &["asn::asnGetSet data_var set_var"],
        "Assumes that an ASN set is at the front of the data stored in the variable named data_var, and stores it into the.",
    ),
    (
        "asn::asnGetApplication",
        Arity::at_least(2),
        &["asn::asnGetApplication data_var appNumber_var"],
        "Assumes that an ASN application construct is at the front of the data stored in the variable named data_var, and.",
    ),
    (
        "asn::asnGetContext",
        Arity::at_least(2),
        &["asn::asnGetContext data_var contextNumber_var"],
        "Assumes that an ASN context tag construct is at the front of the data stored in the variable named data_var, and.",
    ),
    (
        "asn::asnPeekTag",
        Arity::exact(4),
        &["asn::asnPeekTag data_var tag_var tagtype_var constr_var"],
        "The ::asn::asnPeekTag command can be used to take a peek at the data and decode the tag value, without removing.",
    ),
    (
        "asn::asnTag",
        Arity::at_least(1),
        &["asn::asnTag tagnumber"],
        "The ::asn::asnTag can be used to create a tag value.",
    ),
    (
        "asn::asnRetag",
        Arity::exact(2),
        &["asn::asnRetag data_var newTag"],
        "Replaces the tag in front of the data in data_var with newTag.",
    ),
];

/// The `ncgi` package (CGI request/response helpers).
const NCGI_CMDS: &[Row] = &[
    (
        "ncgi::cookie",
        Arity::exact(1),
        &["ncgi::cookie cookie"],
        "Return a list of values for cookie, if any.",
    ),
    (
        "ncgi::decode",
        Arity::exact(1),
        &["ncgi::decode str"],
        "Decode strings in www-url-encoding, which represents special characters with a %xx sequence, where xx is the.",
    ),
    (
        "ncgi::empty",
        Arity::exact(1),
        &["ncgi::empty name"],
        "Returns 1 if the CGI variable name is not present or has the empty string as its value.",
    ),
    (
        "ncgi::exists",
        Arity::exact(1),
        &["ncgi::exists name"],
        "The return value is a boolean.",
    ),
    (
        "ncgi::encode",
        Arity::exact(1),
        &["ncgi::encode string"],
        "Encode string into www-url-encoded format.",
    ),
    (
        "ncgi::header",
        Arity::at_least(1),
        &["ncgi::header args"],
        "Output the CGI header to standard output.",
    ),
    (
        "ncgi::import",
        Arity::at_least(1),
        &["ncgi::import cginame"],
        "This creates a variable in the current scope with the value of the CGI variable cginame.",
    ),
    (
        "ncgi::importAll",
        Arity::at_least(1),
        &["ncgi::importAll args"],
        "This imports several CGI variables as Tcl variables.",
    ),
    (
        "ncgi::importFile",
        Arity::at_least(2),
        &["ncgi::importFile cmd cginame"],
        "This provides information about an uploaded file from a form input field of type file with name cginame.",
    ),
    (
        "ncgi::input",
        Arity::at_least(0),
        &["ncgi::input"],
        "This reads and decodes the CGI values from the environment.",
    ),
    (
        "ncgi::multipart",
        Arity::exact(1),
        &["ncgi::multipart {type query}"],
        "This procedure parses a multipart/form-data query.",
    ),
    (
        "ncgi::nvlist",
        Arity::exact(0),
        &["ncgi::nvlist"],
        "This returns all the query data as a name, value list.",
    ),
    (
        "ncgi::names",
        Arity::exact(0),
        &["ncgi::names"],
        "This returns all names found in the query data, as a list.",
    ),
    (
        "ncgi::parse",
        Arity::exact(0),
        &["ncgi::parse"],
        "This reads and decodes the CGI values from the environment.",
    ),
    (
        "ncgi::parseMimeValue",
        Arity::exact(1),
        &["ncgi::parseMimeValue value"],
        "This decodes the Content-Type and other MIME headers that have the form of primary value; param=val; p2=v2 It.",
    ),
    (
        "ncgi::query",
        Arity::exact(0),
        &["ncgi::query"],
        "This returns the raw query data.",
    ),
    (
        "ncgi::redirect",
        Arity::exact(1),
        &["ncgi::redirect url"],
        "Generate a response that causes a 302 redirect by the Web server.",
    ),
    (
        "ncgi::reset",
        Arity::exact(1),
        &["ncgi::reset {query type}"],
        "Set the query data and Content-Type for the current CGI session.",
    ),
    (
        "ncgi::setCookie",
        Arity::at_least(1),
        &["ncgi::setCookie args"],
        "Set a cookie value that will be returned as part of the reply.",
    ),
    (
        "ncgi::setDefaultValue",
        Arity::exact(1),
        &["ncgi::setDefaultValue {key defvalue}"],
        "Set a CGI value if it does not already exists.",
    ),
    (
        "ncgi::setDefaultValueList",
        Arity::exact(1),
        &["ncgi::setDefaultValueList {key defvaluelist}"],
        "Like ::ncgi::setDefaultValue except that the value already has list structure to represent multiple checkboxes or.",
    ),
    (
        "ncgi::setValue",
        Arity::exact(1),
        &["ncgi::setValue {key value}"],
        "Set a CGI value, overriding whatever was present in the CGI environment already.",
    ),
    (
        "ncgi::setValueList",
        Arity::exact(1),
        &["ncgi::setValueList {key valuelist}"],
        "Like ::ncgi::setValue except that the value already has list structure to represent multiple checkboxes or a.",
    ),
    (
        "ncgi::type",
        Arity::exact(0),
        &["ncgi::type"],
        "Returns the Content-Type of the current CGI values.",
    ),
    (
        "ncgi::urlStub",
        Arity::at_least(0),
        &["ncgi::urlStub"],
        "Returns the current URL, but without the protocol, server, and port.",
    ),
    (
        "ncgi::value",
        Arity::at_least(1),
        &["ncgi::value key"],
        "Return the CGI value identified by key.",
    ),
    (
        "ncgi::valueList",
        Arity::at_least(1),
        &["ncgi::valueList key"],
        "Like ::ncgi::value, but this always returns a list of values (even if there is only one value).",
    ),
];

/// The `htmlparse` package.
const HTMLPARSE_CMDS: &[Row] = &[
    (
        "htmlparse::parse",
        Arity::at_least(1),
        &["htmlparse::parse -cmd -vroot -split -incvar -queue html"],
        "This command is the basic parser for HTML.",
    ),
    (
        "htmlparse::debugCallback",
        Arity::at_least(1),
        &["htmlparse::debugCallback tag slash param textBehindTheTag"],
        "This command is the standard callback used by the parser in ::htmlparse::parse if none was specified by the user.",
    ),
    (
        "htmlparse::mapEscapes",
        Arity::exact(1),
        &["htmlparse::mapEscapes html"],
        "This command takes a HTML string, substitutes all escape sequences with their actual characters and then returns.",
    ),
    (
        "htmlparse::2tree",
        Arity::exact(1),
        &["htmlparse::2tree {html tree}"],
        "This command is a wrapper around ::htmlparse::parse which takes an HTML string (in html) and converts it into a.",
    ),
    (
        "htmlparse::removeVisualFluff",
        Arity::exact(1),
        &["htmlparse::removeVisualFluff tree"],
        "This command walks a tree as generated by ::htmlparse::2tree and removes all the nodes which represent visual.",
    ),
    (
        "htmlparse::removeFormDefs",
        Arity::exact(1),
        &["htmlparse::removeFormDefs tree"],
        "Like ::htmlparse::removeVisualFluff this command is here to cut down on the size of the tree as generated by.",
    ),
];

/// All `asn` / `ncgi` / `htmlparse` command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = rows("asn", ASN_CMDS);
    specs.extend(rows("ncgi", NCGI_CMDS));
    specs.extend(rows("htmlparse", HTMLPARSE_CMDS));
    specs
}
