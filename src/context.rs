#[derive(Debug, PartialEq, Clone)]
pub struct Symbol {
    pub name: String,
    // symbol_type: Type,
}

#[derive(Default)]
pub struct Scope {
    symbols: Vec<Symbol>,
}

pub struct Context {
    scopes: Vec<Scope>,
}

impl Context {
    pub fn new() -> Self {
        let mut scopes = Vec::new();
        scopes.push(Scope::default()); // Global scope

        Self { scopes }
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
