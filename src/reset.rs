use crate::{
    animation::{get_scale_out_anim, TweenDoneAction},
    input_binding::{InputAction, PlayerInput},
    GameState,
};
use bevy::prelude::*;
use bevy_time::{ScaledTime, ScaledTimeDelta};

pub struct ResetPlugin;
impl Plugin for ResetPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<ResetData>()
            .add_system_set(SystemSet::on_enter(GameState::Reset).with_system(start_reset))
            .add_system_set(SystemSet::on_update(GameState::Reset).with_system(reset))
            .add_system_set(SystemSet::on_update(GameState::Game).with_system(handle_reset_input));
    }
}

#[derive(Component)]
pub struct Persistent;

#[derive(Default)]
struct ResetData {
    reset_in: Option<Timer>,
}

fn handle_reset_input(mut input: ResMut<PlayerInput>, mut state: ResMut<State<GameState>>) {
    for id in 1..=4 {
        if input.just_pressed(id, InputAction::Reset) {
            input.use_button_action(id, InputAction::Reset);
            state.overwrite_push(GameState::Reset).unwrap();
            break;
        }
    }
}

fn start_reset(
    mut commands: Commands,
    despawn_q: Query<(Entity, Option<&Transform>), (Without<Persistent>, Without<Parent>)>,
    mut reset: ResMut<ResetData>,
) {
    for (e, t) in despawn_q.iter() {
        if let Some(t) = t {
            commands.entity(e).insert(get_scale_out_anim(
                t.scale,
                350,
                Some(TweenDoneAction::DespawnRecursive),
            ));
        } else {
            commands.entity(e).despawn_recursive();
        }
    }

    reset.reset_in = Some(Timer::from_seconds(0.6, false));
}

fn reset(mut state: ResMut<State<GameState>>, mut reset: ResMut<ResetData>, time: ScaledTime) {
    if let Some(timer) = reset.reset_in.as_mut() {
        timer.tick(time.scaled_delta());

        if timer.just_finished() {
            reset.reset_in = None;
            state.overwrite_push(GameState::Game).unwrap();
        }
    }
}
