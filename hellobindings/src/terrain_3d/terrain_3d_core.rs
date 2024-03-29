use anyhow::{anyhow, Result};
use godot::engine::rendering_server::ShadowCastingSetting;
use godot::engine::utilities::printerr;
use godot::engine::{EditorScript, Engine, INode3D, Node, Node3D, Sprite2D, StaticBody3D};
use godot::prelude::*;

use crate::terrain_3d::geoclipmap::*;
use crate::terrain_3d::terrain_3d_material::Terrain3DMaterial;
use crate::terrain_3d::utils::rs;
use crate::{log_debug, log_error, log_info};

use super::terrain_3d_storage::Terrain3DStorage;

#[derive(Default)]
struct Instances {
    cross: Option<Rid>,
    tiles: Vec<Rid>,
    fillers: Vec<Rid>,
    trims: Vec<Rid>,
    seams: Vec<Rid>,
}

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct Terrain3D {
    base: Base<Node3D>,

    version: GString,
    is_inside_world: bool,
    initialized: bool,
    mesh_size: i32,
    mesh_lods: i32,

    storage: Option<Gd<Terrain3DStorage>>,
    material: Option<Gd<Terrain3DMaterial>>,
    texture_list: Option<Gd<Sprite2D>>,

    // Current editor or gameplay camera we are centering the terrain on.
    camera: Option<Gd<Camera3D>>,
    // X,Z Position of the camera during the previous snapping. Set to max real_t value to force a snap update.
    camera_last_position: Vector2,

    // Meshes and Mesh instances
    meshes: Vec<Rid>,
    data: Instances,

    // Renderer settings
    render_layers: u32,
    shadow_casting: ShadowCastingSetting,
    cull_margin: real,

    // Physics body and settings
    static_body: Rid,
    debug_static_body: Option<Gd<StaticBody3D>>,
    collision_enabled: bool,
    show_debug_collision: bool,
    collision_layer: u32,
    collision_mask: u32,
    collision_priority: real,
}

#[godot_api]
impl INode3D for Terrain3D {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            version: "0.0.1-dev".into(),
            is_inside_world: false,
            initialized: false,
            mesh_size: 48,
            mesh_lods: 7,
            storage: None,
            material: None,
            texture_list: None,
            camera: None,
            camera_last_position: Vector2::new(f32::MAX, f32::MAX),
            meshes: Vec::new(),
            data: Instances::default(),
            render_layers: 1,
            shadow_casting: ShadowCastingSetting::ON,
            cull_margin: 0.0,
            static_body: Rid::Invalid,
            debug_static_body: None,
            collision_enabled: true,
            show_debug_collision: false,
            collision_layer: 1,
            collision_mask: 1,
            collision_priority: 1.0,
        }
    }

    fn ready(&mut self) {
        match self.initialize() {
            Ok(_) => self.base_mut().set_process(true),
            Err(err) => {
                log_error!(Self, "{}", err);
            }
        }
    }

    fn process(&mut self, _delta: f64) {
        if !self.initialized {
            return;
        }

        // If the game/editor camera is not set, find it
        if self.camera().is_none() {
            log_debug!(Self, "camera is null, getting the current one");
            self.grab_camera();
        }

        // If camera has moved enough, re-center the terrain on it.
        if let Some(camera) = self.camera() {
            if camera.is_inside_tree() {
                let cam_pos = camera.get_global_position();
                let cam_pos_2d = Vector2::new(cam_pos.x, cam_pos.z);
                if self.camera_last_position.distance_to(cam_pos_2d) > 0.2 {
                    self.snap(cam_pos);
                    self.camera_last_position = cam_pos_2d;
                }
            }
        }
    }
}

static mut S_DEBUG_LEVEL: LogLevel = LogLevel::DEBUG;

