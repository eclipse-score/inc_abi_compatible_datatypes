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

use super::{State, TryParse};
use crate::ast::{tokenizer::Token, *};

impl TypeDeclKind {
    /// ```text
    /// '{' EnumVariant,* '}'
    /// ``````
    pub(super) fn parse_enum(state: &mut State) -> Self {
        match state.peek(0) {
            (Token::Braces(tokens), _) => {
                let mut variants = state.descend(tokens, |state| state.parse_list("}"));
                let mut decl = EnumTypeDecl { variants };
                decl.update_variant_indices();
                Self::Enum(decl)
            },
            (_, span) => {
                state.error(span, "expected '{…}'");
                Self::Error
            },
        }
    }
}

/// ```text
/// EnumVariant →
///       OuterDoc? Identifier
///     | OuterDoc? Identifier '(' TypeRef ')'
///     | OuterDoc? Identifier '{' StructField,* '}'
/// ``````
impl TryParse for EnumVariant {
    const EXPECTED: &'static str =
        "an enum variant (e.g., `Name`, `Name(Type)`, or `Name { field: Type }`)";

    fn try_parse(state: &mut State) -> Option<Self> {
        let doc = DocComment::parse_opt_outer(state);
        let name = Identifier::try_parse(state)?;
        let kind = EnumVariantKind::parse(state);
        let index = u32::MAX;
        Some(Self {
            doc,
            name,
            kind,
            index,
        })
    }
}

impl EnumVariantKind {
    fn parse(state: &mut State) -> Self {
        match state.peek(0) {
            (Token::Comma | Token::End, _) => Self::Unit,
            (Token::Braces(tokens), _) => state.descend(tokens, |state| {
                let fields = state.parse_list("}");
                Self::Struct { fields }
            }),
            (Token::Parens(tokens), _) => state.descend(tokens, |state| {
                let type_ref = TypeRef::parse(state);
                match state.peek(0) {
                    (Token::End, _) => {},
                    (_, span) => state.error(span, "expected ')'"),
                }
                Self::NewType { type_ref }
            }),
            (_, span) => {
                state.error(span, "expected '(…)', '{…}', ',', or '}'");
                Self::Unit
            },
        }
    }
}
