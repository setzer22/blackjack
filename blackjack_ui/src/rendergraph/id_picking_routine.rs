use std::{
    num::NonZeroU32,
    sync::{Arc, Mutex},
};

use crate::prelude::*;

pub fn add_to_graph<'node>(
    graph: &mut r3::RenderGraph<'node>,
    resolution: UVec2,
    id_map: r3::RenderTargetHandle,
) {
    // The SIZE is chosen so that the amount of data that we copy is a multiple
    // of 256. This is a requirement to run copy_texture_to_buffer below.
    const SIZE: usize = 64;

    // When the window is too small, we can't copy the buffer. We take the easy
    // workaround and simply don't run object picking logic in those cases.
    if resolution.x <= 64 || resolution.y <= 64 {
        return;
    }

    // The first node will create this buffer and copy the offscreen viewport
    // contents into it. The second node will map this buffer and access the data.
    let id_cpu_buffer = graph.add_data::<wgpu::Buffer>();

    let mut builder = graph.add_node("Id Picking: Copy texture");
    let id_map = builder.add_render_target_input(id_map);
    let id_cpu_buffer_handle = builder.add_data_output(id_cpu_buffer);

    builder.build(
        move |pt, renderer, encoder_or_pass, temps, _ready, graph_data| {
            println!("Id picking copy buffer");
            let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Id Picking Output Buffer"),
                size: (SIZE * SIZE * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            let commands = encoder_or_pass.get_encoder();
            let tex = graph_data.get_render_target_texture(id_map);
            commands.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: NonZeroU32::new((SIZE * std::mem::size_of::<u32>()) as u32),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: SIZE as u32,
                    height: SIZE as u32,
                    depth_or_array_layers: 1,
                },
            );

            graph_data.set_data(id_cpu_buffer_handle, Some(buffer));
        },
    );

    let mut builder = graph.add_node("Id Picking: Map buffer");
    let id_cpu_buffer_handle = builder.add_data_input(id_cpu_buffer);

    // Make sure this node won't be pruned
    builder.add_external_output();

    // WIP: Rend3 is not submitting between the two nodes. We have to introduce
    // the buffer from outside and read it after the graph is done executing.

    builder.build(
        move |pt, renderer, encoder_or_pass, temps, _ready, graph_data| {
            println!("Id picking get mapped buffer");
            let buffer = graph_data.get_data(temps, id_cpu_buffer_handle).unwrap();
            let buffer_slice = buffer.slice(..);
            let result_holder = Arc::new(Mutex::new(None));

            let result_holder2 = result_holder.clone();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                *result_holder2.lock().unwrap() = Some(result);
            });
            renderer.device.poll(wgpu::Maintain::Wait);
            let result = result_holder.lock().unwrap();
            if let Some(Ok(())) = &*result {
                let mapped = buffer_slice.get_mapped_range();
                let id_grid = bytemuck::cast_slice::<_, u32>(&mapped);

                let mut ids_set = HashSet::new();

                //for i in 0..SIZE {
                //for j in 0..SIZE {
                //let idx = i * SIZE + j;
                //ids_set.insert(id_grid[idx]);
                //}
                //}

                for id in id_grid {
                    ids_set.insert(id);
                }

                dbg!(ids_set);
            }

            buffer.unmap();
        },
    );
}
