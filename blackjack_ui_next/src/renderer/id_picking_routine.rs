// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{collections::HashSet, num::NonZeroU32, ops::Range};

use glam::{IVec2, UVec2};

/// Splits the u32 range in several sub-regions for different types of ids. For
/// now, this is used to distinguish between subgizmo ids and primitive ids
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Hash, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct PickableId(u32);

impl PickableId {
    const MAX_PRIMITIVE_IDS: u32 = 2 << 30;

    /// Pickable primitives are stored from id 1 onwards. The maximum number of
    /// primitive ids is an implementation detail, but chosen to be large enough
    /// that any reasonable mesh would fit.
    pub const PRIMITIVE_ID_RANGE: Range<u32> = 1..Self::MAX_PRIMITIVE_IDS;

    /// Subgizmo ids are stored after primitive ids. A total of 16 possible
    /// subgizmos is possible.
    pub const SUBGIZMO_ID_RANGE: Range<u32> = Self::MAX_PRIMITIVE_IDS..Self::MAX_PRIMITIVE_IDS + 16;

    /// Constructs a new PickableId if the provided index falls inside one of
    /// the valid ranges. Otherwise returns None.
    pub fn new(index: u32) -> Option<Self> {
        if Self::PRIMITIVE_ID_RANGE.contains(&index) || Self::SUBGIZMO_ID_RANGE.contains(&index) {
            Some(Self(index))
        } else {
            None
        }
    }

    /// Creates a new subgizmo id. Subgizmo ids are used to pick parts of gizmos.
    pub fn new_subgizmo(idx: u32) -> Self {
        Self(idx + Self::SUBGIZMO_ID_RANGE.start)
    }

    /// Creates a new primitive id. Primitive ids are used to select parts of a
    /// mesh, such as halfedges, faces, or vertices.
    pub fn new_primitive(idx: u32) -> Self {
        Self(idx + Self::PRIMITIVE_ID_RANGE.start)
    }

    pub fn is_subgizmo(self) -> bool {
        Self::SUBGIZMO_ID_RANGE.contains(&self.0)
    }

    pub fn is_primitive(self) -> bool {
        Self::PRIMITIVE_ID_RANGE.contains(&self.0)
    }

    pub fn get_subgizmo_index(self) -> Option<u32> {
        if self.is_subgizmo() {
            Some(self.0 - Self::SUBGIZMO_ID_RANGE.start)
        } else {
            None
        }
    }
}

/// Allows filtering different pickable id types. This can be used by
/// interaction code to prioritize certain kinds of selections. For instance, to
/// allow selecting a gizmo even when the cursor is over a mesh face pixel (but
/// close to a gizmo).
pub enum PickableIdFilter {
    Subgizmos,
    Primitives,
    All,
}

impl PickableIdFilter {
    fn passes_filter(&self, index: PickableId) -> bool {
        match self {
            Self::Subgizmos => index.is_subgizmo(),
            Self::Primitives => index.is_primitive(),
            Self::All => true,
        }
    }
}

/// Different metrics for the offscreen buffer we copy from, and where the mouse
/// is located within that texture
#[derive(Clone, Copy)]
pub struct TextureBufferMetrics {
    /// Cursor position, in texels, relative to the origin of the 3d viewport.
    /// None when cursor is outside the 3d viewport.
    #[allow(unused)] // Will become useful later
    cursor_pos: UVec2,

    /// The cursor position inside the `output_buffer`. This will generally be
    /// the center, but might be different when the cursor is near the edges of
    /// the viewport, since we can't copy the texture sub-region centered around
    /// the cursor in that case.
    ///
    /// None when the cursor is outside the 3d viewport.
    cursor_pos_in_buffer: UVec2,

    /// The top-left corner of the region of the texture we will copy from, in
    /// texels. The size of the region is given by `IdPickingRoutine::SIZE`.
    tex_region_origin: UVec2,
}

pub struct IdPickingRoutine {
    /// Stores the result of running object picking: A grid of object ids (as
    /// u32) centered around the cursor position.
    output_buffer: wgpu::Buffer,
    /// If the mouse is over the 3d viewport, stores the metrics. See
    /// [`TextureBufferMetrics`]
    metrics: Option<TextureBufferMetrics>,
}

impl IdPickingRoutine {
    // The SIZE is chosen so that the amount of data that we copy is a multiple
    // of 256. This is a requirement to run copy_texture_to_buffer.
    const SIZE: u32 = 64;

    // The actual distance from the mouse cursor for which we want to check
    // hovered elements. Must be smaller than `Self::SIZE`
    const DISTANCE: u32 = 5;

    pub fn new(device: &wgpu::Device) -> Self {
        let size = Self::SIZE as u64;
        Self {
            output_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Id Picking Output Buffer"),
                size: size * size * std::mem::size_of::<u32>() as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            metrics: None,
        }
    }

    /// Updates the inner data about the cursor position, given the following
    /// parameters:
    ///
    /// - `window_cursor_pos`: Global cursor position, relative to the top-left
    /// corner of the window, in pixels.
    ///
    /// - `viewport_origin`: The top-left corner of the 3d viewport, relative to
    /// the top-left corner of the window, in pixels.

