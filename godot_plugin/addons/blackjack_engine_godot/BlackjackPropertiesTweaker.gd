# Copyright (C) 2023 setzer22 and contributors
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

tool
extends PanelContainer

signal property_changed(prop_addr, new_val)

onready var properties_vbox = find_node("PropertiesVBox")
onready var error_label = find_node("ErrorLabel")

var properties = []
var property_controls = []
    
func on_property_changed(new_val, prop_addr):
    emit_signal("property_changed", prop_addr, new_val)

# Called from the outside, when a property changes (because it's being set from
# the inspector, for instance).
func set_value_externally(prop_addr, new_val):

    var idx = 0
    for prop in properties:
        if prop.addr == prop_addr:
            break
        idx = idx + 1

    
    if idx < len(property_controls):
        var control = property_controls[idx]
        control.set_value_externally(new_val)

func set_error(err_str):
    error_label.text = err_str

func clear_error():
    error_label.text = ""

func initialize(properties_: Array):
    self.properties = properties_
    
func _ready():
    error_label.text = ""
    for prop in properties:
        var control
        match prop.typ:
            "Scalar":
                control = preload("ScalarProp.tscn").instance()
                control.init(prop.label, prop.val, prop.min, prop.max)
            "String":
                control = preload("StringProp.tscn").instance()
                control.init(prop.label, prop.val)
            "Vector":
                control = preload("VectorProp.tscn").instance()
                control.init(prop.label, prop.val)
            "Selection":
                control = preload("SelectionProp.tscn").instance()
                control.init(prop.label, prop.val)
        control.connect("on_changed", self, "on_property_changed", [prop.addr])
        property_controls.push_back(control)

        properties_vbox.add_child(control)
        
