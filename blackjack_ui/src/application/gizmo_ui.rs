use std::{cell::RefCell, hash::Hash, rc::Rc};

use anyhow::Result;
use blackjack_engine::{
    gizmos::{BlackjackGizmo, TransformGizmoMode},
    graph::BjkNodeId,
    graph_interpreter::GizmoState,
};
use egui_node_graph::NodeId;
use glam::Mat4;
use slotmap::SecondaryMap;

use crate::graph::graph_interop::NodeMapping;

use super::viewport_3d::Viewport3d;

pub struct UiGizmoState {
    pub gizmo_state: GizmoState,
    pub locked: bool,
}

/// Stores the gizmo state for each node in the graph. This structure references
/// node ids from the UI graph, so it requires regular bookkeeping (make sure
/// deleted graph nodes have their gizmo data deleted, and so on).
///
/// This struct uses shared ownership + interior mutability so it can be owned
/// by multiple places in the UI simultaneously, since both the graph and the
/// viewport need to access it regularly.
pub struct UiNodeGizmoStates {
    pub gizmos: Rc<RefCell<SecondaryMap<NodeId, UiGizmoState>>>,
}

pub enum GizmoViewportResponse {
    CaptureMouse,
    GizmoIsInteracted,
}

pub fn draw_gizmo_ui_viewport(
    viewport: &Viewport3d,
    ui: &mut egui::Ui,
    gizmo: &mut BlackjackGizmo,
    unique_id: impl Hash,
) -> Result<Vec<GizmoViewportResponse>> {
    let mut responses = Vec::new();

    match gizmo {
        BlackjackGizmo::Transform(transform_gizmo) => {
            ui.allocate_ui_at_rect(viewport.viewport_rect().shrink(10.0), |ui| {
                if ui.button("Move (G)").clicked() || ui.input().key_pressed(egui::Key::G) {
                    transform_gizmo.gizmo_mode = TransformGizmoMode::Translate;
                }
                if ui.button("Rotate (R)").clicked() || ui.input().key_pressed(egui::Key::R) {
                    transform_gizmo.gizmo_mode = TransformGizmoMode::Rotate;
                }
                if ui.button("Scale (S)").clicked() || ui.input().key_pressed(egui::Key::S) {
                    transform_gizmo.gizmo_mode = TransformGizmoMode::Scale;
                }
            });

            let gizmo = egui_gizmo::Gizmo::new(unique_id)
                .view_matrix(viewport.view_matrix().to_cols_array_2d())
                .projection_matrix(viewport.projection_matrix().to_cols_array_2d())
                .model_matrix(transform_gizmo.matrix().to_cols_array_2d())
                .viewport(viewport.viewport_rect())
                .mode(match transform_gizmo.gizmo_mode {
                    TransformGizmoMode::Translate => egui_gizmo::GizmoMode::Translate,
                    TransformGizmoMode::Rotate => egui_gizmo::GizmoMode::Rotate,
                    TransformGizmoMode::Scale => egui_gizmo::GizmoMode::Scale,
                });
            if let Some(response) = gizmo.interact(ui) {
                responses.push(GizmoViewportResponse::CaptureMouse);
                responses.push(GizmoViewportResponse::GizmoIsInteracted);
                let updated_matrix = Mat4::from_cols_array_2d(&response.transform);
                transform_gizmo.set_from_matrix(updated_matrix);
            }
        }
    }

    Ok(responses)
}

impl UiNodeGizmoStates {
    pub fn init() -> Self {
        Self {
            gizmos: Default::default(),
        }
    }

    pub fn share(&self) -> Self {
        Self {
            gizmos: Rc::clone(&self.gizmos),
        }
    }

    /// Returns a map suitable to be sent to blackjack_engine's run_node function
    pub fn to_bjk_data(&self, mapping: &NodeMapping) -> SecondaryMap<BjkNodeId, GizmoState> {
        let mut result = SecondaryMap::new();
        let gizmos = self.gizmos.borrow();
        for (node_id, gizmo_ui) in &*gizmos {
            result.insert(mapping[node_id], gizmo_ui.gizmo_state.clone());
        }
        result
    }

    /// Executes the provided fallible function for each of the active gizmos
    /// for every node in the graph.
    ///
    /// The provided callback `f` must return a boolean indicating whether the
    /// specific gizmo was interacted with during the frame.
    pub fn for_each_gizmo_mut(
        &mut self,
        mut f: impl FnMut(NodeId, usize, &mut BlackjackGizmo) -> Result<bool>,
    ) -> Result<()> {
        let mut gizmos = self.gizmos.borrow_mut();
        for (node_id, ui_state) in &mut *gizmos {
            for (idx, g) in ui_state
                .gizmo_state
                .active_gizmos
                .iter_mut()
                .flatten()
                .enumerate()
            {
                ui_state.gizmo_state.gizmos_changed = f(node_id, idx, g)?;
            }
        }
        Ok(())
    }

    /// Updates the gizmo values after they've been modified by the engine in a call to run_node.
    pub fn update_gizmos(
        &mut self,
        mut updated_gizmos: SecondaryMap<BjkNodeId, Vec<BlackjackGizmo>>,
        mapping: &NodeMapping,
    ) -> Result<()> {
        let mut gizmos = self.gizmos.borrow_mut();
        for (node_id, ui_state) in &mut *gizmos {
            if let Some(updated) = updated_gizmos.remove(mapping[node_id]) {
                ui_state.gizmo_state.active_gizmos = Some(updated);
            }
        }
        Ok(())
    }

    pub fn node_left_active(&self, n: NodeId) {
        let mut gizmos = self.gizmos.borrow_mut();
        if let Some(UiGizmoState { locked: false, .. }) = gizmos.get(n) {
            println!("Was not locked, removing gizmo state");
            gizmos.remove(n);
        }
    }

    pub fn node_is_active(&self, n: NodeId) {
        let mut gizmos = self.gizmos.borrow_mut();
        if gizmos.get(n).is_none() {
            gizmos.insert(
                n,
                UiGizmoState {
                    gizmo_state: GizmoState::default(),
                    locked: false,
                },
            );
        }
    }

    pub fn lock_gizmos_for(&self, n: NodeId) {
        let mut gizmos = self.gizmos.borrow_mut();
        match gizmos.get_mut(n) {
            Some(ui_state) => {
                ui_state.locked = true;
            }
            None => {
                gizmos.insert(
                    n,
                    UiGizmoState {
                        gizmo_state: GizmoState::default(),
                        locked: true,
                    },
                );
            }
        }
    }

    pub fn unlock_gizmos_for(&self, n: NodeId, active: Option<NodeId>) {
        let mut gizmos = self.gizmos.borrow_mut();
        if active.map(|a| a != n).unwrap_or(true) {
            gizmos.remove(n);
        } else if let Some(ui_state) = gizmos.get_mut(n) {
            ui_state.locked = false;
        }
    }

    pub fn reset_for_hot_reload(&self) {
        for (_, gizmo) in &mut *self.gizmos.borrow_mut() {
            gizmo.gizmo_state = GizmoState::default();
        }
    }

    pub fn is_node_locked(&self, n: NodeId) -> bool {
        self.gizmos
            .borrow()
            .get(n)
            .map(|x| x.locked)
            .unwrap_or(false)
    }
}
