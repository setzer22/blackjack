tool
extends Control

func _on_error(err):
    $Label.text = err

func _on_clear_error():
    $Label.text = ""
