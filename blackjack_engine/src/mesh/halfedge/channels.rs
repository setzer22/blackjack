// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{any::Any, collections::BTreeMap, fmt::Debug, marker::PhantomData, ops::Deref, rc::Rc};

use crate::{
    lua_engine::lua_stdlib,
    sync::{BorrowedRef, InteriorMutable, MaybeSync, MutableRef, RefCounted},
};
use glam::Vec3;
use mlua::{FromLua, Lua, ToLua};

use super::*;

/// The key of a channel is the type of element the channel is attaching data
/// to. It can be Vertices, HalfEdges or Faces, and the `ChannelKey` is the
/// corresponding id type.
pub trait ChannelKey:
    slotmap::Key + Default + Debug + Clone + Copy + Sized + FromToLua + MaybeSync + 'static
{
    fn key_type() -> ChannelKeyType;
    fn name() -> &'static str;
    fn cast_from_ffi(x: u64) -> Self;
}
macro_rules! impl_channel_key {
    () => {};
    ($t:ident) => {
        impl ChannelKey for $t {
            fn key_type() -> ChannelKeyType {
                ChannelKeyType::$t
            }
            fn name() -> &'static str {
                stringify!($t)
            }
            fn cast_from_ffi(x: u64) -> Self {
                Self::from(slotmap::KeyData::from_ffi(x))
            }
        }
        impl MaybeSync for $t {}
    };
}
impl_channel_key!(VertexId);
impl_channel_key!(FaceId);
impl_channel_key!(HalfEdgeId);

/// The geometry spreadsheet relies on this trait to display channel values.
pub trait Introspect {
    fn introspect(&self) -> String;
}

impl Introspect for Vec3 {
    fn introspect(&self) -> String {
        format!("{: >6.3} {: >6.3} {: >6.3}", self.x, self.y, self.z)
    }
}

impl Introspect for f32 {
    fn introspect(&self) -> String {
        format!("{self: >6.3}")
    }
}

impl Introspect for bool {
    fn introspect(&self) -> String {
        format!("{self: >6.3}")
    }
}

/// The value of a channel is the data that is associated to a specific key.
/// Values can be scalars (f32) or vectors (Vec3).
pub trait ChannelValue:
    Default + Debug + Clone + Copy + Sized + FromToLua + Introspect + MaybeSync + 'static
{
    fn value_type() -> ChannelValueType;
    fn name() -> &'static str;
}
macro_rules! impl_channel_value {
    () => {};
    ($t:ident) => {
        impl ChannelValue for $t {
            fn value_type() -> ChannelValueType {
                ChannelValueType::$t
            }
            fn name() -> &'static str {
                stringify!($t)
            }
        }
        impl MaybeSync for $t {}
    };
}
impl_channel_value!(Vec3);
impl_channel_value!(f32);
impl_channel_value!(bool);

/// The `FromLua` and `ToLua` traits have a lifetime parameter which is
/// unnecessary for the channel keys and values. We introduce this new trait
/// instead which makes things simpler when implementing dynamic channels.
pub trait FromToLua {
    fn cast_to_lua(self, lua: &Lua) -> mlua::Value;
    fn cast_from_lua(value: mlua::Value, lua: &Lua) -> Result<Self>
    where
        Self: Sized;
}

macro_rules! impl_from_to_lua {
    (wrapped $t:ident $wrapper:ident) => {
        impl FromToLua for $t {
            fn cast_to_lua(self, lua: &Lua) -> mlua::Value {
                lua_stdlib::$wrapper(self).to_lua(lua).unwrap()
            }

            fn cast_from_lua(value: mlua::Value, lua: &Lua) -> Result<Self> {
                let value: lua_stdlib::$wrapper = FromLua::from_lua(value, lua)?;
                Ok(value.0)
            }
        }
    };
    (flat $t:ident) => {
        impl FromToLua for $t {
            fn cast_to_lua(self, lua: &Lua) -> mlua::Value {
                self.to_lua(lua).unwrap()
            }

            fn cast_from_lua(value: mlua::Value, lua: &Lua) -> Result<Self> {
                let value: $t = FromLua::from_lua(value, lua)?;
                Ok(value)
            }
        }
    };
}
impl_from_to_lua!(wrapped Vec3 LVec3);
impl_from_to_lua!(flat f32);
impl_from_to_lua!(flat bool);
impl_from_to_lua!(flat VertexId);
impl_from_to_lua!(flat FaceId);
impl_from_to_lua!(flat HalfEdgeId);

/// An enum representing all the types that implement the [`ChannelKey`] type as
/// variants. The values from this enum are used when dynamic behaviour is
/// required. This can be seen as an ad-hoc replacement for `TypeId`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
#[rustfmt::skip]
pub enum ChannelKeyType { VertexId, FaceId, HalfEdgeId }

/// Same as [`ChannelKeyType`], but for the [`ChannelValue`] trait instead.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
#[rustfmt::skip]
#[allow(non_camel_case_types)]
pub enum ChannelValueType { Vec3, f32, bool }

