interp hide {} set hiddenSet
proc observe {} {
    interp invokehidden {} hiddenSet item LOCAL
    list [interp invokehidden {} -namespace chosen hiddenSet item NAMESPACE] [interp invokehidden {} hiddenSet item]
}
puts [list [observe] [interp invokehidden {} hiddenSet ::chosen::item]]
