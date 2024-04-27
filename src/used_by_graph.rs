use std::collections::HashMap;

use super::anonymous_default_export::SYMBOL_NAME_FOR_ANONYMOUS_DEFAULT_EXPORT;

// local variables can be used by:
// - local variables
//       const name1 = name2; -> Local(name2) is used by Local(name1)
// - named exports
//       export { name2 as name1 } -> Local(name2) is used by NamedExport(name1)
// - default export
//       export default name -> Local(name) is used by DefaultExport
//
// named exports can be used by:
// - local variables of other modules
//       in 'some-module':
//       import { name } from 'this-module' -> NamedExport(name) is used by Local(name) of 'some-module'
//       import { name as name1 } from 'this-module' -> NamedExport(name) is used by Local(name1) of 'some-module'
// - named exports of other modules
//       in 'some-module':
//       export { name } from 'this-module' -> NamedExport(name) is used by NamedExport(name) of 'some-module'
//       export { name as name1 } from 'this-module' -> NamedExport(name) is used by NamedExport(name1) of 'some-module'
//       export * from 'this-module' -> NamedExport(name) is used by NamedExport(name) of 'some-module'
// - default exports of other modules
//       in 'some-module':
//       export { name as default } from 'this-module' -> NamedExport(name) is used by DefaultExport of 'some-module'
//
// default exports can be used by:
// - local variables of other modules
//       in 'some-module':
//       import name from 'this-module' -> DefaultExport is used by Local(name) of 'some-module'
// - named exports of other modules
//       in 'some-module':
//       export { default as name } from 'this-module' -> DefaultExport is used by NamedExport(name) of 'some-module'
// - default exports of other modules
//       in 'some-module':
//       export { default } from 'this-module' -> DefaultExport is used by DefaultExport of 'some-module'

pub struct UsedByGraph {
    reversed_modules: HashMap<String, Module>,
}

impl UsedByGraph {
    pub fn trace(&self, module_id: &str, trace_target: TraceTarget) {}
}

struct Module {
    local_variable_table: HashMap<String, Option<Vec<UsedBy>>>,
    named_export_table: HashMap<String, Option<Vec<UsedBy>>>,
    default_export: Option<Vec<UsedBy>>,
}

enum UsedBy {
    Itself(UsedByType),
    Other(UsedByOther),
}

struct UsedByOther {
    by: String,
    by_type: UsedByType,
}

enum UsedByType {
    NamedExport(String),
    DefaultExport,
    LocalVar(String),
}

enum TraceTarget {
    NamedExport(String),
    DefaultExport,
    LocalVar(String),
}

#[cfg(test)]
mod tests {
    use super::*;

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
            reversed_modules: HashMap::from([
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

        graph.trace("kirby", TraceTarget::LocalVar(s!("Power")));
    }
}
