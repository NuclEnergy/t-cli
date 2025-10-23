use cli::Cli;
use error::Error;

mod cli;
mod collect;
mod config;
mod error;
mod generate;
mod utils;

pub const CONFIG_TEMPLATE_TS: &str = include_str!("../template/t.config.ts");

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli: Cli = clap::Parser::parse();

    match cli.command {
        cli::Commands::Init { output, force } => {
            generate::init_config::generate_config_file(&output, force).await?;
            println!("Config file generated successfully");
        }
        cli::Commands::Collect { config, verbose } => {
            let config = config::load_config_from_file(&config).await?;
            collect::run_collect(config, verbose).await?;
            println!("Collected successfully");
        }
        cli::Commands::Generate { config, verbose } => {
            let config = config::load_config_from_file(&config).await?;
            generate::tgen::run_tgen(config, verbose).await?;
            println!("Generated successfully");
        }
        cli::Commands::Clean { config, verbose } => {
            let config = config::load_config_from_file(&config).await?;
            collect::run_clean(config, verbose).await?;
            println!("Cleaned successfully");
        }
        cli::Commands::Cg { config, verbose } => {
            let config = config::load_config_from_file(&config).await?;
            collect::run_collect(config.clone(), verbose).await?;
            generate::tgen::run_tgen(config, verbose).await?;
            println!("Collected and generated successfully");
        }
        cli::Commands::Gc { config, verbose } => {
            let config = config::load_config_from_file(&config).await?;
            collect::run_collect(config.clone(), verbose).await?;
            generate::tgen::run_tgen(config.clone(), verbose).await?;
            collect::run_clean(config, verbose).await?;
            println!("Collected, generated and cleaned successfully");
        }
    }

    Ok(())
}
