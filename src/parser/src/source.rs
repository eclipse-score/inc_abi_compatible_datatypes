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
use ariadne::Source;
use rustc_hash::FxHashMap;
use std::{fmt, fs, ops, path::Path, sync::Arc};

/// A span within a single source file.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct LocalSpan {
    /// Start position (inclusive) in bytes; must be `<= end`.
    pub start: usize,
    /// End position (exclusive) in bytes; must be `>= start`.
    pub end: usize,
}

impl LocalSpan {
    #[must_use]
    pub fn pos_zero() -> Self {
        Self { start: 0, end: 0 }
    }

    #[must_use]
    pub fn after(&self) -> Self {
        Self {
            start: self.end,
            end: self.end,
        }
    }

    #[must_use]
    pub fn before(&self) -> Self {
        Self {
            start: self.start,
            end: self.start,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl ops::Index<LocalSpan> for str {
    type Output = str;

    fn index(&self, index: LocalSpan) -> &Self::Output {
        &self[index.start..index.end]
    }
}

impl ops::Add<Self> for LocalSpan {
    type Output = LocalSpan;

    fn add(self, rhs: Self) -> Self {
        debug_assert!(self.start <= rhs.start);
        debug_assert!(self.end <= rhs.end);
        Self {
            start: self.start,
            end: rhs.end,
        }
    }
}

impl ops::Add<&Self> for LocalSpan {
    type Output = LocalSpan;

    fn add(self, rhs: &Self) -> Self {
        debug_assert!(self.start <= rhs.start);
        debug_assert!(self.end <= rhs.end);
        Self {
            start: self.start,
            end: rhs.end,
        }
    }
}

impl ops::AddAssign<Self> for LocalSpan {
    fn add_assign(&mut self, rhs: Self) {
        debug_assert!(self.start <= rhs.start);
        debug_assert!(self.end <= rhs.end);
        self.end = rhs.end;
    }
}

impl ops::AddAssign<&Self> for LocalSpan {
    fn add_assign(&mut self, rhs: &Self) {
        debug_assert!(self.start <= rhs.start);
        debug_assert!(self.end <= rhs.end);
        self.end = rhs.end;
    }
}

impl fmt::Debug for LocalSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{start}:{end}", start = self.start, end = self.end)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId(u64);

impl SourceId {
    pub fn with_span(&self, span: LocalSpan) -> GlobalSpan {
        GlobalSpan {
            source_id: *self,
            span,
        }
    }
}

pub struct SourceFile {
    id: SourceId,
    /// A name for the source, typically its path.
    name: String,
    content: Source<String>,
}

impl SourceFile {
    pub fn load(path: &Path, source_store: &mut SourceStore) -> Result<Arc<Self>> {
        let name = path.display().to_string();
        let content = Source::from(fs::read_to_string(path).context("Couldn't read file")?);
        let instance = Self {
            id: SourceId::default(),
            name,
            content,
        };
        Ok(source_store.insert(instance))
    }

    pub fn content(&self) -> &str {
        self.content.text()
    }

    pub fn id(&self) -> SourceId {
        self.id
    }
}

impl fmt::Display for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{name}", name = self.name)
    }
}

#[derive(Clone, Copy)]
pub struct GlobalSpan {
    source_id: SourceId,
    span: LocalSpan,
}

impl ariadne::Span for GlobalSpan {
    type SourceId = SourceId;

    fn source(&self) -> &Self::SourceId {
        &self.source_id
    }

    fn start(&self) -> usize {
        self.span.start
    }

    fn end(&self) -> usize {
        self.span.end
    }

    fn len(&self) -> usize {
        self.span.len()
    }
}

#[derive(Default)]
pub struct SourceStore {
    sources: FxHashMap<SourceId, Arc<SourceFile>>,
    next_id: SourceId,
}

impl SourceStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn insert(&mut self, mut source: SourceFile) -> Arc<SourceFile> {
        source.id = self.next_id;
        let source = Arc::new(source);
        self.sources.insert(source.id, Arc::clone(&source));
        self.next_id.0 += 1;
        source
    }
}

impl ops::Index<SourceId> for SourceStore {
    type Output = SourceFile;

    fn index(&self, index: SourceId) -> &Self::Output {
        &self.sources[&index]
    }
}

impl ariadne::Cache<SourceId> for &SourceStore {
    type Storage = String;

    fn fetch(&mut self, id: &SourceId) -> Result<&Source<Self::Storage>, impl fmt::Debug> {
        match self.sources.get(id) {
            Some(entry) => Ok(&entry.content),
            None => Err("invalid source ID"),
        }
    }

    fn display<'a>(&self, id: &'a SourceId) -> Option<impl fmt::Display + 'a> {
        self.sources
            .get(id)
            .map(move |entry| SourceNameDisplay(Arc::clone(entry)))
    }
}

struct SourceNameDisplay(Arc<SourceFile>);

impl fmt::Display for SourceNameDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0.name, f)
    }
}
