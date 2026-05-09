mod cli;
mod host;
mod ios;

use anyhow::Result;
use clap::Parser;
use cli::{BuildCommand, Cli, Command, DevicesCommand, HostSubcommand, Platform, RunCommand};
use host::{BuildOptions, LoadedHost};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Run(command) => run(command),
        Command::Build(command) => build(command),
        Command::Devices(command) => devices(command),
        Command::Host(command) => match command.command {
            HostSubcommand::Sync { platform } => host_sync(platform),
            HostSubcommand::BuildRust { platform } => host_build_rust(platform),
        },
    }
}

fn run(command: RunCommand) -> Result<()> {
    let host = LoadedHost::load(command.platform)?;
    let options = BuildOptions {
        sim: command.sim,
        release: command.release,
        device: command.device,
    };
    host.run(command.demo.as_deref(), &options)
}

fn build(command: BuildCommand) -> Result<()> {
    let host = LoadedHost::load(command.platform)?;
    let options = BuildOptions {
        sim: command.sim,
        release: command.release,
        device: command.device,
    };
    host.build(&options)
}

fn devices(command: DevicesCommand) -> Result<()> {
    let host = LoadedHost::load(command.platform)?;
    host.list_devices()
}

fn host_sync(platform: Platform) -> Result<()> {
    let host = LoadedHost::load(platform)?;
    host.sync()
}

fn host_build_rust(platform: Platform) -> Result<()> {
    let host = LoadedHost::load(platform)?;
    host.build_rust()
}
