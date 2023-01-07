use bevy::{
    prelude::*,
    render::{
        mesh::{Indices},
        render_resource::{PrimitiveTopology},
    },
};

use fast_poisson::Poisson2D;

use noise::{Fbm, OpenSimplex, Worley};
use bevy_rapier3d::prelude::*;
use std::collections::HashMap;
use std::time::SystemTime;
use bevy::asset::LoadState;


use bevy::render::view::NoFrustumCulling;

use noise::utils::{NoiseMapBuilder, PlaneMapBuilder};

use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{visible_block_faces, MergeVoxel, UnitQuadBuffer, Voxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG, OrientedBlockFace};

use bevy_common_assets::json::JsonAssetPlugin;
use rand::Rng;
use crate::{DigEvent, DigEventType};


const CHUNKS_COUNT_X: i32 = 32;
const CHUNKS_COUNT_Y: i32 = 32;
const CHUNKS_COUNT_Z: i32 = 32;

const CHUNK_SIZE: i32 = 32;


const VISIBLE_CHUNK_DISTANCE: i32 = 1;
const CHUNKS_COUNT_DIM: i32 = VISIBLE_CHUNK_DISTANCE * 2 + 1;
const BUILDER_HEIGHT_SCALE: f32 = 20.0;

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
        alpha_mode: AlphaMode::Mask(0.5),
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 1.0,
        ..default()
    });

    let now = SystemTime::now();
    let fbm = Fbm::<OpenSimplex>::default();
    let builder = PlaneMapBuilder::<_, 2>::new(fbm.clone())
        .set_size((CHUNK_SIZE * CHUNKS_COUNT_X) as usize, (CHUNK_SIZE * CHUNKS_COUNT_Z) as usize)
        .build();

    let biome_builder = PlaneMapBuilder::<_, 2>::new(fbm)
        .set_size((CHUNK_SIZE * CHUNKS_COUNT_X) as usize, (CHUNK_SIZE * CHUNKS_COUNT_Z) as usize)
        .build();

    let mut fbm2 = Fbm::<Worley>::default();
    fbm2.frequency = 0.01;

    let mut chunks: HashMap<[i32; 3], Vec<MaterialVoxel>> = HashMap::new();

    println!("Noise gen time: {}ms", now.elapsed().unwrap().as_millis());
    let now = SystemTime::now();

    for current_chunk_x in -VISIBLE_CHUNK_DISTANCE..=VISIBLE_CHUNK_DISTANCE {
        for current_chunk_y in -VISIBLE_CHUNK_DISTANCE..=VISIBLE_CHUNK_DISTANCE {
            for current_chunk_z in -VISIBLE_CHUNK_DISTANCE..=VISIBLE_CHUNK_DISTANCE {
                let mut samples = Vec::with_capacity(SampleShape::SIZE as usize);

                for i in 0u32..(SampleShape::SIZE) {
                    let local_position_in_chunk = SampleShape::delinearize(i);
                    let builder_world_x = (current_chunk_x + CHUNKS_COUNT_X / 2) * CHUNK_SIZE + local_position_in_chunk[0] as i32;
                    let builder_world_y = (current_chunk_y + CHUNKS_COUNT_Y / 2) * CHUNK_SIZE + local_position_in_chunk[1] as i32;
                    let builder_world_z = (current_chunk_z + CHUNKS_COUNT_Z / 2) * CHUNK_SIZE + local_position_in_chunk[2] as i32;

                    let height = builder.get_value(builder_world_x as usize - 1, builder_world_z as usize - 1) as f32 * BUILDER_HEIGHT_SCALE + (CHUNKS_COUNT_Y * CHUNK_SIZE / 2) as f32;
                    let biome_value = biome_builder.get_value(builder_world_x as usize - 1, builder_world_z as usize - 1);

                    let under_surface = builder_world_y < height.round() as i32;
                    let surface = builder_world_y == height.round() as i32;

                    let voxel_type = if surface {
                        if biome_value < -0.2 {
                            VoxelType::Dirt
                        } else if biome_value > 0.2 {
                            VoxelType::Stone
                        } else {
                            VoxelType::Grass
                        }
                    } else if under_surface {
                        // let val = fbm2.get([x as f64, y as f64, z as f64]);
                        // if val > 0.8 {
                            VoxelType::Sand
                        // } else if val > 0.6 {
                        //     VoxelType::Stone
                        // } else {
                        //     VoxelType::Empty
                        // }
                    } else {
                        VoxelType::Empty
                    };

                    samples.push(MaterialVoxel(voxel_type));
                }

                chunks.insert([current_chunk_x, current_chunk_y, current_chunk_z], samples);
            }
        }
    }

    // Tree generation
    let poisson = Poisson2D::new()
        .with_dimensions([(CHUNKS_COUNT_DIM * CHUNK_SIZE) as f32, (CHUNKS_COUNT_DIM * CHUNK_SIZE) as f32], 6.0)
        .generate();
    for point in poisson {
        let world_x = point[0].floor() - (VISIBLE_CHUNK_DISTANCE * CHUNK_SIZE) as f32;
        let world_z = point[1].floor() - (VISIBLE_CHUNK_DISTANCE * CHUNK_SIZE) as f32;
        let world_y = builder.get_value((world_x as i32 + CHUNKS_COUNT_X / 2 * CHUNK_SIZE) as usize, (world_z as i32 + CHUNKS_COUNT_Z / 2 * CHUNK_SIZE) as usize) as f32 * BUILDER_HEIGHT_SCALE;

        generate_tree(Vec3::new(world_x, world_y.round(), world_z), &mut chunks);
    }

    println!("Fill time: {}ms", now.elapsed().unwrap().as_millis());
    let now = SystemTime::now();

    for (pos, samples) in chunks {
        let (simple_mesh, generated) = generate_simple_mesh(&samples, atlas);

        if generated > 0 {
            spawn_pbr(
                &mut commands,
                &mut meshes,
                simple_mesh,
                material_handle.clone(),
                Transform::from_translation(Vec3::new(
                    (pos[0] * CHUNK_SIZE) as f32,
                    (pos[1] * CHUNK_SIZE) as f32,
                    (pos[2] * CHUNK_SIZE) as f32)),
                samples,
            );
        }
    }

    println!("Mesh gen time: {}ms", now.elapsed().unwrap().as_millis());
}

