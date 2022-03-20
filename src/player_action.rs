use std::time::Duration;

use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};

use crate::{player::PlayerSwing, GameState};

pub struct PlayerActionPlugin;
impl Plugin for PlayerActionPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system_set(
            SystemSet::on_update(GameState::Game)
                .with_system(handle_action_cooldown::<PlayerSwing, f32, false>),
        );
    }
}

#[derive(Inspectable, Clone, Copy, Debug)]
pub enum PlayerActionStatus<TActiveData: Default> {
    Ready,
    Charging(f32),
    Active(TActiveData),
    Cooldown,
}

impl<TActiveData: Default> Default for PlayerActionStatus<TActiveData> {
    fn default() -> Self {
        PlayerActionStatus::Ready
    }
}

pub trait ActionTimer<TActiveData: Default> {
    fn get_timer_mut(&mut self) -> &mut Timer;

    fn get_action_status_mut(&mut self) -> &mut PlayerActionStatus<TActiveData>;

    fn get_cooldown_sec(&self) -> f32;

    fn handle_action_timer(&mut self, scaled_delta_time: Duration, auto_deactivate: bool) {
        let cooldown_sec = self.get_cooldown_sec();
        let status = self.get_action_status_mut();
        let is_cooldown = matches!(status, PlayerActionStatus::Cooldown);
        let is_active = matches!(status, PlayerActionStatus::Active(_));

        if is_cooldown || is_active {
            let t = self.get_timer_mut();
            t.tick(scaled_delta_time);

            if t.just_finished() {
                if is_cooldown {
                    *self.get_action_status_mut() = PlayerActionStatus::Ready;
                } else if auto_deactivate {
                    *t = Timer::from_seconds(cooldown_sec, false);
                    *self.get_action_status_mut() = PlayerActionStatus::Cooldown;
                };
            }
        }
    }
}

fn handle_action_cooldown<
    T: ActionTimer<TActiveData> + Component,
    TActiveData: Default,
    const AUTO_DEACTIVATE: bool,
>(
    mut query: Query<&mut T>,
    time: ScaledTime,
) {
    for mut activity in query.iter_mut() {
        activity.handle_action_timer(time.scaled_delta(), AUTO_DEACTIVATE);
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

            fn get_action_status_mut(&mut self) -> &mut PlayerActionStatus<$value_t> {
                &mut self.status
            }
        }
    };
}
