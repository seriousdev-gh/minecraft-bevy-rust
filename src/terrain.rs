use bevy::{
    prelude::*,
    render::{
        mesh::{Indices},
        render_resource::{PrimitiveTopology},
    },
};


use noise::{Fbm, OpenSimplex};
use bevy_rapier3d::prelude::*;
use std::collections::HashMap;
use bevy::asset::LoadState;


use bevy::render::view::NoFrustumCulling;

use noise::utils::{NoiseMapBuilder, PlaneMapBuilder};

use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{visible_block_faces, MergeVoxel, UnitQuadBuffer, Voxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG, OrientedBlockFace};

use bevy_common_assets::json::JsonAssetPlugin;
use crate::DigEvent;


const CHUNKS_COUNT_X: i32 = 32;
const CHUNKS_COUNT_Y: i32 = 32;
const CHUNKS_COUNT_Z: i32 = 32;

const CHUNK_SIZE: i32 = 32;

type SampleShape = ConstShape3u32<34, 34, 34>;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(JsonAssetPlugin::<TextureAtlas>::new(&["json"]))
            .add_startup_system(load_atlas)
            .add_system(asset_loaded)
            .add_system(dig_event_handler)
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
    atlas: Option<TextureAtlas>,
}

fn load_atlas(asset_server: Res<AssetServer>, mut commands: Commands) {
    let handle = TextureAtlasHandle(asset_server.load("textures/spritesheet.json"));

    commands.insert_resource(AtlasLoading { handle, loaded: false, atlas: None })
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

            atlas_loading.atlas = Some(atlas);
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
        base_color_texture: Some(texture_handle),
        alpha_mode: AlphaMode::Opaque,
        perceptual_roughness: 1.0,
        ..default()
    });

    let fbm = Fbm::<OpenSimplex>::default();
    let builder = PlaneMapBuilder::<_, 2>::new(fbm.clone())
        .set_size((CHUNK_SIZE * CHUNKS_COUNT_X) as usize, (CHUNK_SIZE * CHUNKS_COUNT_Z) as usize)
        .build();

    let biome_builder = PlaneMapBuilder::<_, 2>::new(fbm)
        .set_size((CHUNK_SIZE * CHUNKS_COUNT_X) as usize, (CHUNK_SIZE * CHUNKS_COUNT_Z) as usize)
        .build();

    let max_height_above_surface = 20.0;

    let around = 2;

    for current_chunk_x in CHUNKS_COUNT_X / 2 - around..CHUNKS_COUNT_X / 2 + around {
        for current_chunk_z in CHUNKS_COUNT_Z / 2 - around..CHUNKS_COUNT_Z / 2 + around {
            for current_chunk_y in CHUNKS_COUNT_Y / 2 - around..CHUNKS_COUNT_Y / 2 + around {
                let mut samples = Vec::with_capacity(SampleShape::SIZE as usize);
                for i in 0u32..(SampleShape::SIZE) {
                    let coords = SampleShape::delinearize(i);
                    let x = current_chunk_x * CHUNK_SIZE + coords[0] as i32;
                    let y = current_chunk_y * CHUNK_SIZE + coords[1] as i32;
                    let z = current_chunk_z * CHUNK_SIZE + coords[2] as i32;
                    let noize_height = builder.get_value(x as usize, z as usize) * max_height_above_surface;

                    let height = noize_height + (CHUNKS_COUNT_Y * CHUNK_SIZE / 2) as f64;

                    let voxel_filled = y < height.round() as i32;

                    let voxel_type = if voxel_filled {
                        let biome_value = biome_builder.get_value(x as usize, z as usize);
                        if biome_value < -0.2 {
                            VoxelType::Dirt
                        } else if biome_value > 0.2 {
                            VoxelType::Stone
                        } else {
                            VoxelType::Grass
                        }
                    } else {
                        VoxelType::Empty
                    };

                    samples.push(MaterialVoxel(voxel_type));
                }

                let (simple_mesh, generated) = generate_simple_mesh(&samples, atlas);

                if generated > 0 {
                    spawn_pbr(
                        &mut commands,
                        &mut meshes,
                        simple_mesh,
                        material_handle.clone(),
                        Transform::from_translation(Vec3::new(
                            ((current_chunk_x - CHUNK_SIZE / 2) * CHUNK_SIZE) as f32,
                            ((current_chunk_y - CHUNK_SIZE / 2) * CHUNK_SIZE) as f32,
                            ((current_chunk_z - CHUNK_SIZE / 2) * CHUNK_SIZE) as f32)),
                        samples,
                    );
                }
            }
        }
    }
}


