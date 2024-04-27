use swc_core::{atoms::Atom, common::SyntaxContext, ecma::ast::Id};

pub const SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT: &'static str = "+-*/default@#$%";

pub fn get_anonymous_default_export_id() -> Id {
    (
        Atom::new(SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string()),
        SyntaxContext::empty(),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_get_id() {
        let mut set: HashSet<Id> = HashSet::new();
        set.insert(get_anonymous_default_export_id());
        assert!(set.contains(&get_anonymous_default_export_id()));
    }
}
