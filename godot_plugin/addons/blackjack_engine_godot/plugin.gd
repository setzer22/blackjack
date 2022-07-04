tool
extends EditorPlugin

var inspector_plugin
var import_plugin

func _enter_tree():
    if !ProjectSettings.has_setting("Blackjack/library_path"):
        ProjectSettings.set_setting("Blackjack/library_path", "res://node_libraries")
        
    add_custom_type(
        "BlackjackAsset",
        "Spatial",
        preload("BlackjackAsset.gd"),
        preload("icon.png")
    )
    
    inspector_plugin = preload("BlackjackInspectorPlugin.gd").new()
    add_inspector_plugin(inspector_plugin)
    
    import_plugin = preload("BgaImportPlugin.gd").new()
    add_import_plugin(import_plugin)

func _exit_tree():
    remove_inspector_plugin(inspector_plugin)
    remove_import_plugin(import_plugin)
    remove_custom_type("BlackjackAsset")
