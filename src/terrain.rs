use bevy::{
    prelude::*,
    render::{
        mesh::{Indices},
        render_resource::{PrimitiveTopology},
    },
};


use noise::{Fbm, Perlin};
use bevy_rapier3d::prelude::*;
use std::collections::HashMap;
use bevy::asset::LoadState;

use noise::utils::{NoiseMapBuilder, PlaneMapBuilder};

use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{visible_block_faces, MergeVoxel, UnitQuadBuffer, Voxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG, OrientedBlockFace};
use rand::Rng;
use bevy_common_assets::json::JsonAssetPlugin;


const CHUNKS_COUNT_X: i32 = 32;
const CHUNKS_COUNT_Y: i32 = 32;
const CHUNKS_COUNT_Z: i32 = 32;

const CHUNK_SIZE: i32 = 32;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(JsonAssetPlugin::<TextureAtlas>::new(&["json"]))
            .add_startup_system(load_atlas)
            .add_system(asset_loaded)
        // .add_enter_system(GameState::LoadingBefore, load_atlas)
        // .add_enter_system(GameState::Loading, generate_world)
        ;
    }
}

#[derive(serde::Deserialize, bevy::reflect::TypeUuid)]
#[uuid = "413be529-bfeb-41b3-9db0-4b8b380a2c46"]
struct TextureAtlas {
    frames: HashMap<String, ImageDescription>,
    meta: Meta,
}


#[derive(serde::Deserialize, bevy::reflect::TypeUuid)]
#[uuid = "413be529-bfeb-41b3-9db0-4b8b380a2c47"]
struct Meta {
    size: Size,
}


#[derive(serde::Deserialize, bevy::reflect::TypeUuid)]
#[uuid = "413be529-bfeb-41b3-9db0-4b8b380a2c47"]
struct Size {
    w: f32,
    h: f32,
}


#[derive(serde::Deserialize, bevy::reflect::TypeUuid)]
#[uuid = "413be529-bfeb-41b3-9db0-4b8b380a2c47"]
struct ImageDescription {
    frame: Frame,
}

#[derive(serde::Deserialize, bevy::reflect::TypeUuid)]
#[uuid = "413be529-bfeb-41b3-9db0-4b8b380a2c48"]
struct Frame {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Resource, Clone)]
struct TextureAtlasHandle(Handle<TextureAtlas>);

#[derive(Component)]
struct Cube;

#[derive(Resource)]
struct AtlasLoading {
    loaded: bool,
    handle: TextureAtlasHandle,
}

fn load_atlas(asset_server: Res<AssetServer>, mut commands: Commands) {
    let handle = TextureAtlasHandle(asset_server.load("textures/spritesheet.json"));

    commands.insert_resource(handle.clone());
    commands.insert_resource(AtlasLoading { handle, loaded: false })
}


fn asset_loaded(
    commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlases: ResMut<Assets<TextureAtlas>>,
    mut atlas_loading: ResMut<AtlasLoading>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
) {
    if !atlas_loading.loaded
        && asset_server.get_load_state(atlas_loading.handle.0.clone_weak()) == LoadState::Loaded
    {
        if let Some(atlas) = atlases.remove(atlas_loading.handle.0.id()) {
            generate_world(commands, asset_server, meshes, materials, &atlas);

            atlas_loading.loaded = true;
        }
    }
}

