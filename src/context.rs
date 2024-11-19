use crate::syntax::TypeSyntax;

#[derive(Debug, PartialEq, Clone)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: Type,
}

#[derive(Default)]
pub struct Scope {
    symbols: Vec<Symbol>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    Void,
    Identifier(String),
    Function(TypeFunction),
}

#[derive(Debug, PartialEq, Clone)]
pub struct TypeFunction {
    pub parameters: Vec<TypeFunctionParameter>,
    pub return_type: Box<Type>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TypeFunctionParameter {
    pub label: String,
    pub name: String,
    pub ty: Type,
}

impl TypeFunctionParameter {
    pub fn new(label: Option<String>, name: String, ty: Type) -> Self {
        Self {
            label: label.unwrap_or(name.clone()),
            name,
            ty,
        }
    }
}

impl From<TypeSyntax> for Type {
    fn from(value: TypeSyntax) -> Self {
        match value {
            TypeSyntax::IdentifierType(identifier) => Type::Identifier(identifier.name),
        }
    }
}

pub struct TypeDefinition {
    name: String,
    // methods: HashMap<String, >,
}

pub struct Context {
    scopes: Vec<Scope>,
    types: Vec<TypeDefinition>,
}

impl Context {
    pub fn new() -> Self {
        let mut scopes = Vec::new();
        scopes.push(Scope::default()); // Global scope

        let types = vec![]; // TODO: fill this with primitive types ?

        Self { scopes, types }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    pub fn exit_scope(&mut self) {
        if self.scopes.len() == 1 {
            panic!("Cannot exit global scope");
        }

        self.scopes.pop();
    }

    pub fn register_symbol(&mut self, symbol: Symbol) {
        let scope = self.scopes.last_mut().unwrap();

        if scope.symbols.contains(&symbol) {
            println!("symbol already exists in scope: {:?}", symbol);
            return;
        }

        scope.symbols.push(symbol);
    }

    pub fn lookup(&mut self, identifier: String) -> Option<Symbol> {
        let scopes_len = self.scopes.len();

        for i in (0..scopes_len).rev() {
            let symbols = &self.scopes[i].symbols;

            for symbol in symbols {
                if symbol.name == identifier {
                    return Some(symbol.clone());
                }
            }
        }

        None
    }
}
