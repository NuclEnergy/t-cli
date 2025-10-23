use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use ignore::{WalkBuilder, overrides::OverrideBuilder};
use path_clean::PathClean;

use crate::error::Error;

pub fn resolve_workspaces(
    root: impl AsRef<Path>,
    includes: &[String],
    excludes: &[String],
) -> Result<Vec<PathBuf>, Error> {
    let root = root.as_ref();
    let mut workspaces = HashSet::new();

    for include in includes {
        let include_cleaned = include.trim_start_matches(['/', '.', '\\']);
        let include_path = root.join(include_cleaned).clean();

        if !include_path.exists() {
            continue;
        }

        let mut builder = WalkBuilder::new(&include_path);
        builder.hidden(false);
        builder.git_ignore(false);
        builder.git_exclude(false);
        builder.parents(false);
        builder.follow_links(false);

        // only exclude in current include
        let mut override_builder = OverrideBuilder::new(&include_path);
        for pattern in excludes {
            override_builder.add(&format!("!{pattern}"))?;
        }
        let overrides = override_builder.build()?;
        builder.overrides(overrides);

        for result in builder.build() {
            let dent = result?;
            let path = dent.path();
            if dent.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                workspaces.insert(path.to_path_buf());
            }
        }
    }

    let mut ws = workspaces.into_iter().collect::<Vec<_>>();
    ws.sort_by(|a, b| {
        a.to_string_lossy()
            .to_lowercase()
            .cmp(&b.to_string_lossy().to_lowercase())
    });
    Ok(ws)
}
