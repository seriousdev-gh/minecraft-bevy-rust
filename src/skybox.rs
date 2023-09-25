//! Load a cubemap texture onto a cube like a skybox and cycle through different compressed texture formats

use bevy::{
    asset::LoadState,
    core_pipeline::Skybox,
    prelude::*,
    render::texture::CompressedImageFormats,
    
};

const CUBEMAPS: &[(&str, CompressedImageFormats)] = &[
    (
        "textures/clear_sky.ktx2",
        CompressedImageFormats::BC,
    )
];

pub struct SkyboxPlugin;

impl Plugin for SkyboxPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            asset_loaded,
        );
    }
}

#[derive(Resource)]
struct Cubemap {
    is_loaded: bool,
    index: usize,
    image_handle: Handle<Image>,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let skybox_handle = asset_server.load(CUBEMAPS[0].0);

    // directional 'sun' light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 12000.0,
            shadows_enabled: true,
            ..default()
        },
        // TODO: why with cascade_shadow_config, shadows disappear?
        // cascade_shadow_config: bevy::pbr::CascadeShadowConfigBuilder {
        //     maximum_distance: 3.0,
        //     first_cascade_far_bound: 0.9,
        //     ..default()
        // }.into(),
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_euler(
                EulerRot::XYZ,
                (-45.0_f32).to_radians(),
                (25.0_f32).to_radians(),
                (0.0_f32).to_radians(),
            ),
            ..default()
        },
        ..default()
    });
    

    // ambient light
    // NOTE: The ambient light is used to scale how bright the environment map is so with a bright
    // environment map, use an appropriate color and brightness to match
    commands.insert_resource(AmbientLight {
        color: Color::rgb_u8(210, 220, 240),
        brightness: 0.5,
    });

    commands.insert_resource(Cubemap {
        is_loaded: false,
        index: 0,
        image_handle: skybox_handle,
    });

}

fn asset_loaded(
    asset_server: Res<AssetServer>,
    _images: ResMut<Assets<Image>>,
    mut cubemap: ResMut<Cubemap>,
    mut skyboxes: Query<&mut Skybox>,
    mut camera: Query<
        Entity,
        With<Camera>
    >,
    mut commands: Commands,
) {
    if !cubemap.is_loaded
        && asset_server.get_load_state(cubemap.image_handle.clone_weak()) == LoadState::Loaded
    {
        info!("Swapping to {}...", CUBEMAPS[cubemap.index].0);

        let camera_entity = camera.single_mut();
        let mut camera = commands.entity(camera_entity);
        camera.insert(Skybox(cubemap.image_handle.clone()));

        for mut skybox in &mut skyboxes {
            skybox.0 = cubemap.image_handle.clone();
        }

        cubemap.is_loaded = true;

    }
}