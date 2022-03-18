use crate::{
    ai_player_controller::AiPlayer,
    input_binding::{InputAction, InputAxis, PlayerInput},
    player::{Player, PlayerAim, PlayerMovement, PlayerSwing, SWING_LABEL},
    player_action::PlayerActionStatus,
    GameState,
};
use bevy::prelude::*;
use bevy_input::*;

pub const SWING_STRENGTH_MULTIPLIER: f32 = 0.65;

pub struct PlayerControllerPlugin;
impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system_set(
            SystemSet::on_update(GameState::Game)
                .with_system(process_player_input.label(SWING_LABEL)),
        );
    }
}

fn process_player_input(
    input: Res<PlayerInput>,
    mut q: Query<(&Player, &mut PlayerMovement, &mut PlayerSwing), Without<AiPlayer>>,
    mut aim_q: Query<&mut PlayerAim>,
) {
    for (player, mut player_movement, mut player_swing) in q.iter_mut() {
        // movement
        player_movement.raw_dir = if input.held(player.id, InputAction::LockPosition) {
            Vec2::ZERO
        } else {
            input.get_xy_axes_raw(player.id, &InputAxis::MoveX, &InputAxis::MoveY)
        };

        // aim
        if let Ok(mut player_aim) = aim_q.get_mut(player.aim_e) {
            // start with aim dir
            player_aim.raw_dir =
                input.get_xy_axes_raw(player.id, &InputAxis::AimX, &InputAxis::AimY);
            if player_aim.raw_dir == Vec2::ZERO {
                // fallback to movement dir
                player_aim.raw_dir =
                    input.get_xy_axes_raw(player.id, &InputAxis::MoveX, &InputAxis::MoveY);
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
                            (key_data.duration * SWING_STRENGTH_MULTIPLIER).min(1.),
                        );
                        player_swing.timer = Timer::from_seconds(player_swing.duration_sec, false);
                    }
                }
                _ => {}
            }
        }
    }
}
