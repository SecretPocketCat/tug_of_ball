// nice2have: make this a feature
// // disable console opening on windows
// #![windows_subsystem = "windows"]

#![feature(derive_default_enum)]
#![feature(if_let_guard)]

use bevy::{prelude::*, render::render_resource::FilterMode};
use bevy_tweening::TweeningPlugin;
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
use tween::TweenPlugin;
use wall::WallPlugin;

mod player;
mod ball;
mod wall;
mod debug;
mod score;
mod level;
mod tween;

const NAME: &str = "Tennis Rounds";
const WIN_WIDTH: f32 = 1600.;
const WIN_HEIGHT: f32 = 900.;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum InputAction {
    Swing,
    Dash,
    LockPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum InputAxis {
    MoveX,
    MoveY,
    AimX,
    AimY,
}

type PlayerInput = ActionInput<InputAction, InputAxis>;

#[derive(PhysicsLayer)]
enum PhysLayer {
    All
    // World,
    // Player,
    // Ball,
}

#[derive(Bundle, Default)]
pub struct TransformBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl TransformBundle {
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            transform: Transform::from_xyz(x, y, z),
            ..Default::default()
        }
    }
}

const COURT_Z: f32 = 1.;
const COURT_LINES_Z: f32 = COURT_Z + 1.;
const PLAYER_Z: f32 = COURT_LINES_Z + 1.;
const BALL_Z: f32 = PLAYER_Z + 1.;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            title: NAME.to_string(),
            width: WIN_WIDTH,
            height: WIN_HEIGHT,
            scale_factor_override: Some(1.),
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb_u8(137, 170, 100)))
        .add_plugins(DefaultPlugins)
        .add_plugin(PhysicsPlugin::default())
        .add_plugin(TweeningPlugin)
        .add_plugin(TimePlugin)
        .add_plugin(ActionInputPlugin::<InputAction, InputAxis>::default())
        .add_plugin(PlayerPlugin)
        .add_plugin(BallPlugin)
        .add_plugin(ScorePlugin)
        .add_plugin(TweenPlugin)
        // .add_plugin(WallPlugin)
        .add_plugin(LevelPlugin)
        .add_plugin(DebugPlugin)
        .add_startup_system(setup)
        .add_startup_system(setup_bindings.chain(panic_on_error))
        .add_system(set_img_sampler_filter)
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
    let deadzone = 0.15;

    for id in 1..=2 {
        map
        .bind_button_action(id, InputAction::Dash, GamepadButtonType::RightTrigger)?
        .bind_button_action(id, InputAction::Dash, GamepadButtonType::RightTrigger2)?
        .bind_button_action(id, InputAction::Swing, GamepadButtonType::South)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::West)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::East)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::North)?
            .bind_button_action(id, InputAction::Swing, GamepadButtonType::LeftTrigger2)?
            .bind_button_action(id, InputAction::LockPosition, GamepadButtonType::LeftTrigger)?
            .bind_axis_with_deadzone(
                id,
                InputAxis::MoveX,
                AxisBinding::GamepadAxis(GamepadAxisType::LeftStickX),
                deadzone
            )
            .bind_axis_with_deadzone(
                id,
                InputAxis::MoveX,
                AxisBinding::GamepadAxis(GamepadAxisType::DPadX),
                deadzone
            )
            .bind_axis_with_deadzone(
                id,
                InputAxis::MoveY,
                AxisBinding::GamepadAxis(GamepadAxisType::LeftStickY),
                deadzone
            )
            .bind_axis_with_deadzone(
                id,
                InputAxis::MoveY,
                AxisBinding::GamepadAxis(GamepadAxisType::DPadY),
                deadzone
            ).bind_axis_with_deadzone(
                id,
                InputAxis::AimX,
                AxisBinding::GamepadAxis(GamepadAxisType::RightStickX),
                deadzone
            ).bind_axis_with_deadzone(
                id,
                InputAxis::AimY,
                AxisBinding::GamepadAxis(GamepadAxisType::RightStickY),
                deadzone
            );

        gamepad_map.map_gamepad(id - 1, id);
    }

    // gamepad_map.map_gamepad(0, 1);

    map
        .bind_button_action(1, InputAction::Dash, KeyCode::Space)?
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

    map
        .bind_button_action(2, InputAction::Dash, KeyCode::Numpad0)?
        .bind_button_action(2, InputAction::Swing, KeyCode::NumpadAdd)?
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

fn set_img_sampler_filter(
    mut ev_asset: EventReader<AssetEvent<Image>>,
    mut assets: ResMut<Assets<Image>>,
) {
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } |
            AssetEvent::Modified { handle } => {
                // set sampler filtering to add some AA (quite fuzzy though)
                let mut texture = assets.get_mut(handle).unwrap();
                texture.sampler_descriptor.mag_filter = FilterMode::Linear;
                texture.sampler_descriptor.min_filter = FilterMode::Linear;
            }
            _ => { }
        }
    }
}

fn inverse_lerp(a: f32, b: f32, t: f32) -> f32 {
    (t - a) / (b - a)
}
