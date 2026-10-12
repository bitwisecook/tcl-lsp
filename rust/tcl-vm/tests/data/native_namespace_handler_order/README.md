# Namespace declaration order

The script distinguishes compiled namespace links from generic `variable`
declarations. Generic declarations write the namespace value before creating a
local alias; compiled links create the alias before evaluating the value.
Rejected array-element declarations create the array root without its element.
