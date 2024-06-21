use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use swc_core::{
    common::{
        errors::{ColorConfig, Handler},
        sync::Lrc,
        SourceMap,
    },
    ecma::visit::VisitWith,
};
use swc_core::{ecma::ast::*, ecma::visit::Visit};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

use crate::path_resolver::ToCanonicalString;

use super::path_resolver::{PathResolver, ResolvePath};

type Candidate = PathBuf;

#[derive(Debug)]
pub struct ParserCandidateScheduler {
    root: PathBuf,

    // candidates that are ready to be parsed
    good_candidates: VecDeque<Candidate>,

    // candidates that are still blocked by others
    blocked_candidates: HashMap<Candidate, usize>,

    // x is blocking [a, b, c, ...]
    blocking_table: HashMap<Candidate, Vec<Candidate>>,
}

impl ParserCandidateScheduler {
    pub fn new(root: &PathBuf) -> Self {
        let paths = Self::collect_paths(root);

        let mut scheduler = Self {
            root: root.clone(),
            good_candidates: VecDeque::new(),
            blocked_candidates: HashMap::new(),
            blocking_table: HashMap::new(),
        };

        let path_resolver = PathResolver::new(root.to_str().unwrap());

        for path in paths.iter() {
            if Self::is_valid_path(path) {
                match Self::get_blocked_by(path, &path_resolver) {
                    Some(blocked_by_vec) => {
                        scheduler
                            .blocked_candidates
                            .insert(path.clone(), blocked_by_vec.len());
                        for blocked_by in blocked_by_vec.iter() {
                            if !scheduler.blocking_table.contains_key(blocked_by) {
                                scheduler.blocking_table.insert(blocked_by.clone(), vec![]);
                            }
                            scheduler
                                .blocking_table
                                .get_mut(blocked_by)
                                .unwrap()
                                .push(path.clone());
                        }
                    }
                    None => scheduler.good_candidates.push_back(path.clone()),
                }
            }
        }

        scheduler
    }

    pub fn get_total_remaining_candidate_count(&self) -> usize {
        self.good_candidates.len() + self.blocked_candidates.len()
    }

    pub fn get_one_candidate(&mut self) -> Option<Candidate> {
        self.good_candidates.pop_front()
    }

    pub fn mark_candidate_as_parsed(&mut self, candidate: Candidate) {
        match self.blocking_table.remove(&candidate) {
            Some(blocked_list) => {
                for blocked_candidate in blocked_list {
                    let mut not_blocked_anymore = false;
                    self.blocked_candidates
                        .entry(blocked_candidate.clone())
                        .and_modify(|n| match n {
                            1 => not_blocked_anymore = true,
                            _ => *n -= 1,
                        });
                    if not_blocked_anymore {
                        self.blocked_candidates.remove(&blocked_candidate);
                        self.good_candidates.push_back(blocked_candidate);
                    }
                }
            }
            None => (),
        }
    }

    fn get_blocked_by(path: &PathBuf, path_resolver: &PathResolver) -> Option<Vec<PathBuf>> {
        let blocked_by = BlockedByVisitor::get_blocked_by(path, &path_resolver);
        match blocked_by.len() {
            0 => None,
            _ => Some(blocked_by.into_iter().collect()),
        }
    }

    fn is_valid_path(path: &PathBuf) -> bool {
        path.is_file()
            && path.extension().is_some()
            && ["ts", "tsx", "js", "jsx"].contains(&path.extension().unwrap().to_str().unwrap())
    }

    fn collect_paths(path: &PathBuf) -> Vec<PathBuf> {
        let mut files = vec![];

        match path.is_dir() {
            true => {
                for entry in path.read_dir().unwrap() {
                    if let Ok(entry) = entry {
                        files.append(&mut Self::collect_paths(
                            &entry.path().canonicalize().unwrap(),
                        ));
                    }
                }
            }
            false => files.push(path.clone()),
        }

        files
    }
}

struct BlockedByVisitor<'r> {
    current_path: PathBuf,
    blocked_by: HashSet<PathBuf>,
    path_resolver: &'r PathResolver,
}

impl<'r> BlockedByVisitor<'r> {
    fn get_blocked_by(path: &PathBuf, path_resolver: &'r PathResolver) -> HashSet<PathBuf> {
        let cm: Lrc<SourceMap> = Default::default();
        let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

        let fm = cm
            .load_file(path)
            .expect(format!("failed to load {:?}", path).as_str());

        let lexer = Lexer::new(
            Syntax::Typescript(TsConfig {
                tsx: true,
                decorators: false,
                dts: false,
                no_early_errors: true,
                disallow_ambiguous_jsx_like: true,
            }),
            // EsVersion defaults to es5
            Default::default(),
            StringInput::from(&*fm),
            None,
        );

        let mut parser = Parser::new_from(lexer);

        for e in parser.take_errors() {
            e.into_diagnostic(&handler).emit();
        }

        let module = parser
            .parse_module()
            .map_err(|e| {
                // Unrecoverable fatal error occurred
                e.into_diagnostic(&handler).emit()
            })
            .expect("failed to parser module");

        let mut visitor = Self {
            current_path: path.clone(),
            blocked_by: HashSet::new(),
            path_resolver,
        };
        module.visit_with(&mut visitor);

        visitor.blocked_by
    }

