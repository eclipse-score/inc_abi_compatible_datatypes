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

use anyhow::Result;
use log::{debug, warn};
use std::{
    fmt::{self, Write},
    path::Path,
    process::Command,
};

use crate::{ast, config::Config, format_error, iter_utils::IteratorExt as _};

trait Representable {
    fn repr(&self) -> impl fmt::Display;
}

impl<T: Representable + ?Sized> Representable for &T {
    fn repr(&self) -> impl fmt::Display {
        (*self).repr()
    }
}

trait Snakecase {
    fn repr_snakecase(&self) -> impl fmt::Display;
}

pub fn generate(module: &ast::Module, _: &Config) -> String {
    let mut generator = Generator {
        output: String::with_capacity(16 * 1024),
    };
    if let Err(error) = generator.gen_file(module) {
        panic!(
            "unexpected error during code generation: {}",
            format_error(&error),
        );
    }
    generator.output
}

pub fn format(path: &Path) {
    let status = Command::new("clang-format").arg("-i").arg(path).status();
    match status {
        Ok(status) => debug!("clang-format exited with {status}"),
        Err(error) => {
            warn!(
                "{}",
                format_error(&anyhow::Error::new(error).context("Couldn't run clang-format")),
            );
        },
    }
}

struct Generator {
    output: String,
}

impl Generator {
    fn gen_file(&mut self, top_module: &ast::Module) -> Result<()> {
        self.gen_file_prolog()?;
        self.gen_module(top_module)?;
        self.gen_file_epilog()?;
        Ok(())
    }

    fn gen_module(&mut self, module: &ast::Module) -> Result<()> {
        for item in &module.items {
            match item {
                ast::Item::Import(import) => self.gen_import(import)?,
                ast::Item::ModuleDecl(module_decl) => self.gen_module_decl(module_decl)?,
                ast::Item::TypeDecl(type_decl) => self.gen_type_decl(type_decl)?,
            }
        }
        Ok(())
    }

    fn gen_file_prolog(&mut self) -> Result<()> {
        writeln!(
            self.output,
            r#"\
#pragma once

// NOLINTBEGIN(cppcoreguidelines-avoid-magic-numbers,readability-magic-numbers)
// NOLINTBEGIN(cppcoreguidelines-pro-type-member-init,hicpp-member-init)
// NOLINTBEGIN(cppcoreguidelines-pro-type-union-access)
// NOLINTBEGIN(misc-non-private-member-variables-in-classes)
// NOLINTBEGIN(modernize-use-trailing-return-type)
// NOLINTBEGIN(readability-identifier-naming)

#include <array>
#include <cstdint>
#include <functional>
#include <optional>
#include "iox/vector.hpp"
"#
        )?;
        Ok(())
    }

    fn gen_file_epilog(&mut self) -> Result<()> {
        writeln!(
            self.output,
            "
// NOLINTEND(readability-identifier-naming)
// NOLINTEND(modernize-use-trailing-return-type)
// NOLINTEND(misc-non-private-member-variables-in-classes)
// NOLINTEND(cppcoreguidelines-pro-type-union-access)
// NOLINTEND(cppcoreguidelines-pro-type-member-init,hicpp-member-init)
// NOLINTEND(cppcoreguidelines-avoid-magic-numbers,readability-magic-numbers)"
        )?;
        Ok(())
    }

    fn gen_import(&mut self, import: &ast::Import) -> Result<()> {
        writeln!(self.output, "using {};", import.path.repr())?;
        Ok(())
    }

    fn gen_module_decl(&mut self, module_decl: &ast::ModuleDecl) -> Result<()> {
        let name = module_decl.name.repr();
        if let Some(body) = &module_decl.body {
            writeln!(self.output, "namespace {name} {{")?;
            self.gen_module(body)?;
            writeln!(self.output, "\n}}\n")?;
        } else {
            writeln!(
                self.output,
                "namespace {name} {{\n#include \"{name}.hpp\"\n}}\n",
            )?;
        }
        Ok(())
    }

    fn gen_type_decl(&mut self, type_decl: &ast::TypeDecl) -> Result<()> {
        match &type_decl.kind {
            ast::TypeDeclKind::Simple(decl) => self.gen_simple_type_decl(type_decl, decl)?,
            ast::TypeDeclKind::NewType(decl) => self.gen_newtype_decl(type_decl, decl)?,
            ast::TypeDeclKind::Struct(decl) => self.gen_struct_decl(type_decl, decl)?,
            ast::TypeDeclKind::Enum(decl) => self.gen_enum_decl(type_decl, decl)?,
            ast::TypeDeclKind::Error => unreachable!(),
        }

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
            "
{generics} using {name} = {type_ref};"
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
            "
{generics} struct {name}{{
    {type_ref} value;
}};"
        )?;

