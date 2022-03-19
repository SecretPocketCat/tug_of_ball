use crate::{
    ai_player_controller::AiPlayerInputs,
    ball::{Ball, BallBounce},
    level::CourtRegion,
    player::{Player, PlayerMovement, PlayerSwing},
};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::Path;
use bevy_time::ScaledTime;

pub struct DebugPlugin;
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(test_setup).add_system(test_system);
    }
}

fn test_setup(_commands: Commands) {}

fn test_system(_path_q: Query<&mut Path>, _time: ScaledTime) {}