/// A channel represents a set of data that is associated over all the elements
/// of a mesh. For instance, the well-known `position` channel of a mesh, is a
/// channel storing vectors (Vec3) for each vertex (VertexId)
///
/// Logically, channels behave as an associative array from keys to values. A
/// channel by default acts as an infinite list of
/// [`Default`](std::default::Default) values: It will return the default value
/// for non-set keys. And setting the channel value for any valid key will never
/// fail.
///
/// Internally, a channel is backed by a
/// [`SecondaryMap`](slotmap::SecondaryMap), which takes the same key types as
/// the `MeshConnections` for the mesh.
///
/// Using keys (i.e. VertexId, FaceId, HalfEdgeId) in a channel taken from a
/// different mesh is considered an error. It is not UB but will not behave as
/// expected.
#[derive(Clone, Debug)]
pub struct Channel<K: ChannelKey, V: ChannelValue> {
    inner: slotmap::SecondaryMap<K, V>,
    default: V,
}

slotmap::new_key_type! {
    /// Channels in a [`ChannelGroup`] are stored using a slotmap. This is the
    /// id type for this slotmap. There is a type-safe wrapper [`ChannelId`]
    /// that wraps this but is generic over the key and value types.
    pub struct RawChannelId;
}

/// A generic wrapper over a `RawChannelId`. This is used to provide some extra
/// type safety to the typed channel APIs. To invoke the dynamic channel APIs
/// can use the inner `raw` field.
pub struct ChannelId<K: ChannelKey, V: ChannelValue> {
    raw: RawChannelId,
    _phantom: PhantomData<(K, V)>,
}
impl<K: ChannelKey, V: ChannelValue> ChannelId<K, V> {
    pub fn new(raw: RawChannelId) -> Self {
        Self {
            raw,
            _phantom: Default::default(),
        }
    }
}

/// A [`ChannelGroup`] is a homogeneous group of channels that share the same
/// key and value types. This struct has methods to create, delete and modify
/// channels, either by name or by channel id.
///
/// There are two kinds of APIs for channels: Typed and untyped. The typed APIs
/// are meant to be used from Rust, and provide extra performance and
/// type-safety. The untyped, or dynamic, APIs are used when interfacing with
/// Lua, since they allow to fetch channels with types only known at runtime.
///
/// Channels in a group are stored using shared ownership and interior
/// mutability, that is, `Rc<RefCell<Channel>>`. This creates a more flexible
/// borrowing scheme for channels and allows for things like temporarily lending
/// ownership of a channel to the Lua runtime.
#[derive(Debug)]
pub struct ChannelGroup<K: ChannelKey, V: ChannelValue> {
    channel_names: bimap::BiMap<String, ChannelId<K, V>>,
    channels: SlotMap<RawChannelId, RefCounted<InteriorMutable<Channel<K, V>>>>,
}

impl<K: ChannelKey, V: ChannelValue> MaybeSync for ChannelGroup<K, V> {}

/// The [`MeshChannels`] are one part of a [`HalfEdgeMesh`]. This struct stores
/// an heterogeneous group of channel groups, with potentially one
/// [`ChannelGroup`] for each key and value type combination.
///
/// The methods in this struct mirror the [`ChannelGroup`] API by providing
/// typed and untyped variants for static and dynamic access.
#[derive(Default, Debug, Clone)]
pub struct MeshChannels {
    channels: HashMap<(ChannelKeyType, ChannelValueType), Box<dyn DynChannelGroup>>,
}

/// This helper struct is stored in meshes and contains the channel ids for some
/// "well-known" channels that are always present. This avoids unnecessary
/// string lookups to fetch frequently used channels like `position`.
#[derive(Debug, Clone)]
pub struct DefaultChannels {
    pub position: ChannelId<VertexId, Vec3>,
    pub vertex_normals: Option<ChannelId<VertexId, Vec3>>,
    pub face_normals: Option<ChannelId<FaceId, Vec3>>,
    /// There are no Vec2 channels. Uvs simply use the first two coordinates.
    /// You can store different UVs for every face a vertex belongs to. We use
    /// the outgoing halfedges to represent this relation and store UVs in them
    /// instead.
    pub uvs: Option<ChannelId<HalfEdgeId, Vec3>>,
}

impl<K: ChannelKey, V: ChannelValue> std::ops::Index<K> for Channel<K, V> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        // Will return the default value for never-accessed keys.
        self.inner.get(index).unwrap_or(&self.default)
    }
}
impl<K: ChannelKey, V: ChannelValue> std::ops::IndexMut<K> for Channel<K, V> {
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.inner
            .entry(index)
            // From the `entry` documentation in slotmap: May return None if the
            // key was removed from the originating slot map.
            .expect("Error indexing channel. Key was removed from the originating slotmap.")
            // Will insert the default value for never-accessed keys.
            .or_default()
    }
}
impl<K: ChannelKey, V: ChannelValue> Channel<K, V> {
    /// Constructs a new channel without adding it to a mesh.
    pub fn new() -> Self
    where
        V: Default,
    {
        Self::new_with_default(V::default())
    }

    /// Constructs a new channel without adding it to a mesh. This allows
    /// setting the `default` value of this channel.
    pub fn new_with_default(default: V) -> Self {
        Self {
            inner: SecondaryMap::new(),
            default,
        }
    }

    /// Iterates the inner slotmap, returning an iterator of keys and values
    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> {
        self.inner.iter()
    }
    /// Iterates the inner slotmap, returning a mut iterator of keys and values
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (K, &mut V)> {
        self.inner.iter_mut()
    }
}

