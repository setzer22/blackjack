// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

pub trait Location {}

impl Location for VertexId {}
impl Location for FaceId {}
impl Location for HalfEdgeId {}
impl Location for bool {}

#[derive(Copy, Clone, Debug)]
pub enum TraversalError {
    VertexHasNoHalfedge(VertexId),
    FaceHasNoHalfedge(FaceId),
    HalfEdgeHasNoNext(HalfEdgeId),
    HalfEdgeHasNoTwin(HalfEdgeId),
    HalfEdgeHasNoVertex(HalfEdgeId),
    HalfEdgeHasNoFace(HalfEdgeId),
    NoHalfedgeTo(VertexId),
    HalfedgeBadLoop(HalfEdgeId),
}
impl std::fmt::Display for TraversalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self:?}"))
    }
}
impl std::error::Error for TraversalError {}

#[derive(Clone, Copy)]
pub struct ValidTraversal<'a, L>
where
    L: Location,
{
    inner: &'a MeshConnectivity,
    location: L,
}

pub type Traversal<'a, L> = Result<ValidTraversal<'a, L>, TraversalError>;

/* ===================== */
/* Traversal on vertices */
/* ===================== */

pub trait VertexTraversal<'a> {
    fn halfedge(&'a self) -> Traversal<'a, HalfEdgeId>;
}

impl<'a> VertexTraversal<'a> for Traversal<'a, VertexId> {
    fn halfedge(&'a self) -> Traversal<'a, HalfEdgeId> {
        self.and_then(|valid| {
            Ok(ValidTraversal {
                inner: valid.inner,
                location: valid.inner[valid.location]
                    .halfedge
                    .ok_or(TraversalError::VertexHasNoHalfedge(valid.location))?,
            })
        })
    }
}

/* ================== */
/* Traversal on faces */
/* ================== */

pub trait FaceTraversal<'a> {
    fn halfedge(&'a self) -> Traversal<'a, HalfEdgeId>;
}
impl<'a> FaceTraversal<'a> for Traversal<'a, FaceId> {
    fn halfedge(&'a self) -> Traversal<'a, HalfEdgeId> {
        self.and_then(|valid| {
            Ok(ValidTraversal {
                inner: valid.inner,
                location: valid.inner[valid.location]
                    .halfedge
                    .ok_or(TraversalError::FaceHasNoHalfedge(valid.location))?,
            })
        })
    }
}

/* ====================== */
/* Traversal on halfedges */
/* ====================== */

