#![cfg_attr(feature = "release", windows_subsystem = "windows")]
#![feature(derive_default_enum)]
#![feature(if_let_guard)]
#![feature(drain_filter)]
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use animation::{AnimationPlugin, TweenPlugin};
use asset::AssetPlugin;
use ball::BallPlugin;
use bevy::prelude::*;
use bevy_input::ActionInputPlugin;
use bevy_prototype_lyon::plugin::ShapePlugin;
use bevy_time::TimePlugin;
use bevy_tweening::TweeningPlugin;
use camera::CameraPlugin;
use heron::*;
use input::{InputAction, InputAxis, InputPlugin};
use level::{CourtRegion, InitialRegion, LevelPlugin};
use palette::PalettePlugin;
use player::PlayerPlugin;
use player_action::PlayerActionPlugin;
use player_animation::PlayerAnimationPlugin;
use player_controller::PlayerControllerPlugin;
use score::ScorePlugin;
use trail::TrailPlugin;
use window::{WIN_HEIGHT, WIN_WIDTH};

// todo: namespace modules (e.g. player)
mod ai_player_controller;
mod animation;
mod asset;
mod ball;
mod camera;
mod debug;
mod extra;
mod input;
mod level;
mod palette;
mod physics;
mod player;
mod player_action;
mod player_animation;
mod player_controller;
mod render;
mod score;
mod trail;
mod window;

const NAME: &str = "Tag of Ball";

fn main() {
    let region = CourtRegion::get_random();
    let scale_factor_override = None;

    #[cfg(feature = "debug")]
    {
        region = CourtRegion::TopLeft;
        scale_factor_override = Some(1.);
    }

    let mut app = App::new();
    app.insert_resource(Msaa { samples: 4 })
        // resources needed before default plugins to take effect
        .insert_resource(WindowDescriptor {
            title: NAME.to_string(),
            width: WIN_WIDTH,
            height: WIN_HEIGHT,
            resizable: false,
            scale_factor_override,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::WHITE))
        // game resources
        .insert_resource(InitialRegion(region))
        // bevy plugins
        .add_plugins(DefaultPlugins)
        // 3rd party crates
        .add_plugin(PhysicsPlugin::default())
        .add_plugin(TweeningPlugin)
        // game crates
        .add_plugin(TimePlugin)
        .add_plugin(ActionInputPlugin::<InputAction, InputAxis>::default())
        // game plugins
        .add_plugin(AnimationPlugin)
        .add_plugin(AssetPlugin)
        .add_plugin(BallPlugin)
        .add_plugin(CameraPlugin)
        .add_plugin(InputPlugin)
        .add_plugin(LevelPlugin)
        .add_plugin(PalettePlugin)
        .add_plugin(PlayerPlugin)
        .add_plugin(PlayerControllerPlugin)
        .add_plugin(PlayerActionPlugin)
        .add_plugin(PlayerAnimationPlugin)
        .add_plugin(ScorePlugin)
        .add_plugin(TrailPlugin)
        .add_plugin(TweenPlugin);

    // heron 2d-debug adds lyon plugin as well, which would cause a panic
    #[cfg(not(feature = "debug"))]
    {
        app.add_plugin(ShapePlugin);
    }

    #[cfg(feature = "debug")]
    {
        app.add_plugin(DebugPlugin);
    }

    app.run();
}
