proc foo {} {
    catch {return 1}
}
proc bar {} {
    catch {error "fail"}
}
proc baz {} {
    catch {break}
}
