namespace eval modelTestVerTool923 {
    namespace eval gui923 {
        proc specAddButtonPopUp923 {x y} {
            return "spec $x $y"
        }
        proc testAddButtonPopUp923 {x y} {
            return "test $x $y"
        }
        bind $win.fra.tool.buT_specAddIconLarge <ButtonRelease-1> {::modelTestVerTool923::gui923::specAddButtonPopUp923 %X %Y}
        bind $win.fra.tool.buT_testAddIconLarge <ButtonRelease-1> {testAddButtonPopUp923 %X %Y}
    }
}
