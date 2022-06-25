use gdnative::api as gd;
use gdnative::prelude::*;

use crate::BlackjackAsset;

#[derive(NativeClass)]
#[inherit(gd::EditorInspectorPlugin)]
pub struct BlackjackInspectorPlugin {}

#[methods]
impl BlackjackInspectorPlugin {
    fn new(_owner: &gd::EditorInspectorPlugin) -> Self {
        Self {}
    }

    #[export]
    fn can_handle(&self, _owner: &gd::EditorInspectorPlugin, object: Variant) -> bool {
        (|| unsafe {
            object
                .to_object::<Node>()?
                .assume_safe()
                .cast_instance::<BlackjackAsset>()
        })()
        .is_some()
    }

    #[export]
    fn parse_end(&self, owner: &gd::EditorInspectorPlugin) {
        let button = Button::new();
        button.set_text("Bananas");
        owner.add_custom_control(button);
    }

    #[export]
    #[allow(clippy::too_many_arguments)]
    fn parse_property(
        &self,
        _owner: &gd::EditorInspectorPlugin,
        object: Variant,
        typ: i64,
        path: String,
        hint: i64,
        hint_text: String,
        usage: i64,
    ) {
        godot_print!("{object} {typ} {path} {hint} {hint_text} {usage}")
    }
}
