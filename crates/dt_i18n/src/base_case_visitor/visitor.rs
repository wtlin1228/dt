use super::labels::{collect_labels_from_object_literal, LABELS};
use dt_parser::anonymous_default_export::get_anonymous_default_export_id;
use std::collections::{HashMap, HashSet};
use swc_core::ecma::{
    ast::*,
    visit::{Visit, VisitWith},
};

struct LabelVisitor {
    labels: Option<LABELS>,
}

impl LabelVisitor {
    pub fn new() -> Self {
        Self { labels: None }
    }
}

// Only find the module scopped `LABELS = translate({ /* ... */})`.
// `LABELS` could be defined in smaller scopped, let's ignore it now,
// since our latest style guide only allow defining `LABELS` at module
// scope.
impl Visit for LabelVisitor {
    fn visit_module(&mut self, node: &Module) {
        for module_item in &node.body {
            match module_item {
                ModuleItem::Stmt(stmt) => {
                    match stmt {
                        Stmt::Decl(decl) => match decl {
                            Decl::Var(var_decl) => {
                                for var_declarator in var_decl.decls.iter() {
                                    match labels_translate_args(var_declarator) {
                                        Some(args) => {
                                            if args.len() == 0 {
                                                panic!("translate should have at least 1 argument");
                                            }
                                            let first_arg = &args[0];
                                            match &*first_arg.expr {
                                                Expr::Object(object_lit) => {
                                                    self.labels = Some(
                                                    collect_labels_from_object_literal(object_lit)
                                                        .expect("collect labels from the object literal"));
                                                }
                                                _ => (),
                                            }
                                        }
                                        None => (),
                                    }
                                }
                            }
                            _ => (),
                        },
                        _ => (),
                    }
                }
                _ => (),
            }
        }
    }
}

fn labels_translate_args(decl: &VarDeclarator) -> Option<&Vec<ExprOrSpread>> {
    match &decl.name {
        Pat::Ident(binding_ident) => {
            if binding_ident.id.sym != "LABELS" {
                return None;
            }
            match &decl.init {
                Some(init) => match &**init {
                    Expr::Call(call_expr) => match &call_expr.callee {
                        Callee::Expr(expr) => match &**expr {
                            Expr::Ident(ident) => match ident.sym == "translate" {
                                true => Some(&call_expr.args),
                                false => None,
                            },
                            _ => None,
                        },
                        _ => None,
                    },
                    _ => None,
                },
                None => None,
            }
        }
        _ => None,
    }
}

struct LabelUsageVisitor {
    // current_id is used to track which identifier is using the LABELS
    current_id: Option<Id>,

    // labels is extracted by the LabelVisitor
    labels: LABELS,

    label_usage: HashMap<String, HashSet<String>>,
}

impl LabelUsageVisitor {
    pub fn new(labels: LABELS) -> Self {
        Self {
            current_id: None,
            labels,
            label_usage: HashMap::new(),
        }
    }
}

impl Visit for LabelUsageVisitor {
    fn visit_member_expr(&mut self, node: &MemberExpr) {
        if self.current_id.is_some() && is_labels_obj(node) {
            let translation_keys = self
                .labels
                .get_translation_keys_for_member_expr(node)
                .unwrap();
            if translation_keys.len() == 0 {
                return;
            }
            let current_symbol = self.current_id.as_ref().unwrap().0.to_string();
            if !self.label_usage.contains_key(&current_symbol) {
                self.label_usage
                    .insert(current_symbol.clone(), HashSet::new());
            }
            self.label_usage
                .entry(current_symbol)
                .and_modify(|set| set.extend(translation_keys));
        }
    }

