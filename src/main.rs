// todo: make this a feature
// // disable console opening on windows
// #![windows_subsystem = "windows"]

use bevy::prelude::*;
use debug::DebugPlugin;
use heron::*;
use bevy_extensions::panic_on_error;
use bevy_input::{ActionMap, GamepadMap, BindingError, AxisBinding, ActionInputPlugin, ActionInput};
use bevy_time::TimePlugin;
use level::LevelPlugin;
use score::ScorePlugin;
use serde::{Serialize, Deserialize};
use player::{PlayerPlugin, Player, PlayerMovement, PlayerDash, PlayerSwing};
use ball::{BallPlugin, Ball, BallBounce};
use wall::WallPlugin;

mod player;
mod ball;
mod wall;
mod debug;
mod score;
mod level;

const NAME: &str = "Tennis Rounds";
const WIN_WIDTH: f32 = 1600.;
const WIN_HEIGHT: f32 = 900.;

// todo: lvl plugin

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum InputAction {
    Swing,
    Dash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum InputAxis {
    X,
    Y,
}

type PlayerInput = ActionInput<InputAction, InputAxis>;

#[derive(PhysicsLayer)]
enum PhysLayer {
    All
    // World,
    // Player,
    // Ball,
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: NAME.to_string(),
            width: WIN_WIDTH,
            height: WIN_HEIGHT,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb_u8(137, 170, 100)))
        .add_plugins(DefaultPlugins)
        .add_plugin(PhysicsPlugin::default())
        .add_plugin(TimePlugin)
        .add_plugin(ActionInputPlugin::<InputAction, InputAxis>::default())
        .add_plugin(PlayerPlugin)
        .add_plugin(BallPlugin)
        .add_plugin(ScorePlugin)
        // .add_plugin(WallPlugin)
        .add_plugin(LevelPlugin)
        .add_plugin(DebugPlugin)
        .add_startup_system(setup)
        .add_startup_system(setup_bindings.chain(panic_on_error))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
}

fn setup_bindings(
    mut map: ResMut<ActionMap<InputAction, InputAxis>>,
    mut gamepad_map: ResMut<GamepadMap>,
) -> Result<(), BindingError> {
    for id in 1..=2 {
        map
            .bind_button_action(id, InputAction::Dash, GamepadButtonType::South)?
            .bind_button_action(id, InputAction::Dash, GamepadButtonType::RightTrigger)?
            .bind_button_action(id, InputAction::Dash, GamepadButtonType::RightTrigger2)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::West)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::East)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::North)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::LeftTrigger)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::LeftTrigger2)?
            .bind_axis_with_deadzone(
                id,
                InputAxis::X,
                AxisBinding::GamepadAxis(GamepadAxisType::LeftStickX),
                0.2
            )
            .bind_axis_with_deadzone(
                id,
                InputAxis::X,
                AxisBinding::GamepadAxis(GamepadAxisType::DPadX),
                0.2
            )
            .bind_axis_with_deadzone(
                id,
                InputAxis::Y,
                AxisBinding::GamepadAxis(GamepadAxisType::LeftStickY),
                0.2
            )
            .bind_axis_with_deadzone(
                id,
                InputAxis::Y,
                AxisBinding::GamepadAxis(GamepadAxisType::DPadY),
                0.2
            );

        // gamepad_map.map_gamepad(id - 1, id);
    }

    gamepad_map.map_gamepad(0, 2);

    map
        .bind_button_action(1, InputAction::Dash, KeyCode::Space)?
        .bind_button_action(1, InputAction::Swing, KeyCode::J)?
        .bind_axis(
            1,
            InputAxis::X,
            AxisBinding::Buttons(KeyCode::A.into(), KeyCode::D.into()),
        )
        .bind_axis(
            1,
            InputAxis::Y,
            AxisBinding::Buttons(KeyCode::S.into(), KeyCode::W.into()),
        );

    // map
    //     .bind_button_action(2, InputAction::Dash, KeyCode::Numpad0)?
    //     .bind_button_action(2, InputAction::Swing, KeyCode::NumpadAdd)?
    //     .bind_axis(
    //         2,
    //         InputAxis::X,
    //         AxisBinding::Buttons(KeyCode::Left.into(), KeyCode::Right.into()),
    //     )
    //     .bind_axis(
    //         2,
    //         InputAxis::Y,
    //         AxisBinding::Buttons(KeyCode::Down.into(), KeyCode::Up.into()),
    //     );
    
    Ok(())
}