/// This trait provides dynamic access to a `Channel`. It is mainly used to
/// interface with channels from whithin Lua. The channel internally is a typed
/// storage, but the dynamic interface converts those values to mlua::Value at
/// runtime
pub trait DynChannel: Any + Debug {
    /// Casts this channel into a `dyn Any`. This hack is required to get
    /// around limitations in the dynamic dispatch system.
    fn as_any(&self) -> &dyn Any;
    /// Same as `as_any`, but for a mutable reference instead.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Accesses element at `key`, similar to using the `Index` trait on the
    /// equivalent typed channel.
    fn get_lua<'a, 'lua>(
        &'a self,
        lua: &'lua mlua::Lua,
        key: mlua::Value<'lua>,
    ) -> Result<mlua::Value<'lua>>
    where
        'lua: 'a;

    /// Sets the element at `key` to `value`. Similar to using the `IndexMut`
    /// trait on the equivalent channel
    fn set_lua<'a, 'lua>(
        &'a mut self,
        lua: &'lua mlua::Lua,
        key: mlua::Value,
        value: mlua::Value,
    ) -> Result<()>
    where
        'lua: 'a;

    /// Returns this channel as a Lua table (sequence). When Lua code wants to
    /// modify a full channel, it is generally faster to convert the channel to
    /// a table, let Lua manipulate it freely and then set it back using the
    /// complementary `set_from_table`.
    fn to_seq_table<'lua>(
        &self,
        keys: Box<dyn Iterator<Item = u64> + '_>,
        lua: &'lua mlua::Lua,
    ) -> mlua::Table<'lua>;

    /// Given a Lua table like the one returned by `to_table`, sets the values
    /// in this channel to the ones provided by the table.
    fn set_from_seq_table<'lua>(
        &mut self,
        keys: Box<dyn Iterator<Item = u64> + '_>,
        lua: &'lua mlua::Lua,
        table: mlua::Table<'lua>,
    ) -> Result<()>;

    /// Same as `to_seq_table`, but instead of assuming a sequential table, a
    /// dictionary-like table mapping keys to values is returned
    fn to_assoc_table<'lua>(
        &self,
        keys: Box<dyn Iterator<Item = u64> + '_>,
        lua: &'lua mlua::Lua,
    ) -> mlua::Table<'lua>;

    /// Same as `set_from_seq_table`, but instead of taking sequential table, a
    /// dictionary-like table mapping keys to values is taken
    fn set_from_assoc_table<'lua>(
        &mut self,
        lua: &'lua mlua::Lua,
        table: mlua::Table<'lua>,
    ) -> Result<()>;

    /// Merges this channel with another channel. This method will panic if both
    /// channels are not of the same type.
    ///
    /// The `get_ids` function returns all the ids of a certain channel key type
    /// in channel b. The `id_map` function maps keys from the `other` channel
    /// to keys in this channel.
    fn merge_with_dyn(
        &mut self,
        other: &dyn DynChannel,
        get_ids: &dyn Fn(ChannelKeyType) -> Rc<Vec<slotmap::KeyData>>,
        id_map: &dyn Fn(ChannelKeyType, slotmap::KeyData) -> slotmap::KeyData,
    );
}
impl<K: ChannelKey, V: ChannelValue> DynChannel for Channel<K, V> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_lua<'a, 'lua>(
        &'a self,
        lua: &'lua mlua::Lua,
        key: mlua::Value<'lua>,
    ) -> Result<mlua::Value<'lua>>
    where
        'lua: 'a,
    {
        let key: K = K::cast_from_lua(key, lua)?;
        Ok(self[key].cast_to_lua(lua))
    }

    fn set_lua<'a, 'lua>(
        &'a mut self,
        lua: &'lua mlua::Lua,
        key: mlua::Value,
        value: mlua::Value,
    ) -> Result<()>
    where
        'lua: 'a,
    {
        let key: K = K::cast_from_lua(key, lua)?;
        self[key] = FromToLua::cast_from_lua(value, lua)?;
        Ok(())
    }

    fn to_seq_table<'lua>(
        &self,
        keys: Box<dyn Iterator<Item = u64> + '_>,
        lua: &'lua mlua::Lua,
    ) -> mlua::Table<'lua> {
        lua.create_sequence_from(keys.map(K::cast_from_ffi).map(|k| self[k].cast_to_lua(lua)))
            .unwrap()
    }

    fn set_from_seq_table<'lua>(
        &mut self,
        keys: Box<dyn Iterator<Item = u64> + '_>,
        lua: &'lua mlua::Lua,
        table: mlua::Table<'lua>,
    ) -> Result<()> {
        keys.map(K::cast_from_ffi)
            .zip(table.sequence_values::<mlua::Value>())
            .try_for_each::<_, Result<_>>(|(k, lua_val)| {
                self[k] = FromToLua::cast_from_lua(lua_val.unwrap(), lua)?;
                Ok(())
            })?;
        Ok(())
    }

    fn to_assoc_table<'lua>(
        &self,
        keys: Box<dyn Iterator<Item = u64> + '_>,
        lua: &'lua mlua::Lua,
    ) -> mlua::Table<'lua> {
        lua.create_table_from(
            keys.map(K::cast_from_ffi)
                .map(|k| (k.cast_to_lua(lua), self[k].cast_to_lua(lua))),
        )
        .unwrap()
    }

    fn set_from_assoc_table<'lua>(
        &mut self,
        lua: &'lua mlua::Lua,
        table: mlua::Table<'lua>,
    ) -> Result<()> {
        table.pairs().try_for_each(|pair| {
            let (k, v) = pair?;
            let k = FromToLua::cast_from_lua(k, lua)?;
            let v = FromToLua::cast_from_lua(v, lua)?;
            self[k] = v;
            Ok(())
        })
    }

    fn merge_with_dyn(
        &mut self,
        other: &dyn DynChannel,
        get_ids: &dyn Fn(ChannelKeyType) -> Rc<Vec<slotmap::KeyData>>,
        id_map: &dyn Fn(ChannelKeyType, slotmap::KeyData) -> slotmap::KeyData,
    ) {
        let other_any = other.as_any();
        if let Some(other) = other_any.downcast_ref::<Self>() {
            for id in get_ids(K::key_type()).iter_cpy() {
                let k_self = K::cast_from_ffi(id_map(K::key_type(), id).as_ffi());
                let k_other = K::cast_from_ffi(id.as_ffi());
                self[k_self] = other[k_other];
            }
        } else {
            panic!(
                "Tried to merge dynamic channels with different types. This should never happen."
            )
        }
    }
}

