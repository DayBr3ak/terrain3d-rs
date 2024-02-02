use crate::{log_debug, log_info};
use godot::engine::mesh::ArrayType;
use godot::engine::rendering_server::PrimitiveType;
use godot::prelude::*;

use super::utils::rs;

pub struct GeoClipMap {}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Var)]
#[repr(usize)]
pub enum MeshType {
    TILE = 0,
    FILLER = 1,
    TRIM = 2,
    CROSS = 3,
    SEAM = 4,
}

impl MeshType {
    pub fn ord(self) -> usize {
        return self as usize;
    }
}

impl GeoClipMap {
    const __CLASS__: &'static str = "Terrain3DGeoClipMap";

    #[inline]
    fn patch_2d(x: usize, y: usize, res: usize) -> i32 {
        (y * res + x) as i32
    }

    fn create_mesh(
        p_vertices: PackedVector3Array,
        p_indices: PackedInt32Array,
        p_aabb: &Aabb,
    ) -> Rid {
        let mut arrays: Array<Variant> = Array::new();
        let vertices_len = p_vertices.len();
        arrays.resize(ArrayType::MAX.ord() as usize);
        arrays.set(ArrayType::VERTEX.ord() as usize, Variant::from(p_vertices));
        arrays.set(ArrayType::INDEX.ord() as usize, Variant::from(p_indices));

        let mut normals = PackedVector3Array::new();
        normals.resize(vertices_len);
        normals.fill(Vector3::new(0.0, 1.0, 0.0));
        arrays.set(ArrayType::NORMAL.ord() as usize, Variant::from(normals));

        let mut tangents = PackedFloat32Array::new();
        tangents.resize(vertices_len * 4);
        tangents.fill(0.0);
        arrays.set(ArrayType::TANGENT.ord() as usize, Variant::from(tangents));

        log_debug!(Self, "Creating mesh via the Rendering server");
        let mesh = rs().mesh_create();
        rs().mesh_add_surface_from_arrays(mesh, PrimitiveType::TRIANGLES, arrays);

        log_debug!(
            Self,
            "Setting custom aabb: {}, {}",
            p_aabb.position,
            p_aabb.size
        );
        rs().mesh_set_custom_aabb(mesh, *p_aabb);
        mesh
    }

