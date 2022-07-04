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
