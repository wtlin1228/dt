use anyhow::{bail, Context};
use std::collections::HashMap;

use super::used_by_graph::{UsedBy, UsedByGraph, UsedByOther, UsedByType};

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum TraceTarget {
    NamedExport(String),
    DefaultExport,
    LocalVar(String),
}

impl TraceTarget {
    pub fn to_string(&self) -> String {
        match self {
            TraceTarget::NamedExport(s) => s.to_owned(),
            TraceTarget::DefaultExport => String::from("default"),
            TraceTarget::LocalVar(s) => s.to_owned(),
        }
    }
}

pub type ModuleSymbol = (String, TraceTarget);

#[derive(Debug)]
pub struct DependencyTracker<'graph> {
    cache: HashMap<ModuleSymbol, Vec<Vec<ModuleSymbol>>>,
    graph: &'graph UsedByGraph,
}

impl<'graph> DependencyTracker<'graph> {
    pub fn new(graph: &'graph UsedByGraph) -> Self {
        Self {
            cache: HashMap::new(),
            graph,
        }
    }

    pub fn validate_module_path(&self, module_path: &str) -> anyhow::Result<()> {
        match self.graph.modules.contains_key(module_path) {
            true => Ok(()),
            false => bail!("module {} not found", module_path),
        }
    }

    pub fn get_traceable_named_exports(&self, module_path: &str) -> anyhow::Result<Vec<&str>> {
        Ok(self
            .graph
            .modules
            .get(module_path)
            .context(format!("module {} not found", module_path))?
            .named_export_table
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>())
    }

    pub fn get_traceable_local_variables(&self, module_path: &str) -> anyhow::Result<Vec<&str>> {
        Ok(self
            .graph
            .modules
            .get(module_path)
            .context(format!("module {} not found", module_path))?
            .local_variable_table
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>())
    }

    pub fn trace(&mut self, module_symbol: ModuleSymbol) -> anyhow::Result<Vec<Vec<ModuleSymbol>>> {
        // early return if cached
        if let Some(cached) = self.cache.get(&module_symbol) {
            return Ok(cached.clone());
        }

        let module = self
            .graph
            .modules
            .get(&module_symbol.0)
            .context(format!("module {} not found", module_symbol.0))?;

        let used_by = match module_symbol.1 {
            TraceTarget::NamedExport(ref name) => module
                .named_export_table
                .get(name)
                .context(format!("exported name {} not found", name))?,
            TraceTarget::DefaultExport => &module.default_export,
            TraceTarget::LocalVar(ref name) => module
                .local_variable_table
                .get(name)
                .context(format!("local symbol {} not found", name))?,
        };

        let mut res: Vec<Vec<ModuleSymbol>> = vec![];
        if let Some(used_by) = used_by {
            for next_target in used_by.iter() {
                let mut paths = match next_target {
                    UsedBy::Itself(used_by_type) => match used_by_type {
                        UsedByType::NamedExport(name) => self.trace((
                            module_symbol.0.clone(),
                            TraceTarget::NamedExport(name.to_string()),
                        ))?,
                        UsedByType::DefaultExport => {
                            self.trace((module_symbol.0.clone(), TraceTarget::DefaultExport))?
                        }
                        UsedByType::LocalVar(name) => self.trace((
                            module_symbol.0.clone(),
                            TraceTarget::LocalVar(name.to_string()),
                        ))?,
                    },
                    UsedBy::Other(UsedByOther { by, by_type }) => match by_type {
                        UsedByType::NamedExport(name) => {
                            self.trace((by.clone(), TraceTarget::NamedExport(name.to_string())))?
                        }
                        UsedByType::DefaultExport => {
                            self.trace((by.clone(), TraceTarget::DefaultExport))?
                        }
                        UsedByType::LocalVar(name) => {
                            self.trace((by.clone(), TraceTarget::LocalVar(name.to_string())))?
                        }
                    },
                };
                res.append(&mut paths);
            }
        }

        // append current ModuleSymbol to each path
        for path in res.iter_mut() {
            path.push(module_symbol.clone());
        }
        // append a new path for this target only
        res.push(vec![module_symbol.clone()]);

        // update cache
        self.cache.insert(module_symbol.clone(), res.clone());

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        anonymous_default_export::SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT, used_by_graph::Module,
    };

    macro_rules! s {
        ($s:expr) => {{
            $s.to_string()
        }};
    }

