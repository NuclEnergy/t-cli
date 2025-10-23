use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use indexmap::IndexMap;
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::{Callee, Program};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};
use tokio::fs::{create_dir_all, read_to_string, write};
use walkdir::WalkDir;

use crate::{config::Config, error::Error, utils::resolve::resolve_workspaces};

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

///
/// Clean unused translation keys:
/// 1. Scan source code, collect all used keys in order of workspace a–z (case-insensitive), file a–z (case-insensitive), and source code in file;
/// 2. Traverse each target language's output JSON, delete keys not in "used set";
/// 3. Preserve original order (filter on old file order), fill None values for default language with key itself.
pub async fn run_clean(config: Config, verbose: bool) -> Result<(), Error> {
    let cm: Lrc<SourceMap> = Default::default();

    // 1) Collect all used keys (deduplicated)
    let mut used: HashSet<String> = HashSet::new();

    for target in &config.targets {
        let workspaces = resolve_workspaces(".", &target.includes, &target.excludes)?;
        for workspace in workspaces {
            if verbose {
                println!("Scanning workspace for used keys: {}", workspace.display());
            }

            // Collect one level of files and sort by file name a–z (case-insensitive)
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

                for k in visitor.keys {
                    used.insert(k);
                }
            }
        }
    }

    if verbose {
        println!("Total used keys: {}", used.len());
    }

    // 2) Traverse each target language's output JSON files, delete unused keys
    let default_lang = config.languages.name.clone();
    let all_langs = config.languages.collect_languages();

    for target in &config.targets {
        for workspace in resolve_workspaces(".", &target.includes, &target.excludes)? {
            let output_dir = workspace.join(&target.output);
            for lang in &all_langs {
                let file_path = output_dir.join(format!("{lang}.json"));
                if !file_path.exists() {
                    continue;
                }

                let old_content = read_to_string(&file_path).await?;
                let mut old_map: IndexMap<String, Option<String>> =
                    match serde_json::from_str(&old_content) {
                        Ok(m) => m,
                        Err(_) => {
                            if verbose {
                                println!("Skip invalid JSON: {}", file_path.display());
                            }
                            continue;
                        }
                    };

                let before = old_map.len();
                // Filter on old order, only keep keys in used
                old_map.retain(|k, _| used.contains(k));
                let after = old_map.len();

                // For default language, fill None with key itself (no change in order)
                let mut filled = 0usize;
                if &default_lang == lang {
                    for (k, v) in old_map.iter_mut() {
                        if v.is_none() {
                            *v = Some(k.clone());
                            filled += 1;
                        }
                    }
                }

                if before != after || filled > 0 {
                    if let Some(parent) = file_path.parent() {
                        create_dir_all(parent).await?;
                    }
                    let json = serde_json::to_string_pretty(&old_map)?;
                    write(&file_path, json).await?;
                    if verbose {
                        println!(
                            "Cleaned {}: removed {} unused keys, filled {} ({} → {})",
                            file_path.display(),
                            before - after,
                            filled,
                            before,
                            after
                        );
                    }
                } else if verbose {
                    println!("No unused keys in {}", file_path.display());
                }
            }
        }
    }

    Ok(())
}
