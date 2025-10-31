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

use std::{cell::RefCell, fmt};

pub trait IteratorExt: Iterator {
    fn into_list<'a>(
        self,
        wrapper: Option<(&'a str, &'a str)>,
        separator: &'a str,
    ) -> impl fmt::Display + 'a
    where
        Self: 'a;
}

impl<Iter> IteratorExt for Iter
where
    Iter: Iterator,
    Iter::Item: fmt::Display,
{
    fn into_list<'a>(
        self,
        wrapper: Option<(&'a str, &'a str)>,
        separator: &'a str,
    ) -> impl fmt::Display + 'a
    where
        Iter: 'a,
    {
        ListDelegate {
            iter: RefCell::new(Some(self)),
            wrapper,
            separator,
        }
    }
}

struct ListDelegate<'a, Iter> {
    iter: RefCell<Option<Iter>>,
    wrapper: Option<(&'a str, &'a str)>,
    separator: &'a str,
}

impl<'a, Iter> fmt::Display for ListDelegate<'a, Iter>
where
    Iter: Iterator,
    Iter::Item: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.iter.take().unwrap();
        if let Some(first) = iter.next() {
            if let Some((opening, _)) = self.wrapper {
                f.write_str(opening)?;
            }
            write!(f, "{first}")?;
            for item in iter {
                write!(f, "{separator}{item}", separator = self.separator)?;
            }
            if let Some((_, closing)) = self.wrapper {
                f.write_str(closing)?;
            }
        }
        Ok(())
    }
}
