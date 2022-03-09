use crate::prelude::*;
use std::ops::Range;

use slotmap::SlotMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionFragment {
    Range(Range<u32>),
    Single(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionFragments {
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

pub struct Selection {
    pub kind: SelectionKind,
    pub fragments: SelectionFragments,
}

impl SelectionFragments {
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
    pub fn parse(input: &str) -> Result<SelectionFragments> {
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

        fn fragments_all(input: &str) -> IResult<&str, SelectionFragments> {
            map(tag("*"), |_| SelectionFragments::All).parse(input)
        }

        fn whitespace(input: &str) -> IResult<&str, ()> {
            map(many0(tag(" ")), |_| ()).parse(input)
        }

        fn separator(input: &str) -> IResult<&str, ()> {
            map(tuple((whitespace, tag(","), whitespace)), |_| ()).parse(input)
        }

        fn fragments_explicit(input: &str) -> IResult<&str, SelectionFragments> {
            map(
                separated_list1(separator, selection_fragment),
                SelectionFragments::Explicit,
            )
            .parse(input)
        }

        fn fragments(input: &str) -> IResult<&str, SelectionFragments> {
            map(
                tuple((whitespace, alt((fragments_all, fragments_explicit)))),
                |(_, res)| res,
            )
            .parse(input)
        }

        if input.trim().is_empty() {
            Ok(SelectionFragments::None)
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

impl Selection {
    pub fn new(kind: SelectionKind, input: &str) -> Result<Self> {
        Ok(Self {
            kind,
            fragments: SelectionFragments::parse(input)?,
        })
    }
}

pub enum ResolvedSelection<Id: slotmap::Key> {
    All,
    None,
    Explicit(Vec<Id>),
}

impl HalfEdgeMesh {
    fn resolve_explicit_selection<T: slotmap::Key, U>(
        data: &SlotMap<T, U>,
        fragments: SelectionFragments,
    ) -> ResolvedSelection<T> {
        match fragments {
            SelectionFragments::Explicit(ref fragments) => {
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
            SelectionFragments::All => ResolvedSelection::All,
            SelectionFragments::None => ResolvedSelection::None,
        }
    }

    pub fn resolve_face_selection(&self, fragments: SelectionFragments) -> ResolvedSelection<FaceId> {
        Self::resolve_explicit_selection(&self.faces, fragments)
    }

    pub fn resolve_vertex_selection(&self, fragments: SelectionFragments) -> ResolvedSelection<VertexId> {
        Self::resolve_explicit_selection(&self.vertices, fragments)
    }

    pub fn resolve_halfedge_selection(&self, fragments: SelectionFragments) -> ResolvedSelection<HalfEdgeId> {
        Self::resolve_explicit_selection(&self.halfedges, fragments)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn test_all() {
        assert_eq!(SelectionFragments::parse("*").unwrap(), SelectionFragments::All);
        assert_eq!(SelectionFragments::parse("   *").unwrap(), SelectionFragments::All);
        assert_eq!(SelectionFragments::parse("*   ").unwrap(), SelectionFragments::All);
        assert_eq!(SelectionFragments::parse("   *   ").unwrap(), SelectionFragments::All);
    }

    #[test]
    #[rustfmt::skip]
    fn test_none() {
        assert_eq!(SelectionFragments::parse("").unwrap(), SelectionFragments::None);
        assert_eq!(SelectionFragments::parse("   ").unwrap(), SelectionFragments::None);
    }

    #[test]
    #[rustfmt::skip]
    fn test_explicit() {
        use super::SelectionFragment::*;
        fn expl(v: &[SelectionFragment]) -> SelectionFragments {
            SelectionFragments::Explicit(v.to_vec())
        }
        
        assert_eq!(SelectionFragments::parse("1").unwrap(), expl(&[Single(1)]));
        assert_eq!(SelectionFragments::parse("1, 2, 3").unwrap(), expl(&[Single(1), Single(2), Single(3)]));
        assert_eq!(SelectionFragments::parse("1,2,3").unwrap(), expl(&[Single(1), Single(2), Single(3)]));
        assert_eq!(SelectionFragments::parse("1..5").unwrap(), expl(&[Range(1..5)]));
        assert_eq!(SelectionFragments::parse("1..5, 7..10, 15..16").unwrap(), 
            expl(&[Range(1..5), Range(7..10), Range(15..16)]));
        assert_eq!(SelectionFragments::parse("1..5, 7..10, 15..16, 18, 22, 27").unwrap(), 
            expl(&[Range(1..5), Range(7..10), Range(15..16), Single(18), Single(22), Single(27)]));
    }

    #[test]
    #[rustfmt::skip]
    fn test_error() {
        assert!(SelectionFragments::parse("1, *").is_err());
        assert!(SelectionFragments::parse("1 2 3").is_err());
        assert!(SelectionFragments::parse("*, 1").is_err());
        assert!(SelectionFragments::parse("1,2,3,a").is_err());
        assert!(SelectionFragments::parse("potato").is_err());
    }
}
