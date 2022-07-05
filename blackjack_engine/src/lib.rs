// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Some useful re-exports
pub mod prelude;

/// The halfedge graph data structure and main edit operations
pub mod mesh;

/// The blackjack engine core types
pub mod lua_engine;

/// The graph core datatypes
pub mod graph;

/// Converts graphs into Lua programs
pub mod graph_compiler;
