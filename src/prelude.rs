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
            DirectionalLight, DirectionalLightHandle, Handedness, MaterialHandle, Mesh,
            MeshBuilder, MeshHandle, Object, ObjectHandle, ObjectMeshKind, SampleCount,
            TextureFormat, TextureUsages,
        },
        Renderer,
    };

    pub use rend3_routine::base::{BaseRenderGraph, BaseRenderGraphIntermediateState};
    pub use rend3_routine::pbr::{AlbedoComponent, PbrMaterial, PbrRoutine};
    pub use rend3_routine::tonemapping::TonemappingRoutine;
}

pub use itertools::Itertools;
pub use std::collections::{HashMap, HashSet};

pub use crate::render_context::RenderContext;

pub mod graph {
    pub use crate::graph::graph_types::*;
}

pub use crate::mesh::halfedge::*;

pub use crate::mesh::debug_viz;
pub use crate::mesh::halfedge;

pub use crate::math::{ToOrd, ToVec, Vec3Ord};
