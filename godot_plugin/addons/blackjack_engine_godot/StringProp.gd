# Copyright (C) 2023 setzer22 and contributors
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

tool
extends VBoxContainer

signal on_changed(value)

func init(label: String, value: String):
    $Label.text = label
    $TextEdit.text = value

func _on_TextEdit_text_changed():
    emit_signal("on_changed", $TextEdit.text)

func set_value_externally(val):
    $TextEdit.text = val
