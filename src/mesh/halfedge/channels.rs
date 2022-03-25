use std::{
    any::{type_name, TypeId},
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    marker::PhantomData,
};

use typemap::TypeMap;

use super::*;

pub trait ChannelKey: slotmap::Key + Default + Debug + Copy + Sized + 'static {}
impl ChannelKey for VertexId {}
impl ChannelKey for FaceId {}
impl ChannelKey for HalfEdgeId {}

pub trait ChannelValue: Default + Debug + Copy + Sized + 'static {}
impl ChannelValue for glam::Vec2 {}
impl ChannelValue for glam::Vec3 {}
impl ChannelValue for glam::Vec4 {}
impl ChannelValue for f32 {}
impl ChannelValue for bool {}

#[derive(Clone, Debug)]
pub struct Channel<K: ChannelKey, V: ChannelValue> {
    inner: slotmap::SecondaryMap<K, V>,
}

pub struct ChannelId<K: ChannelKey, V: ChannelValue> {
    data: slotmap::KeyData,
    _phantom: PhantomData<(K, V)>,
}
// SAFETY: Slotmap just needs that custom key types act as a wrapper over its
// KeyData. This is exactly what we're doing to it's safe. We can't use the
// macro because our key type is generic.
unsafe impl<K: ChannelKey, V: ChannelValue> slotmap::Key for ChannelId<K, V> {
    fn data(&self) -> slotmap::KeyData {
        self.data
    }
}
impl<K: ChannelKey, V: ChannelValue> From<slotmap::KeyData> for ChannelId<K, V> {
    fn from(data: slotmap::KeyData) -> Self {
        ChannelId {
            data,
            _phantom: Default::default(),
        }
    }
}

pub struct ChannelGroup<K: ChannelKey, V: ChannelValue> {
    channel_names: bimap::BiMap<String, ChannelId<K, V>>,
    channels: SlotMap<ChannelId<K, V>, RefCell<Channel<K, V>>>,
}

pub struct MeshChannels {
    channels: TypeMap,
}

impl<K: ChannelKey, V: ChannelValue> std::ops::Index<K> for Channel<K, V> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.inner
            .get(index)
            .expect("Error indexing channel. Key not found")
    }
}
impl<K: ChannelKey, V: ChannelValue> std::ops::IndexMut<K> for Channel<K, V> {
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.inner
            .entry(index)
            .expect("Error indexing channel. Key not found")
            .or_default()
    }
}
impl<K: ChannelKey, V: ChannelValue> Channel<K, V> {
    pub fn get(&self, id: K) -> Option<V> {
        self.inner.get(id).copied()
    }
    pub fn get_mut(&mut self, id: K) -> Option<&mut V> {
        Some(self.inner.entry(id)?.or_default())
    }
    pub fn set(&mut self, id: K, val: V) -> Option<()> {
        *self.inner.get_mut(id)? = val;
        Some(())
    }
    pub fn iter(&self) -> impl Iterator<Item=(K, &V)> {
        self.inner.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item=(K, &mut V)> {
        self.inner.iter_mut()
    }
}

impl<K: ChannelKey, V: ChannelValue> ChannelGroup<K, V> {
    pub fn ensure_channel(&mut self, name: String) -> ChannelId<K, V> {
        let ch_id = self.channels.insert(Default::default());
        self.channel_names.insert(name, ch_id);
        ch_id
    }

    pub fn create_channel(&mut self, name: String) -> Result<ChannelId<K, V>> {
        if self.channel_names.contains_left(&name) {
            bail!("The channel named {name} already exists in mesh");
        } else {
            Ok(self.ensure_channel(name))
        }
    }

    pub fn remove_channel(&mut self, id: ChannelId<K, V>) -> Result<Channel<K, V>> {
        self.channel_names.remove_by_right(&id);
        Ok(self
            .channels
            .remove(id)
            .ok_or_else(|| anyhow!("Non-existing channel cannot be removed"))?
            .into_inner())
    }

    pub fn channel_id(&self, name: &str) -> Option<ChannelId<K, V>> {
        self.channel_names.get_by_left(name).copied()
    }

    pub fn channel_name(&self, ch_id: ChannelId<K, V>) -> Option<&str> {
        self.channel_names.get_by_right(&ch_id).map(|x| x.as_str())
    }

    pub fn read_channel(&self, ch_id: ChannelId<K, V>) -> Result<Ref<Channel<K, V>>> {
        self.channels
            .get(ch_id)
            .ok_or_else(|| anyhow!("Channel {ch_id:?} does not exist for this mesh"))?
            .try_borrow()
            .map_err(|err| anyhow!("Channel {ch_id:?} could not be borrowed: {err}"))
    }

