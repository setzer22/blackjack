use rend3::{RenderGraph, RenderTargetHandle};
use rend3_routine::PbrRenderRoutine;
use wgpu::Device;

pub fn build_wireframe_pass_pipeline(_device: &Device, _pbr_routine: &PbrRenderRoutine) -> () {
    /*
    let wireframe_pass_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some("wireframe_pass_vertex"),
        source: wgpu::ShaderSource::Wgsl(include_str!("wireframe.wgsl").into()),
    });

    let mut bgls: Vec<&BindGroupLayout> = Vec::new();
    bgls.push(&pbr_routine.interfaces.per_material_bgl);

    let pll = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("wireframe pass"),
        bind_group_layouts: &bgls,
        push_constant_ranges: &[],
    });

    let cpu_vertex_buffers = cpu_vertex_buffers();

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("wireframe pass"),
        layout: Some(&pll),
        vertex: VertexState {
            module: &wireframe_pass_shader,
            entry_point: "vertex_main",
            buffers: &cpu_vertex_buffers,
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Cw,
            cull_mode: Some(Face::Back),
            clamp_depth: false,
            polygon_mode: PolygonMode::Line,
            conservative: false,
        },
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            ..Default::default()
        },
        fragment: Some(FragmentState {
            module: &wireframe_pass_shader,
            entry_point: "fragment_main",
            targets: &[ColorTargetState {
                format: TextureFormat::Rgba16Float,
                blend: None,
                write_mask: ColorWrites::all(),
            }],
        }),
    })
    */
}

pub struct WireframeRoutine {
    pipeline: (),
}

impl WireframeRoutine {
    pub fn new(device: &Device, pbr_routine: &PbrRenderRoutine) -> Self {
        let pipeline = build_wireframe_pass_pipeline(device, pbr_routine);
        WireframeRoutine { pipeline }
    }

    pub fn add_to_graph(&self, _graph: &mut RenderGraph, _color: RenderTargetHandle) {
        /*
               let mut builder = graph.add_node("Wireframe Pass");

               let hdr_color_handle = builder.add_render_target_output(color);

               let rpass_handle = builder.add_renderpass(RenderPassTargets {
                   targets: vec![RenderPassTarget {
                       color: hdr_color_handle,
                       clear: Color::BLACK,
                       resolve: None,
                   }],
                   depth_stencil: None,
               });

               let _ = builder.add_shadow_array_input();

               let forward_uniform_handle = builder.add_data_input(forward_uniform_bg);
               let cull_handle = builder.add_data_input(culled);

               let pt_handle = builder.passthrough_ref(self);

               builder.build(move |pt, renderer, encoder_or_pass, temps, ready, graph_data| {
                   let this = pt.get(pt_handle);
                   let rpass = encoder_or_pass.get_rpass(rpass_handle);
                   let forward_uniform_bg = graph_data.get_data(temps, forward_uniform_handle).unwrap();
                   let culled = graph_data.get_data(temps, cull_handle).unwrap();

                   let pass = match transparency {
                       TransparencyType::Opaque => &this.primary_passes.opaque_pass,
                       TransparencyType::Cutout => &this.primary_passes.cutout_pass,
                       TransparencyType::Blend => &this.primary_passes.transparent_pass,
                   };

                   let d2_texture_output_bg_ref = ready.d2_texture.bg.as_ref().map(|_| (), |a| &**a);

                   pass.draw(forward::ForwardPassDrawArgs {
                       device: &renderer.device,
                       rpass,
                       materials: graph_data.material_manager,
                       meshes: graph_data.mesh_manager.buffers(),
                       samplers: &this.samplers,
                       forward_uniform_bg,
                       per_material_bg: &culled.per_material,
                       texture_bg: d2_texture_output_bg_ref,
                       culled_objects: &culled.inner,
                   });
               });
        */
    }
}
