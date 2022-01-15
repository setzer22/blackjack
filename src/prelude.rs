pub use anyhow::{Context, Result, bail, anyhow};

pub use crate::color_hex_utils::color_from_hex;
pub use glam::{Mat4, Quat, UVec2, UVec3, Vec2, Vec3, Vec4};

pub mod r3 {
    pub use rend3::{
        types::{MaterialHandle, Mesh, MeshBuilder, MeshHandle, Object, ObjectHandle, DirectionalLight},
        Renderer,
    };

    pub use rend3_routine::material::{AlbedoComponent, PbrMaterial};
}

pub use itertools::Itertools;
pub use std::collections::{HashMap, HashSet};

pub use crate::render_context::RenderContext;

pub mod graph {
    pub use crate::graph::graph_types::*;
}


pub use crate::mesh::halfedge::*;

pub use crate::mesh::halfedge;
pub use crate::mesh::debug_viz;

pub use crate::math::{Vec3Ord, ToOrd, ToVec};