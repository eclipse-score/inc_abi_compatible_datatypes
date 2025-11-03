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

use anyhow::{Result, bail};
use log::{debug, warn};
use std::{fmt, fmt::Write, iter, path::Path, process::Command};

use crate::{ast, config::Config, format_error, iter_utils::IteratorExt as _};

trait Representable {
    fn repr(&self) -> impl fmt::Display;
}

impl<T: Representable + ?Sized> Representable for &T {
    fn repr(&self) -> impl fmt::Display {
        (*self).repr()
    }
}

pub fn generate(module: &ast::Module, config: &Config) -> String {
    let mut generator = Generator {
        output: String::with_capacity(16 * 1024),
        config,
    };
    if let Err(error) = generator.gen_module(module) {
        panic!(
            "unexpected error during code generation: {}",
            format_error(&error),
        );
    }
    generator.output
}

pub fn format(path: &Path) {
    let status = Command::new("rustfmt").arg(path).status();
    match status {
        Ok(status) => debug!("rustfmt exited with {status}"),
        Err(error) => {
            warn!(
                "{}",
                format_error(&anyhow::Error::new(error).context("Couldn't run rustfmt")),
            );
        },
    }
}

struct Generator<'config> {
    output: String,
    config: &'config Config,
}

impl Generator<'_> {
    fn gen_module(&mut self, module: &ast::Module) -> Result<()> {
        self.gen_module_prolog()?;
        for item in &module.items {
            writeln!(self.output)?;
            match item {
                ast::Item::Import(import) => self.gen_import(import)?,
                ast::Item::ModuleDecl(module_decl) => self.gen_module_decl(module_decl)?,
                ast::Item::TypeDecl(type_decl) => self.gen_type_decl(type_decl)?,
            }
        }
        Ok(())
    }

    fn gen_module_prolog(&mut self) -> Result<()> {
        write!(
            self.output,
            "
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(unused_imports)]

use core::mem::ManuallyDrop;

use gateway_common::e2e::E2ETypeConnector;
"
        )?;
        if self.config.rust.derive_iceoryx2_traits {
            writeln!(
                self.output,
                "\
use iceoryx2::prelude::PlacementDefault;
use iceoryx2::prelude::ZeroCopySend;",
            )?;
        }
        if self.config.rust.derive_reloc {
            writeln!(self.output, "use com_api::prelude::Reloc;")?;
        };
        Ok(())
    }

    fn gen_import(&mut self, import: &ast::Import) -> Result<()> {
        self.prepend_doc_comment(&import.doc)?;
        writeln!(self.output, "pub use {};", import.path.repr())?;
        Ok(())
    }

    fn gen_module_decl(&mut self, module_decl: &ast::ModuleDecl) -> Result<()> {
        self.prepend_doc_comment(&module_decl.doc)?;
        let name = module_decl.name.repr();
        if let Some(body) = &module_decl.body {
            writeln!(self.output, "pub mod {name} {{")?;
            self.gen_module(body)?;
            writeln!(self.output, "}}")?;
        } else {
            writeln!(self.output, "pub mod {name};")?;
        }
        Ok(())
    }

    fn gen_type_decl(&mut self, type_decl: &ast::TypeDecl) -> Result<()> {
        self.prepend_doc_comment(&type_decl.doc)?;
        match &type_decl.kind {
            ast::TypeDeclKind::Simple(decl) => self.gen_simple_type_decl(type_decl, decl)?,
            ast::TypeDeclKind::NewType(decl) => self.gen_newtype_decl(type_decl, decl)?,
            ast::TypeDeclKind::Struct(decl) => self.gen_struct_decl(type_decl, decl)?,
            ast::TypeDeclKind::Enum(decl) => self.gen_enum_decl(type_decl, decl)?,
            ast::TypeDeclKind::Error => unreachable!(),
        }

        self.gen_traits(type_decl)?;
        Ok(())
    }

    fn gen_simple_type_decl(
        &mut self,
        type_decl: &ast::TypeDecl,
        content: &ast::SimpleTypeDecl,
    ) -> Result<()> {
        let name = type_decl.name.repr();
        let generics = type_decl.generics.repr();
        let type_ref = content.type_ref.repr();
        writeln!(
            self.output,
            "\
#[derive(Clone, Debug)]
#[repr(C)]
pub type {name}{generics} = {type_ref};"
        )?;
        Ok(())
    }

    fn gen_newtype_decl(
        &mut self,
        type_decl: &ast::TypeDecl,
        content: &ast::NewTypeDecl,
    ) -> Result<()> {
        let name = type_decl.name.repr();
        let generics = type_decl.generics.repr();
        let type_ref = content.type_ref.repr();
        writeln!(
            self.output,
            "\
#[derive(Clone, Debug)]
#[repr(C)]
pub struct {name}{generics}(pub {type_ref});"
        )?;

        if self.config.rust.derive_iceoryx2_traits {
            writeln!(
                self.output,
                "
impl PlacementDefault for {name}{generics} {{
    unsafe fn placement_default(ptr: *mut Self) {{
        PlacementDefault::placement_default(&raw mut (*ptr).0);
    }}
}}",
                name = type_decl.name.repr(),
                generics = generics_placeholder(type_decl.generics.params.len()),
            )?;
        }

        Ok(())
    }

    fn gen_struct_decl(
        &mut self,
        type_decl: &ast::TypeDecl,
        content: &ast::StructTypeDecl,
    ) -> Result<()> {
        writeln!(
            self.output,
            "\
#[derive(Clone, Debug)]
#[repr(C)]
pub struct {name}{generics}{{{fields}}}",
            name = type_decl.name.repr(),
            generics = type_decl.generics.repr(),
            fields = content.fields.repr(),
        )?;

        if self.config.rust.derive_iceoryx2_traits {
            writeln!(
                self.output,
                "
impl PlacementDefault for {name}{generics} {{
    unsafe fn placement_default(ptr: *mut Self) {{",
                name = type_decl.name.repr(),
                generics = generics_placeholder(type_decl.generics.params.len()),
            )?;
            for field in &content.fields {
                writeln!(
                    self.output,
                    "PlacementDefault::placement_default(&raw mut (*ptr).{name});",
                    name = field.name.repr(),
                )?;
            }
            writeln!(self.output, "}}\n}}")?;
        }

        Ok(())
    }

    fn gen_enum_decl(
        &mut self,
        type_decl: &ast::TypeDecl,
        content: &ast::EnumTypeDecl,
    ) -> Result<()> {
        let name = type_decl.name.repr();
        let generics = type_decl.generics.repr();
        let variants = content.variants.repr();
        writeln!(
            self.output,
            "\
#[derive(Clone, Debug)]
#[repr(C)]
pub enum {name}{generics}{{{variants}}}",
        )?;
        Ok(())
    }

    fn gen_traits(&mut self, type_decl: &ast::TypeDecl) -> Result<()> {
        let type_string = format!(
            "{name}{generics}",
            name = type_decl.name.repr(),
            generics = generics_placeholder(type_decl.generics.params.len()),
        );
        if self.config.rust.derive_reloc {
            writeln!(
                self.output,
                "
unsafe impl Reloc for {type_string} {{}}",
            )?;
        }
        if self.config.rust.derive_iceoryx2_traits {
            writeln!(
                self.output,
                "
unsafe impl ZeroCopySend for {type_string} {{}}",
            )?;
        }
        for attribute in &type_decl.attributes {
            if attribute.name == "e2e_profile" {
                let Some(arg) = &attribute.arg else {
                    bail!("e2e_profile attribute requires an argument");
                };
                writeln!(
                    self.output,
                    "
impl E2ETypeConnector for {type_string} {{
    type ConnectedE2EProfile = {profile_name};
}}",
                    profile_name = arg.repr(),
                )?;
            }
        }
        Ok(())
    }

    fn prepend_doc_comment(&mut self, doc_comment: &Option<ast::DocComment>) -> Result<()> {
        if let Some(doc_comment) = doc_comment {
            for line in &doc_comment.lines {
                writeln!(self.output, "///{line}")?;
            }
        }
        Ok(())
    }
}

impl Representable for [ast::StructField] {
    fn repr(&self) -> impl fmt::Display {
        self.iter().map(Representable::repr).into_list(None, ", ")
    }
}

impl Representable for ast::StructField {
    fn repr(&self) -> impl fmt::Display {
        struct Delegate<'this>(&'this ast::StructField);

        impl fmt::Display for Delegate<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let field = &self.0;
                let name = field.name.repr();
                let type_ref = field.type_ref.repr();
                write!(f, "pub {name}: {type_ref}")
            }
        }

        Delegate(self)
    }
}

