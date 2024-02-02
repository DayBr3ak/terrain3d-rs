use godot::engine::RenderingServer;
use godot::prelude::*;
use crate::{log_info};


pub struct GeoClipMap {}

impl GeoClipMap {
    const __CLASS__: &'static str = "Terrain3DGeoClipMap";

    #[inline]
    fn patch_2d(x: i32, y: i32, res: i32) -> i32 {
        y * res + x
    }

    pub fn generate(p_size: i32, p_levels: i32) -> Vec<Rid> {
        log_info!(
            Self,
            "Generating meshes of size: {p_size}, levels: {p_levels}"
        );
        dbg!(Self::__CLASS__);
        // TODO bit of a mess here. someone care to clean up?
        let tile_mesh: Rid;
        let filler_mesh: Rid;
        let trim_mesh: Rid;
        let cross_mesh: Rid;
        let seam_mesh: Rid;


        Vec::new()
    }
}
