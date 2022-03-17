use std::time::Duration;

use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};
use bevy_tweening::lens::TransformScaleLens;
use bevy_tweening::*;

pub struct AnimationPlugin;
impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(rotate).add_system(on_tween_completed);
    }
}

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

pub fn get_scale_out_tween(
    start_scale: Vec3,
    duration_ms: u64,
    on_completed: Option<TweenDoneAction>,
) -> Animator<Transform> {
    get_scale_tween(
        start_scale,
        Vec3::ZERO,
        EaseFunction::QuadraticIn,
        duration_ms,
        on_completed,
    )
}

pub fn get_scale_in_tween(
    end_scale: Vec3,
    duration_ms: u64,
    on_completed: Option<TweenDoneAction>,
) -> Animator<Transform> {
    get_scale_tween(
        Vec3::ZERO,
        end_scale,
        EaseFunction::BackOut,
        duration_ms,
        on_completed,
    )
}

pub fn get_scale_tween(
    start_scale: Vec3,
    end_scale: Vec3,
    ease: EaseFunction,
    duration_ms: u64,
    on_completed: Option<TweenDoneAction>,
) -> Animator<Transform> {
    let mut tween = Tween::new(
        ease,
        TweeningType::Once,
        Duration::from_millis(duration_ms),
        TransformScaleLens {
            start: start_scale,
            end: end_scale,
        },
    );

    if let Some(on_completed) = on_completed {
        tween = tween.with_completed_event(true, on_completed.into());
    }

    Animator::new(tween)
}
