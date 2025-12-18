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

mod enums;
mod generics;
mod structs;

use std::{fmt, rc::Rc};

use crate::{
    ast::{
        tokenizer::{Token, TokenStream},
        *,
    },
    source::LocalSpan,
};

pub fn parse(tokens: &TokenStream<'_>, messages: Rc<MessageTracker>) -> Module {
    Module::parse(tokens, messages)
}

impl Module {
    /// ```text
    /// Module →
    ///     InnerDoc? Item*`
    /// ``````
    fn parse(tokens: &TokenStream, messages: Rc<MessageTracker>) -> Self {
        let state = &mut State::new(tokens, messages);

        let doc = DocComment::parse_opt_inner(state);

        let mut items = vec![];
        loop {
            let item_doc = DocComment::parse_opt_outer(state);
            let attributes = Attribute::parse_zero_or_more(state);

            match state.peek(0) {
                (Token::Word("use"), _) => {
                    items.push(Item::parse_import(state, item_doc, attributes))
                },
                (Token::Word("mod"), _) => {
                    items.push(Item::parse_module_decl(state, item_doc, attributes))
                },
                (Token::Word("type" | "struct" | "enum"), _) => {
                    items.push(Item::parse_type_decl(state, item_doc, attributes))
                },
                (Token::End, _) => {
                    if let Some(doc) = &item_doc {
                        state.error(doc.span, "no item for doc comment");
                    }
                    break;
                },
                (_, span) => {
                    state.error(span, "unexpected token");
                    state.consume();
                },
            }
        }
        Self { doc, items }
    }
}

impl Item {
    fn parse_import(
        state: &mut State,
        doc: Option<DocComment>,
        attributes: Vec<Attribute>,
    ) -> Self {
        Self::Import(Import::parse(state, doc, attributes))
    }

    fn parse_module_decl(
        state: &mut State,
        doc: Option<DocComment>,
        attributes: Vec<Attribute>,
    ) -> Self {
        Self::ModuleDecl(ModuleDecl::parse(state, doc, attributes))
    }

    fn parse_type_decl(
        state: &mut State,
        doc: Option<DocComment>,
        attributes: Vec<Attribute>,
    ) -> Self {
        Self::TypeDecl(TypeDecl::parse(state, doc, attributes))
    }
}

impl Import {
    /// ```text
    /// Import →
    ///     OuterDoc? 'use' Path ';'
    /// ``````
    fn parse(state: &mut State, doc: Option<DocComment>, attributes: Vec<Attribute>) -> Self {
        debug_assert_eq!(state.peek(0).0, &Token::Word("use"));
        state.consume(); // Consume the 'use' token

        let path = Path::parse(state);
        match state.peek(0) {
            (Token::Semicolon, _) => {
                state.consume();
            },
            (_other, span) => {
                state.error(span, "expected ';'");
            },
        }
        Self {
            doc,
            attributes,
            path,
        }
    }
}

impl ModuleDecl {
    /// ```text
    /// ModuleDecl →
    ///       OuterDoc? 'mod' Identifier ';'
    ///     | OuterDoc? 'mod' Identifier '{' Module '}'
    /// ``````
    fn parse(state: &mut State, doc: Option<DocComment>, attributes: Vec<Attribute>) -> Self {
        debug_assert_eq!(state.peek(0).0, &Token::Word("mod"));
        state.consume(); // Consume the 'mod' token

        let name = Identifier::parse(state);

        let body = match state.peek(0) {
            (Token::Semicolon, _) => {
                state.consume();
                None
            },
            (Token::Braces(module_tokens), _) => {
                let module = Module::parse(module_tokens, Rc::clone(&state.messages));
                state.consume();
                Some(module)
            },
            (_, span) => {
                state.error(span, "expected ';' or '{…}'");
                None
            },
        };

        Self {
            doc,
            attributes,
            name,
            body,
        }
    }
}

