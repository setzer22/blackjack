// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{cell::RefCell, hash::Hash, rc::Rc};

use anyhow::Result;
use blackjack_commons::utils::OptionExt;
use blackjack_engine::{
    gizmos::{BlackjackGizmo, TransformGizmoMode},
    graph::BjkNodeId,
    graph_interpreter::GizmoState,
};
use egui_gizmo::GizmoVisuals;
use egui_node_graph::{Node, NodeId};
use glam::Mat4;
use slotmap::SecondaryMap;

use crate::{graph::graph_interop::NodeMapping, prelude::graph::NodeData};

use super::viewport_3d::Viewport3d;

pub struct UiGizmoState {
    /// The gizmo state from `blackjack_engine`.
    pub gizmo_state: GizmoState,
    /// The user has manually locked the gizmos for this node in the user interface
    pub locked: bool,
    /// When false, the gizmo is enabled for this node, but the node didn't run
    /// during the following frame, so the gizmo won't be shown.
    pub visible: bool,
}

/// Stores the gizmo state for each node in the graph. This structure references
/// node ids from the UI graph, so it requires regular bookkeeping (make sure
/// deleted graph nodes have their gizmo data deleted, and so on).
///
/// This struct uses shared ownership + interior mutability so it can be owned
/// by multiple places in the UI simultaneously, since both the graph and the
/// viewport need to access it regularly.
pub struct UiNodeGizmoStates {
    inner: Rc<RefCell<UiNodeGizmoStatesInner>>,
}

