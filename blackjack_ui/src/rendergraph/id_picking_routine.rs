use std::num::NonZeroU32;

use glam::IVec2;

use crate::prelude::*;

pub struct IdPickingRoutine {
    /// Stores the result of running object picking: A grid of object ids (as
    /// u32) centered around the cursor position.
    output_buffer: wgpu::Buffer,
    /// Cursor position, in texels, relative to the origin of the 3d viewport.
    /// None when cursor is outside the 3d viewport.
    cursor_pos: Option<UVec2>,
    /// The cursor position inside the `output_buffer`. This will generally be
    /// the center, but might be different when the cursor is near the edges of
    /// the viewport, since we can't copy the texture sub-region centered around
    /// the cursor in that case.
    ///
    /// None when the cursor is outside the 3d viewport.
    cursor_pos_in_buffer: Option<UVec2>,
}

impl IdPickingRoutine {
    // The SIZE is chosen so that the amount of data that we copy is a multiple
    // of 256. This is a requirement to run copy_texture_to_buffer.
    const SIZE: usize = 64;

    // The actual distance from the mouse cursor for which we want to check
    // hovered elements. Must be smaller than `Self::SIZE`
    const DISTANCE: f32 = 20.0;

    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            output_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Id Picking Output Buffer"),
                size: (Self::SIZE * Self::SIZE * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            cursor_pos: None,
            cursor_pos_in_buffer: None,
        }
    }

    /// Sets the cursor position, given the following parameters:
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
    pub fn set_cursor_pos(&mut self, window_cursor_pos: egui::Pos2, viewport_rect: egui::Rect) {
        if viewport_rect.contains(window_cursor_pos) {
            let cursor_pos_f = window_cursor_pos - viewport_rect.left_top();
            let cursor_pos = UVec2::new(cursor_pos_f.x as u32, cursor_pos_f.y as u32);

            // We will copy a SIZExSIZE region of the id map buffer to the CPU.
            // The logic below is used to compute the cursor position relative
            // to the top-left corner of that buffer.
            let mut in_buffer = cursor_pos.as_ivec2();
            let top_left = in_buffer - IVec2::new(Self::SIZE as i32 / 2, Self::SIZE as i32 / 2);
            let bottom_right = in_buffer + IVec2::new(Self::SIZE as i32 / 2, Self::SIZE as i32 / 2);
            let vw = viewport_rect.width() as i32 ;
            let vh = viewport_rect.height() as i32 ;

            if top_left.x < 0 {
                in_buffer.x += top_left.x;
            }
            if top_left.y < 0 {
                in_buffer.y += top_left.y;
            }
            if bottom_right.x > vw {
                in_buffer.x += bottom_right.x - vw;
            }
            if bottom_right.y > vh {
                in_buffer.y += bottom_right.y - vh;
            }


            self.cursor_pos = Some(cursor_pos);
            self.cursor_pos_in_buffer = Some(in_buffer.as_uvec2());
        } else {
            self.cursor_pos = None;
            self.cursor_pos_in_buffer = None;
        }
    }

    pub fn add_to_graph<'node>(
        &'node self,
        graph: &mut r3::RenderGraph<'node>,
        resolution: UVec2,
        id_map: r3::RenderTargetHandle,
    ) {
        // When the window is too small, we can't copy the buffer. We take the easy
        // workaround and simply don't run object picking logic in those cases.
        if resolution.x <= Self::SIZE as u32 || resolution.y <= Self::SIZE as u32 {
            return;
        }

        // We don't want to run this logic when the cursor is outside the viewport
        let cursor_pos = if let Some(cursor_pos) = self.cursor_pos {
            cursor_pos
        } else {
            return;
        };

        let mut builder = graph.add_node("Id Picking: Copy texture");
        let id_map = builder.add_render_target_input(id_map);
        let this_pt = builder.passthrough_ref(self);

        // Make sure this node won't get pruned
        builder.add_external_output();

        builder.build(
            move |pt, renderer, encoder_or_pass, temps, _ready, graph_data| {
                println!("Id picking copy buffer");
                let this = pt.get(this_pt);
                let commands = encoder_or_pass.get_encoder();
                let tex = graph_data.get_render_target_texture(id_map);

                commands.copy_texture_to_buffer(
                    wgpu::ImageCopyTexture {
                        texture: tex,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            // This subtraction is guaranteed to be in bounds by
                            // the logic in `set_cursor_pos`.
                            x: cursor_pos.x - Self::SIZE as u32 / 2,
                            y: cursor_pos.y - Self::SIZE as u32 / 2,
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::ImageCopyBuffer {
                        buffer: &this.output_buffer,
                        layout: wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: NonZeroU32::new(
                                (Self::SIZE * std::mem::size_of::<u32>()) as u32,
                            ),
                            rows_per_image: None,
                        },
                    },
                    wgpu::Extent3d {
                        width: Self::SIZE as u32,
                        height: Self::SIZE as u32,
                        depth_or_array_layers: 1,
                    },
                );
            },
        );
    }

    pub(crate) fn debug_print_results(&self, device: &wgpu::Device) {
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
        let mut min_id = 0;
        let mut min_dist = f32::INFINITY;

        let center = UVec2::new(Self::SIZE as u32 / 2, Self::SIZE as u32 / 2);
        for i in 0..Self::SIZE {
            for j in 0..Self::SIZE {
                let pos = UVec2::new(j as u32, i as u32);
                let dist = pos.as_vec2().distance(center.as_vec2());

                if dist <= Self::DISTANCE {
                    let idx = i * Self::SIZE + j;
                    let id = id_grid[idx];
                    ids_set.insert(id);

                    if dist < min_dist {
                        min_dist = dist;
                        min_id = id;
                    }
                }
            }
        }

        // WIP: The logic for the inner rect works (!!), but crashes at the
        // edges: On the top / left we crash with overflow. On the bottom /
        // right we get a wgpu validation error (buffer overrun).

        dbg!(ids_set.iter().sorted());
        dbg!(min_id);

        drop(mapped);
        self.output_buffer.unmap();
    }
}