impl<K: ChannelKey, V: ChannelValue> ChannelGroup<K, V> {
    /// Creates a new channel with a given `name`. If the channel with `name`
    /// already exists in the group, this operation is ignored.
    pub fn ensure_channel(&mut self, name: &str) -> ChannelId<K, V> {
        match self.channel_names.get_by_left(name) {
            Some(id) => *id,
            None => {
                let ch_id = ChannelId::new(self.channels.insert(Default::default()));
                self.channel_names.insert(name.into(), ch_id);
                ch_id
            }
        }
    }

    /// Creates a new channel with a given `name`. If the channel with `name`
    /// already exists, returns an error.
    pub fn create_channel(&mut self, name: &str) -> Result<ChannelId<K, V>> {
        if self.channel_names.contains_left(name) {
            bail!("The channel named {name} already exists in mesh");
        } else {
            Ok(self.ensure_channel(name))
        }
    }

    /// Removes a channel with given `id`. Returns error when the channel:
    /// - Doesn't exist
    /// - Is borrowed somewhere else (via a cloned Rc)
    pub fn remove_channel(&mut self, id: ChannelId<K, V>) -> Result<Channel<K, V>> {
        self.channel_names.remove_by_right(&id);
        Ok(RefCounted::try_unwrap(
            self.channels
                .remove(id.raw)
                .ok_or_else(|| anyhow!("Non-existing channel cannot be removed"))?,
        )
        .map_err(|_| {
            anyhow!("This channel can't be deleted because it's still referenced somewhere else.")
        })?
        .into_inner())
    }

    /// Returns the channel id for a channel with given `name`, or `None` if it
    /// doesn't exist.
    pub fn channel_id(&self, name: &str) -> Option<ChannelId<K, V>> {
        self.channel_names.get_by_left(name).copied()
    }

    /// Returns the channel name for a given channel `id`, or `None` if it
    /// doesn't exist
    pub fn channel_name(&self, id: ChannelId<K, V>) -> Option<&str> {
        self.channel_names.get_by_right(&id).map(|x| x.as_str())
    }

    /// Accesses a channel immutably. The operation may fail if that channel is
    /// already mutably borrowed following the RefCell semantics.
    pub fn read_channel(&self, ch_id: ChannelId<K, V>) -> Result<BorrowedRef<Channel<K, V>>> {
        self.channels
            .get(ch_id.raw)
            .ok_or_else(|| anyhow!("Channel {ch_id:?} does not exist for this mesh"))?
            .try_borrow()
            .map_err(|err| anyhow!("Channel {ch_id:?} could not be borrowed: {err}"))
    }

    /// Accesses a channel immutably. The operation may fail if that channel is
    /// already borrowed following the RefCell semantics.
    pub fn write_channel(&self, ch_id: ChannelId<K, V>) -> Result<MutableRef<Channel<K, V>>> {
        self.channels
            .get(ch_id.raw)
            .ok_or_else(|| anyhow!("Channel {ch_id:?} does not exist for this mesh"))?
            .try_borrow_mut()
            .map_err(|err| anyhow!("Channel {ch_id:?} could not be borrowed: {err}"))
    }
}

/// This trait is the dynamic API of a [`ChannelGroup`]
pub trait DynChannelGroup: Any + Debug + dyn_clone::DynClone + MaybeSync {
    /// Used to inspect the contents of this `ChannelGroup`, for UI display
    fn introspect(&self, keys: &[slotmap::KeyData]) -> BTreeMap<String, Vec<String>>;
    /// Casts this channel group into a `dyn Any`. This hack is required to get
    /// around limitations in the dynamic dispatch system.
    fn as_any(&self) -> &dyn Any;
    /// Same as `as_any`, but for a mutable reference instead.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Same as `ensure_channel`, but with erased types.
    fn ensure_channel_dyn(&mut self, name: &str) -> RawChannelId;
    /// Same as `read_channel`, but with erased types.
    fn read_channel_dyn(&self, raw_id: RawChannelId) -> BorrowedRef<dyn DynChannel>;
    /// Same as `write_channel`, but with erased types.
    fn write_channel_dyn(&self, raw_id: RawChannelId) -> MutableRef<dyn DynChannel>;
    /// Same as `channel_id`, but with erased types.
    fn channel_id_dyn(&self, name: &str) -> Option<RawChannelId>;
    /// Returns a shared ownership borrow of the channel. This uses reference
    /// counting and allows storing the channel as a long-lived value. This can
    /// be used to hand channels over to the Lua runtime.
    fn channel_rc_dyn(&self, raw_id: RawChannelId) -> RefCounted<InteriorMutable<dyn DynChannel>>;
    /// Returns the names of the channels present in this group
    fn channel_names(&self) -> Box<dyn Iterator<Item = &str> + '_>;
}

