use std::ops::Deref;

use godot::engine::rendering_server::TextureLayeredType;
use godot::engine::Image;
use godot::obj::Gd;
use godot::builtin::{Array, Rid};

use crate::log_debug;
use crate::terrain_3d::utils::rs;

use super::terrain_3d_core::{LogLevel, Terrain3D};

pub struct GeneratedTex {
    rid: Rid,
    image: Gd<Image>,
    dirty: bool,
}

impl Default for GeneratedTex {
    fn default() -> Self {
        Self {
            rid: Rid::Invalid,
            image: Gd::default(),
            dirty: false
        }
    }
}


impl GeneratedTex {
    const __CLASS__: &'static str = "Terrain3DGeneratedTex";

    pub fn create_from_layers(p_layers: Array<Gd<Image>>) -> Option<Self> {
        if p_layers.is_empty() {
            return None;
        }
        if Terrain3D::debug_level() >= &LogLevel::DEBUG {
            log_debug!(Self, "RenderingServer creating Texture2DArray, layers size: {}", p_layers.len());
            for (i, img) in p_layers.iter_shared().enumerate() {
                log_debug!(Self, "{i}: {img}, empty: {}, size: {}, format: {:?}", img.is_empty(), img.get_size(), img.get_format());
            }
        }
        Some(
            GeneratedTex {
                dirty: false,
                rid: rs().texture_2d_layered_create(p_layers, TextureLayeredType::LAYERED_2D_ARRAY),
                image: Gd::default(),
            }
        )
    }

    pub fn create_from_image(p_image: Gd<Image>) -> Self {
        log_debug!(Self, "RenderingServer creating Texture2D");
        GeneratedTex {
            rid: rs().texture_2d_create(p_image.clone()),
            dirty: false,
            image: p_image,
        }
    }

    pub fn clear(&mut self) {
        if self.rid.is_valid() {
            log_debug!(Self, "GeneratedTex freeing {}", self.rid);
            rs().free_rid(self.rid);
        }
        if self.image.is_instance_valid() {
            log_debug!(Self, "GeneratedTex unref image {}", self.image);
            self.image = Gd::default();
            // drop(self.image.);
        }
        self.rid = Rid::Invalid;
        self.dirty = true;
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn image(&self) -> Gd<Image> {
        self.image.clone()
    }

    pub fn rid(&self) -> Rid {
        self.rid
    }
}
