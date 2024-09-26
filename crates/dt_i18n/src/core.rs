use super::base_case_visitor;
use std::collections::{HashMap, HashSet};
use swc_core::ecma::ast::Module;

#[derive(Debug)]
struct TranslationUsage {
    data: HashMap<String, HashSet<String>>,
}

impl TranslationUsage {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn extend(&mut self, target: HashMap<String, HashSet<String>>) {
        for (key, value) in target.iter() {
            if !self.data.contains_key(key) {
                self.data.insert(key.to_owned(), HashSet::new());
            }
            self.data.entry(key.to_owned()).and_modify(|set| {
                set.extend(value.clone());
            });
        }
    }
}

pub fn collect_translation(module: &Module) -> anyhow::Result<HashMap<String, HashSet<String>>> {
    let mut translation_usage = TranslationUsage::new();
    if let Some(v) = base_case_visitor::get_labels_usage(&module)? {
        translation_usage.extend(v);
    }
    // Handle more cases here, like:
    // - LABEL_KEYS
    // - i18nKey
    // - translate(<String Literal>)
    // - ...

    Ok(translation_usage.data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dt_test_utils::parse_module;

    #[test]
    fn unsupported_trans_components() {
        let module = parse_module(
            r#"
            const Foo = () => <TransBlock i18nKey="i18n.key" />
            const Bar = () => <Trans i18nKey="i18n.key" />
            "#,
        )
        .unwrap();
        assert!(collect_translation(&module).unwrap().len() == 0);
    }

    #[test]
    fn unsupported_styled_trans_components() {
        let module = parse_module(
            r#"
            const StyledTransBlock = styled(TransBlock)``
            const StyledTrans = styled(Trans)``
            const Foo = () => <StyledTransBlock i18nKey="i18n.key" />
            const Bar = () => <StyledTrans i18nKey="i18n.key" />
            "#,
        )
        .unwrap();
        assert!(collect_translation(&module).unwrap().len() == 0);
    }

    #[test]
    fn unsupported_imported_labels() {
        let module = parse_module(
            r#"
            import LABELS from "some/where"
            const Foo = () => <div>{LABELS}</div>
            "#,
        )
        .unwrap();
        assert!(collect_translation(&module).unwrap().len() == 0);
    }

    #[test]
    fn unsupported_direct_usage() {
        let module = parse_module(
            r#"
            const LABELS = translate({
                bird: "i18n.bird",
                cat: "i18n.cat",
                dog: "i18n.dog",
            })
            const Foo = () => (
                <A 
                    l={LABELS} 
                />
            )
            "#,
        )
        .unwrap();
        assert!(collect_translation(&module).unwrap().len() == 0);
    }

    #[test]
    fn unsupported_inner_defined_labels() {
        let module = parse_module(
            r#"
            const Foo = () => {
                const LABELS = translate({
                    bird: "i18n.bird",
                    cat: "i18n.cat",
                    dog: "i18n.dog",
                })
                return (
                    <div>{LABELS.bird}</div>
                )
            }
            "#,
        )
        .unwrap();
        assert!(collect_translation(&module).unwrap().len() == 0);
    }

    #[test]
    fn unsupported_single_translation() {
        let module = parse_module(
            r#"
            const L_BIRD = translate("i18n.bird")
            "#,
        )
        .unwrap();
        assert!(collect_translation(&module).unwrap().len() == 0);
    }

    #[test]
    fn unsupported_inner_single_translation() {
        let module = parse_module(
            r#"
            const Foo = () => {
                const L_BIRD = translate("i18n.bird")
            }
            "#,
        )
        .unwrap();
        assert!(collect_translation(&module).unwrap().len() == 0);
    }

    #[test]
    fn unsupported_label_keys() {
        let module = parse_module(
            r#"
            const LABEL_KEYS = {
                bird: "i18n.bird",
                cat: "i18n.cat",
                dog: "i18n.dog",
            }
            const LABELS = translate(LABEL_KEYS)
            const Foo = () => <div>{LABELS.bird}</div>
            "#,
        )
        .unwrap();
        assert!(collect_translation(&module).unwrap().len() == 0);
    }
}
