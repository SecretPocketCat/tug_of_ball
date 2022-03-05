use bevy::prelude::*;

// don't move this to an common dep as this was addeed to bevy main IIRC
#[derive(Bundle, Default)]
pub struct TransformBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl TransformBundle {
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            transform: Transform::from_xyz(x, y, z),
            ..Default::default()
        }
    }
}