#[godot_api]
impl Terrain3D {
    const __CLASS__: &'static str = "Terrain3DNode";
    pub fn debug_level() -> &'static LogLevel {
        unsafe { &S_DEBUG_LEVEL }
    }

    fn initialize(&mut self) -> Result<()> {
        log_info!(
            Self,
            "Checking material, storage, texture_list, signal, and mesh initialization"
        );

        // Make blank objects if needed
        if self.material.is_none() {
            log_debug!(Self, "Creating blank material");
            self.material = Some(
                Terrain3DMaterial::new_gd()
            );
        }
        if self.storage.is_none() {
            log_debug!(Self, "Creating blank storage");
            let mut st = Terrain3DStorage::new_gd();
            st.bind_mut().set_version(Terrain3DStorage::CURRENT_VERSION);
            self.storage = Some(
                st
            );
        }

        // Initialize the system
        if !self.initialized && /*self.is_inside_world &&*/ self.base().is_inside_tree() {
            log_debug!(Self, "inite");
            match (self.storage.as_mut(), self.material.as_mut()) {
                (Some(storage), Some(material)) => {
                    material.bind_mut().initialize(storage.bind().get_region_size());
                    storage.bind_mut().update_regions(true); // generate map arrays
                },
                _ => {
                    return Err(anyhow!("Storage or material not valid"));
                }
            }
            self.build()?;
            self.initialized = true;
        }
        Ok(())
    }

    fn camera(&self) -> Option<&Gd<Camera3D>> {
        if let Some(camera) = &self.camera {
            if !camera.is_instance_valid() {
                return None;
            }
            return self.camera.as_ref();
        }
        return None;
    }

    fn grab_camera(&mut self) {
        if Engine::singleton().is_editor_hint() {
            let editor_script = EditorScript::new_gd();
            let editor_interface = editor_script.get_editor_interface();
            let from_nodes = editor_interface
                .clone()
                .and_then(|x| x.get_editor_main_screen())
                .map(|x| x.get_children());

            let excluded_node = editor_interface.and_then(|x| x.get_edited_scene_root());

            if let Some(from_nodes) = from_nodes {
                let excluded_node = excluded_node.expect("Excluded node was None");
                let mut cam_array = Vec::<Gd<Camera3D>>::new();
                Self::find_cameras(from_nodes, &excluded_node, &mut cam_array);
                if !cam_array.is_empty() {
                    log_debug!(Self, "Connecting to the first editor camera");
                    self.camera = Some(cam_array[0].clone());
                }
            }
        } else {
            log_debug!(Self, "Connecting to the in-game viewport camera");
            self.camera = self.base().get_viewport().and_then(|v| v.get_camera_3d());
        }
        if self.camera.is_none() {
            self.base_mut().set_process(false);
            log_error!(Self, "Cannot find active camera. Stopping _process()");
        }
    }

    /**
     * Recursive helper function for _grab_camera().
     */
    fn find_cameras(
        from_nodes: Array<Gd<Node>>,
        excluded_node: &Gd<Node>,
        cam_array: &mut Vec<Gd<Camera3D>>,
    ) {
        let cam_str: GString = "Camera3D".into();
        for node in from_nodes.iter_shared() {
            if !node.eq(excluded_node) {
                Self::find_cameras(node.get_children(), excluded_node, cam_array);
            }
            if node.is_class(cam_str.clone()) {
                log_debug!(Self, "Found a Camera3D at: {}", node.get_path());
                let maybe_cam = node.try_cast::<Camera3D>();
                match maybe_cam {
                    Ok(cam) => cam_array.push(cam),
                    _ => {}
                }
            }
        }
    }

    /**
     * Centers the terrain and LODs on a provided position. Y height is ignored.
     */
    fn snap(&mut self, mut p_cam_pos: Vector3) {
        p_cam_pos.y = 0.0;
        let rotations = [0f64, 270., 90., 180.];

        log_debug!(Self, "Snapping terrain to: {:?}", p_cam_pos);

        let transform = Transform3D::new(Basis::default(), p_cam_pos.floor());
        if let Some(cross) = self.data.cross {
            rs().instance_set_transform(cross, transform);
        }

        let mut edge = 0;
        let mut tile = 0;

        for l in 0..self.mesh_lods as usize {
            let scale = (1 << l) as f32;

            let snapped_pos = (p_cam_pos / scale).floor() * scale;
            let tsize = (self.mesh_size << l) as f32;
            let tsize_1 = (self.mesh_size << (l + 1)) as f32;
            let tile_size = Vector3::new(tsize, 0.0, tsize);
            let base = snapped_pos - Vector3::new(tsize_1, 0.0, tsize_1);

            // Position tiles
            for x in 0..4 {
                for y in 0..4 {
                    if l != 0 && (x == 1 || x == 2) && (y == 1 || y == 2) {
                        continue;
                    }
                    let fill = Vector3::new(
                        if x >= 2 { 1.0 } else { 0. },
                        0.,
                        if y >= 2 { 1.0 } else { 0. },
                    ) * scale;
                    let tile_tl = base + Vector3::new(x as f32, 0., y as f32) * tile_size + fill;
                    //Vector3 tile_br = tile_tl + tile_size;
                    let mut transform =
                        Transform3D::default().scaled(Vector3::new(scale, 1., scale));
                    transform.origin = tile_tl;
                    rs().instance_set_transform(self.data.tiles[tile], transform);
                    tile += 1;
                }
            }

            let mut transform = Transform3D::default().scaled(Vector3::new(scale, 1., scale));
            transform.origin = snapped_pos;
            rs().instance_set_transform(self.data.fillers[l], transform);

            if l as i32 != self.mesh_lods - 1 {
                let next_scale = scale * 2.0;
                let next_snapped_pos = (p_cam_pos / next_scale).floor() * next_scale;
                // Position trims
                {
                    let tile_center = Vector3::new(scale, 0., scale) * 0.5 + snapped_pos;
                    let d = p_cam_pos - next_snapped_pos;

                    let r = 0;
                    let r = r | if d.x >= scale { 0 } else { 2 };
                    let r = r | if d.z >= scale { 0 } else { 1 };
                    let angle = utilities::deg_to_rad(rotations[r]) as f32;
                    let mut transform =
                        Transform3D::default().rotated(Vector3::new(0.0, 1.0, 0.0), -angle);
                    transform = transform.scaled(Vector3::new(scale, 1.0, scale));
                    transform.origin = tile_center;
                    rs().instance_set_transform(self.data.trims[edge], transform);
                }
                // Position seams
                {
                    let tsize_1 = (self.mesh_size << (l + 1)) as f32;
                    let next_base = next_snapped_pos - Vector3::new(tsize_1, 0.0, tsize_1);
                    let mut transform =
                        Transform3D::default().scaled(Vector3::new(scale, 1.0, scale));
                    transform.origin = next_base;
                    rs().instance_set_transform(self.data.seams[edge], transform);
                }
                edge += 1;
            }
        }
    }

    fn build(&mut self) -> Result<()> {
        if !self.base().is_inside_tree() && self.storage.is_none()
        {
            log_debug!(
                Self,
                "Not inside the tree or no valid storage, skipping build"
            );
            return Ok(());
        }
        log_info!(Self, "Building the terrain meshes");

        // Generate terrain meshes, lods, seams
        self.meshes = GeoClipMap::generate(self.mesh_size, self.mesh_lods);
        if self.meshes.is_empty() {
            return Err(anyhow!("{}:: Meshes are empty", "build"));
        }

        // Set the current terrain material on all meshes
        if let Some(mat) = self.material.clone() {
            let material_rid = mat.bind().get_material_rid();
            for rid in &self.meshes {
                rs().mesh_surface_set_material(rid.clone(), 0, material_rid);
            }
        } else {
            return Err(anyhow!("{}:: material is empty", "build"));
        }

        log_debug!(Self, "Creating mesh instances");
        // Get current visual scenario so the instances appear in the scene
        let scenario = self
            .base()
            .get_world_3d()
            .and_then(|w| Some(w.get_scenario()));
        if scenario.is_none() {
            return Err(anyhow!("{}:: Could not acquire world_3d scenario", "build"));
        }
        let scenario = scenario.unwrap();
        let cross = rs().instance_create2(self.meshes[MeshType::CROSS.ord()], scenario);
        rs().instance_geometry_set_cast_shadows_setting(cross, self.shadow_casting);
	    rs().instance_set_layer_mask(cross, self.render_layers);
        self.data.cross = Some(cross);

        for l in 0..self.mesh_lods {
            for x in 0..4 {
                for y in 0..4 {
                    if l != 0 && (x == 1 || x == 2) && (y == 1 || y == 2) {
                        continue;
                    }

                    let tile = rs().instance_create2(self.meshes[MeshType::TILE.ord()], scenario);
                    rs().instance_geometry_set_cast_shadows_setting(tile, self.shadow_casting);
                    rs().instance_set_layer_mask(tile, self.render_layers);
                    self.data.tiles.push(tile);
                }
            }

            let filler = rs().instance_create2(self.meshes[MeshType::FILLER.ord()], scenario);
            rs().instance_geometry_set_cast_shadows_setting(filler, self.shadow_casting);
            rs().instance_set_layer_mask(filler, self.render_layers);
            self.data.fillers.push(filler);

            if l != self.mesh_lods - 1 {
                let trim = rs().instance_create2(self.meshes[MeshType::TRIM.ord()], scenario);
                rs().instance_geometry_set_cast_shadows_setting(trim, self.shadow_casting);
                rs().instance_set_layer_mask(trim, self.render_layers);
                self.data.trims.push(trim);

                let seam = rs().instance_create2(self.meshes[MeshType::SEAM.ord()], scenario);
                rs().instance_geometry_set_cast_shadows_setting(seam, self.shadow_casting);
                rs().instance_set_layer_mask(seam, self.render_layers);
                self.data.seams.push(seam);
            }
        }

        // self.update_aabbs();
        // Force a snap update
	    self.camera_last_position = Vector2::new(real::MAX, real::MAX);

        Ok(())
    }

    fn update_aabbs(&mut self) {
        if self.meshes.is_empty() || self.storage.is_none() {
            log_debug!(Self, "Update AABB called before terrain meshes built. Returning.");
            return;
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum LogLevel {
    ERROR = 0,
    INFO = 1,
    DEBUG = 2,
}