fn generate_world(mut commands: Commands,
                  asset_server: Res<AssetServer>,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut materials: ResMut<Assets<StandardMaterial>>,
                  atlas: &TextureAtlas) {
    let texture_handle = asset_server.load("textures/spritesheet.png");

    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(texture_handle.clone()),
        alpha_mode: AlphaMode::Opaque,
        perceptual_roughness: 1.0,
        ..default()
    });

    let fbm = Fbm::<Perlin>::default();
    let builder = PlaneMapBuilder::<_, 2>::new(fbm)
        .set_size((CHUNK_SIZE * CHUNKS_COUNT_X) as usize, (CHUNK_SIZE * CHUNKS_COUNT_Z) as usize)
        .build();

    let max_height_above_surface = 50.0;

    let around = 2;

    for current_chunk_x in CHUNKS_COUNT_X / 2 - around..CHUNKS_COUNT_X / 2 + around {
        for current_chunk_z in CHUNKS_COUNT_Z / 2 - around..CHUNKS_COUNT_Z / 2 + around {
            for current_chunk_y in CHUNKS_COUNT_Y / 2 - around..CHUNKS_COUNT_Y / 2 + around {
                let mut samples = [EMPTY; SampleShape::SIZE as usize];
                for i in 0u32..(SampleShape::SIZE) {
                    let coords = SampleShape::delinearize(i);
                    let x = current_chunk_x * CHUNK_SIZE + coords[0] as i32;
                    let y = current_chunk_y * CHUNK_SIZE + coords[1] as i32;
                    let z = current_chunk_z * CHUNK_SIZE + coords[2] as i32;
                    let noize_height = builder.get_value(x as usize, z as usize) * max_height_above_surface;

                    let height = noize_height + (CHUNKS_COUNT_Y * CHUNK_SIZE / 2) as f64;

                    let voxel_filled = y < height.round() as i32;

                    let voxel_type = if voxel_filled {
                        // let mut rng = rand::thread_rng();
                        // [VoxelType::Dirt, VoxelType::Grass, VoxelType::Stone][rng.gen_range(0..3)]
                        if noize_height < -10.0 {
                            VoxelType::Dirt
                        } else if noize_height > -5.0 {
                            VoxelType::Stone
                        } else {
                            VoxelType::Grass
                        }
                    } else {
                        VoxelType::Empty
                    };

                    // println!("Voxel filled {:?} for height {}", voxel_type, noize_height);
                    samples[i as usize] = MaterialVoxel(voxel_type);
                }

                let (simple_mesh, generated) = generate_simple_mesh(&samples, &atlas);

                if generated > 0 {
                    spawn_pbr(
                        &mut commands,
                        &mut meshes,
                        simple_mesh,
                        material_handle.clone(),
                        Transform::from_translation(Vec3::new(
                            ((current_chunk_x - CHUNK_SIZE / 2) * CHUNKS_COUNT_X) as f32,
                            ((current_chunk_y - CHUNK_SIZE / 2) * CHUNKS_COUNT_Y) as f32,
                            ((current_chunk_z - CHUNK_SIZE / 2) * CHUNKS_COUNT_Z) as f32)),
                    );
                }
            }
        }
    }
}


type SampleShape = ConstShape3u32<34, 34, 34>;

fn generate_simple_mesh(
    samples: &[MaterialVoxel],
    atlas: &TextureAtlas,
) -> (Mesh, usize) {
    let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

    let mut buffer = UnitQuadBuffer::new();
    visible_block_faces(
        &samples,
        &SampleShape {},
        [0; 3],
        [33; 3],
        &faces,
        &mut buffer,
    );

    let num_indices = buffer.num_quads() * 6;
    let num_vertices = buffer.num_quads() * 4;
    let mut indices = Vec::with_capacity(num_indices);
    let mut positions = Vec::with_capacity(num_vertices);
    let mut normals = Vec::with_capacity(num_vertices);
    let mut uvs = Vec::with_capacity(num_vertices);
    let mut colors = Vec::with_capacity(num_vertices);
    for (group, face) in buffer.groups.into_iter().zip(faces.into_iter()) {
        for quad in group.into_iter() {
            let quad_positions = face.quad_mesh_positions(&quad.into(), 1.0);
            indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
            positions.extend_from_slice(&quad_positions);
            normals.extend_from_slice(&face.quad_mesh_normals());

            let normal = Vec3::from_array(face.quad_mesh_normals()[0]);
            let voxel_type = face_to_voxel_type(samples, face, quad_positions);

            let default_color = [[1.0, 1.0, 1.0, 1.0]; 4];
            let color = if voxel_type == VoxelType::Grass {
                if normal.y == 1.0 {
                    [[0.1, 0.8, 0.1, 1.0]; 4]
                } else {
                    default_color
                }
            } else {
                default_color
            };
            colors.extend_from_slice(&color);

            let frame_name = voxel_texture_name(normal, voxel_type);

            uvs.extend_from_slice(&atlas_uv(atlas, &atlas.frames.get(frame_name).unwrap().frame));
        }
    }
    let generated = positions.len();
    let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    render_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    render_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    render_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    render_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    render_mesh.set_indices(Some(Indices::U32(indices.clone())));
    (render_mesh, generated)
}

