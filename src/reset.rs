use crate::{
    input_binding::{InputAction, PlayerInput},
    GameState,
};
use bevy::prelude::*;

pub struct ResetPlugin;
impl Plugin for ResetPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system_set(SystemSet::on_enter(GameState::Reset).with_system(reset))
            .add_system_set(SystemSet::on_update(GameState::Game).with_system(handle_reset_input));
    }
}

#[derive(Component)]
pub struct Persistent;

fn handle_reset_input(mut input: ResMut<PlayerInput>, mut state: ResMut<State<GameState>>) {
    for id in 1..=4 {
        if input.just_pressed(id, InputAction::Reset) {
            input.use_button_action(id, InputAction::Reset);
            state.overwrite_push(GameState::Reset).unwrap();
            break;
        }
    }
}

fn reset(
    mut commands: Commands,
    mut state: ResMut<State<GameState>>,
    despawn_q: Query<Entity, (Without<Persistent>, Without<Parent>)>,
) {
    for e in despawn_q.iter() {
        commands.entity(e).despawn_recursive();
    }

    state.overwrite_push(GameState::Game).unwrap();
}
