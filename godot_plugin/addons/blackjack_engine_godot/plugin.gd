# Copyright (C) 2022 setzer22 and contributors
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

tool
extends EditorPlugin

var inspector_plugin
var import_plugin

func _enter_tree():
    if !ProjectSettings.has_setting("Blackjack/library_path"):
        ProjectSettings.set_setting("Blackjack/library_path", "res://blackjack_lua/run")
        
    add_custom_type(
        "BlackjackJack",
        "Spatial",
        preload("BlackjackJack.gd"),
        preload("icon.png")
    )
    
    inspector_plugin = preload("BlackjackInspectorPlugin.gd").new()
    add_inspector_plugin(inspector_plugin)
    
    import_plugin = preload("JackImportPlugin.gd").new()
    add_import_plugin(import_plugin)

func _exit_tree():
    remove_inspector_plugin(inspector_plugin)
    remove_import_plugin(import_plugin)
    remove_custom_type("BlackjackJack")
