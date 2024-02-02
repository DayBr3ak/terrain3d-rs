use std::collections::HashMap;

use godot::engine::fast_noise_lite::{
    CellularReturnType, DomainWarpFractalType, DomainWarpType, NoiseType,
};
use godot::engine::{FastNoiseLite, Gradient, INode3D, NoiseTexture2D, Resource, Shader, Texture};
use godot::prelude::*;

use crate::{log_debug, log_error, log_info};

use super::terrain_3d_core::{LogLevel, Terrain3D};
use super::utils::rs;

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Var)]
#[repr(i32)]
enum WorldBackground {
    None = 0,
    Flat = 1,
    Noise = 2,
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Var)]
#[repr(i32)]
enum TextureFiltering {
    Linear = 0,
    Nearest = 1,
}

#[derive(GodotClass)]
#[class(tool,  base=Resource)]
pub struct Terrain3DMaterial {
    base: Base<Resource>,
    initialized: bool,

    material: Rid,
    shader: Rid,

    shader_override_enable: bool,
    shader_override: Option<Gd<Shader>>,
    shader_tmp: Gd<Shader>,
    shader_code: HashMap<String, String>,
    active_params: Vec<String>,
    shader_params: HashMap<String, Variant>,

    // Material Features
    #[var]
    world_background: WorldBackground,
    #[var]
    texture_filtering: TextureFiltering,
    #[var]
    auto_shader: bool,
    #[var(get, set = set_dual_scaling)]
    #[export]
    dual_scaling: bool,

    // Editor Functions / Debug views
    show_navigation: bool,
    debug_view_checkered: bool,
    debug_view_grey: bool,
    debug_view_heightmap: bool,
    debug_view_colormap: bool,
    debug_view_roughmap: bool,
    debug_view_control_texture: bool,
    debug_view_control_blend: bool,
    debug_view_autoshader: bool,
    debug_view_holes: bool,
    debug_view_tex_height: bool,
    debug_view_tex_normal: bool,
    debug_view_tex_rough: bool,
    debug_view_vertex_grid: bool,

    #[var(get, set = set_region_size)]
    region_size: i32,
    region_sizev: Vector2i,
}

#[godot_api]
impl IResource for Terrain3DMaterial {
    fn init(base: Base<Resource>) -> Self {
        Self {
            base,
            initialized: false,
            shader_override_enable: false,
            material: Rid::Invalid,
            shader: Rid::Invalid,
            shader_code: HashMap::new(),
            shader_override: None,
            shader_tmp: Gd::default(),
            active_params: Vec::new(),
            shader_params: HashMap::new(),

            world_background: WorldBackground::Flat,
            texture_filtering: TextureFiltering::Linear,
            auto_shader: false,
            dual_scaling: false,

            show_navigation: false,
            debug_view_checkered: false,
            debug_view_grey: false,
            debug_view_heightmap: false,
            debug_view_colormap: false,
            debug_view_roughmap: false,
            debug_view_control_texture: false,
            debug_view_control_blend: false,
            debug_view_autoshader: false,
            debug_view_holes: false,
            debug_view_tex_height: false,
            debug_view_tex_normal: false,
            debug_view_tex_rough: false,
            debug_view_vertex_grid: false,

            region_size: 1024,
            region_sizev: Vector2i::new(1024, 1024),
        }
    }
}

#[godot_api]
impl Terrain3DMaterial {
    const __CLASS__: &'static str = "Terrain3DMaterial";
    // pub fn init_internal() -> Gd<Self> {
    //     let obj = Gd::from_init_fn(|base| {
    //         // accepts the base and returns a constructed object containing it
    //         Self {
    //             base,
    //             initialized: false,
    //             shader_override_enable: false,
    //             material: Rid::Invalid,
    //             shader: Rid::Invalid,
    //             shader_code: HashMap::new(),
    //             shader_override: Gd::default(),
    //             shader_tmp: Gd::default(),
    //         }
    //     });
    //     obj
    // }

    #[func]
    pub fn set_dual_scaling(&mut self, dual_scaling: bool) {
        log_debug!(Self, "New scaling: {dual_scaling}");
        self.dual_scaling = dual_scaling;
    }

