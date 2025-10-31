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

use anyhow::{Context as _, Result};
use serde::Deserialize;
use std::path::Path;

use super::{Config, RustOptions};

#[derive(Default, Deserialize)]
pub struct ConfigFile {
    format: Option<bool>,

    rust: Option<ConfigFileRustOptions>,
}

impl ConfigFile {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Couldn't read config file '{}'", path.display()))?;
        let config = serde_json5::from_str(&contents)
            .with_context(|| format!("Couldn't parse config file '{}' as JSON5", path.display()))?;
        Ok(config)
    }

    pub fn apply(&self, config: &mut Config) {
        if let Some(format) = self.format {
            config.format = format;
        }

        if let Some(rust) = &self.rust {
            rust.apply(&mut config.rust);
        }
    }
}

#[derive(Default, Deserialize)]
struct ConfigFileRustOptions {
    derive_reloc: Option<bool>,
}

impl ConfigFileRustOptions {
    fn apply(&self, options: &mut RustOptions) {
        if let Some(derive_reloc) = self.derive_reloc {
            options.derive_reloc = derive_reloc;
        }
    }
}
