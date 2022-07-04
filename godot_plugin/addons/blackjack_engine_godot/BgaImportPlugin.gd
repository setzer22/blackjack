extends EditorImportPlugin

enum Presets { DEFAULT }

func get_importer_name():
    return "blackjack.gameasset"

func get_visible_name():
    return "Blackjack Game Asset"

func get_recognized_extensions():
    return ["bga"]

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

    
