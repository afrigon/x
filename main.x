extern fun print(value: f64)

fun main() {
    print(if false { 100 } else { 200 })
}

fun test() {
    1 + true
}
