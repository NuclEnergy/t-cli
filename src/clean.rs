use std::{collections::{HashMap, HashSet}, path::PathBuf};

use indexmap::IndexMap;
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::Program;
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::VisitWith;
use tokio::fs::{create_dir_all, read_to_string, write};
use walkdir::WalkDir;

use crate::{
    collect::FnKeyCollector,
    config::Config,
    error::Error,
    utils::{is_target_file::is_target_file, resolve::resolve_workspaces},
};

/// Clean unused translation keys:
/// 1. Scan source code, collect all used keys in order of workspace a–z (case-insensitive), file a–z (case-insensitive), and source code in file;
/// 2. Traverse each target language's output JSON, delete keys not in "used set";
/// 3. Preserve original order (filter on old file order), fill None values for default language with key itself.
pub async fn run_clean(config: Config, verbose: bool) -> Result<(), Error> {
    let cm: Lrc<SourceMap> = Default::default();

    // 1) Collect all used keys per output directory (workspace + target.output)
    let mut used: HashMap<PathBuf, HashSet<String>> = HashMap::new();

    for target in &config.targets {
        let workspaces = resolve_workspaces(".", &target.includes, &target.excludes)?;
        for workspace in workspaces {
            if verbose {
                println!("Scanning workspace for used keys: {}", workspace.display());
            }

            let output_dir = workspace.join(&target.output);

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

                if !visitor.keys.is_empty() {
                    let set = used.entry(output_dir.clone()).or_insert_with(HashSet::new);
                    for k in visitor.keys {
                        set.insert(k);
                    }
                }
            }
        }
    }

    if verbose {
        let total: usize = used.values().map(|s| s.len()).sum();
        println!("Total used keys (all workspaces): {}", total);
    }

    // 2) Traverse each target language's output JSON files, delete unused keys
    let default_lang = config.languages.name.clone();
    let all_langs = config.languages.collect_languages();

    for target in &config.targets {
        for workspace in resolve_workspaces(".", &target.includes, &target.excludes)? {
            let output_dir = workspace.join(&target.output);
            let empty = HashSet::new();
            let used_set = used.get(&output_dir).unwrap_or(&empty);
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
                // Filter on old order, only keep keys in current workspace's used set
                old_map.retain(|k, _| used_set.contains(k));
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
