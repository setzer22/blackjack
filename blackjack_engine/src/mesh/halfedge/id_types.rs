// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

slotmap::new_key_type! { pub struct HalfEdgeId; }
slotmap::new_key_type! { pub struct VertexId; }
slotmap::new_key_type! { pub struct FaceId; }

#[blackjack_macros::blackjack_lua_module]
mod ids_to_string {
    use super::*;

    /// Returns a string representation for this vertex id. This is only useful
    /// for debug purposes.
    #[lua(under = "Debug")]
    pub fn vertex_id_to_string(v: VertexId) -> String {
        format!("{v:?}")
    }

    /// Returns a string representation for this halfedge id. This is only useful
    /// for debug purposes.
    #[lua(under = "Debug")]
    pub fn halfedge_id_to_string(v: HalfEdgeId) -> String {
        format!("{v:?}")
    }

    /// Returns a string representation for this face id. This is only useful
    /// for debug purposes.
    #[lua(under = "Debug")]
    pub fn face_id_to_string(v: FaceId) -> String {
        format!("{v:?}")
    }
}
