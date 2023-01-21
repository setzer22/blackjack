// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::prelude::*;
use std::ops::Range;

use slotmap::SlotMap;

use std::fmt::Write;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionFragment {
    Group(String),
    Range(Range<u32>),
    Single(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
        use nom::character::complete::{alphanumeric1, anychar};
        use nom::combinator::verify;
        use nom::multi::many0_count;
        use nom::sequence::pair;
        use nom::{
            branch::alt,
            bytes::complete::tag,
            character::complete::{char, digit1},
            combinator::{map, opt, recognize},
            multi::{many0, separated_list1},
            sequence::{preceded, tuple},
            IResult, Parser,
        };

        fn str2int(s: &str) -> u32 {
            s.parse().unwrap()
        }

        fn number(input: &str) -> IResult<&str, u32> {
            map(digit1, str2int).parse(input)
        }

        // https://stackoverflow.com/a/61329008
        pub fn identifier<'a, E: nom::error::ParseError<&'a str>>(
            s: &'a str,
        ) -> IResult<&'a str, &'a str, E> {
            recognize(pair(
                verify(anychar, |&c| c.is_lowercase()),
                many0_count(preceded(opt(char('_')), alphanumeric1)),
            ))(s)
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

        fn group_fragment(input: &str) -> IResult<&str, SelectionFragment> {
            map(tuple((tag("@"), identifier)), |(_, y)| {
                SelectionFragment::Group(y.into())
            })
            .parse(input)
        }

        fn selection_fragment(input: &str) -> IResult<&str, SelectionFragment> {
            alt((group_fragment, range, single)).parse(input)
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

    pub fn unparse(&self) -> String {
        match self {
            SelectionExpression::All => "*".into(),
            SelectionExpression::None => "".into(),
            SelectionExpression::Explicit(segments) => {
                let mut out = String::new();
                let mut first = true;
                for segment in segments {
                    if first {
                        first = false;
                    } else {
                        write!(out, ", ").unwrap();
                    }
                    match segment {
                        SelectionFragment::Group(name) => write!(out, "@{name}").unwrap(),
                        SelectionFragment::Range(r) => {
                            write!(out, "{}..{}", r.start, r.end).unwrap()
                        }
                        SelectionFragment::Single(i) => write!(out, "{i}").unwrap(),
                    }
                }
                out
            }
        }
    }
}

pub enum ResolvedSelection<Id: slotmap::Key> {
    All,
    None,
    Explicit(Vec<Id>),
}

impl HalfEdgeMesh {
    fn resolve_explicit_selection<K: ChannelKey, V>(
        &self,
        data: &SlotMap<K, V>,
        fragments: &SelectionExpression,
    ) -> Result<ResolvedSelection<K>> {
        match fragments {
            SelectionExpression::Explicit(ref fragments) => {
                let mut ids = vec![];

                // TODO: Optimize this
                for (i, (id, _)) in data.iter().enumerate() {
                    for fragment in fragments {
                        match fragment {
                            SelectionFragment::Range(r) => {
                                if r.contains(&(i as u32)) {
                                    ids.push(id);
                                }
                            }
                            SelectionFragment::Single(s) => {
                                if *s == i as u32 {
                                    ids.push(id);
                                }
                            }
                            SelectionFragment::Group(group) => {
                                let group_ch =
                                    self.channels.read_channel_by_name::<K, bool>(group)?;
                                if group_ch[id] {
                                    ids.push(id);
                                }
                            }
                        }
                    }
                }
                Ok(ResolvedSelection::Explicit(ids))
            }
            SelectionExpression::All => Ok(ResolvedSelection::All),
            SelectionExpression::None => Ok(ResolvedSelection::None),
        }
    }

    pub fn resolve_face_selection(
        &self,
        fragments: &SelectionExpression,
    ) -> Result<ResolvedSelection<FaceId>> {
        let conn = self.read_connectivity();
        self.resolve_explicit_selection(&conn.faces, fragments)
    }

    pub fn resolve_face_selection_full(
        &self,
        fragments: &SelectionExpression,
    ) -> Result<Vec<FaceId>> {
        match self.resolve_face_selection(fragments)? {
            ResolvedSelection::All => Ok(self
                .read_connectivity()
                .faces
                .iter()
                .map(|(a, _)| a)
                .collect()),
            ResolvedSelection::None => Ok(vec![]),
            ResolvedSelection::Explicit(v) => Ok(v),
        }
    }

    pub fn resolve_vertex_selection(
        &self,
        fragments: &SelectionExpression,
    ) -> Result<ResolvedSelection<VertexId>> {
        let conn = self.read_connectivity();
        self.resolve_explicit_selection(&conn.vertices, fragments)
    }

    pub fn resolve_vertex_selection_full(
        &self,
        fragments: &SelectionExpression,
    ) -> Result<Vec<VertexId>> {
        match self.resolve_vertex_selection(fragments)? {
            ResolvedSelection::All => Ok(self
                .read_connectivity()
                .vertices
                .iter()
                .map(|(a, _)| a)
                .collect()),
            ResolvedSelection::None => Ok(vec![]),
            ResolvedSelection::Explicit(v) => Ok(v),
        }
    }

    pub fn resolve_halfedge_selection(
        &self,
        fragments: &SelectionExpression,
    ) -> Result<ResolvedSelection<HalfEdgeId>> {
        let conn = self.read_connectivity();
        self.resolve_explicit_selection(&conn.halfedges, fragments)
    }

    pub fn resolve_halfedge_selection_full(
        &self,
        fragments: &SelectionExpression,
    ) -> Result<Vec<HalfEdgeId>> {
        match self.resolve_halfedge_selection(fragments)? {
            ResolvedSelection::All => Ok(self
                .read_connectivity()
                .halfedges
                .iter()
                .map(|(a, _)| a)
                .collect()),
            ResolvedSelection::None => Ok(vec![]),
            ResolvedSelection::Explicit(v) => Ok(v),
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
        assert_eq!(SelectionExpression::parse("@test, 4, 3..5, @another").unwrap(), 
            expl(&[Group("test".into()), Single(4), Range(3..5), Group("another".into())]));
    }

    #[test]
    #[rustfmt::skip]
    fn test_error() {
        assert!(SelectionExpression::parse("1, *").is_err());
        assert!(SelectionExpression::parse("1 2 3").is_err());
        assert!(SelectionExpression::parse("*, 1").is_err());
        assert!(SelectionExpression::parse("1,2,3,a").is_err());
        assert!(SelectionExpression::parse("potato").is_err());
        assert!(SelectionExpression::parse("@1").is_err());
    }
}

#[blackjack_macros::blackjack_lua_module]
mod lua_api {
    use super::*;
    use anyhow::Result;

    /// Constructs a new Selection.
    /// TODO: Document selection DSL
    #[lua(under = "SelectionExpression")]
    fn new(expr: String) -> Result<SelectionExpression> {
        SelectionExpression::parse(&expr)
    }

    #[lua_impl]
    impl SelectionExpression {
        /// Returns a canonical string representation for this selection
        /// expression.
        #[lua]
        pub fn unparse(&self) -> String;
    }
}
