use anyhow::bail;
use dt_parser::{
    anonymous_default_export::SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT, types::SymbolDependency,
};
use std::collections::{HashMap, HashSet};
use swc_core::ecma::{
    ast::*,
    visit::{Visit, VisitWith},
};

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

pub fn collect_route_dependency(
    module_ast: &Module,
    symbol_dependency: &SymbolDependency,
) -> anyhow::Result<Vec<Route>> {
    if should_collect(symbol_dependency) {
        let routes = collect(module_ast, symbol_dependency)?;
        return Ok(routes);
    }
    Ok(vec![])
}

#[derive(Debug)]
pub struct SymbolToRoutes {
    // one symbol can be used in multiple routes
    pub table: HashMap<String, HashMap<String, Vec<String>>>,
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
        if should_collect(symbol_dependency) {
            let routes = collect(module_ast, symbol_dependency)?;
            self.aggregate(symbol_dependency.canonical_path.as_str(), &routes);
        }
        Ok(())
    }

    fn aggregate(&mut self, module_path: &str, routes: &Vec<Route>) {
        let mut map = HashMap::new();
        for route in routes {
            for symbol in route.depend_on.iter() {
                if !map.contains_key(symbol) {
                    map.insert(symbol.to_string(), vec![route.path.to_owned()]);
                } else {
                    map.get_mut(symbol).unwrap().push(route.path.to_owned());
                }
            }
        }
        self.table.insert(module_path.to_owned(), map);
    }
}

#[derive(Debug)]
pub struct Route {
    pub path: String,
    pub depend_on: HashSet<String>,
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

    fn get_route_path(&self, route_object: &Expr) -> anyhow::Result<String> {
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
                                                Lit::Str(s) => return Ok(s.value.to_string()),
                                                _ => bail!("path value is literal but not string literal for {}", self.module_path)
                                            },
                                            _ => bail!(
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
            _ => bail!("invalid route object for {}", self.module_path),
        }
        bail!(
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
                                match self.get_route_path(&**value) {
                                    Ok(route_path) => {
                                        self.current_route_path = Some(Route {
                                            path: route_path,
                                            depend_on: HashSet::new(),
                                        });
                                        value.visit_children_with(self);
                                        self.routes.push(self.current_route_path.take().unwrap());
                                    }
                                    Err(_) => (),
                                }
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
            let map = $table.get(MOCK_MODULE_PATH).unwrap();
            $(
                let route_paths = map.get($symbol_name).unwrap();
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

    #[test]
    fn unsupported_template_literal_path() {
        let module_ast = Input::Code(
            r#"
            const A = "A";
            const B = "B";
            const C = "C";

            export default {
                'foo': {
                    path: `${prefix}/route/path`,
                    layouts: [A, B],
                    page: C
                },
            };
            "#,
        )
        .get_module_ast()
        .unwrap();
        let symbol_dependency = collect_symbol_dependency(&module_ast, MOCK_MODULE_PATH).unwrap();
        let mut symbol_to_routes = SymbolToRoutes::new();
        symbol_to_routes
            .collect_route_dependency(&module_ast, &symbol_dependency)
            .unwrap();

        assert!(symbol_to_routes
            .table
            .get(MOCK_MODULE_PATH)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn unsupported_react_router() {
        let module_ast = Input::Code(
            // Copied from https://github.com/remix-run/react-router/blob/dev/examples/basic/src/App.tsx
            r#"
            import { Routes, Route, Outlet, Link } from "react-router-dom";
            
            export default function App() {
              return (
                <div>
                  <h1>Basic Example</h1>
            
                  <p>
                    This example demonstrates some of the core features of React Router
                    including nested <code>&lt;Route&gt;</code>s,{" "}
                    <code>&lt;Outlet&gt;</code>s, <code>&lt;Link&gt;</code>s, and using a
                    "*" route (aka "splat route") to render a "not found" page when someone
                    visits an unrecognized URL.
                  </p>
            
                  {/* Routes nest inside one another. Nested route paths build upon
                        parent route paths, and nested route elements render inside
                        parent route elements. See the note about <Outlet> below. */}
                  <Routes>
                    <Route path="/" element={<Layout />}>
                      <Route index element={<Home />} />
                      <Route path="about" element={<About />} />
                      <Route path="dashboard" element={<Dashboard />} />
            
                      {/* Using path="*"" means "match anything", so this route
                            acts like a catch-all for URLs that we don't have explicit
                            routes for. */}
                      <Route path="*" element={<NoMatch />} />
                    </Route>
                  </Routes>
                </div>
              );
            }
            
            function Layout() {
              return (
                <div>
                  {/* A "layout route" is a good place to put markup you want to
                      share across all the pages on your site, like navigation. */}
                  <nav>
                    <ul>
                      <li>
                        <Link to="/">Home</Link>
                      </li>
                      <li>
                        <Link to="/about">About</Link>
                      </li>
                      <li>
                        <Link to="/dashboard">Dashboard</Link>
                      </li>
                      <li>
                        <Link to="/nothing-here">Nothing Here</Link>
                      </li>
                    </ul>
                  </nav>
            
                  <hr />
            
                  {/* An <Outlet> renders whatever child route is currently active,
                      so you can think about this <Outlet> as a placeholder for
                      the child routes we defined above. */}
                  <Outlet />
                </div>
              );
            }
            
            function Home() {
              return (
                <div>
                  <h2>Home</h2>
                </div>
              );
            }
            
            function About() {
              return (
                <div>
                  <h2>About</h2>
                </div>
              );
            }
            
            function Dashboard() {
              return (
                <div>
                  <h2>Dashboard</h2>
                </div>
              );
            }
            
            function NoMatch() {
              return (
                <div>
                  <h2>Nothing to see here!</h2>
                  <p>
                    <Link to="/">Go to the home page</Link>
                  </p>
                </div>
              );
            }
            "#,
        )
        .get_module_ast()
        .unwrap();
        let symbol_dependency = collect_symbol_dependency(&module_ast, MOCK_MODULE_PATH).unwrap();
        let mut symbol_to_routes = SymbolToRoutes::new();
        symbol_to_routes
            .collect_route_dependency(&module_ast, &symbol_dependency)
            .unwrap();

        assert!(symbol_to_routes.table.get(MOCK_MODULE_PATH).is_none());
    }
}
