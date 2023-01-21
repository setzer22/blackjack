// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use mlua::{FromLua, Lua, ToLua};

use crate::prelude::*;

#[derive(Debug, Copy, Clone)]
pub enum TransformGizmoMode {
    Translate,
    Rotate,
    Scale,
}

/// A gizmo representing a 3d transformation, allowing to translate, rotate and
/// scale the manipulated object.
#[derive(Debug, Copy, Clone)]
pub struct TransformGizmo {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,

    pub pre_translation: Vec3,
    pub pre_rotation: Quat,
    pub pre_scale: Vec3,

    pub translation_enabled: bool,
    pub rotation_enabled: bool,
    pub scale_enabled: bool,

    pub gizmo_mode: TransformGizmoMode,
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
            pre_translation: Vec3::ZERO,
            pre_rotation: Quat::IDENTITY,
            pre_scale: Vec3::ONE,
            gizmo_mode: TransformGizmoMode::Translate,
            translation_enabled: true,
            rotation_enabled: true,
            scale_enabled: true,
        }
    }

    /// Constructs a new transform gizmo representing the identity transform.
    #[lua(under = "TransformGizmo")]
    fn default() -> TransformGizmo {
        TransformGizmo {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            pre_translation: Vec3::ZERO,
            pre_rotation: Quat::IDENTITY,
            pre_scale: Vec3::ONE,
            gizmo_mode: TransformGizmoMode::Translate,
            translation_enabled: true,
            rotation_enabled: true,
            scale_enabled: true,
        }
    }

    #[lua_impl]
    impl TransformGizmo {
        /// Returns the translation of this transform gizmo
        #[lua(map = "LVec3(x)")]
        pub fn translation(&self) -> Vec3 {
            self.translation
        }

        /// Returns the pre-translation of this transform gizmo
        #[lua(map = "LVec3(x)")]
        pub fn pre_translation(&self) -> Vec3 {
            self.pre_translation
        }

        /// Sets the translation component of this transform gizmo
        #[lua]
        pub fn set_translation(&mut self, tr: LVec3) {
            self.translation = tr.0;
        }

        /// Sets the pre-translation component of this transform gizmo
        #[lua]
        pub fn set_pre_translation(&mut self, tr: LVec3) {
            self.pre_translation = tr.0;
        }

        /// Returns the rotation of this transform gizmo, as XYZ euler angles
        #[lua(map = "LVec3(x)")]
        pub fn rotation(&self) -> Vec3 {
            self.rotation.normalize().to_euler(EulerRot::XYZ).into()
        }

        /// Returns the pre_rotation of this transform gizmo, as XYZ euler angles
        #[lua(map = "LVec3(x)")]
        pub fn pre_rotation(&self) -> Vec3 {
            self.pre_rotation.normalize().to_euler(EulerRot::XYZ).into()
        }

        /// Sets the rotation component of this transform gizmo, from XYZ euler angles
        #[lua]
        pub fn set_rotation(&mut self, rot: LVec3) {
            let (rx, ry, rz) = rot.0.into();
            self.rotation = Quat::from_euler(EulerRot::XYZ, rx, ry, rz);
        }

        /// Sets the rotation component of this transform gizmo, from XYZ euler angles
        #[lua]
        pub fn set_pre_rotation(&mut self, rot: LVec3) {
            let (rx, ry, rz) = rot.0.into();
            self.pre_rotation = Quat::from_euler(EulerRot::XYZ, rx, ry, rz);
        }

        /// Returns the scale of this transform gizmo
        #[lua(map = "LVec3(x)")]
        pub fn scale(&self) -> Vec3 {
            self.scale
        }

        /// Returns the scale of this transform gizmo
        #[lua(map = "LVec3(x)")]
        pub fn pre_scale(&self) -> Vec3 {
            self.pre_scale
        }

        /// Sets the scale component of this transform gizmo
        #[lua]
        pub fn set_scale(&mut self, tr: LVec3) {
            self.scale = tr.0;
        }

        /// Sets the pre-scale component of this transform gizmo
        #[lua]
        pub fn set_pre_scale(&mut self, tr: LVec3) {
            self.pre_scale = tr.0;
        }

        /// Enables or disables the translation portion of the gizmo
        #[lua]
        pub fn set_enable_translation(&mut self, locked: bool) {
            self.translation_enabled = locked;
        }

        /// Enables or disables the rotation portion of the gizmo
        #[lua]
        pub fn set_enable_rotation(&mut self, locked: bool) {
            self.rotation_enabled = locked;
        }

        /// Enables or disables the scale portion of the gizmo
        #[lua]
        pub fn set_enable_scale(&mut self, locked: bool) {
            self.scale_enabled = locked;
        }

        /// Returns the full transform matrix for this gizmo, combining the
        /// transform and pre-transform matrices.
        pub fn matrix(&self) -> Mat4 {
            Mat4::from_scale_rotation_translation(
                self.pre_scale * self.scale,
                self.pre_rotation * self.rotation,
                self.pre_translation + self.translation,
            )
        }

        /// Sets this gizmo's parameters from an affine transform matrix. The
        /// given matrix should be an updated version of the one obtained via
        /// `Self::matrix`. This operation will set the transform after undoing
        /// any existing pre-transform.
        pub fn set_from_matrix(&mut self, m: Mat4) {
            let (s, r, t) = m.to_scale_rotation_translation();
            self.scale = s / self.pre_scale;
            self.translation = t - self.pre_translation;
            self.rotation = r * self.pre_rotation.inverse();
        }
    }
}

#[derive(Clone, Debug)]
pub enum BlackjackGizmo {
    Transform(TransformGizmo),
    // This special value is sometimes returned by the UI to indicate a gizmo
    // wasn't initialized. No gizmo should be rendered for this value.
    None,
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
            // The special gizmo value "None" is encoded as nil. Lua functions
            // know that the nil value represents an uninitialized gizmo.
            BlackjackGizmo::None => Ok(mlua::Value::Nil),
        }
    }
}
