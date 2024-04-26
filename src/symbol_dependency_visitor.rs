use std::collections::{HashMap, HashSet};
use swc_core::ecma::{
    ast::*,
    visit::{Visit, VisitWith},
};

use super::anonymous_default_export::get_anonymous_default_export_id;

#[derive(Debug)]
pub struct SymbolDependencyVisitor {
    current_id: Option<Id>,
    pub dependency: HashMap<Id, HashSet<Id>>,
}

impl SymbolDependencyVisitor {
    pub fn new(tracked_ids: HashSet<Id>) -> Self {
        let mut dependency = HashMap::new();
        for id in tracked_ids.iter() {
            dependency.insert(id.clone(), HashSet::new());
        }
        Self {
            current_id: None,
            dependency,
        }
    }
}

impl Visit for SymbolDependencyVisitor {
    fn visit_ident(&mut self, n: &Ident) {
        let id = n.to_id();
        let is_tracked_id = self.dependency.contains_key(&id);
        if self.current_id.is_none() || &id == self.current_id.as_ref().unwrap() || !is_tracked_id {
            return;
        }
        self.dependency
            .get_mut(self.current_id.as_ref().unwrap())
            .unwrap()
            .insert(id);
    }

    fn visit_module(&mut self, n: &Module) {
        for module_item in &n.body {
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
                                self.dependency
                                    .insert(get_anonymous_default_export_id(), HashSet::new());
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
                                self.dependency
                                    .insert(get_anonymous_default_export_id(), HashSet::new());
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
                                self.dependency
                                    .insert(get_anonymous_default_export_id(), HashSet::new());
                                self.current_id = Some(get_anonymous_default_export_id());
                                array_lit.visit_with(self);
                                self.current_id = None;
                            }
                            // export default { name1, name2 };
                            Expr::Object(object_lit) => {
                                self.dependency
                                    .insert(get_anonymous_default_export_id(), HashSet::new());
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
