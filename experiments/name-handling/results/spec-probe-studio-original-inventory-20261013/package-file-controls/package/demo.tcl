proc :lost {} {return COLON}
namespace eval {N[0]} {proc {p*} {} {return BRACKET}}
namespace eval N0 {proc pZ {} {return SIBLING}}
proc __spec_probeOther {} {return OTHER}
package provide demo 1.0
