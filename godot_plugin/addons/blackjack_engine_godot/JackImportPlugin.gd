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

extends EditorImportPlugin

enum Presets { DEFAULT }

func get_importer_name():
    return "blackjack.bjk"

func get_visible_name():
    return "Blackjack Jack Asset"

func get_recognized_extensions():
    return ["bjk"]

func get_save_extension():
    return "res"
    
func get_resource_type():
    return "Resource"
    
func get_preset_count():
    return Presets.size()

func get_preset_name(preset):
    match preset:
        Presets.DEFAULT:
            return "Default"
        _:
            return "Unknown"
            
func get_import_options(preset):
    match preset:
        Presets.DEFAULT:
            return []
        _:
            return []

func get_option_visibility(option, options):
    return true

func import(source_file, save_path, options, platform_variants, gen_files):
    var file = File.new()
    var err = file.open(source_file, File.READ)
    if err != OK:
        return err

    var resource = BgaFileResource.new()
    resource.contents = file.get_as_text()
    
    file.close()
    
    return ResourceSaver.save("%s.%s" % [save_path, get_save_extension()], resource)

    
