use crate::prelude::*;

use super::shader_manager::Shader;

/// Routine to draw wireframe meshes with a custom material, either with lines,
/// or points.
pub struct WireRoutine<M: r3::Material> {
    pub forward_routine: r3::ForwardRoutine<M>,
    pub per_material: r3::PerMaterialArchetypeInterface<M>,
}

impl<M: r3::Material> WireRoutine<M> {
    pub fn new(
        renderer: &r3::Renderer,
        base: &r3::BaseRenderGraph,
        shader: &Shader,
        topology: wgpu::PrimitiveTopology,
    ) -> Self {
        let mut data_core = renderer.data_core.lock();
        data_core
            .material_manager
            .ensure_archetype::<M>(&renderer.device, renderer.profile);

        let per_material =
            r3::PerMaterialArchetypeInterface::new(&renderer.device, renderer.profile);
        let forward_routine = r3::ForwardRoutine::new(
            renderer,
            &mut data_core,
            &base.interfaces,
            &per_material,
            Some((&shader.vs_entry_point, &shader.module)),
            Some((&shader.fs_entry_point, &shader.module)),
            &[],
            None,
            false,
            topology,
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
        rend3_routine::pre_cull::add_to_graph::<M>(
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
        state: &r3::BaseRenderGraphIntermediateState,
        pre_cull_data_in: r3::DataHandle<wgpu::Buffer>,
        cull_data_out: r3::DataHandle<r3::PerMaterialArchetypeData>,
    ) {
        let trans_ty = r3::TransparencyType::Opaque;
        rend3_routine::culling::add_culling_to_graph::<M>(
            graph,
            pre_cull_data_in,
            cull_data_out,
            state.skinned_data,
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
        self.add_forward(
            graph,
            state.forward_uniform_bg,
            cull_data,
            state.color,
            state.depth,
        );
    }
}