fn generate_tree(origin: Vec3, chunks: &mut HashMap<[i32; 3], Vec<MaterialVoxel>>) {
    match rand::thread_rng().gen_range(0..=1) {
        0 => {
            change_voxel(origin + Vec3::new(0.0, 4.0, 0.0), VoxelType::OakLeaves, chunks);
            for x in -1..=1 {
                for z in -1..=1 {
                    change_voxel(origin + Vec3::new(x as f32, 3.0, z as f32), VoxelType::OakLeaves, chunks);
                }
            }
            for z in 0..=2 {
                change_voxel(origin + Vec3::new(0.0, z as f32, 0.0), VoxelType::OakLog, chunks);
            }
        }
        1 => {
            change_voxel(origin + Vec3::new(0.0, 6.0, 0.0), VoxelType::OakLeaves, chunks);
            for x in -1..=1 {
                for z in -1..=1 {
                    change_voxel(origin + Vec3::new(x as f32, 3.0, z as f32), VoxelType::OakLeaves, chunks);
                    change_voxel(origin + Vec3::new(x as f32, 5.0, z as f32), VoxelType::OakLeaves, chunks);
                }
            }
            for x in -2..=2 {
                for z in -2..=2 {
                    change_voxel(origin + Vec3::new(x as f32, 4.0, z as f32), VoxelType::OakLeaves, chunks);
                }
            }
            for z in 0..=4 {
                change_voxel(origin + Vec3::new(0.0, z as f32, 0.0), VoxelType::OakLog, chunks);
            }
        }
        _ => {

        }
    }
}

