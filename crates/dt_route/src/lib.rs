use dt_parser::{
    anonymous_default_export::SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT, types::SymbolDependency,
};
use dt_tracker::{ModuleSymbol, TraceTarget};
use std::collections::{HashMap, HashSet};
use swc_core::ecma::{
    ast::*,
    visit::{Visit, VisitWith},
};

#[derive(Debug)]
pub struct SymbolToRoutes {
    // one symbol can be used in multiple routes
    table: HashMap<ModuleSymbol, Vec<String>>,
}

impl SymbolToRoutes {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub fn collect_route_dependency(
        &mut self,
        module_ast: &Module,
        symbol_dependency: &SymbolDependency,
    ) -> anyhow::Result<()> {
        if Self::should_collect(symbol_dependency) {
            let routes = Self::collect(module_ast, symbol_dependency)?;
            self.aggregate(symbol_dependency.canonical_path.as_str(), routes);
        }
        Ok(())
    }

    fn should_collect(symbol_dependency: &SymbolDependency) -> bool {
        // filename should be "routes.js"
        if !symbol_dependency.canonical_path.ends_with("/routes.js") {
            return false;
        }
        // routes.js should have default export
        if symbol_dependency.default_export.is_none() {
            return false;
        }
        // routes.js should have anonumous default export
        match symbol_dependency.default_export.as_ref().unwrap() {
            dt_parser::types::ModuleExport::Local(exported_symbol) => {
                if exported_symbol != SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT {
                    return false;
                }
            }
            dt_parser::types::ModuleExport::ReExportFrom(_) => return false,
        }
        // default export should depend on some symbols
        match symbol_dependency
            .local_variable_table
            .get(SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT)
            .unwrap()
            .depend_on
        {
            Some(_) => return true,
            None => return false,
        }
    }

    fn collect(
        module_ast: &Module,
        symbol_dependency: &SymbolDependency,
    ) -> anyhow::Result<Vec<Route>> {
        let mut route_visitor = RouteVisitor::new(symbol_dependency);
        module_ast.visit_with(&mut route_visitor);
        Ok(route_visitor.routes)
    }

    fn aggregate(&mut self, module_path: &str, routes: Vec<Route>) {
        for route in routes {
            for symbol in route.depend_on {
                let module_symbol: ModuleSymbol =
                    (module_path.to_owned(), TraceTarget::LocalVar(symbol));
                if !self.table.contains_key(&module_symbol) {
                    self.table
                        .insert(module_symbol, vec![route.path.to_owned()]);
                } else {
                    self.table
                        .get_mut(&module_symbol)
                        .unwrap()
                        .push(route.path.to_owned());
                }
            }
        }
    }
}

#[derive(Debug)]
struct Route {
    path: String,
    depend_on: HashSet<String>,
}

#[derive(Debug)]
pub struct RouteVisitor {
    module_path: String,
    routes: Vec<Route>,
    to_track: HashSet<String>,
    current_route_path: Option<Route>,
}

impl RouteVisitor {
    pub fn new(symbol_dependency: &SymbolDependency) -> Self {
        let depend_on = symbol_dependency
            .local_variable_table
            .get(SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT)
            .unwrap()
            .depend_on
            .as_ref()
            .unwrap()
            .clone();
        Self {
            module_path: symbol_dependency.canonical_path.to_owned(),
            routes: Vec::new(),
            to_track: depend_on.into_iter().collect(),
            current_route_path: None,
        }
    }

    fn get_route_path(&self, route_object: &Expr) -> String {
        match route_object {
            Expr::Object(object_lit) => {
                for prop in object_lit.props.iter() {
                    match prop {
                        PropOrSpread::Prop(prop) => match &**prop {
                            Prop::KeyValue(KeyValueProp { key, value }) => match key {
                                PropName::Ident(ident_name) => {
                                    if ident_name.sym == "path" {
                                        match &**value {
                                            Expr::Lit(lit) => match lit {
                                                Lit::Str(s) => return s.value.to_string(),
                                                _ => panic!("path value is literal but not string literal for {}", self.module_path)
                                            },
                                            _ => panic!(
                                                "failed to transform value to string for {}",
                                                self.module_path
                                            ),
                                        }
                                    }
                                }
                                _ => continue,
                            },
                            _ => (),
                        },
                        _ => (),
                    }
                }
            }
            _ => panic!("invalid route object for {}", self.module_path),
        }
        panic!(
            "failed to find the path in the route object for {}",
            self.module_path
        );
    }
}