pub trait HalfEdgeTraversal<'a> {
    fn twin(&'a self) -> Traversal<'a, HalfEdgeId>;
    fn next(&'a self) -> Traversal<'a, HalfEdgeId>;
    fn face(&'a self) -> Traversal<'a, FaceId>;
    fn vertex(&'a self) -> Traversal<'a, VertexId>;
    fn face_or_boundary(&'a self) -> Result<Option<FaceId>, TraversalError>;
}

impl<'a> HalfEdgeTraversal<'a> for Traversal<'a, HalfEdgeId> {
    fn twin(&'a self) -> Traversal<'a, HalfEdgeId> {
        self.and_then(|valid| {
            Ok(ValidTraversal {
                inner: valid.inner,
                location: valid.inner[valid.location]
                    .twin
                    .ok_or(TraversalError::HalfEdgeHasNoTwin(valid.location))?,
            })
        })
    }

    fn next(&'a self) -> Traversal<'a, HalfEdgeId> {
        self.and_then(|valid| {
            Ok(ValidTraversal {
                inner: valid.inner,
                location: valid.inner[valid.location]
                    .next
                    .ok_or(TraversalError::HalfEdgeHasNoNext(valid.location))?,
            })
        })
    }

    fn face(&'a self) -> Traversal<'a, FaceId> {
        self.and_then(|valid| {
            Ok(ValidTraversal {
                inner: valid.inner,
                location: valid.inner[valid.location]
                    .face
                    .ok_or(TraversalError::HalfEdgeHasNoFace(valid.location))?,
            })
        })
    }

    fn vertex(&'a self) -> Traversal<'a, VertexId> {
        self.and_then(|valid| {
            Ok(ValidTraversal {
                inner: valid.inner,
                location: valid.inner[valid.location]
                    .vertex
                    .ok_or(TraversalError::HalfEdgeHasNoVertex(valid.location))?,
            })
        })
    }

    fn face_or_boundary(&'a self) -> Result<Option<FaceId>, TraversalError> {
        self.and_then(|valid| Ok(valid.inner[valid.location].face))
    }
}

/* =================== */
/*  Generic traversal  */
/* =================== */

pub trait AnyTraversal<'a, L> {
    fn end(&'a self) -> L;
    fn try_end(&'a self) -> Result<L, TraversalError>;
}
impl<'a, L> AnyTraversal<'a, L> for Traversal<'a, L>
where
    L: Location + Copy,
{
    fn end(&'a self) -> L {
        self.map(|valid| valid.location)
            .unwrap_or_else(|err| panic!("Error during traversal: {err:?}"))
    }

    fn try_end(&'a self) -> Result<L, TraversalError> {
        self.map(|valid| valid.location)
    }
}

/* ============ */
/*  Initiators  */
/* ============ */

impl MeshConnectivity {
    pub fn at_halfedge(&self, halfedge_id: HalfEdgeId) -> Traversal<'_, HalfEdgeId> {
        Ok(ValidTraversal {
            inner: self,
            location: halfedge_id,
        })
    }

    pub fn at_face(&self, face_id: FaceId) -> Traversal<'_, FaceId> {
        Ok(ValidTraversal {
            inner: self,
            location: face_id,
        })
    }

    pub fn at_vertex(&self, vertex_id: VertexId) -> Traversal<'_, VertexId> {
        Ok(ValidTraversal {
            inner: self,
            location: vertex_id,
        })
    }
}

/* ================ */
/*  Vertex Helpers  */
/* ================ */

pub trait VertexTraversalHelpers<'a> {
    fn outgoing_halfedges(&'a self) -> Result<SVec<HalfEdgeId>, TraversalError>;
    fn incoming_halfedges(&'a self) -> Result<SVec<HalfEdgeId>, TraversalError>;
    fn halfedge_to(&self, other: VertexId) -> Traversal<HalfEdgeId>;
    fn adjacent_faces(&self) -> Result<SVec<FaceId>, TraversalError>;
}

impl<'a> VertexTraversalHelpers<'a> for Traversal<'a, VertexId> {
    fn outgoing_halfedges(&'a self) -> Result<SVec<HalfEdgeId>, TraversalError> {
        self.and_then(|valid| {
            let mut halfedges = SVec::new();
            // Could be a disconnected vertex. Return an empty list in that case.
            if let Some(h0) = valid.inner[valid.location].halfedge {
                let mut h = h0;
                loop {
                    halfedges.push(h);
                    h = valid.inner.at_halfedge(h).cycle_around_fan().try_end()?;
                    if h == h0 {
                        break;
                    }
                }
            }
            Ok(halfedges)
        })
    }

    fn incoming_halfedges(&'a self) -> Result<SVec<HalfEdgeId>, TraversalError> {
        self.and_then(|valid| {
            self.outgoing_halfedges()?
                .iter()
                .map(|h| {
                    valid.inner[*h]
                        .twin
                        .ok_or(TraversalError::HalfEdgeHasNoTwin(*h))
                })
                .collect()
        })
    }

    /// Returns the halfedge that goes from the current vertex to `other`,
    /// if any.
    fn halfedge_to(&self, other: VertexId) -> Traversal<HalfEdgeId> {
        self.and_then(|valid| {
            let h_to = self
                .outgoing_halfedges()?
                .into_iter()
                .find(|&h| {
                    valid
                        .inner
                        .at_halfedge(h)
                        .dst_vertex()
                        .try_end()
                        .map(|v| v == other)
                        .unwrap_or(false)
                })
                .ok_or(TraversalError::NoHalfedgeTo(other))?;
            Ok(ValidTraversal {
                inner: valid.inner,
                location: h_to,
            })
        })
    }

    /// Returns the polygon fan around this vertex.
    fn adjacent_faces(&self) -> Result<SVec<FaceId>, TraversalError> {
        self.and_then(|valid| {
            Ok(self
                .outgoing_halfedges()?
                .into_iter()
                // NOTE: Skip halfedges without a face. This is not an error,
                // just halfedges that lie on the boundary.
                .filter_map(|h| valid.inner.at_halfedge(h).face().try_end().ok())
                .collect::<SVec<_>>())
        })
    }
}

/* ============== */
/*  Face Helpers  */
/* ============== */

pub trait FaceTraversalHelpers<'a> {
    fn halfedges(&'a self) -> Result<SVec<HalfEdgeId>, TraversalError>;
    fn vertices(&'a self) -> Result<SVec<VertexId>, TraversalError>;
}

impl<'a> FaceTraversalHelpers<'a> for Traversal<'a, FaceId> {
    fn halfedges(&'a self) -> Result<SVec<HalfEdgeId>, TraversalError> {
        self.and_then(|valid| {
            let mut halfedges = SVec::new();
            let h0 = self.halfedge().try_end()?;
            let mut h = h0;
            loop {
                halfedges.push(h);
                h = valid.inner.at_halfedge(h).next().try_end()?;
                if h == h0 {
                    break;
                }
            }
            Ok(halfedges)
        })
    }

    fn vertices(&'a self) -> Result<SVec<VertexId>, TraversalError> {
        self.and_then(|valid| {
            self.halfedges()?
                .iter()
                .map(|h| valid.inner.at_halfedge(*h).vertex().try_end())
                .collect::<Result<SVec<_>, TraversalError>>()
        })
    }
}

/* ================== */
/*  Halfedge Helpers  */
/* ================== */

pub trait HalfedgeTraversalHelpers<'a> {
    fn cycle_around_fan(&'a self) -> Traversal<HalfEdgeId>;
    fn src_vertex(&'a self) -> Traversal<VertexId>;
    fn dst_vertex(&'a self) -> Traversal<VertexId>;
    fn src_dst_pair(&'a self) -> Result<(VertexId, VertexId), TraversalError>;
    fn is_boundary(&'a self) -> Result<bool, TraversalError>;
    fn previous(&'a self) -> Traversal<HalfEdgeId>;
}
impl<'a> HalfedgeTraversalHelpers<'a> for Traversal<'a, HalfEdgeId> {
    fn cycle_around_fan(&'a self) -> Traversal<HalfEdgeId> {
        self.and_then(|valid| {
            Ok(ValidTraversal {
                inner: valid.inner,
                location: self.twin().next().try_end()?,
            })
        })
    }

    fn src_vertex(&'a self) -> Traversal<VertexId> {
        self.vertex()
    }

    fn dst_vertex(&'a self) -> Traversal<VertexId> {
        self.and_then(|valid| {
            Ok(ValidTraversal {
                inner: valid.inner,
                location: self.next().vertex().try_end()?,
            })
        })
    }

    fn src_dst_pair(&'a self) -> Result<(VertexId, VertexId), TraversalError> {
        Ok((self.src_vertex().try_end()?, self.dst_vertex().try_end()?))
    }

    fn is_boundary(&'a self) -> Result<bool, TraversalError> {
        match self {
            Ok(valid) => Ok(valid.inner.at_halfedge(valid.location).face().is_err()),
            Err(err) => Err(*err),
        }
    }

    fn previous(&'a self) -> Traversal<HalfEdgeId> {
        self.and_then(|valid| {
            let h_loop = valid.inner.halfedge_loop(valid.location);
            Ok(ValidTraversal {
                inner: valid.inner,
                location: *h_loop
                    .last()
                    .ok_or(TraversalError::HalfedgeBadLoop(valid.location))?,
            })
        })
    }
}
