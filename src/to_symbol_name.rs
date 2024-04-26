use swc_core::ecma::ast;

pub trait ToSymbolName {
    fn to_symbol_name(&self) -> String;
}

impl ToSymbolName for ast::Ident {
    fn to_symbol_name(&self) -> String {
        self.to_id().0.to_string()
    }
}

impl ToSymbolName for ast::Id {
    fn to_symbol_name(&self) -> String {
        self.0.to_string()
    }
}
