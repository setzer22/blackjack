# Copyright (C) 2023 setzer22 and contributors
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Copyright (C) 2022 setzer22 and contributors
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

tool
extends HBoxContainer

signal on_changed(value)

func init(label: String, value, min_val, max_val):
    var use_slider = min_val != null && max_val != null
    $HSlider.visible = use_slider
    $LineEdit.visible = !use_slider

    $Label.text = label
    $Label2.text = str(value)

    if use_slider:
        $HSlider.min_value = min_val
        $HSlider.max_value = max_val
        $HSlider.step = (max_val - min_val) / 100.0
        $HSlider.value = value
    else:
        $LineEdit.value = str(value)

func _on_HSlider_value_changed(value):
    $Label2.text = str(value)
    emit_signal("on_changed", value)

func set_value_externally(val):
    $HSlider.value = val
    $LineEdit.text = str(val)
    $Label2.text = str(val)

func _on_LineEdit_text_changed(new_text):
    if new_text.is_valid_float():
        var new_val = float(new_text)
        $Label2.text = new_text
        emit_signal("on_changed", new_val)