        Ok(())
    }

    fn gen_struct_decl(
        &mut self,
        type_decl: &ast::TypeDecl,
        content: &ast::StructTypeDecl,
    ) -> Result<()> {
        writeln!(
            self.output,
            "
{generics} struct {name}{{
    {fields}
}};",
            name = type_decl.name.repr(),
            generics = type_decl.generics.repr(),
            fields = content.fields.repr(),
        )?;

        Ok(())
    }

    fn gen_enum_decl(
        &mut self,
        type_decl: &ast::TypeDecl,
        content: &ast::EnumTypeDecl,
    ) -> Result<()> {
        let name = type_decl.name.repr();
        let generics = type_decl.generics.repr();

        writeln!(
            self.output,
            "
{generics} struct {name}{{
public:
    enum class Tag : uint8_t {{
"
        )?;

        for (index, variant) in content.variants.iter().enumerate() {
            writeln!(
                self.output,
                "    {name} = {index},",
                name = variant.name.repr(),
            )?;
        }
        writeln!(self.output, "}};")?;

        for variant in &content.variants {
            self.gen_variant_type(variant)?;
            self.gen_variant_type_tag(&type_decl.name, variant)?;
        }

        writeln!(
            self.output,
            "
    Tag tag() const noexcept {{
        return m_tag;
    }}

private:",
        )?;

        if content.is_tag_only() {
            writeln!(
                self.output,
                "
    Tag m_tag;",
            )?;
        } else {
            writeln!(
                self.output,
                "
    union Variant {{
        explicit Variant() noexcept {{}}",
            )?;

            for variant in content.variants.iter().filter(|variant| !variant.is_unit()) {
                writeln!(
                    self.output,
                    "
explicit Variant(const Variant{name}& {name_snakecase}) noexcept :
    {name_snakecase}({name_snakecase}) {{}}",
                    name = variant.name.repr(),
                    name_snakecase = variant.name.repr_snakecase(),
                )?;
            }
            writeln!(self.output)?;

            for variant in content.variants.iter().filter(|variant| !variant.is_unit()) {
                writeln!(
                    self.output,
                    "Variant{name} {name_snakecase};",
                    name = variant.name.repr(),
                    name_snakecase = variant.name.repr_snakecase(),
                )?;
            }

            writeln!(
                self.output,
                "}};

    Tag m_tag;
    Variant m_value;"
            )?;
        }

        writeln!(self.output, "}};",)?;

        Ok(())
    }

    fn gen_variant_type(&mut self, variant: &ast::EnumVariant) -> Result<()> {
        match &variant.kind {
            ast::EnumVariantKind::Unit => {
                writeln!(
                    self.output,
                    "
void set_to_{name_snakecase}() {{
    m_tag = Tag::{name};
}}",
                    name_snakecase = variant.name.repr_snakecase(),
                    name = variant.name.repr(),
                )?;
            },

            ast::EnumVariantKind::NewType { type_ref } => {
                writeln!(
                    self.output,
                    "
struct Variant{name} {{
    {type_ref} value;
}};
",
                    name = variant.name.repr(),
                    type_ref = type_ref.repr(),
                )?;
            },

            ast::EnumVariantKind::Struct { fields } => {
                writeln!(
                    self.output,
                    "
struct Variant{name} {{
    {fields}
}};
",
                    name = variant.name.repr(),
                    fields = fields.repr(),
                )?;
            },
        }
        Ok(())
    }

    fn gen_variant_type_tag(
        &mut self,
        type_name: &ast::Identifier,
        variant: &ast::EnumVariant,
    ) -> Result<()> {
        writeln!(
            self.output,
            "
struct {name}Tag {{
    static constexpr {type_name}::Tag VALUE = {type_name}::Tag::{name};
}};
static constexpr {name}Tag {name} {{}};",
            type_name = type_name.repr(),
            name = variant.name.repr()
        )?;
        Ok(())
    }
}

impl Representable for [ast::StructField] {
    fn repr(&self) -> impl fmt::Display {
        self.iter().map(Representable::repr).into_list(None, "")
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
                writeln!(f, "{type_ref} {name};")
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
            .into_list(Some(("template<", ">")), ", ")
    }
}

impl Representable for ast::GenericParam {
    fn repr(&self) -> impl fmt::Display {
        struct Delegate<'this>(&'this ast::GenericParam);

        impl fmt::Display for Delegate<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match &self.0 {
                    ast::GenericParam::Type(ident) => write!(f, "typename {}", ident.repr()),
                    ast::GenericParam::Value(ident) => write!(f, "std::size_t {}", ident.repr()),
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
                write!(f, "{}{}", self.0.path.repr(), self.0.generics.repr())
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
                let s = match self.0.name.as_str() {
                    "i8" => "std::int8_t",
                    "i16" => "std::int16_t",
                    "i32" => "std::int32_t",
                    "i64" => "std::int64_t",
                    "u8" => "std::uint8_t",
                    "u16" => "std::uint16_t",
                    "u32" => "std::uint32_t",
                    "u64" => "std::uint64_t",
                    "f32" => "float",
                    "f64" => "double",
                    "array" => "std::array",
                    other => other,
                };
                fmt::Display::fmt(s, f)
            }
        }

        Delegate(self)
    }
}

impl Snakecase for ast::Identifier {
    fn repr_snakecase(&self) -> impl fmt::Display {
        struct Delegate<'this>(&'this ast::Identifier);

        impl fmt::Display for Delegate<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut first = true;
                for c in self.0.name.chars() {
                    if c.is_uppercase() && !first {
                        f.write_str("_")?;
                    }
                    write!(f, "{}", c.to_lowercase())?;
                    first = false;
                }
                Ok(())
            }
        }

        Delegate(self)
    }
}
