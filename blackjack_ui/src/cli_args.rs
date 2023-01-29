// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use clap::Parser;
use once_cell::sync::Lazy;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Loads the given `.bjk` file
    pub load: Option<String>,

    /// Export mock Lua code annotated with ldoc comments for the blackjack API
    /// at the given folder.
    #[arg(long)]
    pub generate_ldoc: Option<String>,

    /// If this argument is present, the Lua file watcher will not be started
    /// and the Lua code will be loaded once at startup.
    #[arg(long)]
    pub disable_lua_watcher: bool,
}

/// CLI args are stored in a lazy static variable so they're accessible from
/// everywhere. Arguments are parsed on first access.
pub static CLI_ARGS: Lazy<Args> = Lazy::new(Args::parse);
