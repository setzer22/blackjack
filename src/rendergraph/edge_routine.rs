use crate::prelude::*;
use glam::Vec4;

use super::{shader_manager::ShaderManager, wire_routine::WireRoutine};

#[derive(Clone, Copy, Default)]
#[repr(C, align(16))]
pub struct EdgeMaterial {
    pub base_color: Vec4,
    pub thickness: f32,
}

macro_rules! impl_material {
    (material = $mat:ty, routine = $routine:ident, shader = $shader:expr, topology = $topology:expr) => {
        impl r3::Material for $mat {
            const TEXTURE_COUNT: u32 = 0;

            const DATA_SIZE: u32 = std::mem::size_of::<$mat>() as u32;

            fn object_key(&self) -> u64 {
                0
            }

            fn to_textures<'a>(&'a self, _slice: &mut [Option<&'a rend3::types::TextureHandle>]) {
                // No textures
            }

            fn to_data(&self, slice: &mut [u8]) {
                slice.copy_from_slice(bytemuck::bytes_of(self));
            }
        }

        // SAFETY: In theory, it's UB to cast pad bytes to u8, so we can't derive the
        // bytemuck traits. In practice, doing this is not a problem on any supported
        // platforms, and rend3 itself also relies on this.
        unsafe impl bytemuck::Pod for $mat {}
        unsafe impl bytemuck::Zeroable for $mat {}

        pub struct $routine {
            wire_routine: WireRoutine<$mat>,
        }

        impl $routine {
            pub fn new(
                renderer: &r3::Renderer,
                base: &r3::BaseRenderGraph,
                shader_manager: &ShaderManager,
            ) -> Self {
                Self {
                    wire_routine: WireRoutine::new(
                        renderer,
                        base,
                        shader_manager.get($shader),
                        $topology,
                    ),
                }
            }

            pub fn add_to_graph<'node>(
                &'node self,
                graph: &mut r3::RenderGraph<'node>,
                base: &'node r3::BaseRenderGraph,
                state: &r3::BaseRenderGraphIntermediateState,
            ) {
                self.wire_routine.add_to_graph(graph, base, state);
            }
        }
    };
}

impl_material!(
    material = EdgeMaterial,
    routine = EdgeRoutine,
    shader = "edge_viewport",
    topology = wgpu::PrimitiveTopology::LineList
);