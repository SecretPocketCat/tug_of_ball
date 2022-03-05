use bevy::prelude::*;

use bevy_tweening::TweenCompleted;



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

pub struct TweenPlugin;
impl Plugin for TweenPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(on_tween_completed);
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

pub fn inverse_lerp(a: f32, b: f32, t: f32) -> f32 {
    (t - a) / (b - a)
}
