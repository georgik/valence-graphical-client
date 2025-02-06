use bevy::asset::Assets;
use bevy::color::Color;
use bevy::math::Vec3;
use bevy::pbr::{PbrBundle, StandardMaterial};
use bevy::prelude::{default, Camera3dBundle, Commands, Cuboid, Mesh, PositionType, ResMut, Style, Text, TextBundle, TextStyle, Transform, Val};
use bevy::render::view::NoFrustumCulling;
use crate::GlowingCube;

pub(crate) fn setup_ui(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    // 3D Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(16.0, 16.0, 16.0).looking_at(Vec3::new(8.0, 0.0, 8.0), Vec3::Y),
            ..default()
        },
        NoFrustumCulling,
    ));

    // Text HUD
    commands.spawn(TextBundle {
        text: Text::from_section(
            "Connecting...",
            TextStyle {
                font_size: 30.0,
                color: Color::WHITE,
                ..default()
            },
        ),
        style: Style {
            position_type: PositionType::Absolute, // Important for HUD
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        ..default()
    });

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: materials.add(Color::rgb(0.0, 1.0, 0.0)), // Initial green color
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
        GlowingCube, // Mark this cube as the one that glows
    ));

}
