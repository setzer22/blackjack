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
