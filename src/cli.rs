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

    #[command(visible_alias = "c")]
    Collect {
        #[arg(short, long, default_value = "t.config.ts")]
        config: String,
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    #[command(visible_alias = "g")]
    Generate {
        #[arg(short, long, default_value = "t.config.ts")]
        config: String,
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    Clean {
        #[arg(short, long, default_value = "t.config.ts")]
        config: String,
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    /// Collect + Generate (equivalent to: t-cli collect && t-cli generate)
    Cg {
        #[arg(short, long, default_value = "t.config.ts")]
        config: String,
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    /// Collect + Generate + Clean (equivalent to: t-cli collect && t-cli generate && t-cli clean)
    Gc {
        #[arg(short, long, default_value = "t.config.ts")]
        config: String,
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },
}
