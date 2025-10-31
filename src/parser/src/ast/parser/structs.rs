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
    ///   '(' TypeRef ')' ';'
    /// | '{' StructField,* '}'
    /// ``````
    pub(super) fn parse_struct(state: &mut State) -> Self {
        match state.peek(0) {
            (Token::Braces(tokens), _) => {
                let fields = state.descend(tokens, |state| state.parse_list("}"));
                Self::Struct(StructTypeDecl { fields })
            },
            (Token::Parens(tokens), _) => {
                let instance = state.descend(tokens, |state| {
                    let type_ref = TypeRef::parse(state);
                    match state.peek(0) {
                        (Token::End, _) => {},
                        (_, span) => state.error(span, "expected ')'"),
                    }
                    Self::NewType(NewTypeDecl { type_ref })
                });

                match state.peek(0) {
                    (Token::Semicolon, _) => {
                        state.consume();
                    },
                    (_, span) => {
                        state.error(span, "expected ';'");
                    },
                }

                instance
            },
            (_, span) => {
                state.error(span, "expected '(…)' or '{…}'");
                Self::Error
            },
        }
    }
}

impl TryParse for StructField {
    const EXPECTED: &'static str = "a struct field (e.g., `field_name: Type`)";

    /// ```text
    /// StructField →
    ///     OuterDoc? Identifier ':' TypeRef
    /// ``````
    fn try_parse(state: &mut State) -> Option<Self> {
        let doc = DocComment::parse_opt_outer(state);
        let name = Identifier::try_parse(state)?;
        match state.peek(0) {
            (Token::Colon, _) => {
                state.consume();
            },
            (_, span) => {
                state.error(span, "expected ':'");
            },
        }
        let type_ref = TypeRef::parse(state);

        Some(Self {
            doc,
            name,
            type_ref,
        })
    }
}
