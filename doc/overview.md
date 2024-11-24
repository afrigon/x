# The x overview

## Comments
### Line Comment

Line comments are used for brief annotations or explanations.
They begin with `//` and extend to the end of the line.

```x
// This is a line comment
let x = 42  // Comments can also appear at the end of a line
```

### Documentation Comment

Documentation comments provide detailed information about code elements
and are used to generate documentation.
They begin with `///` and extend to the end of the line.

> Markdown syntax is supported.

```x
/// Computes the square of a number.
fun square(n: int) -> int {
    n * n
}
```

## Simple Values

```x
let mut variable := 69
variable := 420
let my_constant := 150
```

```x
let implicit_integer := 70
let implicit_double := 70.0
let explicit_double: f64 := 70
```

## Collections

### Arrays

```x
let array := [1, 2, 3, 4, 5]
let float_array: [f64] := [1, 2, 3, 4, 5]
```

### Dictionaries

```x
let dictionary := [
    "key": "value",
    "key2": "value2"
]
```

## Control Flow

### Conditional

```x
if x > y {
    // x is larger than y
} else {
    // x is less than or equal to y
}
```

Since if are expressions, the result can be assigned directly to a variable or
returned.

```x
let x := if x != 0 {
    100
} else {
    200
}
```

### Iteration

```x
let items := [1, 2, 3, 4, 5]

loop item in items {
    // loop over all items in the array.
}
```

```x
loop i in 0..10 {
    // loop from 0 to 9
}
```

```x
loop {
    // loop until a break or return statement is reached.
}
```

## Functions

```x
fun fibonacci(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}
```

## Optional and Results

```x
let maybe_int: int? := nil
let maybe_float: optional<float> := 10.0
```

### Unwrapping

## Types

```x
type Vector2 {
    let x: f64
    let y: f64

    fun magnitude() -> f64 {
        (x * x + y * y).sqrt()
    }

    fun selfless zero() -> Vector2 {
        Vector2(x: 0, y: 0)
    }
}
```

## Enumerations

```x
enum Color {
    red
    green
    blue

    fun hex() -> u32 {
    }
}
```

## Protocols

### Defining a Protocol

```x
proto Drawable {
    fun draw()
}
```

### Conforming to a Protocol

```x
type Circle: Drawable {
    let radius: f64

    fun draw() {
        // draw a circle
    }
}

type Rectangle: Drawable {
    let width: f64
    let height: f64

    fun draw() {
        // draw a rectangle
    }
}
```

### Using the Protocol as a Type

```x
fun draw(drawable: Drawable) {
    drawable.draw()
}
```

## Foreign Function Interface

FFI in x is done by first definning a protocol that describes the foreign library

```x
proto LibMathProto {
    fun selfless add(x: f64, y: f64) -> f64
    fun selfless sub(x: f64, y: f64) -> f64
    fun selfless mul(x: f64, y: f64) -> f64
    fun selfless div(x: f64, y: f64) -> f64
}
```



```x
extern "C" LibMath: LibMathProto
```

then the library will be available in the form of an x type.

```x
LibMath.add(1, 2)
```