    /// - `viewport_size`: The size of the 3d viewport, in pixels.
    ///
    /// NOTE: This function assumes the resolution for the viewport3d render
    /// texture is exactly 1 texel per pixel. This is usually true within a
    /// 1-frame delay.
    pub fn set_cursor_pos(&mut self, window_cursor_pos: epaint::Pos2, viewport_rect: epaint::Rect) {
        if viewport_rect.contains(window_cursor_pos) {
            let cursor_pos_f = window_cursor_pos - viewport_rect.left_top();
            let cursor_pos = UVec2::new(cursor_pos_f.x as u32, cursor_pos_f.y as u32);

            let vw = viewport_rect.width() as i32;
            let vh = viewport_rect.height() as i32;
            let _vo = viewport_rect.left_top();

            // We will copy a SIZExSIZE region of the id map buffer to the CPU.
            // The logic below is used to compute the cursor position relative
            // to the top-left corner of that buffer.
            let center = IVec2::new(Self::SIZE as i32 / 2, Self::SIZE as i32 / 2);
            let mut in_buffer = center;
            let mut top_left = cursor_pos.as_ivec2() - center;
            let bottom_right = cursor_pos.as_ivec2() + center;

            if top_left.x < 0 {
                in_buffer.x += top_left.x;
                top_left.x -= top_left.x;
            }
            if top_left.y < 0 {
                in_buffer.y += top_left.y;
                top_left.y -= top_left.y;
            }
            if bottom_right.x > vw {
                let extra = bottom_right.x - vw;
                in_buffer.x += extra;
                top_left.x -= extra;
            }
            if bottom_right.y > vh {
                let extra = bottom_right.y - vh;
                in_buffer.y += extra;
                top_left.y -= extra;
            }

            debug_assert!(in_buffer.x >= 0 && in_buffer.y >= 0);
            debug_assert!(top_left.x >= 0 && top_left.y >= 0);

            self.metrics = Some(TextureBufferMetrics {
                cursor_pos,
                cursor_pos_in_buffer: in_buffer.as_uvec2(),
                tex_region_origin: top_left.as_uvec2(),
            });
        } else {
            self.metrics = None;
        }
    }

    pub fn run(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        resolution: UVec2,
        id_map: &wgpu::Texture,
    ) {
        // When the window is too small, we can't copy the buffer. We take the easy
        // workaround and simply don't run object picking logic in those cases.
        if resolution.x <= Self::SIZE || resolution.y <= Self::SIZE {
            return;
        }

        // We don't want to run this logic when the cursor is outside the viewport
        let Some(metrics) = self.metrics else {
            return;
        };

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: id_map,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    // This subtraction is guaranteed to be in bounds by
                    // the logic in `set_cursor_pos`.
                    x: metrics.tex_region_origin.x,
                    y: metrics.tex_region_origin.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: NonZeroU32::new(Self::SIZE * std::mem::size_of::<u32>() as u32),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: Self::SIZE,
                height: Self::SIZE,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Returns the current id (from the id map) that is currently under the
    /// mouse. For this to work, this routine has to be in the render graph and
    /// the method `set_cursor_pos` has to be called with the actual mouse
    /// position.
    ///
    /// The returned id is the same value as the one found on the id map. This
    /// value may have to be converted to map back to an actual mesh id.
    pub fn id_under_mouse(
        &self,
        device: &wgpu::Device,
        filter: PickableIdFilter,
    ) -> Option<PickableId> {
        if let Some(metrics) = self.metrics {
            // Grab a portion of the id map from the GPU, and check for all the
            // ids inside that window. The valid id that is closest to the mouse
            // cursor is the one we will pick
            let buffer_slice = self.output_buffer.slice(..);
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                if let Err(err) = result {
                    panic!("Error when mapping buffer: {err}");
                }
            });
            device.poll(wgpu::Maintain::Wait);
            let mapped = buffer_slice.get_mapped_range();
            let id_grid = bytemuck::cast_slice::<_, u32>(&mapped);

            let mut ids_set = HashSet::new();
            let mut min_id = None;
            let mut min_dist = u32::MAX;

            let cursor = metrics.cursor_pos_in_buffer;
            for i in 0..Self::SIZE {
                for j in 0..Self::SIZE {
                    let pos = UVec2::new(j, i);

                    fn manhattan(v1: UVec2, v2: UVec2) -> u32 {
                        v1.x.abs_diff(v2.x) + v1.y.abs_diff(v2.y)
                    }

                    let dist = manhattan(pos, cursor);

                    if dist <= Self::DISTANCE {
                        let idx = i * Self::SIZE + j;
                        if let Some(id) = PickableId::new(id_grid[idx as usize]) {
                            if filter.passes_filter(id) {
                                ids_set.insert(id);

                                if dist < min_dist {
                                    min_dist = dist;
                                    min_id = Some(id);
                                }
                            }
                        }
                    }
                }
            }

            if min_id.is_some() {
                // Write out id_grid as an image (using the image crate)
                let mut img = image::ImageBuffer::new(Self::SIZE as u32, Self::SIZE as u32);
                for i in 0..Self::SIZE {
                    for j in 0..Self::SIZE {
                        let idx = i * Self::SIZE + j;
                        let id = PickableId::new(id_grid[idx as usize]);

                        if metrics.cursor_pos_in_buffer == UVec2::new(j, i) {
                            // Cursor position
                            let color = image::Luma([255]);
                            img.put_pixel(j as u32, i as u32, color);
                        } else if let Some(id) = id {
                            // There are a total of 6 subgizmos, split color
                            // accordingly. Start at luma 50
                            let subgizmo_idx = id.get_subgizmo_index().unwrap();
                            let color = image::Luma([(50 + (subgizmo_idx * 50) % 255) as u8]);
                            img.put_pixel(j as u32, i as u32, color);
                        } else {
                            // Invalid id
                            let color = image::Luma([0]);
                            img.put_pixel(j as u32, i as u32, color);
                        }
                    }
                }
                img.save("/tmp/id_grid.png").unwrap();
            }

            drop(mapped);
            self.output_buffer.unmap();

            min_id
        } else {
            None
        }
    }
}
