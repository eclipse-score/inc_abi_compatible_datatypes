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

use core::{fmt, mem::ManuallyDrop};

#[cfg(any(feature = "iceoryx", feature = "lola"))]
use com_api_concept::Reloc;

#[cfg(feature = "iceoryx")]
use iceoryx2_bb_elementary_traits::{
    placement_default::PlacementDefault, zero_copy_send::ZeroCopySend,
};

#[derive(Copy)]
#[repr(C)]
pub struct IpcResult<T, E> {
    value: ResultUnion<T, E>,
    is_ok: bool,
}

#[derive(Copy)]
#[repr(C)]
union ResultUnion<T, E> {
    ok: ManuallyDrop<T>,
    err: ManuallyDrop<E>,
}

impl<T, E> IpcResult<T, E> {
    pub fn from_result(result: Result<T, E>) -> Self {
        match result {
            Ok(value) => Self {
                value: ResultUnion {
                    ok: ManuallyDrop::new(value),
                },
                is_ok: true,
            },
            Err(error) => Self {
                value: ResultUnion {
                    err: ManuallyDrop::new(error),
                },
                is_ok: false,
            },
        }
    }

    pub fn into_result(self) -> Result<T, E> {
        if self.is_ok {
            Ok(ManuallyDrop::into_inner(unsafe { self.value.ok }))
        } else {
            Err(ManuallyDrop::into_inner(unsafe { self.value.err }))
        }
    }

    pub fn as_ref(&self) -> Result<&T, &E> {
        if self.is_ok {
            Ok(unsafe { &self.value.ok })
        } else {
            Err(unsafe { &self.value.err })
        }
    }

    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {
        if self.is_ok {
            Ok(unsafe { &mut self.value.ok })
        } else {
            Err(unsafe { &mut self.value.err })
        }
    }

    pub fn is_ok(&self) -> bool {
        self.is_ok
    }

    pub fn is_err(&self) -> bool {
        !self.is_ok
    }
}

impl<T: Clone, E: Clone> Clone for IpcResult<T, E> {
    fn clone(&self) -> Self {
        let cloned = match self.as_ref() {
            Ok(value) => Ok(value.clone()),
            Err(error) => Err(error.clone()),
        };
        cloned.into()
    }
}

// This is only needed to enable `#[derive(Copy)]` for `ResultUnion`, we don't actually use it
// for cloning `IpcResult`.
impl<T: Copy, E: Copy> Clone for ResultUnion<T, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Default, E> Default for IpcResult<T, E> {
    fn default() -> Self {
        Self {
            value: ResultUnion {
                ok: ManuallyDrop::new(T::default()),
            },
            is_ok: true,
        }
    }
}

impl<T, E> From<T> for IpcResult<T, E> {
    fn from(value: T) -> Self {
        Self {
            value: ResultUnion {
                ok: ManuallyDrop::new(value),
            },
            is_ok: true,
        }
    }
}

impl<T, E> From<Result<T, E>> for IpcResult<T, E> {
    fn from(result: Result<T, E>) -> Self {
        Self::from_result(result)
    }
}

impl<T, E> From<IpcResult<T, E>> for Result<T, E> {
    fn from(result: IpcResult<T, E>) -> Self {
        result.into_result()
    }
}

impl<T, E> fmt::Display for IpcResult<T, E>
where
    for<'a> Result<&'a T, &'a E>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.as_ref(), f)
    }
}

impl<T, E> fmt::Debug for IpcResult<T, E>
where
    for<'a> Result<&'a T, &'a E>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.as_ref(), f)
    }
}

#[cfg(feature = "iceoryx")]
impl<T, E> PlacementDefault for IpcResult<T, E> {
    unsafe fn placement_default(ptr: *mut Self) {
        unsafe {
            (&raw mut (*ptr).is_ok).write(false);
        }
    }
}

#[cfg(any(feature = "iceoryx", feature = "lola"))]
unsafe impl<T: Reloc, E: Reloc> Reloc for IpcResult<T, E> {}

#[cfg(feature = "iceoryx")]
unsafe impl<T: ZeroCopySend, E: ZeroCopySend> ZeroCopySend for IpcResult<T, E> {}