impl<K: ChannelKey, V: ChannelValue> Clone for ChannelGroup<K, V> {
    fn clone(&self) -> Self {
        let mut new_channels = self.channels.clone();
        for (_, ch) in new_channels.iter_mut() {
            // NOTE: We need to duplicate the contents. If we use the
            // blindly-derived clone implementation we will clone the Rcs
            // instead, and that's not what we want.

            // Also note that this implies cloning a mesh will panic if someone
            // is *writing* to that mesh.
            let ch_inner: Channel<K, V> = ch.borrow().clone();
            *ch = RefCounted::new(InteriorMutable::new(ch_inner.clone()))
        }
        Self {
            channel_names: self.channel_names.clone(),
            channels: new_channels,
        }
    }
}

// DynChannelGroup implements the DynClone trait so we can clone it too. The
// Clone trait can't be implemented by object-safe traits so we can't just add a
// `: Clone` bound to `DynChannelGroup`.
dyn_clone::clone_trait_object!(DynChannelGroup);

impl<K: ChannelKey, V: ChannelValue> DynChannelGroup for ChannelGroup<K, V> {
    fn introspect(&self, keys: &[slotmap::KeyData]) -> BTreeMap<String, Vec<String>> {
        let mut result = BTreeMap::new();
        for (name, id) in self.channel_names.iter() {
            let ch = self.read_channel(*id).unwrap();
            result.insert(
                name.into(),
                keys.iter()
                    .map(|k| ch[K::from(*k)])
                    .map(|x| x.introspect())
                    .collect(),
            );
        }
        result
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn ensure_channel_dyn(&mut self, name: &str) -> RawChannelId {
        self.ensure_channel(name).raw
    }
    fn read_channel_dyn(&self, raw_id: RawChannelId) -> BorrowedRef<dyn DynChannel> {
        let borrowed_ref = self.channels[raw_id].borrow();
        BorrowedRef::map(borrowed_ref, |k| k as &dyn DynChannel)
    }
    fn write_channel_dyn(&self, raw_id: RawChannelId) -> MutableRef<dyn DynChannel> {
        let mutable_ref = self.channels[raw_id].borrow_mut();
        MutableRef::map(mutable_ref, |k| k as &mut dyn DynChannel)
    }
    fn channel_rc_dyn(&self, raw_id: RawChannelId) -> RefCounted<InteriorMutable<dyn DynChannel>> {
        // This standalone function is needed to help the compiler convert
        // between a typed Rc and the dynamic one.
        pub fn convert_channel<K: ChannelKey, V: ChannelValue>(
            it: RefCounted<InteriorMutable<Channel<K, V>>>,
        ) -> RefCounted<InteriorMutable<dyn DynChannel>> {
            it
        }
        convert_channel(RefCounted::clone(&self.channels[raw_id]))
    }
    fn channel_id_dyn(&self, name: &str) -> Option<RawChannelId> {
        self.channel_names.get_by_left(name).map(|x| x.raw)
    }

    fn channel_names(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(self.channel_names.iter().map(|(l, _)| l.as_str()))
    }
}

impl MeshChannels {
    fn key_of<K: ChannelKey, V: ChannelValue>() -> (ChannelKeyType, ChannelValueType) {
        (K::key_type(), V::value_type())
    }

    fn downcast<K: ChannelKey, V: ChannelValue>(group: &dyn Any) -> &ChannelGroup<K, V> {
        match group.downcast_ref::<ChannelGroup<K, V>>() {
            Some(typed_group) => typed_group,
            None => unreachable!("The invariants of MeshChannels should prevent this."),
        }
    }
    fn downcast_mut<K: ChannelKey, V: ChannelValue>(
        group: &mut dyn Any,
    ) -> &mut ChannelGroup<K, V> {
        match group.downcast_mut::<ChannelGroup<K, V>>() {
            Some(typed_group) => typed_group,
            None => unreachable!("The invariants of MeshChannels should prevent this."),
        }
    }

    fn group<K: ChannelKey, V: ChannelValue>(&self) -> Result<&ChannelGroup<K, V>> {
        Ok(Self::downcast(
            self.channels
                .get(&Self::key_of::<K, V>())
                .ok_or_else(|| anyhow!("There is no channel for {} -> {}", K::name(), V::name()))?
                .as_any(),
        ))
    }

    fn group_mut<K: ChannelKey, V: ChannelValue>(&mut self) -> Result<&mut ChannelGroup<K, V>> {
        Ok(Self::downcast_mut(
            self.channels
                .get_mut(&Self::key_of::<K, V>())
                .ok_or_else(|| anyhow!("There is no channel for {} -> {}", K::name(), V::name()))?
                .as_any_mut(),
        ))
    }

    fn group_or_default<K: ChannelKey, V: ChannelValue>(&mut self) -> &mut ChannelGroup<K, V> {
        Self::downcast_mut(
            self.channels
                .entry(Self::key_of::<K, V>())
                .or_insert_with(|| Box::<ChannelGroup<K, V>>::default())
                .as_any_mut(),
        )
    }