    #[test]
    fn it_works() {
        // import Kirby, { Power, Pink as KirbyPink, Puffy } from './kirby';
        // import * as Hawk from './hawk';
        // const sugar = '', salt = '';
        // const cruet = [sugar, salt];
        // export class PicnicBox {
        //     constructor() {
        //         this.cruet = cruet;
        //         this.sandwich = 'beef sandwich';
        //         this.cookie = { color: KirbyPink, texture: Puffy };
        //     }
        // }
        // const deliverPicnicBox = (location) => {
        //     Kirby.bring(new PicnicBox())
        //     Kirby.goto(location);
        //     Kirby.put()
        // }
        // function WelcomeMessage() {
        //     return "Welcome 🤗 Kirby is delivering your picnic box 👜";
        // }
        // export { WelcomeMessage as welcome };
        // export function InvitationCard() {
        //     const [opened, setOpened] = React.useState(false);
        //     if (!opened) {
        //         return (
        //             <Hawk.PigNose
        //                 onPush={() => {
        //                     setOpened(true);
        //                     deliverPicnicBox();
        //                 }}
        //             />
        //         )
        //     } else {
        //         return <WelcomeMessage />
        //     }
        // }
        // export default InvitationCard;
        // export * from './wild';
        // export * as Wild from './wild';
        let graph = UsedByGraph {
            modules: HashMap::from([
                (
                    s!("kirby"),
                    Module {
                        local_variable_table: HashMap::from([
                            (
                                s!("Power"),
                                Some(vec![
                                    UsedBy::Itself(UsedByType::NamedExport(s!("Power"))),
                                    UsedBy::Itself(UsedByType::LocalVar(
                                        SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                                    )),
                                ]),
                            ),
                            (
                                s!("Pink"),
                                Some(vec![
                                    UsedBy::Itself(UsedByType::NamedExport(s!("Pink"))),
                                    UsedBy::Itself(UsedByType::LocalVar(s!("Power"))),
                                ]),
                            ),
                            (
                                s!("Puffy"),
                                Some(vec![
                                    UsedBy::Itself(UsedByType::NamedExport(s!("Puffy"))),
                                    UsedBy::Itself(UsedByType::LocalVar(s!("Power"))),
                                ]),
                            ),
                            (
                                SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT.to_string(),
                                Some(vec![UsedBy::Itself(UsedByType::DefaultExport)]),
                            ),
                        ]),
                        named_export_table: HashMap::from([
                            (
                                s!("Power"),
                                Some(vec![UsedBy::Other(UsedByOther {
                                    by: s!("PicnicTime"),
                                    by_type: UsedByType::LocalVar(s!("Power")),
                                })]),
                            ),
                            (
                                s!("Pink"),
                                Some(vec![UsedBy::Other(UsedByOther {
                                    by: s!("PicnicTime"),
                                    by_type: UsedByType::LocalVar(s!("KirbyPink")),
                                })]),
                            ),
                            (
                                s!("Puffy"),
                                Some(vec![UsedBy::Other(UsedByOther {
                                    by: s!("PicnicTime"),
                                    by_type: UsedByType::LocalVar(s!("Puffy")),
                                })]),
                            ),
                        ]),
                        default_export: Some(vec![UsedBy::Other(UsedByOther {
                            by: s!("PicnicTime"),
                            by_type: UsedByType::LocalVar(s!("Kirby")),
                        })]),
                    },
                ),
                (
                    s!("hawk"),
                    Module {
                        local_variable_table: HashMap::from([
                            (
                                s!("PigNose"),
                                Some(vec![UsedBy::Itself(UsedByType::NamedExport(s!("PigNose")))]),
                            ),
                            (
                                s!("Pink"),
                                Some(vec![
                                    UsedBy::Itself(UsedByType::NamedExport(s!("Pink"))),
                                    UsedBy::Itself(UsedByType::LocalVar(s!("PigNose"))),
                                ]),
                            ),
                        ]),
                        named_export_table: HashMap::from([
                            (
                                s!("PigNose"),
                                Some(vec![UsedBy::Other(UsedByOther {
                                    by: s!("PicnicTime"),
                                    by_type: UsedByType::LocalVar(s!("Hawk")),
                                })]),
                            ),
                            (
                                s!("Pink"),
                                Some(vec![UsedBy::Other(UsedByOther {
                                    by: s!("PicnicTime"),
                                    by_type: UsedByType::LocalVar(s!("Hawk")),
                                })]),
                            ),
                        ]),
                        default_export: None,
                    },
                ),
                (
                    s!("wild"),
                    Module {
                        local_variable_table: HashMap::from([
                            (
                                s!("ZigZagWild"),
                                Some(vec![
                                    UsedBy::Itself(UsedByType::NamedExport(s!("ZigZagWild"))),
                                    UsedBy::Itself(UsedByType::DefaultExport),
                                ]),
                            ),
                            (
                                s!("GuruguruWild"),
                                Some(vec![UsedBy::Itself(UsedByType::NamedExport(s!(
                                    "GuruguruWild"
                                )))]),
                            ),
                        ]),
                        named_export_table: HashMap::from([
                            (
                                s!("ZigZagWild"),
                                Some(vec![
                                    UsedBy::Other(UsedByOther {
                                        by: s!("PicnicTime"),
                                        by_type: UsedByType::NamedExport(s!("ZigZagWild")),
                                    }),
                                    UsedBy::Other(UsedByOther {
                                        by: s!("PicnicTime"),
                                        by_type: UsedByType::NamedExport(s!("Wild")),
                                    }),
                                ]),
                            ),
                            (
                                s!("GuruguruWild"),
                                Some(vec![
                                    UsedBy::Other(UsedByOther {
                                        by: s!("PicnicTime"),
                                        by_type: UsedByType::NamedExport(s!("GuruguruWild")),
                                    }),
                                    UsedBy::Other(UsedByOther {
                                        by: s!("PicnicTime"),
                                        by_type: UsedByType::NamedExport(s!("Wild")),
                                    }),
                                ]),
                            ),
                        ]),
                        default_export: None,
                    },
                ),
                (
                    s!("happy"),
                    Module {
                        local_variable_table: HashMap::new(),
                        named_export_table: HashMap::new(),
                        default_export: None,
                    },
                ),
                (
                    s!("PicnicTime"),
                    Module {
                        local_variable_table: HashMap::from([
                            // import Kirby, { Power, Pink as KirbyPink, Puffy } from './kirby';
                            (
                                s!("Kirby"),
                                Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!(
                                    "deliverPicnicBox"
                                )))]),
                            ),
                            (s!("Power"), None),
                            (
                                s!("KirbyPink"),
                                Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("PicnicBox")))]),
                            ),
                            (
                                s!("Puffy"),
                                Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("PicnicBox")))]),
                            ),
                            // import * as Hawk from './hawk';
                            (
                                s!("Hawk"),
                                Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!(
                                    "InvitationCard"
                                )))]),
                            ),
                            // declare locally
                            (
                                s!("sugar"),
                                Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("curet")))]),
                            ),
                            (
                                s!("salt"),
                                Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("curet")))]),
                            ),
                            (
                                s!("curet"),
                                Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!("PicnicBox")))]),
                            ),
                            (
                                s!("PicnicBox"),
                                Some(vec![
                                    UsedBy::Itself(UsedByType::LocalVar(s!("deliverPicnicBox"))),
                                    UsedBy::Itself(UsedByType::NamedExport(s!("PicnicBox"))),
                                ]),
                            ),
                            (
                                s!("deliverPicnicBox"),
                                Some(vec![UsedBy::Itself(UsedByType::LocalVar(s!(
                                    "InvitationCard"
                                )))]),
                            ),
                            (
                                s!("WelcomeMessage"),
                                Some(vec![
                                    UsedBy::Itself(UsedByType::LocalVar(s!("InvitationCard"))),
                                    UsedBy::Itself(UsedByType::NamedExport(s!("welcome"))),
                                ]),
                            ),
                            (
                                s!("InvitationCard"),
                                Some(vec![
                                    UsedBy::Itself(UsedByType::NamedExport(s!("InvitationCard"))),
                                    UsedBy::Itself(UsedByType::DefaultExport),
                                ]),
                            ),
                        ]),
                        named_export_table: HashMap::from([
                            // export local variables
                            (s!("PicnicBox"), None),
                            (s!("welcome"), None),
                            (s!("InvitationCard"), None),
                            // export * from './wild';
                            (s!("ZigZagWild"), None),
                            (s!("GuruguruWild"), None),
                            // export * as Wild from './wild';
                            (s!("Wild"), None),
                        ]),
                        default_export: None,
                    },
                ),
            ]),
        };

        let mut dt = DependencyTracker::new(&graph);
        let paths = dt
            .trace((String::from("kirby"), TraceTarget::LocalVar(s!("Power"))))
            .unwrap();

        println!("{:#?}", dt);

        assert_eq!(
            paths,
            vec![
                vec![
                    (s!("PicnicTime"), TraceTarget::LocalVar(s!("Power"))),
                    (s!("kirby"), TraceTarget::NamedExport(s!("Power"))),
                    (s!("kirby"), TraceTarget::LocalVar(s!("Power")))
                ],
                vec![
                    (s!("kirby"), TraceTarget::NamedExport(s!("Power"))),
                    (s!("kirby"), TraceTarget::LocalVar(s!("Power")))
                ],
                vec![
                    (
                        s!("PicnicTime"),
                        TraceTarget::NamedExport(s!("InvitationCard"))
                    ),
                    (
                        s!("PicnicTime"),
                        TraceTarget::LocalVar(s!("InvitationCard"))
                    ),
                    (
                        s!("PicnicTime"),
                        TraceTarget::LocalVar(s!("deliverPicnicBox"))
                    ),
                    (s!("PicnicTime"), TraceTarget::LocalVar(s!("Kirby"))),
                    (s!("kirby"), TraceTarget::DefaultExport),
                    (s!("kirby"), TraceTarget::LocalVar(s!("+-*/default@#$%"))),
                    (s!("kirby"), TraceTarget::LocalVar(s!("Power")))
                ],
                vec![
                    (s!("PicnicTime"), TraceTarget::DefaultExport),
                    (
                        s!("PicnicTime"),
                        TraceTarget::LocalVar(s!("InvitationCard"))
                    ),
                    (
                        s!("PicnicTime"),
                        TraceTarget::LocalVar(s!("deliverPicnicBox"))
                    ),
                    (s!("PicnicTime"), TraceTarget::LocalVar(s!("Kirby"))),
                    (s!("kirby"), TraceTarget::DefaultExport),
                    (s!("kirby"), TraceTarget::LocalVar(s!("+-*/default@#$%"))),
                    (s!("kirby"), TraceTarget::LocalVar(s!("Power")))
                ],
                vec![
                    (
                        s!("PicnicTime"),
                        TraceTarget::LocalVar(s!("InvitationCard"))
                    ),
                    (
                        s!("PicnicTime"),
                        TraceTarget::LocalVar(s!("deliverPicnicBox"))
                    ),
                    (s!("PicnicTime"), TraceTarget::LocalVar(s!("Kirby"))),
                    (s!("kirby"), TraceTarget::DefaultExport),
                    (s!("kirby"), TraceTarget::LocalVar(s!("+-*/default@#$%"))),
                    (s!("kirby"), TraceTarget::LocalVar(s!("Power")))
                ],
                vec![
                    (
                        s!("PicnicTime"),
                        TraceTarget::LocalVar(s!("deliverPicnicBox"))
                    ),
                    (s!("PicnicTime"), TraceTarget::LocalVar(s!("Kirby"))),
                    (s!("kirby"), TraceTarget::DefaultExport),
                    (s!("kirby"), TraceTarget::LocalVar(s!("+-*/default@#$%"))),
                    (s!("kirby"), TraceTarget::LocalVar(s!("Power")))
                ],
                vec![
                    (s!("PicnicTime"), TraceTarget::LocalVar(s!("Kirby"))),
                    (s!("kirby"), TraceTarget::DefaultExport),
                    (s!("kirby"), TraceTarget::LocalVar(s!("+-*/default@#$%"))),
                    (s!("kirby"), TraceTarget::LocalVar(s!("Power")))
                ],
                vec![
                    (s!("kirby"), TraceTarget::DefaultExport),
                    (s!("kirby"), TraceTarget::LocalVar(s!("+-*/default@#$%"))),
                    (s!("kirby"), TraceTarget::LocalVar(s!("Power")))
                ],
                vec![
                    (s!("kirby"), TraceTarget::LocalVar(s!("+-*/default@#$%"))),
                    (s!("kirby"), TraceTarget::LocalVar(s!("Power")))
                ],
                vec![(s!("kirby"), TraceTarget::LocalVar(s!("Power")))]
            ]
        )
    }
}
