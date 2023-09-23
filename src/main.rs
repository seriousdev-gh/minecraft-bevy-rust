use std::f32::consts::TAU;


mod terrain;
mod skybox;
mod ui;

use bevy::window::PresentMode;
use bevy_prototype_debug_lines::*;
use bevy_fps_controller::controller::*;
use bevy::core_pipeline::fxaa::Fxaa;
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};

use bevy::{prelude::*, window::CursorGrabMode};

use bevy_rapier3d::prelude::*;

use crate::skybox::SkyboxPlugin;
use crate::terrain::WorldPlugin;
use crate::ui::MyUiPlugin;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
pub enum GameState {
    LoadingBefore,
    Loading,
    #[default]
    InGame,
}

#[derive(Component)]
struct OutlineCube;

pub fn main() {
    App::new()
        .insert_resource(Msaa::Sample8)
        .add_plugins(DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "I am a window!".into(),
                    position: WindowPosition::At(IVec2 { x: 0, y: 0 }),
                    ..default()
                }),
                ..default()
            })
        )
        .add_state::<GameState>()
        .add_plugin(SkyboxPlugin)
        .add_plugin(WorldPlugin)
        .add_plugin(MyUiPlugin)
        .add_plugin(FpsControllerPlugin)
        .add_plugin(DebugLinesPlugin::default())
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_system(setup.in_schedule(OnEnter(GameState::InGame)))
        .add_event::<DigEvent>()
        .add_systems(
            (
                manage_cursor,
                update_system,
                cast_ray
            )
                .in_set(OnUpdate(GameState::InGame)),
        )
        .add_system(bevy::window::close_on_esc)
        .run();
}


fn setup(mut commands: Commands,
         mut meshes: ResMut<Assets<Mesh>>,
         mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((
        Collider::capsule(Vec3::Y * 0.5, Vec3::Y * 1.5, 0.45),
        ActiveEvents::COLLISION_EVENTS,
        Velocity::zero(),
        RigidBody::Dynamic,
        Sleeping::disabled(),
        LockedAxes::ROTATION_LOCKED,
        AdditionalMassProperties::Mass(1.0),
        GravityScale(0.0),
        Ccd { enabled: true }, // Prevent clipping when going fast
        SpatialBundle::from_transform(Transform::from_xyz(0.0, 25.0, 0.0)),
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
    )).with_children(|builder|
        {
            builder.spawn(PointLightBundle {

                // transform: Transform::from_xyz(5.0, 8.0, 2.0),
                transform: Transform::from_xyz(0.0, 1.5, 0.0),
                point_light: PointLight {
                    intensity: 100.0, // lumens - roughly a 100W non-halogen incandescent bulb
                    color: Color::ANTIQUE_WHITE,
                    shadows_enabled: true,
                    ..default()
                },
                ..default()
            });
        });

    commands.spawn((
        Camera3dBundle {
            camera_3d: Camera3d {
                ..default()
            },
            camera: Camera {
                order: 1,
                ..default()
            },
            ..default()
        },
        RenderPlayer(0),
    )).insert(Fxaa::default());

    commands
        .spawn((OutlineCube, SpatialBundle::default()))
        .with_children(|builder| {

            let extent = Vec3::new(0.5, 0.5, 0.5);
            let line_width = 0.005;
            let _color = Color::BLACK;

            let v1 = Vec3::new(-extent.x, -extent.y, -extent.z);
            let v2 = Vec3::new(extent.x, -extent.y, -extent.z);
            let v3 = Vec3::new(extent.x, extent.y, -extent.z);
            let v4 = Vec3::new(-extent.x, extent.y, -extent.z);
            let v5 = Vec3::new(-extent.x, -extent.y, extent.z);
            let v6 = Vec3::new(extent.x, -extent.y, extent.z);
            let v7 = Vec3::new(extent.x, extent.y, extent.z);
            let v8 = Vec3::new(-extent.x, extent.y, extent.z);

            let boxes = [
                shape::Box {
                    min_x: v1.x - line_width, min_z: v1.z - line_width, min_y: v1.y - line_width,
                    max_x: v2.x + line_width, max_z: v2.z + line_width, max_y: v2.y + line_width
                },
                shape::Box {
                    min_x: v2.x - line_width, min_z: v2.z - line_width, min_y: v2.y - line_width,
                    max_x: v3.x + line_width, max_z: v3.z + line_width, max_y: v3.y + line_width
                },
                shape::Box {
                    min_x: v4.x - line_width, min_z: v4.z - line_width, min_y: v4.y - line_width,
                    max_x: v3.x + line_width, max_z: v3.z + line_width, max_y: v3.y + line_width
                },
                shape::Box {
                    min_x: v1.x - line_width, min_z: v1.z - line_width, min_y: v1.y - line_width,
                    max_x: v4.x + line_width, max_z: v4.z + line_width, max_y: v4.y + line_width
                },
                shape::Box {
                    min_x: v5.x - line_width, min_z: v5.z - line_width, min_y: v5.y - line_width,
                    max_x: v6.x + line_width, max_z: v6.z + line_width, max_y: v6.y + line_width
                },
                shape::Box {
                    min_x: v6.x - line_width, min_z: v6.z - line_width, min_y: v6.y - line_width,
                    max_x: v7.x + line_width, max_z: v7.z + line_width, max_y: v7.y + line_width
                },
                shape::Box {
                    min_x: v8.x - line_width, min_z: v8.z - line_width, min_y: v8.y - line_width,
                    max_x: v7.x + line_width, max_z: v7.z + line_width, max_y: v7.y + line_width
                },
                shape::Box {
                    min_x: v5.x - line_width, min_z: v5.z - line_width, min_y: v5.y - line_width,
                    max_x: v8.x + line_width, max_z: v8.z + line_width, max_y: v8.y + line_width
                },
                shape::Box {
                    min_x: v1.x - line_width, min_z: v1.z - line_width, min_y: v1.y - line_width,
                    max_x: v5.x + line_width, max_z: v5.z + line_width, max_y: v5.y + line_width
                },
                shape::Box {
                    min_x: v2.x - line_width, min_z: v2.z - line_width, min_y: v2.y - line_width,
                    max_x: v6.x + line_width, max_z: v6.z + line_width, max_y: v6.y + line_width
                },
                shape::Box {
                    min_x: v3.x - line_width, min_z: v3.z - line_width, min_y: v3.y - line_width,
                    max_x: v7.x + line_width, max_z: v7.z + line_width, max_y: v7.y + line_width
                },
                shape::Box {
                    min_x: v4.x - line_width, min_z: v4.z - line_width, min_y: v4.y - line_width,
                    max_x: v8.x + line_width, max_z: v8.z + line_width, max_y: v8.y + line_width
                }
            ];

            for cuboid in boxes {
                builder.spawn(PbrBundle {
                    mesh: meshes.add(Mesh::from(cuboid)),
                    material: materials.add(StandardMaterial {
                        base_color: Color::rgb(0.1, 0.1, 0.1),
                        unlit: true,
                        ..default()
                    }),
                    transform: Transform::from_xyz(0.0, 0.0, 0.0),
                    ..default()
                }).insert((NotShadowReceiver, NotShadowCaster));
            }


        });
}

