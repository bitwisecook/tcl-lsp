proc g {} {}
namespace eval :: {namespace export g}
namespace eval n {namespace import ::g}
namespace eval n {namespace forget ::g}
info commands n::g