impl Visit for RouteVisitor {
    fn visit_ident(&mut self, node: &Ident) {
        let id = node.sym.to_string();
        if self.current_route_path.is_none() || !self.to_track.contains(&id) {
            return;
        }
        self.current_route_path
            .as_mut()
            .unwrap()
            .depend_on
            .insert(id);
    }

    fn visit_export_default_expr(&mut self, node: &ExportDefaultExpr) {
        match &*node.expr {
            Expr::Object(object_lit) => {
                for prop in object_lit.props.iter() {
                    match prop {
                        PropOrSpread::Prop(prop) => match &**prop {
                            Prop::KeyValue(KeyValueProp { value, .. }) => {
                                let route_path = self.get_route_path(&**value);
                                self.current_route_path = Some(Route {
                                    path: route_path,
                                    depend_on: HashSet::new(),
                                });
                                value.visit_children_with(self);
                                self.routes.push(self.current_route_path.take().unwrap());
                            }
                            _ => (),
                        },
                        PropOrSpread::Spread(_) => (),
                    }
                }
            }
            // only support export default { /* ... */ }
            _ => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dt_parser::{collect_symbol_dependency, Input};

    const MOCK_MODULE_PATH: &'static str = "some/path/routes.js";

    macro_rules! assert_symbol_to_routes_table {
        ($table:expr, $($symbol_name:expr => $expected_route_paths:expr),* $(,)?) => {{
            $(
                let route_paths = $table.get(
                    &(
                        MOCK_MODULE_PATH.to_string(),
                        TraceTarget::LocalVar($symbol_name.to_string())
                    )).unwrap();
                assert_eq!(route_paths.len(), $expected_route_paths.len());
                for (i, expected) in $expected_route_paths.into_iter().enumerate() {
                    assert_eq!(route_paths[i], expected);
                }
            )*
        }};
    }

    #[test]
    fn it_works() {
        let input = r#"
            import lazyLoad from 'some-where';
            import { A } from 'some-where';
            const PageB = lazyLoad(() => import('B'));
            const PageC = lazyLoad(() => import('C'));
            const PageD = lazyLoad(() => import('D'));
            const E = 'E';
            const F = () => 'F';
            const ModalG = () => 'G';
            

            export default {
                'route.1': {
                    path: '/route/path/1',
                    redirect: {
                        to: 'route.2',
                    },
                },
                'route.2': {
                    path: '/route/path/2',
                    page: PageB,
                    layouts: [A, E, F],
                },
                'route.3': {
                    path: '/route/path/3',
                    modal: {
                        component: ModalG,
                        fallbackBackground: {
                            routeName: 'route.2',
                        },
                    },
                    layouts: [A],
                },
                'route.4': {
                    path: '/route/path/4',
                    page: PageC,
                    layouts: [A, F],
                },
                'route.5': {
                    path: '/route/path/5',
                    page: PageD,
                    layouts: [E],
                },
            };
        "#;

        let module_ast = Input::Code(input).get_module_ast().unwrap();
        let symbol_dependency = collect_symbol_dependency(&module_ast, MOCK_MODULE_PATH).unwrap();
        let mut symbol_to_routes = SymbolToRoutes::new();
        symbol_to_routes
            .collect_route_dependency(&module_ast, &symbol_dependency)
            .unwrap();

        assert_symbol_to_routes_table!(
            &symbol_to_routes.table,
            "A" => [
                "/route/path/2",
                "/route/path/3",
                "/route/path/4",
            ],
            "PageB" => [
                "/route/path/2",
            ],
            "PageC" => [
                "/route/path/4",
            ],
            "PageD" => [
                "/route/path/5",
            ],
            "E" => [
                "/route/path/2",
                "/route/path/5",
            ],
            "F" => [
                "/route/path/2",
                "/route/path/4",
            ],
            "ModalG" => [
                "/route/path/3",
            ],
        )
    }
}
