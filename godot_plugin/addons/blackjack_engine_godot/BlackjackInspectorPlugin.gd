# Copyright (C) 2023 setzer22 and contributors
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

extends EditorInspectorPlugin

func can_handle(object):
    return object.get_class() == "BlackjackJack"

func parse_begin(object):
    var gui = object.make_tweaker_gui()
    if gui != null:
        add_custom_control(gui)
