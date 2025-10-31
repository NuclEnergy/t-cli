use std::{collections::HashMap, path::PathBuf};

use indexmap::IndexMap;
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::{Callee, Program};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};
use tokio::fs::{create_dir_all, read_to_string, write};
use walkdir::WalkDir;

use crate::{
    config::Config,
    error::Error,
    utils::{is_target_file::is_target_file, resolve::resolve_workspaces},
};

pub async fn run_collect(config: Config, verbose: bool) -> Result<(), Error> {
    let cm: Lrc<SourceMap> = Default::default();
    let mut collected: HashMap<PathBuf, IndexMap<String, Option<String>>> = HashMap::new();
    let default_lang = config.languages.name.clone();
    let all_langs = config.languages.collect_languages();

    for target in config.targets {
        let workspaces = resolve_workspaces(".", &target.includes, &target.excludes)?;
        for workspace in workspaces {
            if verbose {
                println!("Scanning workspace: {}", workspace.display());
            }

            // Collect one level of files and sort by file name a-z
            let mut files: Vec<PathBuf> = WalkDir::new(&workspace)
                .max_depth(1)
                .into_iter()
                .filter_map(Result::ok)
                .map(|e| e.into_path())
                .filter(|p| p.is_file() && is_target_file(p))
                .collect();

            files.sort_by(|a, b| {
                a.to_string_lossy()
                    .to_lowercase()
                    .cmp(&b.to_string_lossy().to_lowercase())
            });

            for path in files {
                let content = read_to_string(&path).await?;
                let fm =
                    cm.new_source_file(FileName::Real(path.to_path_buf()).into(), content.clone());

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
                                // Default language: key => same value
                                map.insert(k.clone(), Some(k.clone()));
                            } else {
                                // Other languages: only placeholder, keep insertion order
                                map.entry(k.clone()).or_insert(None);
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
            // Use IndexMap to read old file, preserve order semantics
            if let Ok(old_map) =
                serde_json::from_str::<IndexMap<String, Option<String>>>(&old_content)
            {
                for (k, v) in old_map {
                    if map.contains_key(&k) {
                        // Already exists: only override when old value is Some
                        if let Some(val) = v {
                            map.insert(k, Some(val));
                        }
                    } else {
                        // Not exists: append to the end regardless of Some or None
                        map.insert(k, v);
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

pub struct FnKeyCollector {
    pub keys: Vec<String>,
    pub fn_names: Vec<String>,
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
