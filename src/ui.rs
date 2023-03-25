//! This example illustrates the various features of Bevy UI.


use bevy::{
    prelude::*,
};

use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};



pub struct MyUiPlugin;

impl Plugin for MyUiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_startup_system(setup)
            .add_system(text_update_system)
        ;
    }
}

// A unit struct to help identify the FPS UI component, since there may be many Text components
#[derive(Component)]
struct FpsText;


fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    // commands.spawn(Camera2dBundle {
    //     camera_2d: Camera2d {
    //         // Don't draw a clear color on top of the 3d stuff
    //         clear_color: ClearColorConfig::None,
    //         ..default()
    //     },
    //     camera: Camera {
    //         // renders after / on top of the main camera
    //         order: 10,
    //         ..default()
    //     },
    //     ..default()
    // });

    // root node
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                        position_type: PositionType::Absolute,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                size: Size::new(Val::Px(5.0), Val::Px(5.0)),
                                ..default()
                            },
                            background_color: Color::rgb(1.0, 1.0, 1.0).into(),
                            ..default()
                        });
                });
            parent.spawn((
                // Create a TextBundle that has a Text with a list of sections.
                TextBundle::from_section(
                    "-",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 20.0,
                        color: Color::WHITE,
                    },
                ),
                FpsText,
            ));
        });
}

fn text_update_system(diagnostics: Res<Diagnostics>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in &mut query {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                text.sections[0].value = format!("FPS {:.0}", value.round());
            }
        }
    }
}
