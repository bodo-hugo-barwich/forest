// Copyright 2019-2023 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

pub mod archive_cmd;
pub mod benchmark_cmd;
pub mod db_cmd;
pub mod fetch_params_cmd;
pub mod snapshot_cmd;

use crate::cli_shared::cli::HELP_MESSAGE;
use crate::cli_shared::cli::*;
use crate::networks::{ChainConfig, NetworkChain};
use crate::utils::version::FOREST_VERSION_STRING;
use crate::utils::{io::read_file_to_string, io::read_toml};
use clap::Parser;
use std::sync::Arc;

/// Command-line options for the `forest-tool` binary
#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"), author = env!("CARGO_PKG_AUTHORS"), version = FOREST_VERSION_STRING.as_str(), about = env!("CARGO_PKG_DESCRIPTION"))]
#[command(help_template(HELP_MESSAGE))]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Subcommand,
}

/// forest-tool sub-commands
#[derive(clap::Subcommand)]
pub enum Subcommand {
    /// Benchmark various Forest subsystems
    #[command(subcommand)]
    Benchmark(benchmark_cmd::BenchmarkCommands),

    /// Manage snapshots
    #[command(subcommand)]
    Snapshot(snapshot_cmd::SnapshotCommands),

    /// Download parameters for generating and verifying proofs for given size
    #[command(name = "fetch-params")]
    Fetch(fetch_params_cmd::FetchCommands),

    /// Manage archives
    #[command(subcommand)]
    Archive(archive_cmd::ArchiveCommands),

    /// Database management
    #[command(subcommand)]
    DB(db_cmd::DBCommands),
}

fn read_config(config: &Option<String>, chain: &Option<NetworkChain>) -> anyhow::Result<Config> {
    let path = find_config_path(config);
    let mut cfg: Config = match &path {
        Some(path) => {
            // Read from config file
            let toml = read_file_to_string(path.to_path_buf())?;
            // Parse and return the configuration file
            read_toml(&toml)?
        }
        None => Config::default(),
    };

    // Override config with chain if some
    match chain {
        Some(NetworkChain::Mainnet) => cfg.chain = Arc::new(ChainConfig::mainnet()),
        Some(NetworkChain::Calibnet) => cfg.chain = Arc::new(ChainConfig::calibnet()),
        Some(NetworkChain::Devnet(_)) => cfg.chain = Arc::new(ChainConfig::devnet()),
        None => (),
    }

    Ok(cfg)
}