    fn add_to_blocked_by_if_needed(&mut self, import_src: &str) {
        if let Ok(resolved_path) = self.path_resolver.resolve_path(
            &self.current_path.to_canonical_string().unwrap(),
            import_src,
        ) {
            self.blocked_by.insert(Path::new(&resolved_path).into());
        } else {
            // Ignore the unresolvable module on purpose.
            // You can catch the unresolvable module here and adjust the SimplePathResolver or create your own.
        }
    }
}

impl<'r> Visit for BlockedByVisitor<'r> {
    fn visit_import_decl(&mut self, n: &ImportDecl) {
        match n.specifiers.get(0) {
            Some(ImportSpecifier::Namespace(_)) => {
                self.add_to_blocked_by_if_needed(n.src.value.as_str());
            }
            _ => (),
        }
    }

    fn visit_export_all(&mut self, n: &ExportAll) {
        self.add_to_blocked_by_if_needed(n.src.value.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let root = PathBuf::from("./test-project/everybodyyyy/src")
            .canonicalize()
            .unwrap();
        let scheduler = ParserCandidateScheduler::new(&root);

        assert_eq!(scheduler.root, root);

        assert_eq!(
            scheduler.good_candidates,
            vec![
                root.join("main.tsx"),
                root.join("components/unused-components/unused-avatar.tsx"),
                root.join("components/unused-components/unused-banner.tsx"),
                root.join("components/buttons/unused-button.tsx"),
                root.join("components/buttons/counter.tsx"),
                root.join("components/titles/unused-title.tsx"),
                root.join("components/titles/big-title.tsx"),
                root.join("components/links/vite.tsx"),
                root.join("components/links/unused-link.tsx"),
                root.join("components/links/react.tsx"),
                root.join("components/paragraphs/test-hmr.tsx"),
                root.join("components/paragraphs/read-the-docs.tsx"),
                root.join("components/paragraphs/unused-paragraph.tsx"),
                root.join("vite-env.d.ts"),
            ]
        );

        [
            ("App.tsx", 1),
            ("components/links/index.ts", 3),
            ("components/unused-components/index.ts", 2),
            ("components/buttons/index.ts", 2),
            ("components/paragraphs/index.ts", 3),
            ("components/index.ts", 5),
            ("components/titles/index.ts", 2),
        ]
        .iter()
        .for_each(|(key, value)| {
            assert_eq!(
                scheduler.blocked_candidates.get(&root.join(key)),
                Some(value)
            );
        });

        [
            (
                "components/paragraphs/test-hmr.tsx",
                vec!["components/paragraphs/index.ts"],
            ),
            (
                "components/unused-components/index.ts",
                vec!["components/index.ts"],
            ),
            (
                "components/links/vite.tsx",
                vec!["components/links/index.ts"],
            ),
            (
                "components/titles/big-title.tsx",
                vec!["components/titles/index.ts"],
            ),
            ("components/index.ts", vec!["App.tsx"]),
            (
                "components/links/react.tsx",
                vec!["components/links/index.ts"],
            ),
            (
                "components/buttons/unused-button.tsx",
                vec!["components/buttons/index.ts"],
            ),
            ("components/links/index.ts", vec!["components/index.ts"]),
            ("components/titles/index.ts", vec!["components/index.ts"]),
            (
                "components/buttons/counter.tsx",
                vec!["components/buttons/index.ts"],
            ),
            (
                "components/unused-components/unused-avatar.tsx",
                vec!["components/unused-components/index.ts"],
            ),
            (
                "components/links/unused-link.tsx",
                vec!["components/links/index.ts"],
            ),
            (
                "components/paragraphs/unused-paragraph.tsx",
                vec!["components/paragraphs/index.ts"],
            ),
            (
                "components/paragraphs/index.ts",
                vec!["components/index.ts"],
            ),
            (
                "components/titles/unused-title.tsx",
                vec!["components/titles/index.ts"],
            ),
            ("components/buttons/index.ts", vec!["components/index.ts"]),
            (
                "components/unused-components/unused-banner.tsx",
                vec!["components/unused-components/index.ts"],
            ),
            (
                "components/paragraphs/read-the-docs.tsx",
                vec!["components/paragraphs/index.ts"],
            ),
        ]
        .into_iter()
        .for_each(|(module, blocking_list)| {
            let module = root.join(module);
            let expected_blocking_list: Vec<PathBuf> =
                blocking_list.iter().map(|x| root.join(x)).collect();
            let actual_blocking_list = scheduler.blocking_table.get(&module).unwrap();
            assert_eq!(actual_blocking_list, &expected_blocking_list);
        });
    }
}
