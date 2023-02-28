// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(not(feature = "sync"))]
use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

#[cfg(feature = "sync")]
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
#[cfg(feature = "sync")]
use std::sync::Arc;

#[cfg(feature = "sync")]
pub type InteriorMutable<T> = AtomicRefCell<T>;

#[cfg(not(feature = "sync"))]
pub type InteriorMutable<T> = RefCell<T>;

#[cfg(feature = "sync")]
pub type RefCounted<T> = Arc<T>;

#[cfg(not(feature = "sync"))]
pub type RefCounted<T> = Rc<T>;

#[cfg(feature = "sync")]
pub type BorrowedRef<'a, T> = AtomicRef<'a, T>;

#[cfg(not(feature = "sync"))]
pub type BorrowedRef<'a, T> = Ref<'a, T>;

#[cfg(feature = "sync")]
pub type MutableRef<'a, T> = AtomicRefMut<'a, T>;

#[cfg(not(feature = "sync"))]
pub type MutableRef<'a, T> = RefMut<'a, T>;

#[cfg(feature = "sync")]
pub trait MaybeSync: Send + Sync + 'static {}

#[cfg(not(feature = "sync"))]
pub trait MaybeSync {}

#[cfg(feature = "sync")]
fn is_sync() {
    use crate::prelude::HalfEdgeMesh;

    fn assert_thread_safe<T: Send + Sync + 'static>(_: T) {}
    assert_thread_safe(HalfEdgeMesh::new())
}