#[derive(Default)]
struct UiNodeGizmoStatesInner {
    gizmos: SecondaryMap<NodeId, UiGizmoState>,
    /// The gizmo that was last interacted with. This is the one that receives
    /// input shortcuts and shows buttons on the screen.
    current_focus: Option<(NodeId, usize)>,
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
    node: &Node<NodeData>,
    has_focus: bool,
) -> Result<Vec<GizmoViewportResponse>> {
    let mut responses = Vec::new();

    let gizmo_label = |ui: &mut egui::Ui| {
        use slotmap::Key;
        ui.label(format!("{} ({:?})", node.label, node.id.data()));
    };

    match gizmo {
        BlackjackGizmo::Transform(transform_gizmo) => {
            if has_focus {
                ui.allocate_ui_at_rect(viewport.viewport_rect().shrink(10.0), |ui| {
                    gizmo_label(ui);
                    if transform_gizmo.translation_enabled
                        && (ui.button("Move (G)").clicked() || ui.input().key_pressed(egui::Key::G))
                    {
                        transform_gizmo.gizmo_mode = TransformGizmoMode::Translate;
                    }
                    if transform_gizmo.rotation_enabled
                        && (ui.button("Rotate (R)").clicked()
                            || ui.input().key_pressed(egui::Key::R))
                    {
                        transform_gizmo.gizmo_mode = TransformGizmoMode::Rotate;
                    }
                    if transform_gizmo.scale_enabled
                        && (ui.button("Scale (S)").clicked()
                            || ui.input().key_pressed(egui::Key::S))
                    {
                        transform_gizmo.gizmo_mode = TransformGizmoMode::Scale;
                    }
                });
            }

            let mut visuals = GizmoVisuals::default();
            visuals.gizmo_size *= 0.8;
            if !has_focus {
                visuals.gizmo_size *= 0.8;
                visuals.stroke_width *= 0.8;
                visuals.inactive_alpha *= 0.6;
                visuals.highlight_alpha *= 0.6;
            } else {
                visuals.inactive_alpha *= 1.2;
                visuals.highlight_alpha *= 1.2;
            }

            let gizmo = egui_gizmo::Gizmo::new(unique_id)
                .view_matrix(viewport.view_matrix().to_cols_array_2d())
                .projection_matrix(viewport.projection_matrix().to_cols_array_2d())
                .model_matrix(transform_gizmo.matrix().to_cols_array_2d())
                .viewport(viewport.viewport_rect())
                .visuals(visuals)
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
        BlackjackGizmo::None => {}
    }

    Ok(responses)
}

impl UiNodeGizmoStates {
    pub fn init() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn share(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }

    /// Returns a map suitable to be sent to blackjack_engine's run_node function
    pub fn to_bjk_data(&self, mapping: &NodeMapping) -> SecondaryMap<BjkNodeId, GizmoState> {
        let mut result = SecondaryMap::new();
        let inner = &self.inner.borrow();
        for (node_id, gizmo_ui) in &inner.gizmos {
            result.insert(mapping[node_id], gizmo_ui.gizmo_state.clone());
        }
        result
    }

    /// Executes the provided fallible function for each of the active and
    /// visible gizmos for every node in the graph.
    ///
    /// The provided callback `f` must return a boolean indicating whether the
    /// specific gizmo was interacted with during the frame.
    pub fn iterate_gizmos_for_drawing(
        &mut self,
        mut f: impl FnMut(NodeId, usize, &mut BlackjackGizmo, bool) -> Result<bool>,
    ) -> Result<()> {
        let mut inner = self.inner.borrow_mut();
        let mut current_focus = inner.current_focus;
        for (node_id, ui_state) in &mut inner.gizmos {
            // Reset flag for this frame. Set if any of the gizmos for this node is interacted.
            ui_state.gizmo_state.gizmos_changed = false;

            for (idx, g) in ui_state
                .gizmo_state
                .active_gizmos
                .iter_mut()
                .flatten()
                .enumerate()
            {
                // Skip running for invisible gizmos
                if ui_state.visible {
                    let has_focus = current_focus.is_some_and_(|x| x.0 == node_id && x.1 == idx);
                    let interacted = f(node_id, idx, g, has_focus)?;
                    ui_state.gizmo_state.gizmos_changed |= interacted;
                    if interacted {
                        current_focus = Some((node_id, idx));
                    }
                }
            }
        }
        inner.current_focus = current_focus;
        Ok(())
    }

    /// Updates the gizmo values after they've been modified by the engine in a call to run_node.
    pub fn update_gizmos(
        &mut self,
        mut updated_gizmos: SecondaryMap<BjkNodeId, Vec<BlackjackGizmo>>,
        mapping: &NodeMapping,
    ) -> Result<()> {
        let mut inner = self.inner.borrow_mut();

        // Reset the visible flag for all the nodes. We set it back if we had an
        // update for this node during the update.
        for (_, ui_state) in &mut inner.gizmos {
            ui_state.visible = false;
        }

        for (node_id, ui_state) in &mut inner.gizmos {
            if let Some(updated) = updated_gizmos.remove(mapping[node_id]) {
                ui_state.gizmo_state.active_gizmos = Some(updated);
                ui_state.visible = true;
            }
        }
        Ok(())
    }

    pub fn node_left_active(&self, n: NodeId) {
        let mut inner = self.inner.borrow_mut();
        if let Some(UiGizmoState { locked: false, .. }) = inner.gizmos.get(n) {
            inner.gizmos.remove(n);
        }
        if inner.current_focus.is_some_and_(|x| x.0 == n) {
            inner.current_focus = None;
        }
    }

    pub fn node_is_active(&self, n: NodeId) {
        let mut inner = self.inner.borrow_mut();
        if inner.gizmos.get(n).is_none() {
            inner.gizmos.insert(
                n,
                UiGizmoState {
                    gizmo_state: GizmoState::default(),
                    locked: false,
                    visible: true,
                },
            );
        }
        // We use 0 here, because it's a good default, since many nodes show a
        // single gizmo. Even if the node returned a 0-length gizmo list, that
        // won't cause an OOB access. The focus would behave as if it was None
        // in that case.
        inner.current_focus = Some((n, 0));
    }

    pub fn lock_gizmos_for(&self, n: NodeId) {
        let mut inner = self.inner.borrow_mut();
        match inner.gizmos.get_mut(n) {
            Some(ui_state) => {
                ui_state.locked = true;
            }
            None => {
                inner.gizmos.insert(
                    n,
                    UiGizmoState {
                        gizmo_state: GizmoState::default(),
                        locked: true,
                        visible: true,
                    },
                );
            }
        }
        inner.current_focus = Some((n, 0));
    }

    pub fn unlock_gizmos_for(&self, n: NodeId, active: Option<NodeId>) {
        let mut inner = self.inner.borrow_mut();
        if active.is_none_or_(|a| *a != n) {
            inner.gizmos.remove(n);
            if inner.current_focus.is_some_and_(|x| x.0 == n) {
                inner.current_focus = None;
            }
        } else if let Some(ui_state) = inner.gizmos.get_mut(n) {
            ui_state.locked = false;
        }
    }

    pub fn reset_for_hot_reload(&self) {
        let mut inner = self.inner.borrow_mut();
        for (_, gizmo) in &mut inner.gizmos {
            gizmo.gizmo_state = GizmoState::default();
        }
        inner.current_focus = None;
    }

    pub fn is_node_locked(&self, n: NodeId) -> bool {
        self.inner.borrow().gizmos.get(n).is_some_and_(|x| x.locked)
    }

    pub fn node_deleted(&mut self, node_id: NodeId) {
        let mut inner = self.inner.borrow_mut();
        inner.gizmos.remove(node_id);
        if inner.current_focus.is_some_and_(|x| x.0 == node_id) {
            inner.current_focus = None;
        }
    }

    pub fn get_all_locked_nodes(&self) -> Vec<NodeId> {
        let mut locked = vec![];
        for (node_id, ui_state) in &self.inner.borrow().gizmos {
            if ui_state.locked {
                locked.push(node_id);
            }
        }
        locked
    }

    pub fn restore_locked_nodes(&self, locked_nodes: impl Iterator<Item = NodeId>) {
        for locked in locked_nodes {
            self.lock_gizmos_for(locked);
        }
    }
}