impl Representable for [ast::EnumVariant] {
    fn repr(&self) -> impl fmt::Display {
        self.iter().map(Representable::repr).into_list(None, ", ")
    }
}

impl Representable for ast::EnumVariant {
    fn repr(&self) -> impl fmt::Display {
        struct Delegate<'this>(&'this ast::EnumVariant);

        impl fmt::Display for Delegate<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let variant = &self.0;
                let name = variant.name.repr();
                match &variant.kind {
                    ast::EnumVariantKind::Unit => write!(f, "{name}"),
                    ast::EnumVariantKind::NewType { type_ref } => {
                        let type_ref = type_ref.repr();
                        write!(f, "{name}({type_ref})")
                    },
                    ast::EnumVariantKind::Struct { fields } => {
                        let fields = fields.repr();
                        write!(f, "{name}{{{fields}}}")
                    },
                }
            }
        }

        Delegate(self)
    }
}

impl Representable for ast::GenericParams {
    fn repr(&self) -> impl fmt::Display {
        self.params
            .iter()
            .map(Representable::repr)
            .into_list(Some(("<", ">")), ", ")
    }
}

impl Representable for ast::GenericParam {
    fn repr(&self) -> impl fmt::Display {
        struct Delegate<'this>(&'this ast::GenericParam);

