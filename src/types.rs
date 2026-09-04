use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Unit,
    Void,
    Bool,
    Char,
    Integer {
        width: Width,
        signed: bool,
    },
    Float {
        width: Width,
    },
    Pointer {
        mutable: bool,
        pointee: Box<Type>,
    },
    Function {
        parameters: Vec<Type>,
        variadic: bool,
        result: Box<Type>,
    },
    Never,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Width {
    Bits8,
    Bits16,
    Bits32,
    Bits64,
    Pointer,
}

impl Type {
    pub const I32: Type = Type::Integer {
        width: Width::Bits32,
        signed: true,
    };
    pub const U8: Type = Type::Integer {
        width: Width::Bits8,
        signed: false,
    };
    pub const F64: Type = Type::Float {
        width: Width::Bits64,
    };

    pub fn primitive(name: &str) -> Option<Type> {
        use Width::*;
        let integer = |width, signed| Some(Type::Integer { width, signed });
        match name {
            "i8" => integer(Bits8, true),
            "i16" => integer(Bits16, true),
            "i32" => integer(Bits32, true),
            "i64" => integer(Bits64, true),
            "isize" => integer(Pointer, true),
            "u8" => integer(Bits8, false),
            "u16" => integer(Bits16, false),
            "u32" => integer(Bits32, false),
            "u64" => integer(Bits64, false),
            "usize" => integer(Pointer, false),
            "f32" => Some(Type::Float { width: Bits32 }),
            "f64" => Some(Type::Float { width: Bits64 }),
            "bool" => Some(Type::Bool),
            "char" => Some(Type::Char),
            "void" => Some(Type::Void),
            _ => None,
        }
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Type::Integer { .. })
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Type::Float { .. })
    }

    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            Type::Integer { signed: true, .. } | Type::Float { .. }
        )
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Type::Error | Type::Never)
    }

    pub fn accepts(&self, actual: &Type) -> bool {
        self.is_unknown() || actual.is_unknown() || self == actual
    }

    pub fn is_representable_in_c(&self) -> bool {
        match self {
            Type::Unit | Type::Bool | Type::Integer { .. } | Type::Float { .. } => true,
            Type::Pointer { pointee, .. } => {
                matches!(**pointee, Type::Void) || pointee.is_representable_in_c()
            }
            Type::Function {
                parameters, result, ..
            } => {
                parameters.iter().all(Type::is_representable_in_c) && result.is_representable_in_c()
            }
            Type::Char | Type::Void | Type::Never | Type::Error => false,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Unit => write!(formatter, "unit"),
            Type::Void => write!(formatter, "void"),
            Type::Bool => write!(formatter, "bool"),
            Type::Char => write!(formatter, "char"),
            Type::Integer { width, signed } => {
                let prefix = if *signed { "i" } else { "u" };
                match width {
                    Width::Bits8 => write!(formatter, "{prefix}8"),
                    Width::Bits16 => write!(formatter, "{prefix}16"),
                    Width::Bits32 => write!(formatter, "{prefix}32"),
                    Width::Bits64 => write!(formatter, "{prefix}64"),
                    Width::Pointer => write!(formatter, "{prefix}size"),
                }
            }
            Type::Float { width } => match width {
                Width::Bits32 => write!(formatter, "f32"),
                _ => write!(formatter, "f64"),
            },
            Type::Pointer { mutable, pointee } => {
                if *mutable {
                    write!(formatter, "*mut {pointee}")
                } else {
                    write!(formatter, "*{pointee}")
                }
            }
            Type::Function {
                parameters,
                variadic,
                result,
            } => {
                write!(formatter, "fun(")?;
                for (index, parameter) in parameters.iter().enumerate() {
                    if index > 0 {
                        write!(formatter, ", ")?;
                    }
                    write!(formatter, "{parameter}")?;
                }
                if *variadic {
                    if !parameters.is_empty() {
                        write!(formatter, ", ")?;
                    }
                    write!(formatter, "...")?;
                }
                write!(formatter, ")")?;
                if **result != Type::Unit {
                    write!(formatter, " -> {result}")?;
                }
                Ok(())
            }
            Type::Never => write!(formatter, "never"),
            Type::Error => write!(formatter, "{{error}}"),
        }
    }
}