    #[func]
    pub fn set_region_size(&mut self, region_size: i32) {
        log_debug!(Self, "Setting region size in material: {region_size}");

        self.region_size = region_size.clamp(64, 4096);
        self.region_sizev = Vector2i::new(self.region_size, self.region_size);

        rs().material_set_param(
            self.material,
            "_region_size".into(),
            Variant::from(self.region_size as f64),
        );
        rs().material_set_param(
            self.material,
            "_region_pixel_size".into(),
            Variant::from(1.0f64 / self.region_size as f64),
        );
    }

    pub fn initialize(&mut self, region_size: i32) {
        log_info!(Self, "Initializing material");
        self.preload_shaders();

        self.material = rs().material_create();
        self.shader = rs().shader_create();

        self.set_region_size(region_size);
        log_debug!(
            Self,
            "Mat RID: {}, _shader RID: {}",
            self.material,
            self.shader
        );

        self.initialized = true;
        self.update_shader();
    }

    #[func]
    fn update_shader(&mut self) {
        if !self.initialized {
            return;
        }

        log_info!(Self, "Updating Shader");
        let mut shader_rid: Option<Rid> = None;
        let shader_ov = if self.shader_override_enable {
            self.shader_override.as_mut()
        } else {
            None
        };

        if let Some(shader_override) = shader_ov {
            let mut shader_override = shader_override.clone();
            if shader_override.get_code().is_empty() {
                let code = self.generate_shader_code();
                shader_override.set_code(code.into());
            }

            let s = self.to_gd();
            let callable = s.callable("update_shader");

            if !shader_override.is_connected("changed".into(), callable.clone()) {
                log_debug!(Self, "Connecting changed signal to _update_shader()");
                shader_override.connect("changed".into(), callable);
            }
            let code = shader_override.get_code().to_string();
            let code = self.inject_editor_code(&code);
            self.shader_tmp.set_code(code.into());
            shader_rid = Some(self.shader_tmp.get_rid());
        } else {
            let code = self.generate_shader_code();
            let code = self.inject_editor_code(&code);
            rs().shader_set_code(self.shader, code.into());
            shader_rid = Some(self.shader);
        }

        if let Some(shader_rid) = shader_rid {
            rs().material_set_shader(self.material, shader_rid);
            log_debug!(
                Self,
                "Material rid: {}, shader rid: {}",
                self.material,
                shader_rid
            );
        }

        // Update custom shader params in RenderingServer
        {
            // Populate _active_params
            // List<PropertyInfo> pi;
            // _get_property_list(&pi);
            // LOG(DEBUG, "_active_params: ", _active_params);
            // Util::print_dict("_shader_params", _shader_params, DEBUG);
        };

        // Fetch saved shader parameters, converting textures to RIDs
        log_info!(Self, "Before setting texture to mats");
        for param in self.active_params.iter() {
            let value = self.shader_params.get(param);
            if let Some(value) = value {
                if value.get_type() == VariantType::Object {
                    let tex: Gd<Texture> = value.to();
                    if tex.is_instance_valid() {
                        let v_rid = Variant::from(tex.get_rid());
                        rs().material_set_param(self.material, param.into(), v_rid);
                    } else {
                        rs().material_set_param(self.material, param.into(), Variant::default());
                    }
                } else {
                    rs().material_set_param(self.material, param.into(), value.clone());
                }
            }
        }

        // Set specific shader parameters
        rs().material_set_param(
            self.material,
            "_background_mode".into(),
            Variant::from(self.world_background as i32),
        );

        // If no noise texture, generate one
        let noise_texture_name = "noise_texture".to_owned();
        if self.active_params.contains(&noise_texture_name)
            && rs()
                .material_get_param(self.material, noise_texture_name.into())
                .get_type()
                == VariantType::Nil
        {
            log_info!(Self, "Generating default noise_texture for shader");

            let mut fnoise = FastNoiseLite::new_gd();
            fnoise.set_noise_type(NoiseType::CELLULAR);
            fnoise.set_frequency(0.03);
            fnoise.set_cellular_jitter(3.0);
            fnoise.set_cellular_return_type(CellularReturnType::CELL_VALUE);
            fnoise.set_domain_warp_enabled(true);
            fnoise.set_domain_warp_type(DomainWarpType::SIMPLEX_REDUCED);
            fnoise.set_domain_warp_amplitude(50.0);
            fnoise.set_domain_warp_fractal_type(DomainWarpFractalType::INDEPENDENT);
            fnoise.set_domain_warp_fractal_lacunarity(1.5);
            fnoise.set_domain_warp_fractal_gain(1.0);

            let mut curve = Gradient::new_gd();
            let mut pfa = PackedFloat32Array::new();
            pfa.push(0.2);
            pfa.push(1.0);
            curve.set_offsets(pfa);
            let mut pca = PackedColorArray::new();
            pca.push(Color::from_rgba(1.0, 1.0, 1.0, 1.0));
            pca.push(Color::from_rgba(0.0, 0.0, 0.0, 1.0));
            curve.set_colors(pca);

            let mut noise_tex = NoiseTexture2D::new_gd();
            noise_tex.set_seamless(true);
            noise_tex.set_generate_mipmaps(true);
            noise_tex.set_noise(fnoise.upcast());
            noise_tex.set_color_ramp(curve);

            let pname = "noise_texture";
            let pname: StringName = pname.into();
            self.set(&pname, &noise_tex.to_variant());
        }

        //self.notify_property_list_changed();
    }