fn face_to_voxel_type(samples: &[MaterialVoxel], face: OrientedBlockFace, quad_positions: [[f32; 3]; 4]) -> VoxelType {
    let normal = Vec3::from_array(face.quad_mesh_normals()[0]);
    let quad_center = Vec3::new(
        (quad_positions[0][0] + quad_positions[1][0] + quad_positions[2][0] + quad_positions[3][0]) / 4.0,
        (quad_positions[0][1] + quad_positions[1][1] + quad_positions[2][1] + quad_positions[3][1]) / 4.0,
        (quad_positions[0][2] + quad_positions[1][2] + quad_positions[2][2] + quad_positions[3][2]) / 4.0,
    );

    let block_center = quad_center - normal / 2.0;

    let voxel_index = SampleShape::linearize([
        block_center.x.floor() as u32,
        block_center.y.floor() as u32,
        block_center.z.floor() as u32]);

    samples[voxel_index as usize].0
}

fn voxel_texture_name(normal: Vec3, voxel_type: VoxelType) -> &'static str {
    let mut rng = rand::thread_rng();
    match voxel_type
    {
        VoxelType::Grass => {
            if normal.y == 1.0 {
                let variants = [
                    "grass_block_top.png",
                    "grass_block_top1.png",
                    "grass_block_top2.png"];
                variants[rng.gen_range(0..variants.len())]
            } else if normal.y == -1.0 {
                "empty.png"
            } else {
                "grass_block_side.png"
            }
        }
        VoxelType::Dirt => {
            let variants = [
                "coarse_dirt.png",
                "coarse_dirt1.png",
                "coarse_dirt2.png",
                "coarse_dirt3.png",
                "coarse_dirt4.png",
                "coarse_dirt5.png",
                "coarse_dirt6.png"];
            variants[rng.gen_range(0..variants.len())]
        }
        VoxelType::Stone => {
            let variants = [
                "cobblestone.png",
                "cobblestone1.png",
                "cobblestone2.png",
                "cobblestone3.png",
                "cobblestone4.png",
                "cobblestone5.png",
                "cobblestone6.png",
                "cobblestone7.png",
                "cobblestone8.png",
                "cobblestone9.png"];
            variants[rng.gen_range(0..variants.len())]
        }
        VoxelType::Empty => {
            "empty.png"
        }
    }
}

fn atlas_uv(atlas: &TextureAtlas, desc: &Frame) -> Vec<[f32; 2]> {
    vec![
        [desc.x / atlas.meta.size.w, (desc.y + desc.h) / atlas.meta.size.h],
        [(desc.x + desc.w) / atlas.meta.size.w, (desc.y + desc.h) / atlas.meta.size.h],
        [desc.x / atlas.meta.size.w, desc.y / atlas.meta.size.h],
        [(desc.x + desc.w) / atlas.meta.size.w, desc.y / atlas.meta.size.h]]
}


fn spawn_pbr(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mesh: Mesh,
    material_handle: Handle<StandardMaterial>,
    transform: Transform,
) {
    let handle = meshes.add(mesh.clone());
    commands.spawn(PbrBundle {
        mesh: handle,
        material: material_handle,
        transform,
        ..Default::default()
    }).insert(RigidBody::Fixed)
        .insert(Collider::from_bevy_mesh(&mesh, &ComputedColliderShape::TriMesh).unwrap());
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
enum VoxelType {
    Empty,
    Grass,
    Stone,
    Dirt,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct MaterialVoxel(VoxelType);

const EMPTY: MaterialVoxel = MaterialVoxel(VoxelType::Empty);

impl Voxel for MaterialVoxel {
    fn get_visibility(&self) -> VoxelVisibility {
        if *self == EMPTY {
            VoxelVisibility::Empty
        } else {
            VoxelVisibility::Opaque
        }
    }
}

impl MergeVoxel for MaterialVoxel {
    type MergeValue = Self;
    type MergeValueFacingNeighbour = Self;

    fn merge_value(&self) -> Self::MergeValue {
        *self
    }

    fn merge_value_facing_neighbour(&self) -> Self::MergeValueFacingNeighbour {
        *self
    }
}