    pub fn write_channel(&self, ch_id: ChannelId<K, V>) -> Result<RefMut<Channel<K, V>>> {
        self.channels
            .get(ch_id)
            .ok_or_else(|| anyhow!("Channel {ch_id:?} does not exist for this mesh"))?
            .try_borrow_mut()
            .map_err(|err| anyhow!("Channel {ch_id:?} could not be borrowed: {err}"))
    }
}

impl<K: ChannelKey, V: ChannelValue> typemap::Key for ChannelGroup<K, V> {
    type Value = ChannelGroup<K, V>;
}

impl MeshChannels {
    fn group<K: ChannelKey, V: ChannelValue>(&self) -> Result<&ChannelGroup<K, V>> {
        self.channels.get::<ChannelGroup<K, V>>().ok_or_else(|| {
            anyhow!(
                "There is no channel for {} -> {}",
                type_name::<K>(),
                type_name::<V>()
            )
        })
    }

    fn group_mut<K: ChannelKey, V: ChannelValue>(&mut self) -> Result<&mut ChannelGroup<K, V>> {
        self.channels
            .get_mut::<ChannelGroup<K, V>>()
            .ok_or_else(|| {
                anyhow!(
                    "There is no channel for {} -> {}",
                    type_name::<K>(),
                    type_name::<V>()
                )
            })
    }

    fn group_or_default<K: ChannelKey, V: ChannelValue>(&mut self) -> &mut ChannelGroup<K, V> {
        self.channels
            .entry::<ChannelGroup<K, V>>()
            .or_insert_with(Default::default)
    }

    pub fn ensure_channel<K: ChannelKey, V: ChannelValue>(
        &mut self,
        name: String,
    ) -> ChannelId<K, V> {
        self.group_or_default().ensure_channel(name)
    }

    pub fn create_channel<K: ChannelKey, V: ChannelValue>(
        &mut self,
        name: String,
    ) -> Result<ChannelId<K, V>> {
        self.group_or_default().create_channel(name)
    }

    pub fn remove_channel<K: ChannelKey, V: ChannelValue>(
        &mut self,
        ch_id: ChannelId<K, V>,
    ) -> Result<Channel<K, V>> {
        self.group_mut()?.remove_channel(ch_id)
    }

    pub fn read_channel<K: ChannelKey, V: ChannelValue>(
        &self,
        ch_id: ChannelId<K, V>,
    ) -> Result<Ref<Channel<K, V>>> {
        self.group()?.read_channel(ch_id)
    }

    pub fn read_channel_by_name<K: ChannelKey, V: ChannelValue>(
        &self,
        name: &str,
    ) -> Result<Ref<Channel<K, V>>> {
        let group = self.group()?;
        group.read_channel(
            group
                .channel_id(name)
                .ok_or_else(|| anyhow!("Channel named {name} does not exist"))?,
        )
    }

    pub fn write_channel<K: ChannelKey, V: ChannelValue>(
        &self,
        ch_id: ChannelId<K, V>,
    ) -> Result<RefMut<Channel<K, V>>> {
        self.group()?.write_channel(ch_id)
    }

    pub fn write_channel_by_name<K: ChannelKey, V: ChannelValue>(
        &self,
        name: &str,
    ) -> Result<RefMut<Channel<K, V>>> {
        let group = self.group()?;
        group.write_channel(
            group
                .channel_id(name)
                .ok_or_else(|| anyhow!("Channel named {name} does not exist"))?,
        )
    }

    pub fn channel_id<K: ChannelKey, V: ChannelValue>(
        &self,
        name: &str,
    ) -> Option<ChannelId<K, V>> {
        self.group().ok()?.channel_id(name)
    }

    pub fn channel_name<K: ChannelKey, V: ChannelValue>(
        &self,
        ch_id: ChannelId<K, V>,
    ) -> Option<&str> {
        self.group().ok()?.channel_name(ch_id)
    }

    fn get_dyn_group(&self, id: TypeId) -> (&'static str, &'static str, &dyn DynChannelGroup) {
        // Please don't make me do this... *sigh*... ok, here we go:
        fn short_type_name<T>() -> &'static str {
            let name = type_name::<T>();
            &name[name.rfind(':').map(|x| x + 1).unwrap_or(0)..]
        }

        // I'm only gonna do this once, don't blink
        macro_rules! check_type {
            ($K:ty, $V:ty) => {
                if TypeId::of::<ChannelGroup<$K, $V>>() == id {
                    let dgroup = self.group::<$K, $V>().unwrap() as &dyn DynChannelGroup;
                    return (short_type_name::<$K>(), short_type_name::<$V>(), dgroup);
                }
            };
        }

