tool
extends Spatial

signal error_occurred(err_str)
signal clear_error(err_str)

var BlackjackApi = preload("res://addons/blackjack_engine_godot/BlackjackApi.gdns").new()
var BlackjackPropertiesTweaker = preload("res://addons/blackjack_engine_godot/BlackjackPropertiesTweaker.tscn")

# Exported vars, except we can't export because of
# https://github.com/godotengine/godot/issues/5988
#
# These get saved with the scene
var asset_resource : Resource setget set_asset,get_asset
var show_gui : bool = false setget set_show_gui
var overriden_params : Dictionary = {}
var materials : Array = []

# Non exported vars
var asset_id = null
var needs_update = false
var child_mesh
var asset_params
var runtime_child_gui = null

onready var is_ready = false

func set_asset(new_asset):
    if asset_resource != null and new_asset != null:
        # We're replacing this asset with another one. Clear any overriden params to avoid conflicts
        overriden_params = {}

    asset_resource = new_asset
    if is_ready and asset_resource != null:
        on_reload_asset_resource()

func get_asset():
    return asset_resource

func set_show_gui(new_show_gui):
    show_gui = new_show_gui
    if not Engine.editor_hint:
        if new_show_gui != null and new_show_gui == true:
            if new_show_gui and asset_resource != null and asset_id != null and asset_params != null:
                if runtime_child_gui != null:
                    remove_child(runtime_child_gui)
                runtime_child_gui = make_tweaker_gui()
                sync_gui_to_props(runtime_child_gui)
                add_child(runtime_child_gui)
        else:
            if runtime_child_gui != null:
                remove_child(runtime_child_gui)
    

func on_reload_asset_resource():
    if asset_resource != null and asset_resource.get("contents") != null:
        # Set the asset
        var err = BlackjackApi.set_asset(asset_id, asset_resource)
        asset_params = BlackjackApi.get_params(asset_id)
        needs_update = true
        for k in overriden_params.keys():
            on_property_changed(k, overriden_params[k])
        set_show_gui(show_gui) # Force reload of gui state.

    # Make sure the editor gets the changes and redraws the inspector
    property_list_changed_notify()

func make_tweaker_gui():
    if asset_params != null:
        var gui = BlackjackPropertiesTweaker.instance() 
        gui.initialize(asset_params)
        gui.connect("property_changed", self, "on_property_changed")
        self.connect("error_occurred", gui, "set_error")
        self.connect("clear_error", gui, "clear_error")
        sync_gui_to_props(gui)
        return gui

func sync_gui_to_props(gui):
    for prop_addr in overriden_params.keys():
        # Need to call this deferred, because the GUI might not have setup itsp child Controls yet.
        gui.call_deferred("set_value_externally", prop_addr, overriden_params[prop_addr])

func on_property_changed(prop_addr, new_val):
    if asset_id != null:
        overriden_params[prop_addr] = new_val
        BlackjackApi.set_param(asset_id, prop_addr, new_val)
        needs_update = true

func _ready():
    if Engine.editor_hint:
        start()
    else:
        call_deferred("start")
    
func start():
    is_ready = true
    asset_id = BlackjackApi.make_asset()
    on_reload_asset_resource()
    child_mesh = MeshInstance.new()
    add_child(child_mesh)

func _process(delta):
    if needs_update:
        needs_update = false
        var results = BlackjackApi.update_asset(asset_id, materials)
        if results != null and results.has("Ok"):
            child_mesh.mesh = results.Ok
            emit_signal("clear_error")
        elif results != null and results.has("Err"):
            emit_signal("error_occurred", str(results.Err))
        else:
            push_error("Blackjack encountered an unexpected error")
            emit_signal("error_occurred", "Blackjack encountered an unexpected error")

func is_class(other): return other == "BlackjackAsset" or .is_class(other)
func get_class(): return "BlackjackAsset"

func _get_property_list():
    var properties = [
        {
            name = "asset_resource",
            type = TYPE_OBJECT
        },
        {
            name = "show_gui",
            type = TYPE_BOOL,
        },
        {
            name = "overriden_params",
            type = TYPE_DICTIONARY,
            usage = PROPERTY_USAGE_STORAGE, # Do not show on inspector
        },
    ]

    # Add one more property than we have materials. This allows setting the next material by drag & drop
    for i in range(0, len(materials) + 1):
        properties.push_back({
            name = "material_slot_" + str(i),
            type = TYPE_OBJECT,
            hint = PROPERTY_HINT_RESOURCE_TYPE,
            hint_string = "Material",
        })

    return properties

func _set(property, value):
    if property.begins_with("material_slot_"):
        var idx = int(property.trim_prefix("material_slot_"))
        while len(materials) <= idx:
            materials.push_back(null)
        materials[idx] = value
        property_list_changed_notify()

        for i in range(0, len(materials)):
            if materials[len(materials) - 1] == null:
                materials.pop_back()
            else:
                break

        needs_update = true
    else:
        ._set(property, value)

func _get(property):
    if property.begins_with("material_slot_"):
        var idx = int(property.trim_prefix("material_slot_"))
        if len(materials) > idx:
            return materials[idx]
        else:
            return null
    else:
        return ._get(property)
