use std::time::Duration;

use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};

use crate::player::{PlayerDash, PlayerSwing};

pub struct PlayerActionPlugin;
impl Plugin for PlayerActionPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(handle_action_cooldown::<PlayerDash, Vec2>)
                .with_system(handle_action_cooldown::<PlayerSwing, f32>),
        );
    }
}

#[derive(Inspectable, Clone, Copy)]
pub enum ActionStatus<TActiveData: Default> {
    Ready,
    Active(TActiveData),
    Cooldown,
}

impl<TActiveData: Default> Default for ActionStatus<TActiveData> {
    fn default() -> Self {
        ActionStatus::Ready
    }
}

pub trait ActionTimer<TActiveData: Default> {
    fn get_timer_mut(&mut self) -> &mut Timer;

    fn get_action_status_mut(&mut self) -> &mut ActionStatus<TActiveData>;

    fn get_cooldown_sec(&self) -> f32;

    fn handle_action_timer(&mut self, scaled_delta_time: Duration) {
        let cooldown_sec = self.get_cooldown_sec();
        let status = self.get_action_status_mut();
        let is_cooldown = matches!(status, ActionStatus::Cooldown);
        let is_active = matches!(status, ActionStatus::Active(_));

        if is_cooldown || is_active {
            let t = self.get_timer_mut();
            t.tick(scaled_delta_time);

            if t.just_finished() {
                *t = Timer::from_seconds(cooldown_sec, false);
                *self.get_action_status_mut() = if is_cooldown {
                    ActionStatus::Ready
                } else {
                    ActionStatus::Cooldown
                };
            }
        }
    }
}

fn handle_action_cooldown<T: ActionTimer<TActiveData> + Component, TActiveData: Default>(
    mut query: Query<&mut T>,
    time: ScaledTime,
) {
    for mut activity in query.iter_mut() {
        activity.handle_action_timer(time.scaled_delta());
    }
}

#[macro_export]
macro_rules! impl_player_action_timer {
    ($t: ty, $value_t: ty) => {
        impl ActionTimer<$value_t> for $t {
            fn get_cooldown_sec(&self) -> f32 {
                self.cooldown_sec
            }

            fn get_timer_mut(&mut self) -> &mut Timer {
                &mut self.timer
            }

            fn get_action_status_mut(&mut self) -> &mut ActionStatus<$value_t> {
                &mut self.status
            }
        }
    };
}
