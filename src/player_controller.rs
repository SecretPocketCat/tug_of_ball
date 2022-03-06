use crate::{
    input_binding::{InputAction, InputAxis, PlayerInput},
    player::{
        get_swing_multiplier_clamped, Player, PlayerDash, PlayerMovement, PlayerSwing,
        SWING_LABEL,
    },
    player_action::PlayerActionStatus,
};
use bevy::prelude::*;

use bevy_input::*;



pub struct PlayerControllerPlugin;
impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(process_player_input.label(SWING_LABEL));
    }
}

// todo: decouple from input, just set target pos and fire an event?
// nice2have: lerp dash
fn process_player_input(
    input: Res<PlayerInput>,
    mut query: Query<(
        &Player,
        &mut PlayerMovement,
        &mut PlayerDash,
        &mut PlayerSwing,
    )>,
) {
    for (player, mut player_movement, mut player_dash, mut player_swing) in query.iter_mut() {
        // movement
        player_movement.raw_dir = if input.held(player.id, InputAction::LockPosition) {
            Vec2::ZERO
        } else {
            input.get_xy_axes_raw(player.id, &InputAxis::MoveX, &InputAxis::MoveY)
        };

        // todo:
        // aim

        // dash
        if input.just_pressed(player.id, InputAction::Dash) {
            if let PlayerActionStatus::Ready = player_dash.status {
                let dir = player_movement.raw_dir.normalize_or_zero();
                player_dash.status = PlayerActionStatus::Active(if dir != Vec2::ZERO {
                    dir
                } else {
                    // todo:
                    // p_aim.direction
                    Vec2::ZERO
                });
                player_dash.timer = Timer::from_seconds(player_dash.duration_sec, false);
            }
        }

        // swing
        // nice2have: on swing down cancel prev swing?
        if let Some(input_action_state) =
            input.get_button_action_state(player.id, &InputAction::Swing)
        {
            match input_action_state {
                ActionState::Pressed => {
                    player_swing.status = PlayerActionStatus::Charging(0.);
                }
                ActionState::Held(key_date) => {
                    player_swing.status = PlayerActionStatus::Charging(key_date.duration);
                }
                ActionState::Released(key_data) => {
                    if let PlayerActionStatus::Ready | PlayerActionStatus::Charging(..) =
                        player_swing.status
                    {
                        player_swing.status = PlayerActionStatus::Active(
                            get_swing_multiplier_clamped(key_data.duration),
                        );
                        player_swing.timer = Timer::from_seconds(player_swing.duration_sec, false);
                    }
                }
                _ => {}
            }
        }
    }
}
