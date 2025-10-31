// *******************************************************************************
// Copyright (c) 2025 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
// *******************************************************************************

use abi_types_codegen as codegen;
use std::path::PathBuf;

pub fn load() -> Config {
    let config = <Config as clap::Parser>::parse();
    init_logging(config.verbosity);
    config
}

#[derive(Clone, Debug, clap::Parser)]
#[command(version, about)]
pub struct Config {
    #[arg(global = true, short, long = "verbose", action = clap::ArgAction::Count)]
    pub verbosity: u8,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Debug, clap::Subcommand)]
pub enum Command {
    Build(BuildConfig),
    Check { source: PathBuf },
    Watch(BuildConfig),
}

#[derive(Clone, Debug, clap::Args)]
pub struct BuildConfig {
    /// Whether to format the generated code.
    #[arg(short, long, default_value = "true")]
    pub format: bool,

    /// The target language.
    #[arg(short, long)]
    pub target: Target,

    /// The output path for the generated code.
    ///
    /// By default, the generated code will be written to the same directory as the source file.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// The type definitions file.
    pub source: PathBuf,
}

impl From<BuildConfig> for codegen::config::Config {
    fn from(config: BuildConfig) -> Self {
        Self {
            format: config.format,
            target: config.target.into(),
            source: config.source,
            output: config.output,
            rust: codegen::config::RustOptions::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Target {
    Rust,
    Cpp,
}

impl From<Target> for codegen::config::Target {
    fn from(target: Target) -> Self {
        match target {
            Target::Rust => codegen::config::Target::Rust,
            Target::Cpp => codegen::config::Target::Cpp,
        }
    }
}

fn init_logging(verbosity: u8) {
    use env_logger::{Builder, Env};
    use log::LevelFilter;

    let mut builder = Builder::new();
    builder.filter_level(LevelFilter::Warn);
    builder.format_timestamp(None);
    builder.format_target(false);
    builder.parse_env(Env::new().filter("LOG_LEVEL").write_style("LOG_STYLE"));
    match verbosity {
        0 => {},
        1 => {
            builder.filter_level(log::LevelFilter::Info);
        },
        2 => {
            builder.filter_level(log::LevelFilter::Debug);
        },
        _ => {
            builder.filter_level(log::LevelFilter::Trace);
        },
    }
    builder.init();
}
