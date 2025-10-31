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

use std::{fmt, mem, rc::Rc};

use super::{MessageTracker, lexer::Lexeme};
use crate::source::LocalSpan;

#[derive(Clone, PartialEq)]
pub struct TokenStream<'src> {
    span: LocalSpan,
    tokens: Vec<(Token<'src>, LocalSpan)>,
}

impl<'src> TokenStream<'src> {
    pub fn tokenize(
        source: &'src str,
        messages: Rc<MessageTracker>,
        lexemes: impl Iterator<Item = (Lexeme<'src>, LocalSpan)>,
    ) -> Self {
        let mut state = SubtreeStack::new(messages);
        let mut lexemes = lexemes.peekable();
        while let Some((lexeme, span)) = lexemes.next() {
            match lexeme {
                Lexeme::Comma => state.push(Token::Comma, span),
                Lexeme::Semicolon => state.push(Token::Semicolon, span),
                Lexeme::Colon => state.push(Token::Colon, span),
                Lexeme::DoubleColon => state.push(Token::DoubleColon, span),
                Lexeme::Equals => state.push(Token::Equals, span),
                Lexeme::Hash => state.push(Token::Hash, span),

                Lexeme::Word(w) => state.push(Token::Word(w), span),
                Lexeme::Integer(s) => {
                    let n = s.parse().unwrap_or_else(|e| {
                        state.error(span, format!("invalid u64 value: {e}"));
                        0
                    });
                    state.push(Token::Integer(n), span);
                },

                Lexeme::InnerDoc(content_span) => {
                    let mut lines = vec![&source[content_span]];
                    let mut full_span = span;
                    while let Some(&(Lexeme::InnerDoc(content_span), span)) = lexemes.peek() {
                        lines.push(&source[content_span]);
                        full_span += span;
                        lexemes.next();
                    }
                    state.push(Token::InnerDoc(lines), full_span);
                },
                Lexeme::OuterDoc(content_span) => {
                    let mut lines = vec![&source[content_span]];
                    let mut full_span = span;
                    while let Some(&(Lexeme::OuterDoc(content_span), span)) = lexemes.peek() {
                        lines.push(&source[content_span]);
                        full_span += span;
                        lexemes.next();
                    }
                    state.push(Token::OuterDoc(lines), full_span);
                },

                Lexeme::OpenAngle => state.open_subtree(SubtreeKind::Angles, span),
                Lexeme::CloseAngle => state.close_subtree(SubtreeKind::Angles, span),

                Lexeme::OpenBrace => state.open_subtree(SubtreeKind::Braces, span),
                Lexeme::CloseBrace => state.close_subtree(SubtreeKind::Braces, span),

                Lexeme::OpenParen => state.open_subtree(SubtreeKind::Parens, span),
                Lexeme::CloseParen => state.close_subtree(SubtreeKind::Parens, span),

                Lexeme::OpenSquare => state.open_subtree(SubtreeKind::Squares, span),
                Lexeme::CloseSquare => state.close_subtree(SubtreeKind::Squares, span),

                Lexeme::Invalid(_) => {
                    let mut full_span = span;
                    while let Some((Lexeme::Invalid(_), span)) = lexemes.peek() {
                        full_span += span;
                        lexemes.next();
                    }
                    state.error(full_span, "invalid token");
                    state.push(Token::Error, full_span);
                },
            }
        }
        state.finish()
    }

    pub fn tokens(&self) -> &[(Token<'src>, LocalSpan)] {
        &self.tokens
    }

    pub fn span(&self) -> LocalSpan {
        self.span
    }

    fn push(&mut self, token: Token<'src>, span: LocalSpan) {
        self.span += span;
        self.tokens.push((token, span));
    }
}

impl fmt::Debug for TokenStream<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tokens = self.tokens.iter();
        if let Some(token) = tokens.next() {
            fmt::Debug::fmt(token, f)?;

            for token in tokens {
                f.write_str(" ")?;
                fmt::Debug::fmt(token, f)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq)]
pub enum Token<'src> {
    Comma,
    Semicolon,
    Colon,
    DoubleColon,
    Equals,
    Hash,

    // TODO split into keyword and identifier?
    Word(&'src str),
    Integer(u64),

    InnerDoc(
        /// The list of lines (after the `//!`) making up the doc comment.
        ///
        /// This list is never empty.
        Vec<&'src str>,
    ),
    OuterDoc(
        /// The list of lines (after the `///`) making up the doc comment.
        ///
        /// This list is never empty.
        Vec<&'src str>,
    ),

    // TODO unify these?
    Angles(TokenStream<'src>),
    Braces(TokenStream<'src>),
    Parens(TokenStream<'src>),
    Squares(TokenStream<'src>),

    Error,

    /// Marker token to be used by the parser;
    /// it is never generated by the tokenizer.
    End,
}

impl fmt::Debug for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Comma => write!(f, "','"),
            Self::Semicolon => write!(f, "';'"),
            Self::Colon => write!(f, "':'"),
            Self::DoubleColon => write!(f, "'::'"),
            Self::Equals => write!(f, "'='"),
            Self::Hash => write!(f, "'#'"),

            Self::Word(s) => write!(f, "'{s}'"),
            Self::Integer(n) => write!(f, "{n}"),

            Self::InnerDoc(_) => write!(f, "'//!'"),
            Self::OuterDoc(_) => write!(f, "'///'"),

            Self::Angles(inner) => {
                write!(f, "<")?;
                fmt::Debug::fmt(inner, f)?;
                write!(f, ">")?;
                Ok(())
            },
            Self::Braces(inner) => {
                write!(f, "{{")?;
                fmt::Debug::fmt(inner, f)?;
                write!(f, "}}")?;
                Ok(())
            },
            Self::Parens(inner) => {
                write!(f, "(")?;
                fmt::Debug::fmt(inner, f)?;
                write!(f, ")")?;
                Ok(())
            },
            Self::Squares(inner) => {
                write!(f, "[")?;
                fmt::Debug::fmt(inner, f)?;
                write!(f, "]")?;
                Ok(())
            },

            Self::Error => write!(f, "❌"),

            Self::End => write!(f, "⏹"),
        }
    }
}

struct SubtreeStack<'src> {
    /// Stack of open token streams, each with the token that started the next level.
    ///
    /// Example: for the lexeme sequence `a b < c { d e`, the stack would look like this:
    ///
    /// ```plain
    /// levels[0]:    [a, b], Angles
    /// levels[1]:    [c], Braces
    /// current_list: [d, e]
    /// ```
    levels: Vec<(TokenStream<'src>, SubtreeKind, LocalSpan)>,
    current_list: TokenStream<'src>,
    messages: Rc<MessageTracker>,
}

impl<'src> SubtreeStack<'src> {
    fn new(messages: Rc<MessageTracker>) -> Self {
        Self {
            levels: vec![],
            current_list: TokenStream {
                span: LocalSpan::pos_zero(),
                tokens: vec![],
            },
            messages,
        }
    }

    fn push(&mut self, token: Token<'src>, span: LocalSpan) {
        self.current_list.push(token, span);
    }

    fn open_subtree(&mut self, opening_kind: SubtreeKind, opening_span: LocalSpan) {
        let initial_span = opening_span.after();
        let prev = mem::replace(
            &mut self.current_list,
            TokenStream {
                span: initial_span,
                tokens: vec![],
            },
        );
        self.levels.push((prev, opening_kind, opening_span));
    }

    fn close_subtree(&mut self, closing_kind: SubtreeKind, closing_span: LocalSpan) {
        loop {
            if let Some((previous_list, opening_kind, opening_span)) = self.levels.pop() {
                if opening_kind != closing_kind {
                    self.current_list.push(Token::Error, closing_span);
                }
                let mut tokens = mem::replace(&mut self.current_list, previous_list);
                tokens.span += closing_span.before();
                let subtree_token = match opening_kind {
                    SubtreeKind::Angles => Token::Angles(tokens),
                    SubtreeKind::Braces => Token::Braces(tokens),
                    SubtreeKind::Parens => Token::Parens(tokens),
                    SubtreeKind::Squares => Token::Squares(tokens),
                };
                let span = opening_span + closing_span;
                self.current_list.push(subtree_token, span);
                if opening_kind == closing_kind {
                    break;
                }
            } else {
                self.current_list.push(Token::Error, closing_span);
                break;
            }
        }
    }

    fn finish(self) -> TokenStream<'src> {
        self.current_list
    }

    fn error(&self, span: LocalSpan, msg: impl ToString) {
        self.messages.push(span, msg.to_string());
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubtreeKind {
    Angles,
    Braces,
    Parens,
    Squares,
}