    fn set(&mut self, p_name: &StringName, p_property: &Variant) -> bool {
        let p_name0: String = p_name.into();
        if !self.initialized || self.active_params.contains(&p_name0) {
            self.base_mut().set(p_name.clone(), p_property.clone());
            return true;
        }

        if p_property.get_type() == VariantType::Nil {
            rs().material_set_param(self.material, p_name.clone(), Variant::default());
            return true;
        }

        // If value is an object, assume a Texture. RS only wants RIDs, but
        // Inspector wants the object, so set the RID and save the latter for _get
        if p_property.get_type() == VariantType::Object {
            let tex: Gd<Texture> = p_property.to();
            if tex.is_instance_valid() {
                let v_rid = Variant::from(tex.get_rid());
                rs().material_set_param(self.material, p_name.clone(), v_rid);
            } else {
                rs().material_set_param(self.material, p_name.clone(), Variant::default());
            }
        } else {
            self.shader_params.insert(p_name0, p_property.clone());
            rs().material_set_param(self.material, p_name.clone(), p_property.clone());
        }
        true
    }

    fn inject_editor_code(&self, p_shader: &str) -> String {
        let mut shader = p_shader.to_owned();
        let idx = p_shader.rfind("}");
        if idx.is_none() {
            return shader;
        }
        let mut idx = idx.unwrap();
        if idx < 1 {
            return shader;
        }
        let mut insert_names: Vec<&str> = Vec::new();
        if self.debug_view_checkered {
            insert_names.push("DEBUG_CHECKERED");
        }
        if self.debug_view_grey {
            insert_names.push("DEBUG_GREY");
        }
        if self.debug_view_heightmap {
            insert_names.push("DEBUG_HEIGHTMAP");
        }
        if self.debug_view_colormap {
            insert_names.push("DEBUG_COLORMAP");
        }
        if self.debug_view_roughmap {
            insert_names.push("DEBUG_ROUGHMAP");
        }
        if self.debug_view_control_texture {
            insert_names.push("DEBUG_CONTROL_TEXTURE");
        }
        if self.debug_view_control_blend {
            insert_names.push("DEBUG_CONTROL_BLEND");
        }
        if self.debug_view_autoshader {
            insert_names.push("DEBUG_AUTOSHADER");
        }
        if self.debug_view_tex_height {
            insert_names.push("DEBUG_TEXTURE_HEIGHT");
        }
        if self.debug_view_tex_normal {
            insert_names.push("DEBUG_TEXTURE_NORMAL");
        }
        if self.debug_view_tex_rough {
            insert_names.push("DEBUG_TEXTURE_ROUGHNESS");
        }
        if self.debug_view_vertex_grid {
            insert_names.push("DEBUG_VERTEX_GRID");
        }
        if self.show_navigation {
            insert_names.push("EDITOR_NAVIGATION");
        }

        for name in insert_names {
            let insert = self.shader_code.get(name);
            if let Some(insert) = insert {
                let x = "\n".to_owned() + insert;
                shader.insert_str(idx - 1, &x);
                idx += insert.len();
            }
        }

        shader
    }

