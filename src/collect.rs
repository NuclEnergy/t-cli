use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::{Callee, Program};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};
use tokio::fs::{create_dir_all, read_to_string, write};
use walkdir::WalkDir;

use crate::{config::Config, error::Error, utils::resolve::resolve_workspaces};

pub async fn run_collect(config: Config, verbose: bool) -> Result<(), Error> {
    let cm: Lrc<SourceMap> = Default::default();
    let mut collected: HashMap<PathBuf, BTreeMap<String, Option<String>>> = HashMap::new();
    let default_lang = config.languages.name.clone();
    let all_langs = config.languages.collect_languages();

    for target in config.targets {
        let workspaces = resolve_workspaces(".", &target.includes, &target.excludes)?;
        for workspace in workspaces {
            if verbose {
                println!("Scanning workspace: {}", workspace.display());
            }
            // only walk one level deep
            let walker = WalkDir::new(&workspace).max_depth(1).into_iter();
            for entry in walker.filter_map(Result::ok) {
                let path = entry.path();

                if path.is_file() && is_target_file(path) {
                    let content = read_to_string(path).await?;
                    let fm = cm.new_source_file(
                        FileName::Real(path.to_path_buf()).into(),
                        content.clone(),
                    );

                    let mut parser = Parser::new(
                        Syntax::Typescript(TsSyntax {
                            tsx: true,
                            decorators: true,
                            ..Default::default()
                        }),
                        StringInput::from(&*fm),
                        None,
                    );

                    let module = parser.parse_module().map_err(Error::ParseModule)?;

                    let mut visitor = FnKeyCollector {
                        keys: vec![],
                        fn_names: target.fn_names.clone(),
                    };

                    let program = Program::Module(module);
                    program.visit_with(&mut visitor);

                    if !visitor.keys.is_empty() {
                        let output_dir = workspace.join(&target.output);
                        for lang in &all_langs {
                            let map = collected
                                .entry(output_dir.join(format!("{lang}.json")))
                                .or_default();
                            for k in &visitor.keys {
                                if lang == &default_lang {
                                    map.insert(k.clone(), Some(k.clone()));
                                } else {
                                    map.entry(k.clone()).or_insert(None);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for (file_path, mut map) in collected {
        if file_path.exists() {
            let old_content = read_to_string(&file_path).await?;
            if let Ok(old_map) =
                serde_json::from_str::<BTreeMap<String, Option<String>>>(&old_content)
            {
                for (k, v) in old_map {
                    if let Some(val) = v {
                        map.insert(k, Some(val));
                    }
                }
            }
        }
        if let Some(parent) = file_path.parent() {
            create_dir_all(parent).await?;
        }
        let json = serde_json::to_string_pretty(&map)?;
        write(file_path, json).await?;
    }

    Ok(())
}

fn is_target_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("ts" | "tsx" | "js" | "jsx")
    )
}

struct FnKeyCollector {
    keys: Vec<String>,
    fn_names: Vec<String>,
}

impl Visit for FnKeyCollector {
    fn visit_call_expr(&mut self, expr: &swc_ecma_ast::CallExpr) {
        use swc_ecma_ast::{Expr, ExprOrSpread};

        if let Callee::Expr(boxed_expr) = &expr.callee {
            if let Expr::Ident(ident) = &**boxed_expr {
                if self.fn_names.contains(&ident.sym.to_string()) {
                    if let Some(ExprOrSpread { expr, .. }) = expr.args.first() {
                        if let Expr::Lit(swc_ecma_ast::Lit::Str(s)) = &**expr {
                            self.keys.push(s.value.to_string());
                        }
                    }
                }
            }
        }
        expr.visit_children_with(self);
    }
}
