extends EditorInspectorPlugin

func can_handle(object):
    return object.get_class() == "BlackjackAsset"

func parse_begin(object):
    var gui = object.make_tweaker_gui()
    if gui != null:
        add_custom_control(gui)