impl TypeDecl {
    /// ```text
    /// TypeDecl →
    ///       OuterDoc? Attributes* 'type' Identifier GenericParams? '=' TypeRef ';'
    ///     | OuterDoc? Attributes* 'struct' Identifier GenericParams? '(' TupleField,+ ')' ';'
    ///     | OuterDoc? Attributes* 'struct' Identifier GenericParams? '{' StructField,+ '}'
    ///     | OuterDoc? Attributes* 'enum' Identifier GenericParams? '{' EnumVariant,+ '}'
    ///
    /// GenericParams →
    ///     '<' ( Identifier | 'const' Identifier ),* '>'
    /// ```
    fn parse(state: &mut State, doc: Option<DocComment>, attributes: Vec<Attribute>) -> Self {
        let kind_token = state.consume();

        let name = Identifier::parse(state);

        let params = if let Token::Angles(param_tokens) = state.peek(0).0 {
            state.descend(param_tokens, |state| state.parse_list(">"))
        } else {
            vec![]
        };
        let generics = GenericParams { params };

        let kind = match kind_token.0 {
            Token::Word("type") => TypeDeclKind::parse_simple(state),
            Token::Word("struct") => TypeDeclKind::parse_struct(state),
            Token::Word("enum") => TypeDeclKind::parse_enum(state),
            _ => unreachable!(),
        };

        Self {
            doc,
            attributes,
            name,
            generics,
            kind,
        }
    }
}

impl TypeDeclKind {
    /// ```text
    /// '=' TypeRef ';'
    /// ``````
    fn parse_simple(state: &mut State) -> Self {
        match state.peek(0) {
            (Token::Equals, _) => {
                state.consume();
            },
            (Token::Semicolon, span) => {
                state.consume();
                state.error(span, "expected '=', found ';'");
                return Self::Error;
            },
            (_, span) => {
                state.error(span, "expected '='");
            },
        }

        let type_ref = TypeRef::parse(state);

        match state.peek(0) {
            (Token::Semicolon, _) => {
                state.consume();
            },
            (_, span) => state.error(span, "expected ';'"),
        }

        Self::Simple(SimpleTypeDecl { type_ref })
    }
}

impl TypeRef {
    /// ```text
    /// TypeRef →
    ///     Path GenericArgs?
    ///
    /// GenericArgs →
    ///     '<' ( TypeRef | Integer ),* '>'
    /// ``````
    fn parse(state: &mut State) -> Self {
        let path = Path::parse(state);
        let args = if let Token::Angles(arg_tokens) = state.peek(0).0 {
            state.descend(arg_tokens, |state| state.parse_list(">"))
        } else {
            vec![]
        };
        let generics = GenericArgs { args };
        Self { path, generics }
    }
}

impl TryParse for TypeRef {
    const EXPECTED: &'static str = "a type reference (e.g., `A::B::C<X, Y>`)";

    /// ```text
    /// TypeRef →
    ///     Path GenericArgs?
    /// ``````
    fn try_parse(state: &mut State) -> Option<Self> {
        if matches!(state.peek(0).0, Token::Word(_) | Token::DoubleColon) {
            Some(Self::parse(state))
        } else {
            None
        }
    }
}

impl Attribute {
    /// ```text
    /// Attribute →
    ///     '#' '[' Identifier ( '=' Path )? ']'
    /// ``````
    fn parse_zero_or_more(state: &mut State) -> Vec<Self> {
        let mut attributes = vec![];
        while let Token::Hash = state.peek(0).0 {
            attributes.push(Attribute::parse(state));
        }
        attributes
    }

    fn parse(state: &mut State) -> Self {
        debug_assert_eq!(state.peek(0).0, &Token::Hash);
        state.consume();
        match state.peek(0) {
            (Token::Squares(tokens), _) => state.descend(tokens, |state| {
                let name = Identifier::parse(state);
                let arg = match state.peek(0) {
                    (Token::End, _) => None,
                    (Token::Equals, _) => {
                        state.consume();
                        Some(Path::parse(state))
                    },
                    (_, span) => {
                        state.error(span, "expected '=' or ']'");
                        None
                    },
                };
                Self { name, arg }
            }),
            (_, span) => {
                state.error(span, "expected '( … )'");
                Self {
                    name: Identifier::invalid(span),
                    arg: None,
                }
            },
        }
    }
}

impl Path {
    /// ```text
    /// Path →
    ///     '::'? ( Identifier '::' )* Identifier
    /// ``````
    fn parse(state: &mut State) -> Self {
        let mut is_global = false;
        let mut elements = vec![];

        // '::'
        if state.peek(0).0 == &Token::DoubleColon {
            state.consume();
            is_global = true;
        }

        // '( Identifier '::' )* Identifier
        loop {
            match state.peek(0) {
                (Token::Word(identifier), span) => {
                    state.consume();
                    elements.push(Identifier::new(identifier, span));
                },
                (Token::DoubleColon, span) => {
                    state.error(span, "expected identifier");
                    state.consume();
                    continue;
                },
                (_, span) => {
                    state.error(span, "expected identifier");
                    break;
                },
            }
            if state.peek(0).0 == &Token::DoubleColon {
                state.consume();
            } else {
                break;
            }
        }
        Self {
            is_global,
            elements,
        }
    }
}

