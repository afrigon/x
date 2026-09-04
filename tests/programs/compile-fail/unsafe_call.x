//@ compile-fail
@extern(.c)
fun printf(_ format: *u8, ...) -> i32

fun main() {
    printf(c"hello, world\n")
}
