
use super::*;

#[blackjack_macros::blackjack_lua_module]
mod lua_api {
    use super::*;

    #[lua(under = "HalfEdgeMesh")]
    fn new() -> HalfEdgeMesh {
        HalfEdgeMesh::default()
    }

    #[lua_impl]
    impl HalfEdgeMesh {
        // WIP: I was converting this, but SVec is not available in Lua. I'd
        // still like to keep the forwarded impl if possible, so add a
        // #[lua(into)] that will simply insert an `.into()` call at the end of
        // the wrapper fn, and adjust the return type as needed. Can also be
        // used for input args since .into<T>() for T is a no-op
        #[lua]
        pub fn face_edges(&self, face_id: FaceId) -> SVec<HalfEdgeId>;
    }


}
pub use lua_api::*;
