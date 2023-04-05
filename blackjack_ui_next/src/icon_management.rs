use egui_wgpu::RenderState;
use epaint::{Pos2, Rect, TextureId, Vec2};
use image::DynamicImage;
use std::collections::HashMap;

use crate::renderer::texture_manager::TextureManager;

static ICON_ATLAS_TEXTURE_NAME: &str = "__icon_atlas";

pub struct IconAtlas {
    texture: TextureId,
    coords_map: HashMap<String, Rect>,
}

impl IconAtlas {
    /// Create a new icon atlas. Icon list is hard-coded in the source file.
    pub fn new(render_state: &RenderState, texture_manager: &mut TextureManager) -> Self {
        let mut icons: Vec<(String, DynamicImage)> = vec![];

        macro_rules! def_icon {
            ($name:expr, $path:expr) => {
                icons.push(($name.to_string(), {
                    image::load_from_memory(include_bytes!(concat!("../resources/icons/", $path)))
                        .expect("Wrong format for icon")
                }));
            };
        }

        def_icon!("close", "close.png");
        def_icon!("cursor", "cursor.png");
        def_icon!("floppy-disk", "floppy-disk.png");
        def_icon!("open-folder", "open-folder.png");

        let (image, coords_map) = Self::pack_icons(icons, 5);
        texture_manager.add_texture2d(ICON_ATLAS_TEXTURE_NAME.into(), image);
        let view = texture_manager
            .get_texture_view(ICON_ATLAS_TEXTURE_NAME)
            .unwrap();
        let texture_id = render_state.renderer.write().register_native_texture(
            &render_state.device,
            view,
            wgpu::FilterMode::Linear,
        );

        IconAtlas {
            texture: texture_id,
            coords_map,
        }
    }

    /// Packs a list of image icons into a single image texture, and returns a
    /// map of UV coordinates for every icon name.
    fn pack_icons(
        icons: Vec<(String, DynamicImage)>,
        padding: u32,
    ) -> (DynamicImage, HashMap<String, Rect>) {
        // Compute the size of the square texture atlas
        let num_icons = icons.len() as f32;
        let icon_size = icons[0].1.width(); // Assumes all icons have the same size
        let atlas_size = (num_icons.sqrt().ceil() as u32) * icon_size
            + padding * (num_icons.sqrt().ceil() as u32 - 1);

        // Create a new image for the texture atlas
        let mut atlas_img = DynamicImage::new_rgba8(atlas_size, atlas_size);
        let mut atlas = HashMap::new();

        // Pack the icons into the texture atlas
        let mut x = 0;
        let mut y = 0;
        for (icon_name, icon) in icons {
            let padded_icon =
                image::imageops::crop_imm(&icon, 0, 0, icon.width(), icon.height()).to_image();
            image::imageops::overlay(&mut atlas_img, &padded_icon, x as i64, y as i64);
            atlas.insert(
                icon_name,
                Rect::from_min_size(
                    Pos2::new(x as f32 / atlas_size as f32, y as f32 / atlas_size as f32),
                    Vec2::new(
                        icon_size as f32 / atlas_size as f32,
                        icon_size as f32 / atlas_size as f32,
                    ),
                ),
            );
            x += icon_size + padding;
            if x + icon_size > atlas_size {
                x = 0;
                y += icon_size + padding;
            }
        }

        // TODO DEBUG:
        atlas_img.save("/tmp/atlas.png").unwrap();

        (atlas_img, atlas)
    }

    /// Returns the UV region for the given icon name, or None if not found.
    pub fn get_icon(&self, icon_name: &str) -> Option<(TextureId, Rect)> {
        self.coords_map
            .get(icon_name)
            .cloned()
            .map(|x| (self.texture, x))
    }

    /// Returns the texture id of the icon atlas
    pub fn get_texture(&self) -> TextureId {
        self.texture
    }
}
