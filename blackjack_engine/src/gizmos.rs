use mlua::{FromLua, Lua, ToLua};

use crate::prelude::*;

/// A gizmo representing a 3d transformation, allowing to translate, rotate and
/// scale the manipulated object.
#[derive(Debug, Copy, Clone)]
pub struct TransformGizmo {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

#[blackjack_macros::blackjack_lua_module]
mod tr_gizmo {
    use crate::lua_engine::lua_stdlib::LVec3;

    use super::*;
    use glam::{EulerRot, Vec3};

    /// Constructs a new transform gizmo. Rotation is given as XYZ euler angles.
    #[lua(under = "TransformGizmo")]
    fn new(translation: LVec3, rotation: LVec3, scale: LVec3) -> TransformGizmo {
        let (rx, ry, rz) = rotation.0.into();
        TransformGizmo {
            translation: translation.0,
            rotation: Quat::from_euler(EulerRot::XYZ, rx, ry, rz),
            scale: scale.0,
        }
    }

    /// Constructs a new transform gizmo representing the identity transform.
    #[lua(under = "TransformGizmo")]
    fn default() -> TransformGizmo {
        TransformGizmo {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    #[lua_impl]
    impl TransformGizmo {
        /// Returns the translation of this transform gizmo
        #[lua(map = "LVec3(x)")]
        pub fn translation(&self) -> Vec3 {
            self.translation
        }

        /// Sets the translation component of this transform gizmo
        #[lua]
        pub fn set_translation(&mut self, tr: LVec3) {
            self.translation = tr.0;
        }

        /// Returns the rotation of this transform gizmo, as XYZ euler angles
        #[lua(map = "LVec3(x)")]
        pub fn rotation(&self) -> Vec3 {
            self.rotation.normalize().to_euler(EulerRot::XYZ).into()
        }

        /// Sets the rotation component of this transform gizmo, from XYZ euler angles
        #[lua]
        pub fn set_rotation(&mut self, rot: LVec3) {
            let (rx, ry, rz) = rot.0.into();
            self.rotation = Quat::from_euler(EulerRot::XYZ, rx, ry, rz);
        }

        /// Returns the scale of this transform gizmo
        #[lua(map = "LVec3(x)")]
        pub fn scale(&self) -> Vec3 {
            self.scale
        }

        /// Sets the scale component of this transform gizmo
        #[lua]
        pub fn set_scale(&mut self, tr: LVec3) {
            self.scale = tr.0;
        }

        /// Returns a transform matrix for this gizmo
        pub fn matrix(&self) -> Mat4 {
            Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
        }

        /// Sets this gizmo's parameters from an affine transform matrix.
        pub fn set_from_matrix(&mut self, m: Mat4) {
            let (s, r, t) = m.to_scale_rotation_translation();
            self.scale = s;
            self.translation = t;
            self.rotation = r;
        }
    }
}

#[derive(Clone, Debug)]
pub enum BlackjackGizmo {
    Transform(TransformGizmo),
}

/// Boilerplate: Implement FromLua by attempting downcast of each UserData type
impl<'lua> FromLua<'lua> for BlackjackGizmo {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua Lua) -> mlua::Result<Self> {
        macro_rules! gizmo_type {
            ($x:ident, $t:ident, $w:ident) => {
                if $x.is::<$t>() {
                    return Ok(BlackjackGizmo::$w($x.borrow::<$t>()?.clone()));
                }
            };
        }

        if let mlua::Value::UserData(x) = lua_value {
            // NOTE: Add more cases here:
            gizmo_type!(x, TransformGizmo, Transform);
        }
        mlua::Result::Err(mlua::Error::FromLuaConversionError {
            from: "Value",
            to: "BlackjackGizmo",
            message: Some("Invalid data for blackjack gizmo.".into()),
        })
    }
}

/// Boilerplate: Implement ToLua by deferring to each variant's ToLua
/// implementation
impl<'lua> ToLua<'lua> for BlackjackGizmo {
    fn to_lua(self, lua: &'lua Lua) -> mlua::Result<mlua::Value<'lua>> {
        match self {
            BlackjackGizmo::Transform(t) => t.to_lua(lua),
        }
    }
}