#[derive(Component)]
struct ChunkInfo {
    samples: Vec<MaterialVoxel>,
}

fn dig_event_handler(
    query: Query<(Entity, &ChunkInfo, &Transform)>,
    mut commands: Commands,
    atlas_loading: Res<AtlasLoading>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut ev: EventReader<DigEvent>,
) {

    if let Some(atlas) = &atlas_loading.atlas {
        for ev in ev.iter() {
            println!("Handle event {}", ev.world_position);
            for (entity, chunk, transform) in query.iter() {
                let position = voxel_position_from_world(transform.translation, ev.world_position);

                if !(position.x >= 0.0 && position.x <= 33.0 &&
                    position.y >= 0.0 && position.y <= 33.0 &&
                    position.z >= 0.0 && position.z <= 33.0) {
                    continue;
                }

                println!("Must update chunk! {} trans={}", position, transform.translation);
                let mut samples = chunk.samples.clone();
                let voxel_index = SampleShape::linearize(position.as_uvec3().to_array());

                samples[voxel_index as usize] = EMPTY;

                let (simple_mesh, generated) = generate_simple_mesh(&samples, atlas);
                let mesh_handle = meshes.add(simple_mesh.clone());

                if generated > 0 {
                    commands.entity(entity)
                        .insert(mesh_handle)
                        .insert(ChunkInfo { samples })
                        .insert(Collider::from_bevy_mesh(&simple_mesh, &ComputedColliderShape::TriMesh).unwrap());
                }
            }
        }
    }

}

fn voxel_position_from_world(chunk_translation: Vec3, world_position: Vec3) -> Vec3 {
    (world_position - chunk_translation).floor()
}

fn generate_simple_mesh(
    samples: &[MaterialVoxel],
    atlas: &TextureAtlas,
) -> (Mesh, usize) {
    let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

    let mut buffer = UnitQuadBuffer::new();
    visible_block_faces(
        samples,
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

    let voxel_index = SampleShape::linearize(block_center.floor().as_uvec3().to_array());

    samples[voxel_index as usize].0
}

fn voxel_texture_name(normal: Vec3, voxel_type: VoxelType) -> &'static str {
    // let mut rng = rand::thread_rng();
    match voxel_type
    {
        VoxelType::Grass => {
            if normal.y == 1.0 {
                "grass_block_top.png"
            } else if normal.y == -1.0 {
                "dirt.png"
            } else {
                "grass_block_side.png"
            }
        }
        VoxelType::Dirt => {
            "dirt.png"
        }
        VoxelType::Stone => {
            "stone.png"
        }
        VoxelType::Empty => {
            "debug.png"
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
    samples: Vec<MaterialVoxel>,
) {
    let handle = meshes.add(mesh.clone());
    commands.spawn(PbrBundle {
        mesh: handle,
        material: material_handle,
        transform,
        ..Default::default()
    })
        .insert(RigidBody::Fixed)
        .insert(Collider::from_bevy_mesh(&mesh, &ComputedColliderShape::TriMesh).unwrap())
        // TODO: why chunks doesn't render without NoFrustumCulling ?
        .insert(NoFrustumCulling)
        .insert(ChunkInfo { samples });
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