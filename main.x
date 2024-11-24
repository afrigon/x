type Vector2 {
    let x: f64
    let y: f64

    fun selfless zero() -> Vector2 {
        Vector2(x: 0, y: 0)
    }

    fun selfless from(direction: Direction) -> Vector2 {
        match direction {
            .up =>    Vector2(x:  0, y:  1)
            .down =>  Vector2(x:  0, y: -1)
            .left =>  Vector2(x: -1, y:  0)
            .right => Vector2(x:  1, y:  0)
        }
    }
}

enum Direction {
    up,
    down,
    left,
    right,
}

fun main() {
    let origin: Vector2 := .zero()
    let size: Vector2 := .from(direction: .up)
}