    pub fn generate(p_size: i32, p_levels: i32) -> Vec<Rid> {
        log_info!(
            Self,
            "Generating meshes of size: {p_size}, levels: {p_levels}"
        );
        let tile_resolution = p_size as usize;
        let patch_vert_resolution = tile_resolution + 1;
        let clipmap_resolution = tile_resolution * 4 + 1;
        let clipmap_vert_resolution = clipmap_resolution + 1;
        let _num_clipmap_levels = p_levels as usize;
        let mut n = 0usize;

        // Create a tile mesh
        // A tile is the main component of terrain panels
        // LOD0: 4 tiles are placed as a square in each center quadrant, for a total of 16 tiles
        // LOD1..N 3 tiles make up a corner, 4 corners uses 12 tiles

        let (aabb, tile_mesh) = {
            let mut vertices = PackedVector3Array::default();
            vertices.resize(patch_vert_resolution * patch_vert_resolution);
            let mut indices = PackedInt32Array::default();
            indices.resize(tile_resolution * tile_resolution * 6);
            n = 0;

            let vertices_mut = vertices.as_mut_slice();
            for y in 0..patch_vert_resolution {
                for x in 0..patch_vert_resolution {
                    vertices_mut[n].x = x as f32;
                    vertices_mut[n].y = 0.0;
                    vertices_mut[n].z = y as f32;
                    n += 1;
                }
            }

            n = 0;
            let indices_mut = indices.as_mut_slice();
            for y in 0..tile_resolution {
                for x in 0..tile_resolution {
                    indices_mut[n] = Self::patch_2d(x, y, patch_vert_resolution);
                    n += 1;
                    indices_mut[n] = Self::patch_2d(x + 1, y + 1, patch_vert_resolution);
                    n += 1;
                    indices_mut[n] = Self::patch_2d(x, y + 1, patch_vert_resolution);
                    n += 1;

                    indices_mut[n] = Self::patch_2d(x, y, patch_vert_resolution);
                    n += 1;
                    indices_mut[n] = Self::patch_2d(x + 1, y, patch_vert_resolution);
                    n += 1;
                    indices_mut[n] = Self::patch_2d(x + 1, y + 1, patch_vert_resolution);
                    n += 1;
                }
            }

            let aabb = Aabb::new(
                Vector3::ZERO,
                Vector3::new(
                    patch_vert_resolution as f32,
                    0.1,
                    patch_vert_resolution as f32,
                ),
            );
            let tile_mesh = Self::create_mesh(vertices, indices, &aabb);

            (aabb, tile_mesh)
        };

        // Create a filler mesh
        // These meshes are small strips that fill in the gaps between LOD1+,
        // but only on the camera X and Z axes, and not on LOD0.
        let filler_mesh = {
            let mut vertices = PackedVector3Array::default();
            vertices.resize(patch_vert_resolution * 8);
            let mut indices = PackedInt32Array::default();
            indices.resize(tile_resolution * 24);
            n = 0;
            let offset = tile_resolution;

            let vertices_mut = vertices.as_mut_slice();
            for i in 0..patch_vert_resolution {
                vertices_mut[n].x = (offset + i + 1) as f32;
                vertices_mut[n].y = 0.0;
                vertices_mut[n].z = 0.0;
                aabb.expand(vertices_mut[n]);
                n += 1;

                vertices_mut[n].x = (offset + i + 1) as f32;
                vertices_mut[n].y = 0.0;
                vertices_mut[n].z = 1.0;
                aabb.expand(vertices_mut[n]);
                n += 1;
            }

            for i in 0..patch_vert_resolution {
                vertices_mut[n].x = 1.0;
                vertices_mut[n].y = 0.0;
                vertices_mut[n].z = (offset + i + 1) as f32;
                aabb.expand(vertices_mut[n]);
                n += 1;

                vertices_mut[n].x = 0.0;
                vertices_mut[n].y = 0.0;
                vertices_mut[n].z = (offset + i + 1) as f32;
                aabb.expand(vertices_mut[n]);
                n += 1;
            }

            for i in 0..patch_vert_resolution {
                vertices_mut[n].x = -((offset + i) as f32);
                vertices_mut[n].y = 0.0;
                vertices_mut[n].z = 1.0;
                aabb.expand(vertices_mut[n]);
                n += 1;

                vertices_mut[n].x = -((offset + i) as f32);
                vertices_mut[n].y = 0.0;
                vertices_mut[n].z = 0.0;
                aabb.expand(vertices_mut[n]);
                n += 1;
            }

            for i in 0..patch_vert_resolution {
                vertices_mut[n].x = 0.0;
                vertices_mut[n].y = 0.0;
                vertices_mut[n].z = -((offset + i) as f32);
                aabb.expand(vertices_mut[n]);
                n += 1;

                vertices_mut[n].x = 1.0;
                vertices_mut[n].y = 0.0;
                vertices_mut[n].z = -((offset + i) as f32);
                aabb.expand(vertices_mut[n]);
                n += 1;
            }

            n = 0;
            let indices_mut = indices.as_mut_slice();
            for i in 0..(tile_resolution * 4) as i32 {
                let arm = i / tile_resolution as i32;
                let bl = (arm + i) * 2 + 0;
                let br = (arm + i) * 2 + 1;
                let tl = (arm + i) * 2 + 2;
                let tr = (arm + i) * 2 + 3;

                if arm % 2 == 0 {
                    indices_mut[n] = br;
                    n += 1;
                    indices_mut[n] = bl;
                    n += 1;
                    indices_mut[n] = tr;
                    n += 1;
                    indices_mut[n] = bl;
                    n += 1;
                    indices_mut[n] = tl;
                    n += 1;
                    indices_mut[n] = tr;
                    n += 1;
                } else {
                    indices_mut[n] = br;
                    n += 1;
                    indices_mut[n] = bl;
                    n += 1;
                    indices_mut[n] = tl;
                    n += 1;
                    indices_mut[n] = br;
                    n += 1;
                    indices_mut[n] = tl;
                    n += 1;
                    indices_mut[n] = tr;
                    n += 1;
                }
            }

            // Filler mesh
            Self::create_mesh(vertices, indices, &aabb)
        };

        // Create trim mesh
        // This mesh is a skinny L shape that fills in the gaps between
        // LOD meshes when they are moving at different speeds and have gaps
        let trim_mesh = {
            let mut vertices = PackedVector3Array::default();
            vertices.resize((clipmap_vert_resolution * 2 + 1) * 2);
            let mut indices = PackedInt32Array::default();
            indices.resize((clipmap_vert_resolution * 2 - 1) * 6);
            n = 0;
            let offset = Vector3::new(
                0.5 * (clipmap_vert_resolution + 1) as f32,
                0.0,
                0.5 * (clipmap_vert_resolution + 1) as f32,
            );
            let vertices_mut = vertices.as_mut_slice();

            for i in 0..(clipmap_vert_resolution + 1) {
                vertices_mut[n] =
                    Vector3::new(0.0, 0.0, (clipmap_vert_resolution - i) as f32) - offset;
                aabb.expand(vertices_mut[n]);
                n += 1;

                vertices_mut[n] =
                    Vector3::new(1.0, 0.0, (clipmap_vert_resolution - i) as f32) - offset;
                aabb.expand(vertices_mut[n]);
                n += 1;
            }

            let start_of_horizontal = n as i32;
            for i in 0..clipmap_vert_resolution {
                vertices_mut[n] = Vector3::new((i + 1) as f32, 0.0, 0.0) - offset;
                aabb.expand(vertices_mut[n]);
                n += 1;

                vertices_mut[n] = Vector3::new((i + 1) as f32, 0.0, 1.0) - offset;
                aabb.expand(vertices_mut[n]);
                n += 1;
            }

            n = 0;
            let indices_mut = indices.as_mut_slice();
            for i in 0..clipmap_vert_resolution as i32 {
                indices_mut[n] = (i + 0) * 2 + 1;
                n += 1;
                indices_mut[n] = (i + 0) * 2 + 0;
                n += 1;
                indices_mut[n] = (i + 1) * 2 + 0;
                n += 1;

                indices_mut[n] = (i + 1) * 2 + 1;
                n += 1;
                indices_mut[n] = (i + 0) * 2 + 1;
                n += 1;
                indices_mut[n] = (i + 1) * 2 + 0;
                n += 1;
            }

            for i in 0..(clipmap_vert_resolution - 1) as i32 {
                indices_mut[n] = start_of_horizontal + (i + 0) * 2 + 1;
                n += 1;
                indices_mut[n] = start_of_horizontal + (i + 0) * 2 + 0;
                n += 1;
                indices_mut[n] = start_of_horizontal + (i + 1) * 2 + 0;
                n += 1;

                indices_mut[n] = start_of_horizontal + (i + 1) * 2 + 1;
                n += 1;
                indices_mut[n] = start_of_horizontal + (i + 0) * 2 + 1;
                n += 1;
                indices_mut[n] = start_of_horizontal + (i + 1) * 2 + 0;
                n += 1;
            }

            // Trim mesh
            Self::create_mesh(vertices, indices, &aabb)
        };

        // Create center cross mesh
        // This mesh is the small cross shape that fills in the gaps along the
        // X and Z axes between the center quadrants on LOD0.
        let cross_mesh = {
            let mut vertices = PackedVector3Array::default();
            vertices.resize(patch_vert_resolution * 8);
            let mut indices = PackedInt32Array::default();
            indices.resize(tile_resolution * 24 + 6);
            n = 0;
            let vertices_mut = vertices.as_mut_slice();
            for i in 0..(patch_vert_resolution * 2) as i32 {
                vertices_mut[n] = Vector3::new((i - tile_resolution as i32) as f32, 0.0, 0.0);
                aabb.expand(vertices_mut[n]);
                n += 1;

                vertices_mut[n] = Vector3::new((i - tile_resolution as i32) as f32, 0.0, 1.0);
                aabb.expand(vertices_mut[n]);
                n += 1;
            }

            let start_of_vertical = n as i32;
            for i in 0..(patch_vert_resolution * 2) as i32 {
                vertices_mut[n] = Vector3::new(0.0, 0.0, (i - tile_resolution as i32) as f32);
                aabb.expand(vertices_mut[n]);
                n += 1;

                vertices_mut[n] = Vector3::new(1.0, 0.0, (i - tile_resolution as i32) as f32);
                aabb.expand(vertices_mut[n]);
                n += 1;
            }

            n = 0;
            let indices_mut = indices.as_mut_slice();
            for i in 0..(tile_resolution * 2 + 1) as i32 {
                let bl = i * 2 + 0;
                let br = i * 2 + 1;
                let tl = i * 2 + 2;
                let tr = i * 2 + 3;

                indices_mut[n] = br;
                n += 1;
                indices_mut[n] = bl;
                n += 1;
                indices_mut[n] = tr;
                n += 1;
                indices_mut[n] = bl;
                n += 1;
                indices_mut[n] = tl;
                n += 1;
                indices_mut[n] = tr;
                n += 1;
            }

            for i in 0..(tile_resolution * 2 + 1) as i32 {
                if i == tile_resolution as i32 {
                    continue;
                }
                let bl = i * 2 + 0;
                let br = i * 2 + 1;
                let tl = i * 2 + 2;
                let tr = i * 2 + 3;

                indices_mut[n] = start_of_vertical + br;
                n += 1;
                indices_mut[n] = start_of_vertical + tr;
                n += 1;
                indices_mut[n] = start_of_vertical + bl;
                n += 1;
                indices_mut[n] = start_of_vertical + bl;
                n += 1;
                indices_mut[n] = start_of_vertical + tr;
                n += 1;
                indices_mut[n] = start_of_vertical + tl;
                n += 1;
            }

            // Cross Mesh
            Self::create_mesh(vertices, indices, &aabb)
        };

        // Create seam mesh
        // This is a very thin mesh that is supposed to cover tiny gaps
        // between tiles and fillers when the vertices do not line up
        let seam_mesh = {
            let mut vertices = PackedVector3Array::default();
            vertices.resize(clipmap_vert_resolution * 4);
            let mut indices = PackedInt32Array::default();
            indices.resize(clipmap_vert_resolution * 6);
            n = 0;
            let vertices_mut = vertices.as_mut_slice();
            for i in 0..clipmap_vert_resolution as i32 {
                n = clipmap_resolution * 0 + i as usize;
                vertices_mut[n] = Vector3::new(i as f32, 0.0, 0.0);
                aabb.expand(vertices_mut[n]);

                n = clipmap_resolution * 1 + i as usize;
                vertices_mut[n] = Vector3::new(clipmap_vert_resolution as f32, 0.0, i as f32);
                aabb.expand(vertices_mut[n]);

                n = clipmap_resolution * 2 + i as usize;
                vertices_mut[n] = Vector3::new(
                    (clipmap_vert_resolution as i32 - i) as f32,
                    0.0,
                    clipmap_vert_resolution as f32,
                );
                aabb.expand(vertices_mut[n]);

                n = clipmap_resolution * 3 + i as usize;
                vertices_mut[n] =
                    Vector3::new(0.0, 0.0, (clipmap_vert_resolution as i32 - i) as f32);
                aabb.expand(vertices_mut[n]);
            }

            n = 0;
            let indices_mut = indices.as_mut_slice();
            for i in (0..(clipmap_vert_resolution * 4) as i32).step_by(2) {
                indices_mut[n] = i + 1;
                n += 1;
                indices_mut[n] = i;
                n += 1;
                indices_mut[n] = i + 2;
                n += 1;
            }
            indices_mut[indices_mut.len() - 1] = 0;

            // Seam Mesh
            Self::create_mesh(vertices, indices, &aabb)
        };

        // skirt mesh
        /*{
            real_t scale = real_t(1 << (NUM_CLIPMAP_LEVELS - 1));
            real_t fbase = real_t(tile_resolution << NUM_CLIPMAP_LEVELS);
            Vector2 base = -Vector2(fbase, fbase);

            Vector2 clipmap_tl = base;
            Vector2 clipmap_br = clipmap_tl + (Vector2(CLIPMAP_RESOLUTION, CLIPMAP_RESOLUTION) * scale);

            real_t big = 10000000.0;
            Array vertices = Array::make(
                Vector3(-1, 0, -1) * big,
                Vector3(+1, 0, -1) * big,
                Vector3(-1, 0, +1) * big,
                Vector3(+1, 0, +1) * big,
                Vector3(clipmap_tl.x, 0, clipmap_tl.y),
                Vector3(clipmap_br.x, 0, clipmap_tl.y),
                Vector3(clipmap_tl.x, 0, clipmap_br.y),
                Vector3(clipmap_br.x, 0, clipmap_br.y)
            );

            Array indices = Array::make(
                0, 1, 4, 4, 1, 5,
                1, 3, 5, 5, 3, 7,
                3, 2, 7, 7, 2, 6,
                4, 6, 0, 0, 6, 2
            );

            skirt_mesh = _create_mesh(PackedVector3Array(vertices), PackedInt32Array(indices));

        }*/

        vec![tile_mesh, filler_mesh, trim_mesh, cross_mesh, seam_mesh]
    }
}