impl Identifier {
    fn parse(state: &mut State) -> Self {
        match state.peek(0) {
            (&Token::Word(name), span) => {
                state.consume();
                Self::new(name, span)
            },
            (_, span) => {
                state.error(span, "expected identifier");
                Self::invalid(span)
            },
        }
    }

    fn new(name: &str, span: LocalSpan) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }

    fn invalid(span: LocalSpan) -> Self {
        Self {
            name: "<error>".into(),
            span,
        }
    }
}

impl TryParse for Identifier {
    const EXPECTED: &'static str = "an identifier";

    fn try_parse(state: &mut State) -> Option<Self> {
        match state.peek(0) {
            (&Token::Word(name), span) => {
                state.consume();
                Some(Self {
                    name: name.into(),
                    span,
                })
            },
            _ => None,
        }
    }
}

impl DocComment {
    /// ```text
    /// OuterDoc → '///'… ?
    /// ``````
    fn parse_opt_outer(state: &mut State) -> Option<Self> {
        if let (Token::OuterDoc(lines), span) = state.peek(0) {
            state.consume();
            let lines = lines.iter().map(<&str>::to_string).collect();
            Some(Self { lines, span })
        } else {
            None
        }
    }

    /// ```text
    /// InnerDoc → '//!'… ?
    /// ``````
    fn parse_opt_inner(state: &mut State) -> Option<Self> {
        if let (Token::InnerDoc(lines), span) = state.peek(0) {
            state.consume();
            let lines = lines.iter().map(<&str>::to_string).collect();
            Some(Self { lines, span })
        } else {
            None
        }
    }
}

trait TryParse: Sized {
    const EXPECTED: &'static str;

    fn try_parse(state: &mut State<'_, '_>) -> Option<Self>;
}

type ParseResult<T> = Result<T, ParseError>;

struct ParseError {
    expected: Vec<&'static str>,
}

struct State<'src, 'tokens> {
    tokens: &'tokens [(Token<'src>, LocalSpan)],
    end: LocalSpan,
    messages: Rc<MessageTracker>,
}

impl<'src, 'tokens> State<'src, 'tokens> {
    #[must_use]
    fn new(stream: &'tokens TokenStream<'src>, messages: Rc<MessageTracker>) -> Self {
        Self {
            tokens: stream.tokens(),
            end: stream.span().after(),
            messages,
        }
    }

    fn consume(&mut self) -> (&'tokens Token<'src>, LocalSpan) {
        if let Some(((token, span), remaining)) = self.tokens.split_first() {
            self.tokens = remaining;
            (token, *span)
        } else {
            (&Token::End, self.end)
        }
    }

    fn peek(&self, distance: usize) -> (&'tokens Token<'src>, LocalSpan) {
        match self.tokens.get(distance) {
            Some((token, span)) => (token, *span),
            None => (&Token::End, self.end),
        }
    }

    fn descend<R>(
        &mut self,
        tokens: &'tokens TokenStream<'src>,
        f: impl FnOnce(&mut State<'src, 'tokens>) -> R,
    ) -> R {
        let inner = &mut Self::new(tokens, Rc::clone(&self.messages));
        self.consume();
        f(inner)
    }

    /// Parses the rest of the token stream as a comma-delimited list.
    fn parse_list<T: TryParse>(&mut self, closing: &str) -> Vec<T> {
        let mut items = vec![];

        loop {
            match self.peek(0) {
                (Token::End, _) => return items,
                (Token::Comma, span) => {
                    self.error(span, format!("expected {}", T::EXPECTED));
                    self.consume();
                    continue;
                },
                (other, span) => {
                    if let Some(item) = T::try_parse(self) {
                        items.push(item);
                    } else {
                        self.error(span, format!("expected {}", T::EXPECTED));
                        self.skip_until_including(&Token::Comma);
                        continue;
                    }
                },
            }

            match self.peek(0) {
                (Token::End, _) => return items,
                (Token::Comma, _) => {
                    self.consume();
                    continue;
                },
                (_, span) => {
                    self.error(span, format!("expected ',' or {closing}"));
                    self.skip_until_including(&Token::Comma);
                    continue;
                },
            }
        }
    }

    fn skip_until_including(&mut self, target: &Token<'src>) {
        loop {
            match self.consume().0 {
                Token::End => break,
                other if other == target => break,
                _ => continue,
            }
        }
    }

    fn error(&self, span: LocalSpan, msg: impl ToString) {
        self.messages.push(span, msg.to_string());
    }
}