    /// Calls `ensure_channel` for the channel group with key and value type
    pub fn ensure_channel<K: ChannelKey, V: ChannelValue>(
        &mut self,
        name: &str,
    ) -> ChannelId<K, V> {
        self.group_or_default().ensure_channel(name)
    }

    /// Calls `create_channel` for the channel group with key and value type
    pub fn create_channel<K: ChannelKey, V: ChannelValue>(
        &mut self,
        name: &str,
    ) -> Result<ChannelId<K, V>> {
        self.group_or_default().create_channel(name)
    }

    /// Calls `remove_channel` for the channel group with key and value type
    pub fn remove_channel<K: ChannelKey, V: ChannelValue>(
        &mut self,
        ch_id: ChannelId<K, V>,
    ) -> Result<Channel<K, V>> {
        self.group_mut()?.remove_channel(ch_id)
    }

    /// Calls `read_channel` for the channel group with key and value type
    pub fn read_channel<K: ChannelKey, V: ChannelValue>(
        &self,
        ch_id: ChannelId<K, V>,
    ) -> Result<BorrowedRef<Channel<K, V>>> {
        self.group()?.read_channel(ch_id)
    }

    /// Calls `read_channel` for the channel group with key and value type. Uses
    /// the channel name instead of its id.
    pub fn read_channel_by_name<K: ChannelKey, V: ChannelValue>(
        &self,
        name: &str,
    ) -> Result<BorrowedRef<Channel<K, V>>> {
        let group = self.group()?;
        group.read_channel(
            group
                .channel_id(name)
                .ok_or_else(|| anyhow!("Channel named {name} does not exist"))?,
        )
    }

    /// Calls `write_channel` for the channel group with key and value type
    pub fn write_channel<K: ChannelKey, V: ChannelValue>(
        &self,
        ch_id: ChannelId<K, V>,
    ) -> Result<MutableRef<Channel<K, V>>> {
        self.group()?.write_channel(ch_id)
    }

    /// Calls `write_channel` for the channel group with key and value type. Uses
    /// the channel name instead of its id.
    pub fn write_channel_by_name<K: ChannelKey, V: ChannelValue>(
        &self,
        name: &str,
    ) -> Result<MutableRef<Channel<K, V>>> {
        let group = self.group()?;
        group.write_channel(
            group
                .channel_id(name)
                .ok_or_else(|| anyhow!("Channel named {name} does not exist"))?,
        )
    }

    fn ensure_group_dyn(
        &mut self,
        kty: ChannelKeyType,
        vty: ChannelValueType,
    ) -> &mut dyn DynChannelGroup {
        type K = ChannelKeyType;
        type V = ChannelValueType;

        macro_rules! ret {
            ($kt:ident, $vt:ident) => {
                self.group_or_default::<$kt, $vt>() as &mut dyn DynChannelGroup
            };
        }

        macro_rules! do_match {
            ($($kt:ident, $vt:ident);*) => {
                match (kty, vty) { $(
                    (K::$kt, V::$vt) => { ret!($kt, $vt) }
                )* }
            }
        }

        do_match! {
            VertexId, Vec3;
            VertexId, f32;
            VertexId, bool;
            FaceId, Vec3;
            FaceId, f32;
            FaceId, bool;
            HalfEdgeId, Vec3;
            HalfEdgeId, f32;
            HalfEdgeId, bool
        }
    }

    /// Creates a channel with `name` for a group with dynamic key and value
    /// types given at runtime.
    pub fn ensure_channel_dyn(
        &mut self,
        kty: ChannelKeyType,
        vty: ChannelValueType,
        name: &str,
    ) -> RawChannelId {
        let group = self.ensure_group_dyn(kty, vty);
        group.ensure_channel_dyn(name)
    }

    /// Calls `read_channel` for a group with dynamic key and value
    /// types given at runtime.
    pub fn dyn_read_channel(
        &self,
        kty: ChannelKeyType,
        vty: ChannelValueType,
        id: RawChannelId,
    ) -> Result<BorrowedRef<dyn DynChannel>> {
        let group = self
            .channels
            .get(&(kty, vty))
            .ok_or_else(|| anyhow!("Channel type does not exist"))?;
        Ok(group.read_channel_dyn(id))
    }

    /// Calls `write_channel` for a group with dynamic key and value
    /// types given at runtime.
    pub fn dyn_write_channel(
        &self,
        kty: ChannelKeyType,
        vty: ChannelValueType,
        id: RawChannelId,
    ) -> Result<MutableRef<dyn DynChannel>> {
        let group = self
            .channels
            .get(&(kty, vty))
            .ok_or_else(|| anyhow!("Channel type does not exist"))?;
        Ok(group.write_channel_dyn(id))
    }

    /// Calls `read_channel` for a group with dynamic key and value
    /// types given at runtime.
    pub fn dyn_read_channel_by_name(
        &self,
        kty: ChannelKeyType,
        vty: ChannelValueType,
        name: &str,
    ) -> Result<BorrowedRef<dyn DynChannel>> {
        let group = self
            .channels
            .get(&(kty, vty))
            .ok_or_else(|| anyhow!("Channel type does not exist"))?;
        let raw_id = group
            .channel_id_dyn(name)
            .ok_or_else(|| anyhow!("Channel value does not exist"))?;
        Ok(group.read_channel_dyn(raw_id))
    }

