use crate::prelude::*;
use std::ops::Range;

use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionFragment {
    Range(Range<u32>),
    Single(u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionExpression {
    All,
    None,
    Explicit(Vec<SelectionFragment>),
}

pub enum SelectionKind {
    Vertices,
    Faces,
    Edges,
    HalfEdges,
}

impl SelectionExpression {
    /// Parses a [`SelectionFragments`] from a string input.
    ///
    /// Syntax Examples:
    /// ```ignore
    /// 0, 1, 2 // Select elements 0, 1 and 2
    /// * // Select all elements
    /// 0..1 // Select a range of elements
    /// 0..5, 7..10, 13, 17, 22 // Select multiple ranges, and some single faces
    ///  // (empty string), selects nothing
    /// ```
    pub fn parse(input: &str) -> Result<SelectionExpression> {
        use nom_prelude::*;
        fn str2int(s: &str) -> u32 {
            s.parse().unwrap()
        }

        fn number(input: &str) -> IResult<&str, u32> {
            map(digit1, str2int).parse(input)
        }

        fn single(input: &str) -> IResult<&str, SelectionFragment> {
            map(number, SelectionFragment::Single).parse(input)
        }

        fn range(input: &str) -> IResult<&str, SelectionFragment> {
            map(tuple((number, tag(".."), number)), |(x, _, y)| {
                SelectionFragment::Range(x..y)
            })
            .parse(input)
        }

        fn selection_fragment(input: &str) -> IResult<&str, SelectionFragment> {
            alt((range, single)).parse(input)
        }

        fn fragments_all(input: &str) -> IResult<&str, SelectionExpression> {
            map(tag("*"), |_| SelectionExpression::All).parse(input)
        }

        fn whitespace(input: &str) -> IResult<&str, ()> {
            map(many0(tag(" ")), |_| ()).parse(input)
        }

        fn separator(input: &str) -> IResult<&str, ()> {
            map(tuple((whitespace, tag(","), whitespace)), |_| ()).parse(input)
        }

        fn fragments_explicit(input: &str) -> IResult<&str, SelectionExpression> {
            map(
                separated_list1(separator, selection_fragment),
                SelectionExpression::Explicit,
            )
            .parse(input)
        }

        fn fragments(input: &str) -> IResult<&str, SelectionExpression> {
            map(
                tuple((whitespace, alt((fragments_all, fragments_explicit)))),
                |(_, res)| res,
            )
            .parse(input)
        }

        if input.trim().is_empty() {
            Ok(SelectionExpression::None)
        } else {
            fragments(input)
                .map_err(|err| anyhow::anyhow!("Error parsing selection: {}", err))
                .and_then(|(extra_input, parsed)| {
                    if !extra_input.trim().is_empty() {
                        anyhow::bail!("Extra input when parsing selection: '{extra_input}'")
                    } else {
                        Ok(parsed)
                    }
                })
        }
    }
}

pub enum ResolvedSelection<Id: slotmap::Key> {
    All,
    None,
    Explicit(Vec<Id>),
}

impl MeshConnectivity {
    fn resolve_explicit_selection<T: slotmap::Key, U>(
        data: &SlotMap<T, U>,
        fragments: SelectionExpression,
    ) -> ResolvedSelection<T> {
        match fragments {
            SelectionExpression::Explicit(ref fragments) => {
                let mut ids = vec![];
                // TODO: Optimize this
                for (i, (id, _)) in data.iter().enumerate() {
                    for fragment in fragments {
                        match fragment {
                            SelectionFragment::Range(r) if r.contains(&(i as u32)) => {
                                ids.push(id);
                            }
                            SelectionFragment::Single(s) if *s == i as u32 => {
                                ids.push(id);
                            }
                            _ => {}
                        }
                    }
                }
                ResolvedSelection::Explicit(ids)
            }
            SelectionExpression::All => ResolvedSelection::All,
            SelectionExpression::None => ResolvedSelection::None,
        }
    }

    pub fn resolve_face_selection(
        &self,
        fragments: SelectionExpression,
    ) -> ResolvedSelection<FaceId> {
        Self::resolve_explicit_selection(&self.faces, fragments)
    }

    pub fn resolve_face_selection_full(&self, fragments: SelectionExpression) -> Vec<FaceId> {
        match Self::resolve_explicit_selection(&self.faces, fragments) {
            ResolvedSelection::All => self.faces.iter().map(|(a, _)| a).collect(),
            ResolvedSelection::None => vec![],
            ResolvedSelection::Explicit(v) => v,
        }
    }

    pub fn resolve_vertex_selection(
        &self,
        fragments: SelectionExpression,
    ) -> ResolvedSelection<VertexId> {
        Self::resolve_explicit_selection(&self.vertices, fragments)
    }

    pub fn resolve_vertex_selection_full(&self, fragments: SelectionExpression) -> Vec<VertexId> {
        match Self::resolve_explicit_selection(&self.vertices, fragments) {
            ResolvedSelection::All => self.vertices.iter().map(|(a, _)| a).collect(),
            ResolvedSelection::None => vec![],
            ResolvedSelection::Explicit(v) => v,
        }
    }

    pub fn resolve_halfedge_selection(
        &self,
        fragments: SelectionExpression,
    ) -> ResolvedSelection<HalfEdgeId> {
        Self::resolve_explicit_selection(&self.halfedges, fragments)
    }

    pub fn resolve_halfedge_selection_full(
        &self,
        fragments: SelectionExpression,
    ) -> Vec<HalfEdgeId> {
        match Self::resolve_explicit_selection(&self.halfedges, fragments) {
            ResolvedSelection::All => self.halfedges.iter().map(|(a, _)| a).collect(),
            ResolvedSelection::None => vec![],
            ResolvedSelection::Explicit(v) => v,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn test_all() {
        assert_eq!(SelectionExpression::parse("*").unwrap(), SelectionExpression::All);
        assert_eq!(SelectionExpression::parse("   *").unwrap(), SelectionExpression::All);
        assert_eq!(SelectionExpression::parse("*   ").unwrap(), SelectionExpression::All);
        assert_eq!(SelectionExpression::parse("   *   ").unwrap(), SelectionExpression::All);
    }

    #[test]
    #[rustfmt::skip]
    fn test_none() {
        assert_eq!(SelectionExpression::parse("").unwrap(), SelectionExpression::None);
        assert_eq!(SelectionExpression::parse("   ").unwrap(), SelectionExpression::None);
    }

    #[test]
    #[rustfmt::skip]
    fn test_explicit() {
        use super::SelectionFragment::*;
        fn expl(v: &[SelectionFragment]) -> SelectionExpression {
            SelectionExpression::Explicit(v.to_vec())
        }
        
        assert_eq!(SelectionExpression::parse("1").unwrap(), expl(&[Single(1)]));
        assert_eq!(SelectionExpression::parse("1, 2, 3").unwrap(), expl(&[Single(1), Single(2), Single(3)]));
        assert_eq!(SelectionExpression::parse("1,2,3").unwrap(), expl(&[Single(1), Single(2), Single(3)]));
        assert_eq!(SelectionExpression::parse("1..5").unwrap(), expl(&[Range(1..5)]));
        assert_eq!(SelectionExpression::parse("1..5, 7..10, 15..16").unwrap(), 
            expl(&[Range(1..5), Range(7..10), Range(15..16)]));
        assert_eq!(SelectionExpression::parse("1..5, 7..10, 15..16, 18, 22, 27").unwrap(), 
            expl(&[Range(1..5), Range(7..10), Range(15..16), Single(18), Single(22), Single(27)]));
    }

    #[test]
    #[rustfmt::skip]
    fn test_error() {
        assert!(SelectionExpression::parse("1, *").is_err());
        assert!(SelectionExpression::parse("1 2 3").is_err());
        assert!(SelectionExpression::parse("*, 1").is_err());
        assert!(SelectionExpression::parse("1,2,3,a").is_err());
        assert!(SelectionExpression::parse("potato").is_err());
    }
}
