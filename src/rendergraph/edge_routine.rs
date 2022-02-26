use crate::prelude::*;
use glam::Vec4;

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
#[repr(C, align(16))]
pub struct EdgeMaterial {
    pub base_color: Vec4,
    pub thickness: f32,
    // TODO: Is this padding necessary?
    pub _pad1: f32,
    pub _pad2: f32,
    pub _pad3: f32,
}

impl r3::Material for EdgeMaterial {
    const TEXTURE_COUNT: u32 = 0;

    const DATA_SIZE: u32 = std::mem::size_of::<EdgeMaterial>() as u32;

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

/// A render routine made to draw edges for a mesh.
pub struct EdgeRoutine {
    pub forward_routine: r3::ForwardRoutine<EdgeMaterial>,
    // TODO: I don't think we need this?
    //pub depth_routine: r3::DepthRoutine<EdgeRoutine>,
    pub per_material: r3::PerMaterialArchetypeInterface<EdgeMaterial>,
}

impl EdgeRoutine {
    pub fn new(renderer: &r3::Renderer, base: &r3::BaseRenderGraph) -> Self {
        let shader = renderer.device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("edge_viewport.wgsl").into()),
        });

        let per_material = r3::PerMaterialArchetypeInterface::new(&renderer.device, renderer.mode);
        let mut data_core = renderer.data_core.lock();
        let forward_routine = r3::ForwardRoutine::new(
            renderer,
            &mut data_core,
            &base.interfaces,
            &per_material,
            Some(("vs_main", &shader)),
            Some(("fs_main", &shader)),
            &[],
            None,
            false,
            "Edge Forward",
        );

        Self {
            per_material,
            forward_routine,
        }
    }

    /// Adds pre-culling for objects with EdgeMaterial. This generates the
    /// data buffer that is going to be used during the culling phase
    pub fn add_pre_cull<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        pre_cull_data_out: r3::DataHandle<wgpu::Buffer>,
    ) {
        let trans_ty = r3::TransparencyType::Opaque;
        rend3_routine::pre_cull::add_to_graph::<EdgeMaterial>(
            graph,
            trans_ty as u64,
            trans_ty.to_sorting(),
            "Glow pass pre-cull",
            pre_cull_data_out,
        );
    }

    /// Performs culling and generates draw call information for objects with
    /// an EdgeMaterial.
    pub fn add_culling<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        base: &'node r3::BaseRenderGraph,
        _state: &r3::BaseRenderGraphIntermediateState,
        pre_cull_data_in: r3::DataHandle<wgpu::Buffer>,
        cull_data_out: r3::DataHandle<r3::PerMaterialArchetypeData>,
    ) {
        let trans_ty = r3::TransparencyType::Opaque;
        rend3_routine::culling::add_culling_to_graph::<EdgeMaterial>(
            graph,
            pre_cull_data_in,
            cull_data_out,
            &self.per_material,
            &base.gpu_culler,
            None,
            trans_ty as u64,
            trans_ty.to_sorting(),
            "Glow pass culling",
        );
    }

    /// Using the output from the culling phase, runs the forward render pass
    pub fn add_forward<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        forward_uniform_bg: r3::DataHandle<wgpu::BindGroup>,
        cull_data_in: r3::DataHandle<r3::PerMaterialArchetypeData>,
        color_target: r3::RenderTargetHandle,
        depth_target: r3::RenderTargetHandle,
    ) {
        self.forward_routine.add_forward_to_graph(
            graph,
            forward_uniform_bg,
            cull_data_in,
            None,
            "Glow forward pass",
            r3::SampleCount::One,
            color_target,
            None,
            depth_target,
        );
    }

    pub fn add_to_graph<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        base: &'node r3::BaseRenderGraph,
        state: &r3::BaseRenderGraphIntermediateState,
    ) {
        let pre_cull_data = graph.add_data();
        let cull_data = graph.add_data();
        self.add_pre_cull(graph, pre_cull_data);
        self.add_culling(graph, base, state, pre_cull_data, cull_data);
        self.add_forward(graph, state.forward_uniform_bg, cull_data, state.color, state.depth);

    }
}
