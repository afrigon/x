use crate::context::*;
use crate::syntax::*;

pub struct TypeCheckVisitor {}

impl TypeCheckVisitor {
    pub fn new() -> TypeCheckVisitor {
        Self {}
    }

    pub fn visit_source_file(&self, source_file: &SourceFile, context: &mut Context) {}
}
