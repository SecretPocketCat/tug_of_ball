use crate::{
    physics::PhysLayer,
    player::{get_swing_multiplier_clamped, Player, PlayerSwing, SWING_LABEL},
    player_action::ActionStatus,
    player_animation::{AgentAnimation, AgentAnimationData},
};
use bevy::prelude::*;
use bevy_extensions::panic_on_error;
use bevy_input::*;
use heron::CollisionLayers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputAction {
    Swing,
    Dash,
    LockPosition,
    ChangePalette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputAxis {
    MoveX,
    MoveY,
    AimX,
    AimY,
}

pub type PlayerInput = ActionInput<InputAction, InputAxis>;

pub struct InputBindingPlugin;
impl Plugin for InputBindingPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(setup_bindings.chain(panic_on_error));
    }
}

fn setup_bindings(
    mut map: ResMut<ActionMap<InputAction, InputAxis>>,
    mut gamepad_map: ResMut<GamepadMap>,
) -> Result<(), BindingError> {
    let deadzone = 0.15;

    for id in 1..=2 {
        map.bind_button_action(id, InputAction::Dash, GamepadButtonType::RightTrigger)?
            .bind_button_action(id, InputAction::Dash, GamepadButtonType::RightTrigger2)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::South)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::West)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::East)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::North)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::LeftTrigger2)?
            .bind_button_action(id, InputAction::ChangePalette, GamepadButtonType::Select)?
            .bind_button_action(
                id,
                InputAction::LockPosition,
                GamepadButtonType::LeftTrigger,
            )?
            .bind_axis_with_deadzone(
                id,
                InputAxis::MoveX,
                AxisBinding::GamepadAxis(GamepadAxisType::LeftStickX),
                deadzone,
            )
            .bind_axis_with_deadzone(
                id,
                InputAxis::MoveX,
                AxisBinding::GamepadAxis(GamepadAxisType::DPadX),
                deadzone,
            )
            .bind_axis_with_deadzone(
                id,
                InputAxis::MoveY,
                AxisBinding::GamepadAxis(GamepadAxisType::LeftStickY),
                deadzone,
            )
            .bind_axis_with_deadzone(
                id,
                InputAxis::MoveY,
                AxisBinding::GamepadAxis(GamepadAxisType::DPadY),
                deadzone,
            )
            .bind_axis_with_deadzone(
                id,
                InputAxis::AimX,
                AxisBinding::GamepadAxis(GamepadAxisType::RightStickX),
                deadzone,
            )
            .bind_axis_with_deadzone(
                id,
                InputAxis::AimY,
                AxisBinding::GamepadAxis(GamepadAxisType::RightStickY),
                deadzone,
            );

        gamepad_map.map_gamepad(id - 1, id);
    }

    map.bind_button_action(1, InputAction::Dash, KeyCode::Space)?
        .bind_button_action(1, InputAction::Swing, KeyCode::J)?
        .bind_axis(
            1,
            InputAxis::MoveX,
            AxisBinding::Buttons(KeyCode::A.into(), KeyCode::D.into()),
        )
        .bind_axis(
            1,
            InputAxis::MoveY,
            AxisBinding::Buttons(KeyCode::S.into(), KeyCode::W.into()),
        );

    map.bind_button_action(2, InputAction::Dash, KeyCode::Numpad0)?
        .bind_button_action(2, InputAction::Swing, KeyCode::NumpadAdd)?
        .bind_button_action(2, InputAction::ChangePalette, KeyCode::P)?
        .bind_axis(
            2,
            InputAxis::MoveX,
            AxisBinding::Buttons(KeyCode::Left.into(), KeyCode::Right.into()),
        )
        .bind_axis(
            2,
            InputAxis::MoveY,
            AxisBinding::Buttons(KeyCode::Down.into(), KeyCode::Up.into()),
        );
    Ok(())
}
