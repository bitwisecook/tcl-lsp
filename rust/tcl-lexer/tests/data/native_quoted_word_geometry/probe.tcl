set _gen_all_operators {a b}
variable _tmm_operator_re "\\m([join $_gen_all_operators |])\\M"
puts $_tmm_operator_re
