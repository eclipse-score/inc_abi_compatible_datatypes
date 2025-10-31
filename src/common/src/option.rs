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

use core::{fmt, mem::MaybeUninit};

#[cfg(any(feature = "iceoryx", feature = "lola"))]
use com_api_concept::Reloc;

#[cfg(feature = "iceoryx")]
use iceoryx2_bb_elementary_traits::{
    placement_default::PlacementDefault, zero_copy_send::ZeroCopySend,
};

#[repr(C)]
pub struct IpcOption<T> {
    value: MaybeUninit<T>,
    is_some: bool,
}

impl<T> IpcOption<T> {
    pub fn from_option(option: Option<T>) -> Self {
        if let Some(value) = option {
            Self {
                value: MaybeUninit::new(value),
                is_some: true,
            }
        } else {
            Self {
                value: MaybeUninit::uninit(),
                is_some: false,
            }
        }
    }

    pub fn into_option(self) -> Option<T> {
        if self.is_some {
            Some(unsafe { self.value.assume_init() })
        } else {
            None
        }
    }

    pub fn as_ref(&self) -> Option<&T> {
        if self.is_some {
            Some(unsafe { &*self.value.as_ptr() })
        } else {
            None
        }
    }

    pub fn as_mut(&mut self) -> Option<&mut T> {
        if self.is_some {
            Some(unsafe { &mut *self.value.as_mut_ptr() })
        } else {
            None
        }
    }

    pub fn is_some(&self) -> bool {
        self.is_some
    }

    pub fn is_none(&self) -> bool {
        !self.is_some
    }
}

impl<T> Default for IpcOption<T> {
    fn default() -> Self {
        Self {
            value: MaybeUninit::uninit(),
            is_some: false,
        }
    }
}

impl<T> From<T> for IpcOption<T> {
    fn from(value: T) -> Self {
        Self {
            value: MaybeUninit::new(value),
            is_some: true,
        }
    }
}

impl<T> From<Option<T>> for IpcOption<T> {
    fn from(option: Option<T>) -> Self {
        Self::from_option(option)
    }
}

impl<T> From<IpcOption<T>> for Option<T> {
    fn from(option: IpcOption<T>) -> Self {
        option.into_option()
    }
}

impl<T: Clone> Clone for IpcOption<T> {
    fn clone(&self) -> Self {
        match self.as_ref() {
            Some(value) => Self {
                value: MaybeUninit::new(value.clone()),
                is_some: true,
            },
            None => Self {
                value: MaybeUninit::uninit(),
                is_some: false,
            },
        }
    }
}

impl<T: Copy> Copy for IpcOption<T> {}

impl<T> fmt::Display for IpcOption<T>
where
    for<'a> Option<&'a T>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.as_ref(), f)
    }
}

impl<T> fmt::Debug for IpcOption<T>
where
    for<'a> Option<&'a T>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.as_ref(), f)
    }
}

#[cfg(feature = "iceoryx")]
impl<T> PlacementDefault for IpcOption<T> {
    unsafe fn placement_default(ptr: *mut Self) {
        unsafe {
            (&raw mut (*ptr).is_some).write(false);
        }
    }
}

#[cfg(any(feature = "iceoryx", feature = "lola"))]
unsafe impl<T: Reloc> Reloc for IpcOption<T> {}

#[cfg(feature = "iceoryx")]
unsafe impl<T: ZeroCopySend> ZeroCopySend for IpcOption<T> {}
