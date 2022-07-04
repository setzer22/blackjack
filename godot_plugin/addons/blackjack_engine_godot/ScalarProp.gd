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
