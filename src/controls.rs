use bevy::prelude::*;

pub(crate) fn handle_keyboard_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera3d>>, // Query for the 3D camera's transform
    time: Res<Time>,
) {
    let mut camera_transform = query.single_mut(); // Get the camera's transform

    let movement_speed = 5.0; // Adjust movement speed as needed
    let rotation_speed = 1.0;

    // WASD movement
    if keyboard_input.pressed(KeyCode::KeyW) {
        let forward = camera_transform.forward();
        camera_transform.translation += forward * movement_speed * time.delta_seconds();
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        let back = -camera_transform.forward();
        camera_transform.translation += back * movement_speed * time.delta_seconds();
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        let left = -camera_transform.right();
        camera_transform.translation += left * movement_speed * time.delta_seconds();
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        let right = camera_transform.right();
        camera_transform.translation += right * movement_speed * time.delta_seconds();
    }

    // Q/E rotation (yaw)
    if keyboard_input.pressed(KeyCode::KeyQ) {
        camera_transform.rotate_y(-rotation_speed * time.delta_seconds());
    }
    if keyboard_input.pressed(KeyCode::KeyE) {
        camera_transform.rotate_y(rotation_speed * time.delta_seconds());
    }

    // Shift/Control up/down movement
    if keyboard_input.pressed(KeyCode::ShiftLeft) {
        camera_transform.translation.y -= movement_speed * time.delta_seconds();
    }
    if keyboard_input.pressed(KeyCode::ControlLeft) {
        camera_transform.translation.y += movement_speed * time.delta_seconds();
    }


    // ESC to quit
    if keyboard_input.pressed(KeyCode::Escape) {
        std::process::exit(0);
    }
}