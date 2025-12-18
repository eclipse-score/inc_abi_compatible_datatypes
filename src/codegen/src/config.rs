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

#[cfg(feature = "json5_config")]
pub mod file;

use std::{fmt, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    /// Whether to format the generated code.
    pub format: bool,

    /// The target language.
    pub target: Target,

    /// The type definitions file.
    pub source: PathBuf,

    /// The output path for the generated code.
    pub output: Option<PathBuf>,

    /// Options for the Rust target.
    pub rust: RustOptions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Rust,
    Cpp,
}

impl Target {
    pub fn extension(&self) -> &str {
        match self {
            Target::Rust => "rs",
            Target::Cpp => "hpp",
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Target::Rust => "Rust",
            Target::Cpp => "C++",
        };
        fmt::Display::fmt(s, f)
    }
}

#[derive(Clone, Debug)]
pub struct RustOptions {
    pub derive_reloc: bool,
    pub derive_iceoryx2_traits: bool,
}

impl RustOptions {
    pub fn iceoryx2() -> Self {
        Self {
            derive_reloc: true,
            derive_iceoryx2_traits: true,
        }
    }

    pub fn lola() -> Self {
        Self {
            derive_reloc: true,
            derive_iceoryx2_traits: false,
        }
    }
}

impl Default for RustOptions {
    fn default() -> Self {
        Self {
            derive_reloc: true,
            derive_iceoryx2_traits: true,
        }
    }
}
