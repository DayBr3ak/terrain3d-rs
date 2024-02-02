use godot::{engine::Image, prelude::*};

use crate::{log_debug, log_error, log_info, log_warn};
use super::utils::rs;
use super::generated_tex::GeneratedTex;

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Var)]
#[repr(usize)]
pub enum MapType {
    TYPE_HEIGHT = 0,
    TYPE_CONTROL = 1,
    TYPE_COLOR = 2,
    TYPE_MAX = 3,
}
impl MapType {
    pub fn ord(self) -> usize {
        return self as usize;
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Var)]
#[repr(i32)]
pub enum RegionSize {
    SIZE_64 = 64,
    SIZE_128 = 128,
    SIZE_256 = 256,
    SIZE_512 = 512,
    SIZE_1024 = 1024,
    SIZE_2048 = 2048,
}
impl RegionSize {
    pub fn ord(self) -> i32 {
        return self as i32;
    }
}

#[derive(GodotClass)]
#[class(tool,  base=Resource)]
pub struct Terrain3DStorage {
    base: Base<Resource>,
    // Storage Settings & flags
    version: real,
    modified: bool,
    save_16_bit: bool,
    region_size: RegionSize,
    region_sizev: Vector2i,

    // Stored Data
    height_range: Vector2,
    edited_area: Aabb,

    /**
	 * These arrays house all of the map data.
	 * The Image arrays are region_sized slices of all heightmap data. Their world
	 * location is tracked by region_offsets. The region data are combined into one large
	 * texture in generated_*_maps.
	 */
	region_map_dirty: bool,
	region_map: PackedInt32Array, // 16x16 Region grid with index into region_offsets (1 based array)
	region_offsets: Array<Vector2i>, // Array of active region coordinates
	height_maps: Array<Gd<Image>>,
	control_maps:  Array<Gd<Image>>,
	color_maps:  Array<Gd<Image>>,

	// Generated Texture RIDs
	// These contain the TextureLayered RID from the RenderingServer, no Image
	generated_height_maps: GeneratedTex,
	generated_control_maps: GeneratedTex,
	generated_color_maps: GeneratedTex,
}

#[godot_api]
impl IResource for Terrain3DStorage {
    fn init(base: Base<Resource>) -> Self {
        Self {
            base,
            version: real!(0.8),
            modified: false,
            save_16_bit: false,
            region_size: RegionSize::SIZE_1024,
            region_sizev: Vector2i::new( RegionSize::SIZE_1024.ord(),  RegionSize::SIZE_1024.ord()),
            height_range: Vector2::ZERO,
            edited_area: Aabb::default(),
            region_map_dirty: true,
            region_map: PackedInt32Array::new(),
            region_offsets: Array::new(),
            height_maps: Array::new(),
            control_maps: Array::new(),
            color_maps: Array::new(),
            generated_height_maps: GeneratedTex::default(),
            generated_color_maps: GeneratedTex::default(),
            generated_control_maps: GeneratedTex::default(),
        }
    }
}

impl Drop for Terrain3DStorage {
    fn drop(&mut self) {
        self.clear();
    }
}


#[godot_api]
impl Terrain3DStorage {
    const __CLASS__: &'static str = "Terrain3DStorage";
    pub const CURRENT_VERSION: real = 0.842;
    const REGION_MAP_SIZE: i32 = 16;
    const REGION_MAP_VSIZE: Vector2i = Vector2i { x: Self::REGION_MAP_SIZE, y: Self::REGION_MAP_SIZE };

    ///////////////////////////
    // Private Functions
    ///////////////////////////
    fn clear(&mut self) {
        log_info!(Self, "Clearing storage");
        self.region_map_dirty = true;
        self.region_map.clear();
        self.generated_height_maps.clear();
        self.generated_control_maps.clear();
        self.generated_color_maps.clear();
    }

    ///////////////////////////
    // Public Functions
    ///////////////////////////

    // Lots of the upgrade process requires this to run first
    // It only runs if the version is saved in the file, which only happens if it was
    // different from the in the file is different from _version
    pub fn set_version(&mut self, p_version: real) {
        log_info!(Self, "{}", p_version);
        self.version = p_version;
        if p_version < Self::CURRENT_VERSION {
            log_warn!(Self, "Storage version {} will be updated to {} upon save", p_version, Self::CURRENT_VERSION);
            self.modified = true;
        }
    }

    pub fn get_region_size(&self) -> i32 {
        self.region_size.ord()
    }

    pub fn update_regions(&mut self, mut force_emit: bool) {
        if self.generated_height_maps.dirty() {
            log_debug!(Self, "Regenerating height layered texture from {} maps", self.height_maps.len());
            match GeneratedTex::create_from_layers(self.height_maps.clone()) {
                Some(x) => {
                    self.generated_height_maps = x;
                },
                None => {
                    log_error!(Self, "Could not create a height maps from stored value");
                    return;
                }
            }
            force_emit = true;
            self.modified = true;
            self.base_mut().emit_signal("height_maps_changed".into(), &[Variant::nil()]);
        }
    }
}
