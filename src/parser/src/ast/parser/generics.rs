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
use crate::ast::{
    tokenizer::{Token, TokenStream},
    *,
};

impl TryParse for GenericParam {
    const EXPECTED: &'static str = "a generic parameter (e.g., `T` or `const N`)";

    /// ```text
    /// GenericParam →
    ///     Identifier | 'const' Identifier
    /// ``````
    fn try_parse(state: &mut State) -> Option<Self> {
        match state.peek(0) {
            (Token::Word("const"), _) => {
                state.consume();
                Identifier::try_parse(state).map(Self::Value)
            },
            (Token::Word(identifier), span) => {
                state.consume();
                Some(Self::Type(Identifier::new(identifier, span)))
            },
            _ => None,
        }
    }
}

impl TryParse for GenericArg {
    const EXPECTED: &'static str = "a generic argument (a type or an integer)";

    /// ```text
    /// GenericArg →
    ///     TypeRef | Integer
    /// ``````
    fn try_parse(state: &mut State) -> Option<Self> {
        match state.peek(0) {
            (&Token::Integer(value), _) => {
                let arg = Self::Value(value);
                state.consume();
                Some(arg)
            },
            (Token::Word(_) | Token::DoubleColon, _) => {
                let type_ref = TypeRef::parse(state);
                Some(Self::Type(type_ref))
            },
            _ => None,
        }
    }
}
