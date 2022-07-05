# Copyright (C) 2022 setzer22 and contributors
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

tool
extends HBoxContainer

signal on_changed(value)

func init(label: String, value, min_val, max_val):
    $Label.text = label
    $HSlider.min_value = min_val
    $HSlider.max_value = max_val
    $HSlider.step = (max_val - min_val) / 100.0
    $HSlider.value = value
    $Label2.text = str(value)

func _on_HSlider_value_changed(value):
    $Label2.text = str(value)
    emit_signal("on_changed", value)

func set_value_externally(val):
    $HSlider.value = val
    $Label2.text = str(val)
