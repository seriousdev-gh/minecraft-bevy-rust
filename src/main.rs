use std::f32::consts::TAU;

mod terrain;
mod skybox;
mod ui;

use bevy_prototype_debug_lines::*;
use bevy_fps_controller::controller::*;
use bevy::{prelude::*};

use bevy::window::CursorGrabMode;
use bevy_rapier3d::prelude::*;

use crate::skybox::SkyboxPlugin;
use crate::terrain::WorldPlugin;
use crate::ui::MyUiPlugin;

use iyes_loopless::prelude::*;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    LoadingBefore,
    Loading,
    InGame,
}

#[derive(Component)]
struct Movable;

#[derive(Component)]
struct Ground;

pub fn main() {
    App::new()
        .insert_resource(Msaa { samples: 1 })
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_loopless_state(GameState::InGame)
        .add_plugin(SkyboxPlugin)
        .add_plugin(WorldPlugin)
        .add_plugin(MyUiPlugin)
        .add_plugin(FpsControllerPlugin)
        .add_plugin(DebugLinesPlugin::default())
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_enter_system(GameState::InGame, setup_physics)
        .add_enter_system(GameState::InGame, setup)
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::InGame)
                .with_system(manage_cursor)
                .with_system(update_system)
                .with_system(cast_ray)
                .into()
        )
        .add_system(bevy::window::close_on_esc)
        .run();
}


fn setup_physics(mut commands: Commands,
                 _asset_server: Res<AssetServer>,
                 _meshes: ResMut<Assets<Mesh>>,
                 _materials: ResMut<Assets<StandardMaterial>>) {

    // commands.spawn(
    //     DirectionalLightBundle {
    //         transform: Transform::from_xyz(0.0,0.0,0.0).looking_at(Vec3::new(-0.1, -0.5, -0.2), Vec3::Y),
    //         directional_light: DirectionalLight {
    //             shadows_enabled: true,
    //             illuminance: 10000.0,
    //             ..default()
    //         },
    //         ..default()
    //     }
    // );

    commands.spawn((
        Collider::capsule(Vec3::Y * 0.5, Vec3::Y * 1.5, 0.5),
        ActiveEvents::COLLISION_EVENTS,
        Velocity::zero(),
        RigidBody::Dynamic,
        Sleeping::disabled(),
        LockedAxes::ROTATION_LOCKED,
        AdditionalMassProperties::Mass(1.0),
        GravityScale(0.0),
        Ccd { enabled: true }, // Prevent clipping when going fast
        TransformBundle::from_transform(Transform::from_xyz(0.0, 25.0, 0.0)),
        LogicalPlayer(0),
        FpsControllerInput {
            pitch: -TAU / 12.0,
            yaw: TAU * 5.0 / 8.0,
            ..default()
        },
        FpsController {
            walk_speed: 5.0,
            run_speed: 10.0,
            forward_speed: 10.0,
            side_speed: 10.0,
            air_speed_cap: 1.0,
            sensitivity: 0.002,
            ..default()
        }
    ));
    // commands.spawn((
    //     Collider::capsule(Vec3::Y * 0.5, Vec3::Y * 1.5, 0.5),
    //     ActiveEvents::COLLISION_EVENTS,
    //     Velocity::zero(),
    //     RigidBody::Dynamic,
    //     Sleeping::disabled(),
    //     LockedAxes::ROTATION_LOCKED,
    //     AdditionalMassProperties::Mass(1.0),
    //     GravityScale(0.0),
    //     Ccd { enabled: true }, // Prevent clipping when going fast
    //     TransformBundle::from_transform(Transform::from_xyz(2.0, 25.0, 0.0)),
    //     LogicalPlayer(1),
    //     FpsControllerInput {
    //         pitch: -TAU / 12.0,
    //         yaw: TAU * 5.0 / 8.0,
    //         ..default()
    //     },
    //     FpsController {
    //         walk_speed: 5.0,
    //         run_speed: 10.0,
    //         forward_speed: 10.0,
    //         side_speed: 10.0,
    //         air_speed_cap: 1.0,
    //         sensitivity: 0.002,
    //         ..default()
    //     }
    // ));
    commands.spawn((
        Camera3dBundle {
            camera_3d: Camera3d {
                ..default()
            },
            camera: Camera {
                priority: 1,
                ..default()
            },
            ..default()
        },
        RenderPlayer(0),
    ));
    // commands.spawn((
    //     Camera3dBundle {
    //         camera_3d: Camera3d {
    //             ..default()
    //         },
    //         camera: Camera {
    //             priority: 0,
    //             ..default()
    //         },
    //         ..default()
    //     },
    //     RenderPlayer(1),
    // ));
}

