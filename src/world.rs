use bevy::asset::Assets;
use bevy::color::Color;
use bevy::pbr::{PbrBundle, StandardMaterial};
use bevy::prelude::{default, Commands, Cuboid, Mesh, Res, ResMut, Resource, Transform};
use crate::WorldCube;
use bevy::prelude::DetectChanges;

#[derive(Resource)]
struct WorldData {
    // Store world data here.  For example, a 2D grid of block colors:
    blocks: Vec<Vec<Vec<Color>>>,
    width: usize,
    height: usize,
    depth: usize
}

fn update_world_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>, // Use StandardMaterial
    world_data: Res<WorldData>,
) {
    if world_data.is_changed() {
        // Clear existing cubes


        if !world_data.blocks.is_empty() {
            let width = world_data.width;
            let height = world_data.height;
            let depth = world_data.depth; // Assuming you have depth in your world data

            for x in 0..width {
                for y in 0..height {
                    for z in 0..depth {
                        let color = world_data.blocks[x][y][z]; // Access 3D block data

                        commands.spawn((
                            PbrBundle { // Use PbrBundle for 3D
                                mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)), // Create a cube mesh
                                material: materials.add(color), // Use the block color as material
                                transform: Transform::from_xyz(x as f32, y as f32, z as f32),
                                ..default()
                            },
                            WorldCube { x, y, z },
                        ));
                    }
                }
            }
        }
    }
}

