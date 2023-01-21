// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub use anyhow::{anyhow, bail, Context, Result};

pub use crate::color_hex_utils::color_from_hex;
pub use glam::{Mat4, Quat, UVec2, UVec3, Vec2, Vec3, Vec4};

pub mod r3 {
    pub use rend3::{
        graph::{
            DataHandle, DepthHandle, ReadyData, RenderGraph, RenderPassDepthTarget,
            RenderPassHandle, RenderPassTarget, RenderPassTargets, RenderTargetDescriptor,
            RenderTargetHandle,
        },
        types::{
            DirectionalLight, DirectionalLightHandle, Handedness, Material, MaterialHandle, Mesh,
            MeshBuilder, MeshHandle, Object, ObjectHandle, ObjectMeshKind, SampleCount,
            TextureFormat, TextureUsages,
        },
        Renderer, RendererDataCore,
    };

    pub use rend3_routine::base::{BaseRenderGraph, BaseRenderGraphIntermediateState};
    pub use rend3_routine::common::PerMaterialArchetypeInterface;
    pub use rend3_routine::culling::PerMaterialArchetypeData;
    pub use rend3_routine::pbr::{AlbedoComponent, PbrMaterial, PbrRoutine, TransparencyType};
    pub use rend3_routine::tonemapping::TonemappingRoutine;
    pub use rend3_routine::{depth::DepthRoutine, forward::ForwardRoutine};
}

pub use itertools::Itertools;
pub use std::collections::{HashMap, HashSet};

pub use crate::render_context::RenderContext;

pub use crate::egui_ext::*;
pub use blackjack_commons::math::*;
pub use blackjack_commons::utils::*;

pub mod graph {
    pub use crate::graph::node_graph::*;
    pub use egui_node_graph::{InputId, Node, NodeId, OutputId};
}
