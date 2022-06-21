pub use anyhow::{anyhow, bail, Context, Result};

pub use glam::{Mat4, Quat, UVec2, UVec3, Vec2, Vec3, Vec4};

pub use itertools::Itertools;
pub use std::collections::{HashMap, HashSet};

pub use crate::mesh::halfedge::*;
pub use crate::mesh::halfedge;

pub use blackjack_commons::math::*;
pub use blackjack_commons::utils::*;

pub mod nom_prelude {
    pub use nom::{
        branch::alt,
        bytes::complete::tag,
        character::complete::{alpha1, char, digit0, digit1, multispace0, multispace1, one_of},
        combinator::{cut, map, map_res, opt, recognize},
        error::{context, ErrorKind, VerboseError},
        multi::{many0, many1, separated_list0, separated_list1},
        sequence::{delimited, preceded, terminated, tuple},
        IResult, Parser,
    };
}
