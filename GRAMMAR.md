# Grammar

EBNF for every construct `LANGUAGE.md` has decided. Terminals are quoted; `identifier`, `newline`, and the literal tokens come from the lexer. Newlines separate items, blank lines and comments are ignored, and `items(x)` is the newline-separated list used at every level that holds items.

Side conditions the parser enforces sit beside the productions they constrain.

## Program

```
program      = script | unit ;

unit         = items( declaration ) ;
script       = items( declaration | statement ) ;

items( x )   = { newline } , [ x , { newline , { newline } , x } ] , { newline } ;
```

- A `unit` is every file of a package. A `script` is a single file compiled without a manifest.
- A script with statements must not declare `main`. A script with neither gets an empty implicit `main`.

## Declarations

```
declaration  = { attribute , { newline } } , function ;

function     = "fun" , identifier , "(" , [ parameters ] , ")" , [ "->" , type ] , [ block ] ;
parameters   = parameter , { "," , parameter } , [ "," ] , [ "..." ]
             | "..." ;
parameter    = ( "_" | identifier ) , [ identifier ] , ":" , type , [ ":=" , expression ] ;

attribute    = "@" , identifier , [ "(" , [ arguments ] , ")" ] ;
arguments    = argument , { "," , argument } , [ "," ] ;
argument     = [ identifier , ":" ] , expression ;
```

- A `parameter` beginning with `"_"` must be followed by an internal name.
- A `function` without a `block`, and a `parameters` ending in `"..."`, each require an `@extern` attribute. A `block` together with `"..."` is an error.
- `"..."` must be the last thing inside the parentheses.
