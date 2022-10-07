use clap::Parser;
use once_cell::sync::Lazy;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Loads the given `.blj` file
    pub load: Option<String>,

    /// Export mock Lua code annotated with ldoc comments for the blackjack API
    /// at the given folder.
    #[arg(long)]
    pub generate_ldoc: Option<String>,
}

/// CLI args are stored in a lazy static variable so they're accessible from
/// everywhere. Arguments are parsed on first access.
pub static CLI_ARGS: Lazy<Args> = Lazy::new(Args::parse);
