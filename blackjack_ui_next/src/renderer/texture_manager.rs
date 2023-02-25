use std::sync::Arc;

use epaint::ahash::HashMap;
use image::DynamicImage;
use wgpu::{util::DeviceExt, *};

pub struct TextureManager {
    pub textures: HashMap<String, Texture>,
    pub views: HashMap<String, TextureView>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
}

impl TextureManager {
    pub fn add_texture2d(&mut self, name: String, image: DynamicImage) {
        let size = Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };

        let desc = TextureDescriptor {
            label: Some("Blackjack Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::COPY_DST,
        };

        let texture = self
            .device
            .create_texture_with_data(&self.queue, &desc, &image.to_rgba8());

        let view = texture.create_view(&TextureViewDescriptor {
            label: None,
            format: Some(TextureFormat::Rgba8UnormSrgb),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        self.textures.insert(name.clone(), texture);
        self.views.insert(name, view);
    }

    pub fn get_texture(&self, name: &str) -> Option<&Texture> {
        self.textures.get(name)
    }

    pub fn get_texture_view(&self, name: &str) -> Option<&TextureView> {
        self.views.get(name)
    }
}