        impl fmt::Display for Delegate<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match &self.0 {
                    ast::GenericParam::Type(ident) => write!(f, "{}", ident.repr()),
                    ast::GenericParam::Value(ident) => write!(f, "const {}: usize", ident.repr()),
                }
            }
        }

        Delegate(self)
    }
}

impl Representable for ast::GenericArgs {
    fn repr(&self) -> impl fmt::Display {
        self.args
            .iter()
            .map(Representable::repr)
            .into_list(Some(("<", ">")), ", ")
    }
}

impl Representable for ast::GenericArg {
    fn repr(&self) -> impl fmt::Display {
        struct Delegate<'this>(&'this ast::GenericArg);

        impl fmt::Display for Delegate<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match &self.0 {
                    ast::GenericArg::Type(type_ref) => write!(f, "{}", type_ref.repr()),
                    ast::GenericArg::Value(value) => write!(f, "{value}"),
                }
            }
        }

        Delegate(self)
    }
}

impl Representable for ast::TypeRef {
    fn repr(&self) -> impl fmt::Display {
        struct Delegate<'this>(&'this ast::TypeRef);

        impl fmt::Display for Delegate<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let path = &self.0.path;
                let generics = &self.0.generics;
                if let Some("array") = path.as_simple_name() {
                    let typ = generics.args[0].repr();
                    let size = generics.args[1].repr();
                    write!(f, "[{typ}; {size}]")
                } else {
                    write!(f, "{}{}", path.repr(), generics.repr())
                }
            }
        }

        Delegate(self)
    }
}

impl Representable for ast::Path {
    fn repr(&self) -> impl fmt::Display {
        let wrapper = if self.is_global {
            Some(("::", ""))
        } else {
            None
        };
        self.elements
            .iter()
            .map(Representable::repr)
            .into_list(wrapper, "::")
    }
}

impl Representable for ast::Identifier {
    fn repr(&self) -> impl fmt::Display {
        struct Delegate<'this>(&'this ast::Identifier);

        impl fmt::Display for Delegate<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", &self.0.name)
            }
        }

        Delegate(self)
    }
}

fn generics_placeholder(length: usize) -> impl fmt::Display {
    iter::repeat_n("_", length).into_list(Some(("<", ">")), ", ")
}
