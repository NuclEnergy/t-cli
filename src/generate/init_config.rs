use std::{fs::File, io::Write, path::Path};

use tokio::fs::remove_file;

use crate::CONFIG_TEMPLATE_TS;

pub async fn generate_config_file(path: &str, force: bool) -> std::io::Result<()> {
    let path_obj = Path::new(path);
    if path_obj.exists() {
        match force {
            true => {
                remove_file(path_obj).await?;
            }
            false => {
                println!("⚠️ Config file {path} already exists, use --force to overwrite");
                return Ok(());
            }
        }
    }

    let mut file = File::create(path)?;
    file.write_all(CONFIG_TEMPLATE_TS.as_bytes())?;

    println!("✅ Config file created at {path}");
    Ok(())
}
