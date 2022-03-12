use bevy::prelude::*;

use crate::reset::Persistent;

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(setup);
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(Persistent);
    commands
        .spawn_bundle(UiCameraBundle::default())
        .insert(Persistent);
}