        // https://ibb.co/MD8YX2J
        macro_rules! all_pairs {
            ([$($ks:ty),+], [$($vs:ty),+]) => {
                all_pairs!(@[$($ks),+], [$($ks),+], [$($vs),+])
            };
            (@[$($original_ks:ty),*], [$k:ty], [$v:ty]) => {
                check_type!($k, $v);
            };
            (@[$($original_ks:ty),*], [$k:ty], [$v:ty, $($vs:ty),*]) => {
                check_type!($k, $v);
                all_pairs!(@[$($original_ks),*], [$($original_ks),*], [$($vs),*])
            };
            (@[$($original_ks:ty),*], [$k:ty, $($ks:ty),*], [$v:ty]) => {
                check_type!($k, $v);
                all_pairs!(@[$($original_ks),*], [$($ks),*], [$v])
            };
            (@[$($original_ks:ty),*], [$k:ty, $($ks:ty),*], [$v:ty, $($vs:ty),*]) => {
                check_type!($k, $v);
                all_pairs!(@[$($original_ks),*], [$($ks),*], [$v, $($vs),*])
            };
        }

        all_pairs!(
            [VertexId, FaceId, HalfEdgeId],
            [glam::Vec2, glam::Vec3, glam::Vec4, f32, bool]
        );

        panic!(
            "Fatal error during channel introspection: The combination for type id {id:?}\
         is not registered. Please fix the `kv_name` function in `channel.rs`."
        );
    }

    pub fn introspect(&self) {
        // SAFETY: We're not doing anything stupid with the data. I don't even
        // know why this method is marked as unsafe, but the author didn't
        // clarify in the docstring. ¯\_(ツ)_/¯
        let data = unsafe { self.channels.data() };
        data.iter().for_each(|(k, _)| {
            let (ks, vs, dyn_ch) = self.get_dyn_group(*k);
            println!("Channels for {ks} -> {vs}");
            dbg!(dyn_ch.introspect());
        });
    }
}

pub trait DynChannelGroup {
    fn introspect(&self) -> HashMap<String, Vec<String>>;
}

impl<K: ChannelKey, V: ChannelValue> DynChannelGroup for ChannelGroup<K, V> {
    fn introspect(&self) -> HashMap<String, Vec<String>> {
        let mut result = HashMap::new();
        for (name, id) in self.channel_names.iter() {
            result.insert(
                name.into(),
                self.read_channel(*id)
                    .unwrap()
                    .iter()
                    .map(|x| format!("{:?}", x))
                    .collect(),
            );
        }
        result
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
            .create_channel::<VertexId, Vec3>("position".into())
            .unwrap();
        let color = mesh_channels
            .create_channel::<VertexId, Vec4>("color".into())
            .unwrap();
        let size = mesh_channels
            .create_channel::<VertexId, f32>("size".into())
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

            colors[v1] = Vec4::splat(0.0);
            colors[v2] = Vec4::splat(0.5);
            colors[v3] = Vec4::splat(1.0);

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

            assert_eq!(colors[v1], Vec4::splat(0.0));
            assert_eq!(colors[v2], Vec4::splat(0.5));
            assert_eq!(colors[v3], Vec4::splat(1.0));

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

        mesh_channels.introspect();
    }
}

// ------------- Boilerplate zone ------------

// NOTE: Slotmap requires a bunch of traits that we can't derive on our
// ChannelKey type because it's generic and has a PhantomData, which rust's std
// derives can't handle. A crate like `derivative` could be used here, but the
// extra dependency for a single usage is not justified.

impl<K: ChannelKey, V: ChannelValue> Clone for ChannelId<K, V> {
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            _phantom: self._phantom,
        }
    }
}
impl<K: ChannelKey, V: ChannelValue> Copy for ChannelId<K, V> {}
impl<K: ChannelKey, V: ChannelValue> Default for ChannelId<K, V> {
    fn default() -> Self {
        Self {
            data: Default::default(),
            _phantom: Default::default(),
        }
    }
}
impl<K: ChannelKey, V: ChannelValue> PartialEq for ChannelId<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self._phantom == other._phantom
    }
}
impl<K: ChannelKey, V: ChannelValue> Eq for ChannelId<K, V> {}
impl<K: ChannelKey, V: ChannelValue> Ord for ChannelId<K, V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.data.cmp(&other.data)
    }
}
impl<K: ChannelKey, V: ChannelValue> PartialOrd for ChannelId<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.data.partial_cmp(&other.data)
    }
}
impl<K: ChannelKey, V: ChannelValue> std::hash::Hash for ChannelId<K, V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}
impl<K: ChannelKey, V: ChannelValue> Debug for ChannelId<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelId")
            .field("data", &self.data)
            .finish()
    }
}

impl<K: ChannelKey, V: ChannelValue> Default for Channel<K, V> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
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

impl Default for MeshChannels {
    fn default() -> Self {
        Self {
            channels: TypeMap::new(),
        }
    }
}
