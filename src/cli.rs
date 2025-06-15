use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "t-cli")]
#[command(version = "0.1.0")]
#[command(about = "Translation key collector and generator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init {
        #[arg(short, long, default_value = "t.config.ts")]
        output: String,
        #[arg(short, long, default_value = "false")]
        force: bool,
    },

    Collect {
        #[arg(short, long, default_value = "t.config.ts")]
        config: String,
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    Generate {
        #[arg(short, long, default_value = "t.config.ts")]
        config: String,
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },
}
