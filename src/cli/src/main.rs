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

mod config;

use self::config::Command;
use abi_types_codegen as codegen;
use anyhow::{Result, bail};
use log::error;
use std::{process::ExitCode, thread::sleep, time::Duration};

fn main() -> ExitCode {
    if let Err(error) = main_() {
        error!("{}", format_error(&error));
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn main_() -> Result<()> {
    let config = config::load();

    match config.command {
        Command::Build(build_config) => {
            let config = build_config.into();
            codegen::build(&config)?;
        },
        Command::Check { ref source } => {
            codegen::check(source)?;
        },
        Command::Watch(build_config) => {
            let config = build_config.into();
            watch(&config)?;
        },
    }
    Ok(())
}

fn watch(config: &codegen::config::Config) -> Result<()> {
    use notify::{RecursiveMode, Watcher};
    let (events_tx, events_rx) = crossbeam_channel::bounded(16);
    let mut watcher =
        notify::recommended_watcher(move |event: std::result::Result<notify::Event, _>| {
            if let Ok(event) = event {
                if !event.kind.is_access() {
                    events_tx.send(()).unwrap();
                }
            }
        })?;

    if let Err(error) = watcher.watch(&config.source, RecursiveMode::NonRecursive) {
        if matches!(error.kind, notify::ErrorKind::PathNotFound) {
            bail!("Couldn't find source file '{}'", config.source.display());
        } else {
            return Err(anyhow::Error::from(error).context(format!(
                "Failed to watch source file '{}'",
                config.source.display(),
            )));
        }
    }
    codegen::build(config)?;
    while events_rx.recv().is_ok() {
        sleep(Duration::from_millis(1));
        while events_rx.try_recv().is_ok() {
            // Debounce events
        }
        codegen::build(config)?;
    }

    Ok(())
}

fn format_error(error: &anyhow::Error) -> String {
    use std::fmt::Write;
    let mut msg = format!("{error}");
    for cause in error.chain().skip(1) {
        write!(&mut msg, "\nCaused by: {cause}").ok();
    }
    msg
}
