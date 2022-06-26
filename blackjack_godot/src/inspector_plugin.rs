use gdnative::api as gd;
use gdnative::prelude::*;

use crate::BlackjackAsset;

#[derive(NativeClass)]
#[inherit(gd::EditorInspectorPlugin)]
pub struct BlackjackInspectorPlugin {
    current: Option<Instance<BlackjackAsset>>,
}

#[methods]
impl BlackjackInspectorPlugin {
    fn new(_owner: &gd::EditorInspectorPlugin) -> Self {
        Self { current: None }
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
    fn parse_begin(&mut self, _owner: &gd::EditorInspectorPlugin, object: Variant) {
        if let Some(inst) = unsafe {
            object
                .to_object::<Node>()
                .and_then(|obj| obj.assume_safe().cast_instance::<BlackjackAsset>())
        } {
            self.current = Some(inst.claim());
        } else {
            godot_error!(
                "Unexpected error. Can handle returned true but in parse start,\
                the object was not of the expected type."
            )
        }
    }

    #[export]
    fn parse_end(&self, owner: &gd::EditorInspectorPlugin) {
        if let Some(current) = self.current.as_ref() {
            let current = unsafe { current.assume_safe() };
            current
                .map(|asset, asset_owner| {
                    let gui = asset.generate_params_gui(asset_owner);
                    owner.add_custom_control(gui);
                })
                .unwrap_or_else(|err| {
                    godot_error!("FATAL: Could not map asset because: {err}")
                })
        } else {
            godot_error!("No current node, but parse_end was called.")
        }
    }
}
