use egui_wgpu::wgpu;
use egui_wgpu::RenderState;
use guee::prelude::*;

#[derive(PartialEq, Eq, Default)]
pub enum EdgeDrawMode {
    HalfEdge,
    #[default]
    FullEdge,
    NoDraw,
}

#[derive(PartialEq, Eq, Default)]
pub enum FaceDrawMode {
    /// Will read the actual configured value for the mesh and use its channel,
    /// if any. Defaults to flat shading otherwise.
    #[default]
    Real,
    /// Force flat shading, ignoring mesh data.
    Flat,
    /// Force smooth shading, ignoring mesh data
    Smooth,
    /// Don't draw faces.
    NoDraw,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TextOverlayMode {
    /// No text overlay
    #[default]
    NoDraw,
    /// Display face ids
    MeshInfoFaces,
    /// Display vertex ids
    MeshInfoVertices,
    /// Display halfedge ids
    MeshInfoHalfedges,
    /// Display all edge ids
    MeshInfoAll,
    /// Display mesh debug information set by the developers when debugging a
    /// problem. This is not intended to be used by regular users.
    DevDebug,
}

pub struct Viewport3dSettings {
    pub render_vertices: bool,
    pub matcap: usize,
    pub edge_mode: EdgeDrawMode,
    pub face_mode: FaceDrawMode,
    pub overlay_mode: TextOverlayMode,
}

impl Default for Viewport3dSettings {
    fn default() -> Self {
        Self {
            render_vertices: true,
            matcap: 0,
            edge_mode: Default::default(),
            face_mode: Default::default(),
            overlay_mode: Default::default(),
        }
    }
}

pub struct Viewport3d {
    settings: Viewport3dSettings,
}

impl Viewport3d {
    pub fn new(render_ctx: &RenderState) -> Self {
        Self {
            settings: Default::default()
        }
    }

    pub fn view(&self, render_ctx: &RenderState) -> DynWidget {
        let viewport_texture = render_ctx.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: 1024,
                height: 768,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("Blackjack Viewport3d Texture"),
        });

        Text::new("Potato".into()).build()
    }
}
