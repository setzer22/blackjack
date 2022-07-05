tool
extends HBoxContainer

signal on_changed(value)

var stored_value: Vector3

func init(label: String, value: Vector3):
    $Label.text = label
    stored_value = value
    $X.text = str(value.x)
    $Y.text = str(value.y)
    $Z.text = str(value.z)

func emit_stored_value():
    emit_signal("on_changed", stored_value)

func _on_X_text_changed(new_text):
    stored_value.x = float(new_text)
    emit_stored_value()

func _on_Y_text_changed(new_text):
    stored_value.y = float(new_text)
    emit_stored_value()

func _on_Z_text_changed(new_text):
    stored_value.z = float(new_text)
    emit_stored_value()

func set_value_externally(val):
    stored_value = val
    $X.text = val.x
    $Y.text = val.y
    $Z.text = val.z