    fn generate_shader_code(&self) -> String {
        log_info!(Self, "Generating default shader code");

        let mut excludes: Vec<&str> = Vec::new();
        if self.world_background != WorldBackground::Noise {
            excludes.push("WORLD_NOISE1");
            excludes.push("WORLD_NOISE2");
        }
        if self.texture_filtering == TextureFiltering::Linear {
            excludes.push("TEXTURE_SAMPLERS_NEAREST");
        } else {
            excludes.push("TEXTURE_SAMPLERS_LINEAR");
        }
        if self.auto_shader {
            excludes.push("TEXTURE_ID");
        } else {
            excludes.push("AUTO_SHADER_UNIFORMS");
            excludes.push("AUTO_SHADER_TEXTURE_ID");
        }
        if self.dual_scaling {
            excludes.push("UNI_SCALING_BASE");
        } else {
            excludes.push("DUAL_SCALING_UNIFORMS");
            excludes.push("DUAL_SCALING_VERTEX");
            excludes.push("DUAL_SCALING_BASE");
            excludes.push("DUAL_SCALING_OVERLAY");
        }

        let p_shader = self
            .shader_code
            .get("main")
            .expect("main shader parse error");
        self.apply_inserts(p_shader, excludes)
    }

    /**
     *	`//INSERT: ID` blocks in p_shader are replaced by the entry in the DB
     *	returns a shader string with inserts applied
     *  Skips `EDITOR_*` and `DEBUG_*` inserts
     */
    fn apply_inserts(&self, p_shader: &str, excludes: Vec<&str>) -> String {
        let parsed = p_shader.split("//INSERT:");
        let mut shader = "".to_owned();
        for (i, token) in parsed.enumerate() {
            // First section of the file before any //INSERT:
            if i == 0 {
                shader = token.into();
            } else {
                // There is at least one //INSERT:
                // Get the first ID on the first line
                let segment = token.splitn(2, "\n").collect::<Vec<_>>();
                // If there isn't an ID AND body, skip this insert
                if segment.len() < 2 {
                    continue;
                }
                let id = segment[0].trim();
                // Process the insert
                if !id.is_empty()
                    && !id.starts_with("DEBUG_")
                    && !id.starts_with("EDITOR_")
                    && !excludes.contains(&id)
                    && self.shader_code.contains_key(id)
                {
                    shader += &self.shader_code[id];
                }
                shader += segment[1];
            }
        }

        shader
    }

    fn preload_shaders(&mut self) {
        self.parse_shader(include_str!("shaders/uniforms.glsl"), "uniforms");
        self.parse_shader(include_str!("shaders/world_noise.glsl"), "world_noise");
        self.parse_shader(include_str!("shaders/auto_shader.glsl"), "auto_shader");
        self.parse_shader(include_str!("shaders/dual_scaling.glsl"), "dual_scaling");
        self.parse_shader(include_str!("shaders/debug_views.glsl"), "debug_views");
        self.parse_shader(
            include_str!("shaders/editor_functions.glsl"),
            "editor_functions",
        );

        self.shader_code
            .insert("main".into(), include_str!("shaders/main.glsl").into());

        if Terrain3D::debug_level() >= &LogLevel::DEBUG {
            for key in self.shader_code.keys() {
                log_debug!(Self, "Loaded shader insert: {}", key);
            }
        }
    }

    fn parse_shader(&mut self, p_shader: &str, p_name: &str) {
        if p_name.is_empty() {
            // push_error(Variant::from("No dictionary key for saving shader snippets specified"));
            log_error!(
                Self,
                "No dictionary key for saving shader snippets specified"
            );
            // godot_script_error!();
            return;
        }

        let parsed = p_shader.split("//INSERT:");

        for (i, token) in parsed.enumerate() {
            // First section of the file before any //INSERT:
            if i == 0 {
                self.shader_code.insert(p_name.into(), token.into());
            } else {
                // There is at least one //INSERT:
                // Get the first ID on the first line
                let segment = token.splitn(2, "\n").collect::<Vec<_>>();
                // If there isn't an ID AND body, skip this insert
                if segment.len() < 2 {
                    continue;
                }
                let id = segment[0].trim();
                // Process the insert
                if !id.is_empty() && !segment[1].is_empty() {
                    self.shader_code.insert(id.into(), segment[1].into());
                }
            }
        }
    }
}
