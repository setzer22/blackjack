use gdnative::{api::Resource, prelude::*};

pub fn load_resource<T>(path: &str) -> Option<Ref<T>>
where
    T: GodotObject<Memory = RefCounted> + SubClass<Resource>,
{
    let res = ResourceLoader::godot_singleton().load(path, "", false);
    res.and_then(|s| s.cast::<T>())
}

pub fn instance_preloaded<'a, T, P>(scene: Ref<PackedScene>, parent: &P) -> TRef<'a, T>
where
    T: GodotObject + SubClass<Node>,
    P: GodotObject + SubClass<Node>,
{
    unsafe {
        let parent = parent.upcast::<Node>();
        let instance = scene.assume_safe().instance(0).expect("Instance succeeds");
        parent.add_child(instance, false);
        return instance.assume_safe().cast::<T>().expect("Can cast");
    }
}

macro_rules! preload {
    ($symbol:ident, $res_type:ty, $path:literal) => {
        lazy_static::lazy_static! {
            pub static ref $symbol : gdnative::prelude::Ref<$res_type> = {
                match crate::helpers::load_resource::<$res_type>($path) {
                    Some(x) => x,
                    None => {
                        panic!("Could not preload resource {:?} ({:?}) at {:?}",
                               stringify!($symbol), stringify!($res_type), $path)
                    }
                }
            };
        }
    };
}

macro_rules! gdcall {
    ( $node:expr, $method:ident ) => {
        #[allow(unused_unsafe)] // Necessary because sometimes we can't check if we're already on an unsafe block.
        unsafe { $node.call(stringify!($method), &[]) } };
    ( $node:expr, $method:ident $(, $arg:expr )+) => {
        #[allow(unused_unsafe)]
        unsafe { $node.call(stringify!($method), &[ $( $arg.to_variant() , )+ ]) }
    };
}
