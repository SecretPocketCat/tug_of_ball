use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};
use bevy_tweening::TweenCompleted;

pub struct AnimationPlugin;
impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(rotate).add_system(on_tween_completed);
    }
}

// todo: struct
#[derive(Default, Component, Inspectable)]
pub struct TransformRotation {
    pub rotation_rad: f32,
    pub rotation_max_rad: f32,
}

impl TransformRotation {
    pub fn new(rotation_rad: f32) -> Self {
        Self {
            rotation_rad,
            rotation_max_rad: rotation_rad,
        }
    }
}

#[repr(u64)]
pub enum TweenDoneAction {
    None = 0,
    DespawnRecursive = 1,
}

impl From<u64> for TweenDoneAction {
    fn from(val: u64) -> Self {
        unsafe { ::std::mem::transmute(val) }
    }
}

impl From<TweenDoneAction> for u64 {
    fn from(val: TweenDoneAction) -> Self {
        val as u64
    }
}

fn on_tween_completed(mut commands: Commands, mut ev_reader: EventReader<TweenCompleted>) {
    for ev in ev_reader.iter() {
        match TweenDoneAction::from(ev.user_data) {
            TweenDoneAction::None => {}
            TweenDoneAction::DespawnRecursive => {
                commands.entity(ev.entity).despawn_recursive();
            }
        }
    }
}

fn rotate(mut q: Query<(&TransformRotation, &mut Transform)>, time: ScaledTime) {
    for (r, mut t) in q.iter_mut() {
        t.rotate(Quat::from_rotation_z(
            r.rotation_rad * time.scaled_delta_seconds(),
        ));
    }
}

pub fn inverse_lerp(a: f32, b: f32, t: f32) -> f32 {
    ((t - a) / (b - a)).clamp(0., 1.)
}
