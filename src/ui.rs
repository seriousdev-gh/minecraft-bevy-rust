use bevy::prelude::*;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, DiagnosticsStore, Diagnostic, RegisterDiagnostic};

pub struct MyUiPlugin;

impl Plugin for MyUiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, setup)
            .add_systems(Update, text_update_system)
            .register_diagnostic(Diagnostic::new(FrameTimeDiagnosticsPlugin::FPS, "FPS", 10));
    }
}

// A unit struct to help identify the FPS UI component, since there may be many Text components
#[derive(Component)]
struct FpsText;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // root node
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
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
                                width: Val::Px(5.0),
                                height: Val::Px(5.0),
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

fn text_update_system(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in &mut query {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                text.sections[0].value = format!("FPS {:.0}", value.round());
            }
        }
    }
}
