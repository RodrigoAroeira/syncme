#![expect(dead_code, unused_variables)]

use clap::Parser;

mod cli;
mod config;

fn main() {
    use cli::Command as Cmd;
    let args = cli::Args::parse();
    match args.cmd {
        Cmd::Push => todo!(),
        Cmd::Pull { machine } => todo!(),
    }
}