    /// Calls `write_channel` for a group with dynamic key and value
    /// types given at runtime.
    pub fn dyn_write_channel_by_name(
        &self,
        kty: ChannelKeyType,
        vty: ChannelValueType,
        name: &str,
    ) -> Result<MutableRef<dyn DynChannel>> {
        let group = self
            .channels
            .get(&(kty, vty))
            .ok_or_else(|| anyhow!("Channel type does not exist"))?;
        let raw_id = group
            .channel_id_dyn(name)
            .ok_or_else(|| anyhow!("Channel value does not exist"))?;
        Ok(group.write_channel_dyn(raw_id))
    }

    /// Calls `channel_rc` for a group with dynamic key and value
    /// types given at runtime.
    pub fn channel_rc_dyn(
        &self,
        kty: ChannelKeyType,
        vty: ChannelValueType,
        name: &str,
    ) -> Result<RefCounted<InteriorMutable<dyn DynChannel>>> {
        let group = self
            .channels
            .get(&(kty, vty))
            .ok_or_else(|| anyhow!("Channel type does not exist"))?;
        let raw_id = group
            .channel_id_dyn(name)
            .ok_or_else(|| anyhow!("Channel value does not exist"))?;
        Ok(group.channel_rc_dyn(raw_id))
    }

    /// Calls `channel_id` for the channel group with key and value type
    pub fn channel_id<K: ChannelKey, V: ChannelValue>(
        &self,
        name: &str,
    ) -> Option<ChannelId<K, V>> {
        self.group().ok()?.channel_id(name)
    }

    /// Calls `channel_id` for the channel group with key and value type
    pub fn channel_id_dyn(
        &self,
        kty: ChannelKeyType,
        vty: ChannelValueType,
        name: &str,
    ) -> Option<RawChannelId> {
        self.channels.get(&(kty, vty))?.channel_id_dyn(name)
    }

    /// Calls `channel_name` for the channel group with key and value type
    pub fn channel_name<K: ChannelKey, V: ChannelValue>(
        &self,
        ch_id: ChannelId<K, V>,
    ) -> Option<&str> {
        self.group().ok()?.channel_name(ch_id)
    }

    /// Used to inspect the contents of this `MeshChannels`, for UI display
    pub fn introspect(
        &self,
        get_ids: impl Fn(ChannelKeyType) -> Rc<Vec<slotmap::KeyData>>,
    ) -> BTreeMap<(ChannelKeyType, ChannelValueType), BTreeMap<String, Vec<String>>> {
        self.channels
            .iter()
            .map(|((k, v), group)| ((*k, *v), group.introspect(&get_ids(*k))))
            .collect()
    }

    pub fn merge_with(
        &mut self,
        other: &Self,
        get_ids: impl Fn(ChannelKeyType) -> Rc<Vec<slotmap::KeyData>>,
        id_map: impl Fn(ChannelKeyType, slotmap::KeyData) -> slotmap::KeyData,
    ) {
        // - Any channels not present in B can be kept as is (new values take default)
        // - Any channels present in B, but not present in A will need to be copied.
        for ((kty, vty), other_group) in other.channels.iter() {
            let self_group = self.ensure_group_dyn(*kty, *vty);
            for ch_name in other_group.channel_names() {
                let other_id = other_group
                    .channel_id_dyn(ch_name)
                    .expect("We know it exists because we're iterating the channel names");
                let self_id = self_group.ensure_channel_dyn(ch_name);

                let other_ch = other_group.read_channel_dyn(other_id);
                let mut self_ch = self_group.write_channel_dyn(self_id);

                self_ch.merge_with_dyn(other_ch.deref(), &get_ids, &id_map);
            }
        }
    }

