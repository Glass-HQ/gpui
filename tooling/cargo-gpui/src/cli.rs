use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "cargo-gpui", bin_name = "cargo gpui")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Run(RunCommand),
    Build(BuildCommand),
    Devices(DevicesCommand),
    Host(HostCommand),
}

#[derive(Debug, Args)]
pub struct RunCommand {
    pub platform: Platform,
    pub demo: Option<String>,
    #[arg(long)]
    pub sim: bool,
    #[arg(long)]
    pub release: bool,
    #[arg(long)]
    pub device: Option<String>,
}

#[derive(Debug, Args)]
pub struct BuildCommand {
    pub platform: Platform,
    #[arg(long)]
    pub sim: bool,
    #[arg(long)]
    pub release: bool,
    #[arg(long)]
    pub device: Option<String>,
}

#[derive(Debug, Args)]
pub struct DevicesCommand {
    pub platform: Platform,
}

#[derive(Debug, Args)]
pub struct HostCommand {
    #[command(subcommand)]
    pub command: HostSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum HostSubcommand {
    Sync { platform: Platform },
    BuildRust { platform: Platform },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Platform {
    Ios,
}
