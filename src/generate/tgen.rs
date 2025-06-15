use std::collections::BTreeMap;

use tokio::fs::{create_dir_all, read_to_string, try_exists, write};

use crate::{
    config::{Config, LanguageNode},
    error::Error,
    utils::resolve::resolve_workspaces,
};

pub async fn run_tgen(config: Config, verbose: bool) -> Result<(), Error> {
    for target in config.targets {
        let workspaces = resolve_workspaces(".", &target.includes, &target.excludes)?;
        for workspace in workspaces {
            let output_dir = workspace.join(&target.output);
            if !try_exists(&output_dir).await? {
                continue;
            }

            let mut all_translations: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

            let mut lang_order = Vec::new();
            walk_language_tree(&config.languages, None, &mut lang_order);

            for (lang, parent_lang) in lang_order {
                let mut lang_data = match parent_lang {
                    Some(parent) => all_translations.get(&parent).cloned().unwrap_or_default(),
                    None => BTreeMap::new(),
                };

                let file_path = output_dir.join(format!("{lang}.json"));
                if try_exists(&file_path).await? {
                    let content = read_to_string(&file_path).await?;
                    let file_map: BTreeMap<String, Option<String>> =
                        serde_json::from_str(&content)?;

                    for (k, v) in file_map {
                        if let Some(real_value) = v {
                            lang_data.insert(k, real_value);
                        }
                    }
                }
                all_translations.insert(lang.clone(), lang_data);
            }

            let output_path = output_dir.join("index.ts");
            if let Some(parent) = output_path.parent() {
                create_dir_all(parent).await?;
            }
            let ts_output = format!(
                "export const {output} = {dictionaries} as const;\n\nexport type Dict = typeof {output}[keyof typeof {output}];\n",
                output = target.output,
                dictionaries = serde_json::to_string_pretty(&all_translations)?
            );
            write(&output_path, ts_output).await?;
            if verbose {
                println!("Generated: {}", output_path.display());
            }
        }
    }
    Ok(())
}

fn walk_language_tree(
    node: &LanguageNode,
    parent: Option<String>,
    list: &mut Vec<(String, Option<String>)>,
) {
    list.push((node.name.clone(), parent.clone()));
    for child in &node.children {
        walk_language_tree(child, Some(node.name.clone()), list);
    }
}
