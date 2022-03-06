use winit::event::MouseButton;

use crate::app_window::input::InputSystem;
use crate::{prelude::*, rendergraph};

use super::app_viewport::AppViewport;

pub struct Viewport3dSettings {
    pub smooth_normals: bool,
    pub render_edges: bool,
    pub render_faces: bool,
    pub render_vertices: bool,
    pub matcap: usize,
}

pub struct Viewport3d {
    camera: OrbitCamera,
    input: InputSystem,
    viewport_rect: egui::Rect,
    parent_scale: f32,
    pub settings: Viewport3dSettings,
}

struct OrbitCamera {
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: -30.0,
            pitch: 30.0,
            distance: 8.0,
        }
    }
}

impl Viewport3d {
    pub fn new() -> Self {
        Self {
            camera: OrbitCamera::default(),
            input: InputSystem::default(),
            // Initial size and scale is not important. It will get reset after
            // the first update.
            viewport_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(10.0, 10.0)),
            parent_scale: 1.0,
            settings: Viewport3dSettings {
                render_edges: true,
                render_faces: true,
                render_vertices: true,
                matcap: 0,
                smooth_normals: false,
            },
        }
    }

    pub fn on_winit_event(
        &mut self,
        parent_scale: f32,
        viewport_rect: egui::Rect,
        event: winit::event::Event<'static, ()>,
    ) {
        #[allow(clippy::single_match)]
        match event {
            winit::event::Event::WindowEvent { event, .. } => {
                self.input
                    .on_window_event(&event, parent_scale, viewport_rect);
            }
            _ => {}
        }
    }

    fn update_camera(&mut self, render_ctx: &mut RenderContext) {
        // Update status
        if self.input.mouse.buttons().pressed(MouseButton::Left) {
            self.camera.yaw += self.input.mouse.cursor_delta().x * 2.0;
            self.camera.pitch += self.input.mouse.cursor_delta().y * 2.0;
        }
        self.camera.distance += self.input.mouse.wheel_delta();

        // Compute view matrix
        let view = Mat4::from_translation(Vec3::Z * self.camera.distance)
            * Mat4::from_rotation_x(-self.camera.pitch.to_radians())
            * Mat4::from_rotation_y(-self.camera.yaw.to_radians());
        render_ctx.set_camera(view);
    }

    pub fn update(
        &mut self,
        parent_scale: f32,
        viewport_rect: egui::Rect,
        render_ctx: &mut RenderContext,
    ) {
        self.viewport_rect = viewport_rect;
        self.parent_scale = parent_scale;

        self.update_camera(render_ctx);
        self.input.update();

        // TODO: What if we ever have multiple 3d viewports? There's no way to
        // set the aspect ratio differently for different render passes in rend3
        // right now. The camera is global.
        //
        // See: https://github.com/BVE-Reborn/rend3/issues/327
        render_ctx
            .renderer
            .set_aspect_ratio(self.viewport_rect.width() / self.viewport_rect.height());
    }

    fn ambient_light() -> Vec4 {
        Vec4::splat(0.25)
    }

    fn get_resolution(&self) -> UVec2 {
        UVec2::new(
            (self.viewport_rect.width() * self.parent_scale) as u32,
            (self.viewport_rect.height() * self.parent_scale) as u32,
        )
    }

    pub fn add_to_graph<'node>(
        &'node mut self,
        graph: &mut r3::RenderGraph<'node>,
        ready: &r3::ReadyData,
        viewport_routines: super::ViewportRoutines<'node>,
    ) -> r3::RenderTargetHandle {
        rendergraph::blackjack_viewport_rendergraph(
            graph,
            ready,
            viewport_routines,
            self.get_resolution(),
            r3::SampleCount::One,
            Self::ambient_light(),
            &self.settings,
        )
    }

    pub fn show_ui(&mut self, ui: &mut egui::Ui, offscreen_viewport: &mut AppViewport) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.settings.render_edges, "E");
                ui.checkbox(&mut self.settings.render_vertices, "V");
                ui.checkbox(&mut self.settings.render_faces, "F");

                if ui.button("<").clicked() {
                    self.settings.matcap -= 1;
                }
                ui.add(egui::DragValue::new(&mut self.settings.matcap)
                    .clamp_range(0..=crate::rendergraph::face_routine::NUM_MATCAPS-1));
                if ui.button(">").clicked() {
                    self.settings.matcap += 1;
                }

                ui.checkbox(&mut self.settings.smooth_normals, "S");
            });
            offscreen_viewport.show(ui, ui.available_size());
        });
    }
}

impl Default for Viewport3d {
    fn default() -> Self {
        Self::new()
    }
}