    fn visit_module(&mut self, node: &Module) {
        for module_item in &node.body {
            match module_item {
                ModuleItem::ModuleDecl(module_decl) => match module_decl {
                    ModuleDecl::ExportDecl(ExportDecl { decl, .. }) => match decl {
                        // export class Foo {}
                        Decl::Class(ClassDecl { ident, class, .. }) => {
                            self.current_id = Some(ident.to_id());
                            class.visit_with(self);
                            self.current_id = None;
                        }
                        // export function foo() {}
                        Decl::Fn(FnDecl {
                            ident, function, ..
                        }) => {
                            self.current_id = Some(ident.to_id());
                            function.visit_with(self);
                            self.current_id = None;
                        }
                        // export const foo = init, bar = init
                        Decl::Var(var_decl) => {
                            for var_decl in &var_decl.decls {
                                match &var_decl.name {
                                    Pat::Ident(BindingIdent { id, .. }) => {
                                        self.current_id = Some(id.to_id());
                                        var_decl.init.visit_with(self);
                                        self.current_id = None;
                                    }
                                    _ => (),
                                }
                            }
                        }
                        _ => (),
                    },
                    ModuleDecl::ExportDefaultDecl(ExportDefaultDecl { decl, .. }) => match decl {
                        DefaultDecl::Class(ClassExpr { ident, class }) => match ident {
                            // export default class ClassName { /* … */ }
                            Some(ident) => {
                                self.current_id = Some(ident.to_id());
                                class.visit_with(self);
                                self.current_id = None;
                            }
                            // export default class { /* … */ }
                            None => {
                                self.current_id = Some(get_anonymous_default_export_id());
                                class.visit_with(self);
                                self.current_id = None;
                            }
                        },
                        DefaultDecl::Fn(FnExpr { ident, function }) => match ident {
                            // export default function functionName() { /* … */ }
                            Some(ident) => {
                                self.current_id = Some(ident.to_id());
                                function.visit_with(self);
                                self.current_id = None;
                            }
                            // export default function () { /* … */ }
                            None => {
                                self.current_id = Some(get_anonymous_default_export_id());
                                function.visit_with(self);
                                self.current_id = None;
                            }
                        },
                        DefaultDecl::TsInterfaceDecl(_) => (),
                    },
                    ModuleDecl::ExportDefaultExpr(ExportDefaultExpr { expr, .. }) => {
                        match &**expr {
                            // export default name1;
                            Expr::Ident(_) => (),
                            // export default [name1, name2];
                            Expr::Array(array_lit) => {
                                self.current_id = Some(get_anonymous_default_export_id());
                                array_lit.visit_with(self);
                                self.current_id = None;
                            }
                            // export default { name1, name2 };
                            Expr::Object(object_lit) => {
                                self.current_id = Some(get_anonymous_default_export_id());
                                object_lit.visit_with(self);
                                self.current_id = None;
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                },
                ModuleItem::Stmt(stmt) => match stmt {
                    Stmt::Decl(decl) => match decl {
                        // class Foo {}
                        Decl::Class(ClassDecl { ident, class, .. }) => {
                            self.current_id = Some(ident.to_id());
                            class.visit_with(self);
                            self.current_id = None;
                        }
                        // function foo() {}
                        Decl::Fn(FnDecl {
                            ident, function, ..
                        }) => {
                            self.current_id = Some(ident.to_id());
                            function.visit_with(self);
                            self.current_id = None;
                        }
                        // const foo = init, bar = init;
                        Decl::Var(var_decl) => {
                            for var_decl in &var_decl.decls {
                                match &var_decl.name {
                                    Pat::Ident(BindingIdent { id, .. }) => {
                                        self.current_id = Some(id.to_id());
                                        var_decl.init.visit_with(self);
                                        self.current_id = None;
                                    }
                                    _ => (),
                                }
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                },
            }
        }
    }
}

fn is_labels_obj(member_expr: &MemberExpr) -> bool {
    let mut obj: &Box<Expr> = &member_expr.obj;

    // find the ident by following the obj path, once found, check if it's sym is "LABELS"
    loop {
        match &**obj {
            Expr::Member(member_expr) => {
                obj = &member_expr.obj;
            }
            Expr::Ident(ident) => return ident.sym == "LABELS",
            _ => return false,
        }
    }
}

pub fn get_labels_usage(
    module: &Module,
) -> anyhow::Result<Option<HashMap<String, HashSet<String>>>> {
    let mut label_visitor = LabelVisitor::new();
    module.visit_with(&mut label_visitor);

    if let Some(labels) = label_visitor.labels {
        let mut label_usage_visitor = LabelUsageVisitor::new(labels);
        module.visit_with(&mut label_usage_visitor);
        return Ok(Some(label_usage_visitor.label_usage));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dt_test_utils::parse_module;

    macro_rules! assert_label_usage {
        ($code:expr, $($symbol_name:expr => $expected_keys:expr),* $(,)?) => {{
            let module = parse_module($code).unwrap();
            let label_usage = get_labels_usage(&module).unwrap().unwrap();

            $(
                let keys = label_usage.get($symbol_name).unwrap();
                assert_eq!(keys.len(), $expected_keys.len(), "keys count mismatch");
                for &expected_key in $expected_keys.iter() {
                    assert!(keys.contains(expected_key), "missing key: {}", expected_key);
                }
            )*
        }};
    }

    #[test]
    fn simple() {
        assert_label_usage!(
            r#"
            const LABELS = translate({
                bird: "i18n.bird",
                cat: "i18n.cat",
                dog: "i18n.dog",
            })

            const Bird = () => {
                return <div>{LABELS.bird}</div>
            }
            const Cat = () => {
                return <div>{LABELS.cat}</div>
            }
            const Dog = () => {
                return <div>{LABELS.dog}</div>
            }
            "#,
            "Bird" => ["i18n.bird"],
            "Cat" => ["i18n.cat"],
            "Dog" => ["i18n.dog"],
        );
    }

    #[test]
    fn complex() {
        assert_label_usage!(
            r#"
            const LABELS = translate({
                title: "i18n.pet.party",
                desc: ["i18n.pet.party.desc", "lazy"],
                attendants: {
                    bird: {
                        name: "i18n.bird",
                        desc: ["i18n.bird.desc", "lazy"],
                        size: {
                            [SIZE.samll]: "i18n.bird.small",
                            [SIZE.large]: ["i18n.bird.large", "lazy"],
                        },
                    },
                    cat: {
                        name: "i18n.cat",
                        desc: ["i18n.cat.desc", "lazy"],
                        size: {
                            [SIZE.samll]: "i18n.cat.small",
                            [SIZE.large]: ["i18n.cat.large", "lazy"],
                        },
                    },
                    dog: {
                        name: "i18n.dog",
                        desc: ["i18n.dog.desc", "lazy"],
                        size: {
                            [SIZE.samll]: "i18n.dog.small",
                            [SIZE.large]: ["i18n.dog.large", "lazy"],
                        },
                    },
                }
            })

            const Header = (props) => {
                const desc = LABELS.desc({ date: props.date })
                return (
                    <div>
                        <h1>{LABELS.title}</h1>
                        <p>{desc}</p>
                    </div>
                )   
            }

            const Pet = (props) => {
                return (
                    <div>
                        <h2>{props.name}</h2>
                        <p>{props.desc}</p>
                        <p>{props.size}</p>
                    </div>
                )
            }

            const PetParty = () => {
                const pets = usePets();
                return (
                    <div>
                        <Header />
                        {pets.map((pet) => {
                            <Pet 
                                name={LABELS.attendants[pet.type].name}
                                desc={LABELS.attendants[pet.type].desc({ name: pet.name })}
                                size={
                                    pet.size === SIZE.small
                                        ?  LABELS.attendants[pet.type].size.small
                                        :  LABELS.attendants[pet.type].size.large({ weight: pet.weight })
                                }
                            />
                        })}
                    </div>
                )
            }
            "#,
            "Header" => ["i18n.pet.party", "i18n.pet.party.desc"],
            "PetParty" => [
                "i18n.bird",
                "i18n.bird.desc",
                "i18n.bird.small",
                "i18n.bird.large",
                "i18n.cat",
                "i18n.cat.desc",
                "i18n.cat.small",
                "i18n.cat.large",
                "i18n.dog",
                "i18n.dog.desc",
                "i18n.dog.small",
                "i18n.dog.large",
            ]
        )
    }

    #[test]
    #[should_panic]
    fn only_support_module_scope_labels_for_now() {
        assert_label_usage!(
            r#"
            function Foo() {
                const LABELS = translate({
                    bird: 'i18n.bird',
                    cat: 'i18n.cat',
                    dog: 'i18n.dog',
                })
                return <div>{LABELS.bird}</div>
            }
            "#,
            "Foo" => ["i18n.bird"],
        );
    }
}