fn setup() {}

fn update_system(
    key_input: Res<Input<KeyCode>>,
    mut cameras: Query<
        (&mut Camera, &RenderPlayer),
        Without<LogicalPlayer>
    >,
    mut controllers: Query<
        (&mut FpsController, &LogicalPlayer),
        With<LogicalPlayer>
    >,
) {
    if !key_input.just_pressed(KeyCode::Key1) &&
        !key_input.just_pressed(KeyCode::Key2) { return; }

    let enabled_id = if key_input.just_pressed(KeyCode::Key1) {
        0
    } else {
        1
    };

    println!("Change player to {}", enabled_id);

    for (mut controller, player_id) in controllers.iter_mut() {
        if player_id.0 == enabled_id {
            controller.enable_input = true;
        } else {
            controller.enable_input = false;
        }
    }

    for (mut camera, player_id) in cameras.iter_mut() {
        if player_id.0 == enabled_id {
            camera.priority = 1;
        } else {
            camera.priority = 0;
        }
    }
}

/* Cast a ray inside of a system. */
fn cast_ray(rapier_context: Res<RapierContext>,
            controllers: Query<(&Transform, &Collider, &FpsController)>,
            mut lines: ResMut<DebugLines>,
            btn: Res<Input<MouseButton>>,) {
    if !btn.just_pressed(MouseButton::Left) { return }

    for (transform, collider, controller) in controllers.iter() {
        if let Some(capsule) = collider.as_capsule() {
            let camera_height = capsule.segment().b().y + capsule.radius() * 0.75;
            let ray_pos = transform.translation + Vec3::Y * camera_height;
            let quat = Quat::from_euler(EulerRot::ZYX, 0.0, controller.yaw, controller.pitch);
            let ray_dir = -quat.mul_vec3(Vec3::Z);
            let max_toi = 3.0;
            let solid = false;
            let filter = QueryFilter { flags: QueryFilterFlags::ONLY_FIXED, ..default() };

            lines.line_colored(ray_pos, ray_pos + ray_dir * max_toi, 0.0, Color::WHITE);

            if let Some((entity, toi)) = rapier_context.cast_ray(
                ray_pos, ray_dir, max_toi, solid, filter,
            ) {
                let hit_point = ray_pos + ray_dir * toi;
                println!("Entity {:?} hit at point {}", entity, hit_point);

                lines.line_colored(hit_point - Vec3::X * 0.5, hit_point + Vec3::X * 0.5, 0.5, Color::RED);
                lines.line_colored(hit_point - Vec3::Y * 0.5, hit_point + Vec3::Y * 0.5, 0.5, Color::GREEN);
                lines.line_colored(hit_point - Vec3::Z * 0.5, hit_point + Vec3::Z * 0.5, 0.5, Color::BLUE);
            }
        }
    }
}

pub fn manage_cursor(
    mut windows: ResMut<Windows>,
    btn: Res<Input<MouseButton>>,
    key: Res<Input<KeyCode>>,
    _controllers: Query<&mut FpsController>,
) {
    let window = windows.get_primary_mut().unwrap();
    if btn.just_pressed(MouseButton::Left) {
        window.set_cursor_grab_mode(CursorGrabMode::Locked);
        window.set_cursor_visibility(false);
        // for mut controller in &mut controllers {
        //     controller.enable_input = true;
        // }
    }
    if key.just_pressed(KeyCode::Escape) {
        window.set_cursor_grab_mode(CursorGrabMode::None);
        window.set_cursor_visibility(true);
        // for mut controller in &mut controllers {
        //     controller.enable_input = false;
        // }
    }
}