    /// Sets a channel directly, by name. If the channel doesn't exist, it is
    /// created, otherwise its contents are dropped and the new channel data is
    /// used. Returns the id of the channel that was created.
    pub fn replace_or_create_channel<K: ChannelKey, V: ChannelValue>(
        &mut self,
        name: &str,
        ch: Channel<K, V>,
    ) -> ChannelId<K, V> {
        let ch_id = self.group_or_default().ensure_channel(name);
        *self
            .write_channel(ch_id)
            .expect("We just ensured the channel exists") = ch;
        ch_id
    }
}

impl DefaultChannels {
    pub fn with_position(channels: &mut MeshChannels) -> Self {
        let position = channels.ensure_channel::<VertexId, Vec3>("position");
        Self {
            position,
            vertex_normals: None,
            face_normals: None,
            uvs: None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_channels() {
        let mut vertices: slotmap::SlotMap<VertexId, ()> = slotmap::SlotMap::with_key();
        let v1 = vertices.insert(());
        let v2 = vertices.insert(());
        let v3 = vertices.insert(());

        let mut mesh_channels = MeshChannels::default();
        let position = mesh_channels
            .create_channel::<VertexId, Vec3>("position")
            .unwrap();
        let color = mesh_channels
            .create_channel::<VertexId, Vec3>("color")
            .unwrap();
        let size = mesh_channels
            .create_channel::<VertexId, f32>("size")
            .unwrap();

        assert!(mesh_channels.channel_id("position").unwrap() == position);
        assert!(mesh_channels.channel_id("color").unwrap() == color);
        assert!(mesh_channels.channel_id("size").unwrap() == size);

        {
            let mut positions = mesh_channels.write_channel(position).unwrap();
            let mut colors = mesh_channels.write_channel(color).unwrap();
            let mut sizes = mesh_channels.write_channel(size).unwrap();

            positions[v1] = Vec3::X;
            positions[v2] = Vec3::Y;
            positions[v3] = Vec3::Z;

            colors[v1] = Vec3::splat(0.0);
            colors[v2] = Vec3::splat(0.5);
            colors[v3] = Vec3::splat(1.0);

            sizes[v1] = 0.25;
            sizes[v2] = 0.50;
            sizes[v3] = 1.0;

            // Re-borrowing the position channel should fail now
            assert!(mesh_channels.read_channel(position).is_err());
        }

        {
            let positions = mesh_channels.read_channel(position).unwrap();
            let colors = mesh_channels.read_channel(color).unwrap();
            let sizes = mesh_channels.read_channel(size).unwrap();

            assert_eq!(positions[v1], Vec3::X);
            assert_eq!(positions[v2], Vec3::Y);
            assert_eq!(positions[v3], Vec3::Z);

            assert_eq!(colors[v1], Vec3::splat(0.0));
            assert_eq!(colors[v2], Vec3::splat(0.5));
            assert_eq!(colors[v3], Vec3::splat(1.0));

            assert_eq!(sizes[v1], 0.25);
            assert_eq!(sizes[v2], 0.50);
            assert_eq!(sizes[v3], 1.0);

            // Re-reading a channel works, because we only hold Refs
            assert!(mesh_channels.read_channel(position).is_ok());
            // But trying to write still fails
            assert!(mesh_channels.write_channel(position).is_err());
        }

        // Once the refs are dropped, we can write again
        assert!(mesh_channels.write_channel(position).is_ok());

        // The introspection API can be used to inspect the existing channels
        // without necessarily knowing which channels are registered or their
        // types.
        use slotmap::Key;
        let vs = Rc::new(vec![v1.data(), v2.data(), v3.data()]);
        let introspected = mesh_channels.introspect(move |k| match k {
            ChannelKeyType::VertexId => vs.clone(),
            ChannelKeyType::FaceId => unreachable!(),
            ChannelKeyType::HalfEdgeId => unreachable!(),
        });
        assert_eq!(
            &introspected[&(ChannelKeyType::VertexId, ChannelValueType::Vec3)]["color"],
            &[
                " 0.000  0.000  0.000",
                " 0.500  0.500  0.500",
                " 1.000  1.000  1.000",
            ]
        );
        assert_eq!(
            &introspected[&(ChannelKeyType::VertexId, ChannelValueType::f32)]["size"],
            &[" 0.250", " 0.500", " 1.000",]
        );
        assert_eq!(
            &introspected[&(ChannelKeyType::VertexId, ChannelValueType::Vec3)]["position"],
            &[
                " 1.000  0.000  0.000",
                " 0.000  1.000  0.000",
                " 0.000  0.000  1.000",
            ],
        );

        // Channels can also be read and written using a type-erased API. This
        // is mainly used for interfacing with Lua and looks very clunky here.
        // When programming in Rust, using the type-safe API is preferred
        let lua = Lua::new();
        let dyn_pos = mesh_channels
            .dyn_read_channel_by_name(ChannelKeyType::VertexId, ChannelValueType::f32, "size")
            .unwrap();
        match dyn_pos.get_lua(&lua, v1.cast_to_lua(&lua)).unwrap() {
            mlua::Value::Number(x) if x == 0.25 => {}
            _ => panic!("Expected the number 0.25"),
        }
        drop(dyn_pos);
    }

    #[test]
    pub fn test_ensure_channel() {
        let mut mesh_channels = MeshChannels::default();

        let position = mesh_channels
            .create_channel::<VertexId, Vec3>("position")
            .unwrap();
        assert_eq!(
            position,
            mesh_channels.ensure_channel::<VertexId, Vec3>("position")
        );
    }
}

// ------------- Boilerplate zone ------------

// NOTE: An unfortunate consequence about using PhantomData is that rust's std
// derives stop working, so we need to do some boilerplate impls by hand. A
// crate like `derivative` can be used to solve this problem, but the extra
// dependency for a single usage is not justified.

impl<K: ChannelKey, V: ChannelValue> Clone for ChannelId<K, V> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw,
            _phantom: self._phantom,
        }
    }
}
impl<K: ChannelKey, V: ChannelValue> Copy for ChannelId<K, V> {}
impl<K: ChannelKey, V: ChannelValue> Default for ChannelId<K, V> {
    fn default() -> Self {
        Self {
            raw: Default::default(),
            _phantom: Default::default(),
        }
    }
}
impl<K: ChannelKey, V: ChannelValue> PartialEq for ChannelId<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw && self._phantom == other._phantom
    }
}
impl<K: ChannelKey, V: ChannelValue> Eq for ChannelId<K, V> {}
impl<K: ChannelKey, V: ChannelValue> Ord for ChannelId<K, V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}
impl<K: ChannelKey, V: ChannelValue> PartialOrd for ChannelId<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.raw.partial_cmp(&other.raw)
    }
}
impl<K: ChannelKey, V: ChannelValue> std::hash::Hash for ChannelId<K, V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}
impl<K: ChannelKey, V: ChannelValue> Debug for ChannelId<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelId")
            .field("data", &self.raw)
            .finish()
    }
}

impl<K: ChannelKey, V: ChannelValue> Default for Channel<K, V> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            default: Default::default(),
        }
    }
}

impl<K: ChannelKey, V: ChannelValue> Default for ChannelGroup<K, V> {
    fn default() -> Self {
        Self {
            channel_names: Default::default(),
            channels: Default::default(),
        }
    }
}
