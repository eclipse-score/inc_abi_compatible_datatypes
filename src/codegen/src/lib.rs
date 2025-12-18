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

pub mod config;
mod cpp;
mod iter_utils;
mod rust;

use abi_types_parser::{
    ast,
    source::{SourceFile, SourceStore},
};
use anyhow::{Context as _, Result};
use log::info;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use self::config::{Config, Target};

pub fn check(source_path: &Path) -> Result<Option<ast::Module>> {
    let mut source_store = SourceStore::new();
    let source = SourceFile::load(source_path, &mut source_store)
        .with_context(|| format!("Couldn't parse input '{}'", source_path.display()))?;

    let (top_module, messages) = ast::Module::parse_file(&source);
    for message in &messages.reports() {
        message.eprint(&source_store).unwrap();
    }
    if messages.has_errors() {
        Ok(None)
    } else {
        Ok(Some(top_module))
    }
}

pub fn build(config: &Config) -> Result<()> {
    if let Some(top_module) = check(&config.source)? {
        generate(&top_module, config)?;
    }
    Ok(())
}

pub fn generate(module: &ast::Module, config: &Config) -> Result<()> {
    let paths = if let Some(output) = &config.output {
        TargetPaths::from_output(output)
    } else {
        TargetPaths::from_source(&config.source, config.target.extension())
    };
    info!(
        "Generating {target} code in '{path}'",
        target = config.target,
        path = paths.target.display(),
    );
    let code = match config.target {
        Target::Rust => rust::generate(module, config),
        Target::Cpp => cpp::generate(module, config),
    };
    fs::write(&paths.tmp, code).with_context(|| {
        format!(
            "Couldn't write generated code to '{tmp}'",
            tmp = paths.tmp.display(),
        )
    })?;

    if config.format {
        match config.target {
            Target::Rust => rust::format(&paths.tmp),
            Target::Cpp => cpp::format(&paths.tmp),
        }
    }

    paths.replace_target().with_context(|| {
        format!(
            "Couldn't move generated code to '{target}'",
            target = paths.target.display(),
        )
    })?;

    Ok(())
}

struct TargetPaths {
    tmp: PathBuf,
    target: PathBuf,
}

impl TargetPaths {
    fn from_source(source: &Path, extension: &str) -> Self {
        let target = source.with_extension(extension);
        let file_stem = source.file_stem().unwrap_or("generated".as_ref());
        let mut tmp_file_name = OsString::with_capacity(5 + file_stem.len() + 1 + extension.len());
        tmp_file_name.push(".tmp.");
        tmp_file_name.push(file_stem);
        tmp_file_name.push(".");
        tmp_file_name.push(extension);
        let tmp = source.with_file_name(tmp_file_name);
        Self { tmp, target }
    }

    fn from_output(output: &Path) -> Self {
        let file_stem = output.file_stem().unwrap_or("generated".as_ref());
        let mut tmp_file_name = OsString::with_capacity(5 + file_stem.len() + 1 + 3);
        tmp_file_name.push(".tmp.");
        tmp_file_name.push(file_stem);
        tmp_file_name.push(".");
        if let Some(extension) = output.extension() {
            tmp_file_name.push(extension);
        }
        let tmp = output.with_file_name(tmp_file_name);
        Self {
            tmp,
            target: output.to_path_buf(),
        }
    }

    fn replace_target(&self) -> Result<()> {
        fs::rename(&self.tmp, &self.target).with_context(|| {
            format!(
                "Couldn't replace target '{target}' with intermediate file",
                target = self.target.display(),
            )
        })
    }
}

fn format_error(error: &anyhow::Error) -> String {
    use std::fmt::Write;
    let mut msg = format!("{error}");
    for cause in error.chain().skip(1) {
        write!(&mut msg, "\nCaused by: {cause}").ok();
    }
    msg
}
