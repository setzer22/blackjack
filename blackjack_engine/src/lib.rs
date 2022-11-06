// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Used by proc macros to refer to this crate unambiguously
extern crate self as blackjack_engine;

/// Some useful re-exports
pub mod prelude;

/// The halfedge graph data structure and main edit operations
pub mod mesh;

/// The blackjack engine core types
pub mod lua_engine;

/// The graph core datatypes
pub mod graph;

/// High level interpreter of blackjack graphs.
pub mod graph_interpreter;

/// Gizmos allow visual modifications of a node's parameters.
pub mod gizmos;
