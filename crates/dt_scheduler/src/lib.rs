use dt_path_resolver::{PathResolver, ToCanonicalString};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
};
use swc_core::{
    common::{
        errors::{ColorConfig, Handler},
        sync::Lrc,
        SourceMap,
    },
    ecma::visit::VisitWith,
    {ecma::ast::*, ecma::visit::Visit},
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};

type Candidate = PathBuf;

#[derive(Debug)]
pub struct ParserCandidateScheduler {
    // candidates that are ready to be parsed
    good_candidates: VecDeque<Candidate>,

    // candidates that are still blocked by others
    blocked_candidates: HashMap<Candidate, usize>,

    // x is blocking [a, b, c, ...]
    blocking_table: HashMap<Candidate, Vec<Candidate>>,
}

impl ParserCandidateScheduler {
    pub fn new(root: &str) -> Self {
        let paths = Self::collect_paths(&PathBuf::from(root));

        let mut scheduler = Self {
            good_candidates: VecDeque::new(),
            blocked_candidates: HashMap::new(),
            blocking_table: HashMap::new(),
        };

        let path_resolver = PathResolver::new(root);

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
            Syntax::Typescript(TsSyntax {
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
