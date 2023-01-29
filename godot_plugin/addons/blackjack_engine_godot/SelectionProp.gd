# Copyright (C) 2023 setzer22 and contributors
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

tool
extends HBoxContainer

signal on_changed(value)

func init(label: String, value: String):
    $Label.text = label
    $LineEdit3.text = value

func _on_LineEdit3_text_changed(new_text):
    emit_signal("on_changed", new_text)

func set_value_extenrally(val):
    $LineEdit3.text = val
