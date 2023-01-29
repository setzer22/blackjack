// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::Index;

use slotmap::{SecondaryMap, SlotMap};

use crate::prelude::{FaceId, HalfEdgeId, VertexId};

pub struct MeshMapping<K: slotmap::Key> {
    forward: SecondaryMap<K, u32>,
    inverse: Vec<K>,
}

impl<K: slotmap::Key> MeshMapping<K> {
    pub fn new<V>(s: &SlotMap<K, V>) -> Self {
        let forward = s
            .iter()
            .enumerate()
            .map(|(i, (k, _))| (k, i as u32))
            .collect();
        let inverse = s.iter().map(|(k, _)| k).collect();

        Self { forward, inverse }
    }
}

// NOTE: Macro impls are required because implementing for every slotmap::Key
// would cause a conflict between the forward and inverse impls, since rustc
// cannot know u32 isn't goint to implement slotmap::Key in the future.

macro_rules! impl_forward {
    ($kty:ty) => {
        impl Index<$kty> for MeshMapping<$kty> {
            type Output = u32;
            fn index(&self, index: $kty) -> &Self::Output {
                &self.forward[index]
            }
        }
    };
}

impl_forward!(VertexId);
impl_forward!(FaceId);
impl_forward!(HalfEdgeId);

macro_rules! impl_inverse {
    ($kty:ty) => {
        impl Index<u32> for MeshMapping<$kty> {
            type Output = $kty;
            fn index(&self, index: u32) -> &Self::Output {
                &self.inverse[index as usize]
            }
        }
    };
}

impl_inverse!(VertexId);
impl_inverse!(FaceId);
impl_inverse!(HalfEdgeId);

macro_rules! impl_map_seq {
    ($kty:ty) => {
        impl MeshMapping<$kty> {
            pub fn map_seq(&self, seq: &[$kty]) -> Vec<u32> {
                seq.iter().map(|x| self[*x]).collect()
            }
        }
    };
}

impl_map_seq!(VertexId);
impl_map_seq!(FaceId);
impl_map_seq!(HalfEdgeId);
