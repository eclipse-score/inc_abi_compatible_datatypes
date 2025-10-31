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

#![expect(unused, reason = "TODO remove before release")]

mod lexer;
mod parser;
mod tokenizer;

use anyhow::{Context as _, Result};
use ariadne::Span as _;
use compact_str::CompactString;
use std::{cell::RefCell, cmp, fmt, iter::FusedIterator, rc::Rc, sync::Arc};

use crate::source::{GlobalSpan, LocalSpan, SourceFile, SourceId, SourceStore};

/// The *abstract syntax tree* (AST) parsed from a module.
#[derive(Debug)]
pub struct Module {
    pub doc: Option<DocComment>,
    pub items: Vec<Item>,
}

impl Module {
    pub fn parse_file(source: &SourceFile) -> (Self, Rc<MessageTracker>) {
        let messages = Rc::new(MessageTracker::new(source.id()));
        let lexemes = lexer::lex(source.content());
        let token_stream =
            tokenizer::TokenStream::tokenize(source.content(), Rc::clone(&messages), lexemes);
        let module = parser::parse(&token_stream, Rc::clone(&messages));
        messages.sort();
        (module, messages)
    }
}

pub enum Item {
    Import(Import),
    ModuleDecl(ModuleDecl),
    TypeDecl(TypeDecl),
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Import(import) => fmt::Debug::fmt(import, f),
            Self::ModuleDecl(module_decl) => fmt::Debug::fmt(module_decl, f),
            Self::TypeDecl(type_decl) => fmt::Debug::fmt(type_decl, f),
        }
    }
}

#[derive(Debug)]
pub struct Import {
    pub doc: Option<DocComment>,
    pub attributes: Vec<Attribute>,
    pub path: Path,
}

#[derive(Debug)]
pub struct ModuleDecl {
    pub doc: Option<DocComment>,
    pub attributes: Vec<Attribute>,
    pub name: Identifier,
    pub body: Option<Module>,
}

#[derive(Debug)]
pub struct TypeDecl {
    pub doc: Option<DocComment>,
    pub attributes: Vec<Attribute>,
    pub name: Identifier,
    pub generics: GenericParams,
    pub kind: TypeDeclKind,
}

#[derive(Debug)]
pub enum TypeDeclKind {
    /// A simple type declaration of the form `type NAME = … ;`.
    Simple(SimpleTypeDecl),
    NewType(NewTypeDecl),
    Struct(StructTypeDecl),
    Enum(EnumTypeDecl),
    Error,
}

#[derive(Debug)]
pub struct SimpleTypeDecl {
    pub type_ref: TypeRef,
}

#[derive(Debug)]
pub struct NewTypeDecl {
    pub type_ref: TypeRef,
}

#[derive(Debug)]
pub struct StructTypeDecl {
    pub fields: Vec<StructField>,
}

#[derive(Debug)]
pub struct EnumTypeDecl {
    pub variants: Vec<EnumVariant>,
}

impl EnumTypeDecl {
    pub fn is_tag_only(&self) -> bool {
        self.variants.iter().all(EnumVariant::is_unit)
    }
}

#[derive(Debug)]
pub struct StructField {
    pub doc: Option<DocComment>,
    pub name: Identifier,
    pub type_ref: TypeRef,
}

#[derive(Debug)]
pub struct EnumVariant {
    pub doc: Option<DocComment>,
    pub name: Identifier,
    pub kind: EnumVariantKind,
}

impl EnumVariant {
    pub fn is_unit(&self) -> bool {
        matches!(self.kind, EnumVariantKind::Unit)
    }
}

#[derive(Debug)]
pub enum EnumVariantKind {
    Unit,
    NewType { type_ref: TypeRef },
    Struct { fields: Vec<StructField> },
}

#[derive(Debug)]
pub struct GenericParams {
    pub params: Vec<GenericParam>,
}

#[derive(Debug)]
pub enum GenericParam {
    Type(Identifier),
    Value(Identifier),
}

#[derive(Debug)]
pub struct TypeRef {
    pub path: Path,
    pub generics: GenericArgs,
}

#[derive(Debug)]
pub struct GenericArgs {
    pub args: Vec<GenericArg>,
}

#[derive(Debug)]
pub enum GenericArg {
    Type(TypeRef),
    Value(u64),
}

#[derive(Debug)]
pub struct Attribute {
    pub name: Identifier,
    pub arg: Option<Path>,
}

pub struct Path {
    /// Whether the path starts with `::`.
    pub is_global: bool,

    /// The identifiers in the path.
    ///
    /// In a valid path, this list is never empty.
    pub elements: Vec<Identifier>,
}

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_global {
            f.write_str("::")?;
        }
        let mut elements = self.elements.iter();
        if let Some(first) = elements.next() {
            write!(f, "{first:?}")?;
        }
        for element in elements {
            write!(f, "::{element:?}")?;
        }
        Ok(())
    }
}

pub struct Identifier {
    pub name: CompactString,
    pub span: LocalSpan,
}

impl fmt::Debug for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.name, f)
    }
}

impl cmp::PartialEq<&str> for Identifier {
    fn eq(&self, other: &&str) -> bool {
        self.name == other
    }
}

#[derive(Debug)]
pub struct DocComment {
    pub lines: Vec<String>,
    pub span: LocalSpan,
}

type Report = ariadne::Report<'static, GlobalSpan>;

pub struct MessageTracker {
    source_id: SourceId,
    reports: RefCell<Vec<(usize, Report)>>,
}

impl MessageTracker {
    fn new(source_id: SourceId) -> Self {
        Self {
            reports: RefCell::new(vec![]),
            source_id,
        }
    }

    fn push(&self, span: LocalSpan, msg: String) {
        use ariadne::ReportKind;

        let span = self.source_id.with_span(span);
        let report = Report::build(ReportKind::Error, span)
            .with_message(msg)
            .with_label(ariadne::Label::new(span).with_message("here"))
            .finish();
        self.reports.borrow_mut().push((span.start(), report));
    }

    fn sort(&self) {
        self.reports.borrow_mut().sort_by_key(|&(start, _)| start);
    }

    pub fn reports(&self) -> Reports {
        Reports {
            reports: self.reports.borrow(),
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.reports.borrow().is_empty()
    }
}

pub struct Reports<'owner> {
    reports: std::cell::Ref<'owner, Vec<(usize, Report)>>,
}

impl<'owner, 'report> IntoIterator for &'report Reports<'owner> {
    type Item = &'report Report;
    type IntoIter = ReportIter<'report>;

    fn into_iter(self) -> Self::IntoIter {
        ReportIter {
            iter: self.reports.iter(),
        }
    }
}

pub struct ReportIter<'report> {
    iter: std::slice::Iter<'report, (usize, Report)>,
}

impl<'report> Iterator for ReportIter<'report> {
    type Item = &'report Report;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, report)| report)
    }
}

impl ExactSizeIterator for ReportIter<'_> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

// std::slice::Iter is fused, therefore ReportIter is also fused
impl FusedIterator for ReportIter<'_> {}