fn change_voxel(world_point: Vec3, voxel_type: VoxelType, chunks: &mut HashMap<[i32; 3], Vec<MaterialVoxel>>) {
    let chunk_position = [
        (world_point.x / CHUNK_SIZE as f32).floor() as i32,
        (world_point.y / CHUNK_SIZE as f32).floor() as i32,
        (world_point.z / CHUNK_SIZE as f32).floor() as i32];
    let samples = chunks.get_mut(&chunk_position);
    if let Some(samples) = samples {

        let voxel_x = (world_point.x - (chunk_position[0] * CHUNK_SIZE) as f32).floor() as u32;
        let voxel_y = (world_point.y - (chunk_position[1] * CHUNK_SIZE) as f32).floor() as u32;
        let voxel_z = (world_point.z - (chunk_position[2] * CHUNK_SIZE) as f32).floor() as u32;
        let index = SampleShape::linearize([voxel_x + 1, voxel_y + 1, voxel_z + 1]);
        samples[index as usize] = MaterialVoxel(voxel_type);
    } else {
        println!("Chunk not found {chunk_position:?}");
    }
}

#[derive(Component)]
struct ChunkInfo {
    samples: Vec<MaterialVoxel>,
}

fn dig_event_handler(
    mut query: Query<(Entity, &mut ChunkInfo, &Transform)>,
    mut commands: Commands,
    atlas_loading: Res<AtlasLoading>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut ev: EventReader<DigEvent>,
) {
    if let Some(atlas) = &atlas_loading.atlas {
        for ev in ev.iter() {
            println!("Handle event {}", ev.world_position);
            for (entity, mut chunk, transform) in query.iter_mut() {
                let position = voxel_position_from_world(transform.translation, ev.world_position);

                println!("Check pos! {position}");

                if !(position.x >= 0.0 && position.x <= 33.0 &&
                    position.y >= 0.0 && position.y <= 33.0 &&
                    position.z >= 0.0 && position.z <= 33.0) {
                    continue;
                }

                println!("Must update chunk! {} trans={}", position, transform.translation);
                // let mut samples = chunk.samples;
                let voxel_index = SampleShape::linearize(position.as_uvec3().to_array());

                chunk.samples[voxel_index as usize] = match ev.event_type {
                    DigEventType::Dig => EMPTY,
                    DigEventType::Build => MaterialVoxel(VoxelType::Cobblestone)
                };

                let (simple_mesh, generated) = generate_simple_mesh(&chunk.samples, atlas);
                let mesh_handle = meshes.add(simple_mesh.clone());

                if generated > 0 {
                    commands.entity(entity)
                        .insert(mesh_handle)
                        // .insert(ChunkInfo { samples })
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
            let color = match voxel_type {
                VoxelType::Grass => {
                    if normal.y == 1.0 {
                        [[0.1, 0.8, 0.1, 1.0]; 4]
                    } else {
                        default_color
                    }
                }
                VoxelType::OakLeaves => {
                    [[0.1, 0.8, 0.1, 1.0]; 4]
                },
                _ => default_color
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
        VoxelType::Sand => {
            "sand.png"
        }
        VoxelType::OakLog => {
            if normal.y == 1.0 || normal.y == -1.0 {
                "oak_log_top.png"
            } else {
                "oak_log.png"
            }
        }
        VoxelType::OakLeaves => {
            "oak_leaves.png"
        }
        VoxelType::Cobblestone => {
            "cobblestone.png"
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
    Cobblestone,
    Dirt,
    Sand,
    OakLog,
    OakLeaves,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct MaterialVoxel(VoxelType);

const EMPTY: MaterialVoxel = MaterialVoxel(VoxelType::Empty);

impl Voxel for MaterialVoxel {
    fn get_visibility(&self) -> VoxelVisibility {
        match self.0 {
            VoxelType::Empty => VoxelVisibility::Empty,
            VoxelType::OakLeaves => VoxelVisibility::Translucent,
            _ => VoxelVisibility::Opaque
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