fn update_system(
    _lines: ResMut<DebugLines>
) {

    // show chunks positions
    // for x in -3 .. 3 {
    //     for y in -3 .. 3 {
    //         for z in -3 .. 3 {
    //             let node = Vec3::new(x as f32 * 32.0, y as f32 * 32.0, z as f32 * 32.0);
    //             lines.line_colored(node - Vec3::X * 0.5, node + Vec3::X * 0.5, 0.0, Color::RED);
    //             lines.line_colored(node - Vec3::Y * 0.5, node + Vec3::Y * 0.5, 0.0, Color::GREEN);
    //             lines.line_colored(node - Vec3::Z * 0.5, node + Vec3::Z * 0.5, 0.0, Color::BLUE);
    //         }
    //     }
    // }
}

enum DigEventType {
    Build,
    Dig,
}

struct DigEvent {
    event_type: DigEventType,
    world_position: Vec3,
}

const DIG_DISTANCE: Real = 4.0;

fn cast_ray(rapier_context: Res<RapierContext>,
            controllers: Query<(&Transform, &Collider, &FpsController), Without<OutlineCube>>,
            mut outline_cube: Query<(&mut Transform, &mut Visibility), With<OutlineCube>>,
            btn: Res<Input<MouseButton>>,
            mut ev: EventWriter<DigEvent>) {

    let mut outline_cube = outline_cube.single_mut();
 
    for (transform, collider, controller) in controllers.iter() {
        if let Some(capsule) = collider.as_capsule() {
            let camera_height = capsule.segment().b().y + capsule.radius() * 0.75;
            let ray_pos = transform.translation + Vec3::Y * camera_height;
            let quat = Quat::from_euler(EulerRot::ZYX, 0.0, controller.yaw, controller.pitch);
            let ray_dir = -quat.mul_vec3(Vec3::Z);
            let max_toi = DIG_DISTANCE;
            let solid = false;
            let filter = QueryFilter { flags: QueryFilterFlags::ONLY_FIXED, ..default() };


            if let Some((_entity, toi)) = rapier_context.cast_ray(
                ray_pos, ray_dir, max_toi, solid, filter,
            ) {

                // TODO: use normal to identify which side of cube build
                let inside_hit_point = ray_pos + ray_dir * (toi * 1.1);

                let position = inside_hit_point.floor() + Vec3::new(0.5, 0.5, 0.5);

                outline_cube.0.translation = position;
                *outline_cube.1 = Visibility::Visible;

                if btn.just_pressed(MouseButton::Left) {
                    ev.send(DigEvent { event_type: DigEventType::Dig, world_position: inside_hit_point });
                } else if btn.just_pressed(MouseButton::Right) {
                    let shape = Collider::cuboid(0.5, 0.5, 0.5);
                    let outside_hit_point = ray_pos + ray_dir * (toi * 0.9);
                    let shape_pos = outside_hit_point.floor() + Vec3::new(0.5, 0.5, 0.5);
                    let shape_rot = Quat::IDENTITY;
                    let filter = QueryFilter::only_dynamic();

                    let mut allow = true;
                    rapier_context.intersections_with_shape(
                        shape_pos, shape_rot, &shape, filter, |_entity| {
                            allow = false;
                            true
                        });
                    if allow {
                        ev.send(DigEvent { event_type: DigEventType::Build, world_position: outside_hit_point });
                    }
                }
            } else {
                *outline_cube.1 = Visibility::Hidden;
            }
        }
    }
}

pub fn manage_cursor(
    mut windows: Query<&mut Window>,
    btn: Res<Input<MouseButton>>,
    key: Res<Input<KeyCode>>,
    mut controllers: Query<&mut FpsController>,
) {
    let mut window = windows.single_mut();
    if btn.just_pressed(MouseButton::Left) {
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
        for mut controller in controllers.iter_mut() {
            controller.enable_input = true;
        }
    }
    if key.just_pressed(KeyCode::Escape) {
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;
        for mut controller in controllers.iter_mut() {
            controller.enable_input = false;
        }
    }
}