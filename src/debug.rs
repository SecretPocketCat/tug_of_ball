use crate::{
    ball::{Ball, BallBounce},
    level::CourtRegion,
    player::{Player, PlayerDash, PlayerMovement, PlayerSwing},
};
use bevy::prelude::*;
use bevy_inspector_egui::{RegisterInspectable, WorldInspectorPlugin};
use bevy_prototype_lyon::prelude::{
    DrawMode, GeometryBuilder, LineCap, LineJoin, Path, PathBuilder, StrokeMode, StrokeOptions,
};
use bevy_time::ScaledTime;

pub struct DebugPlugin;
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(WorldInspectorPlugin::new())
            .register_inspectable::<Player>()
            .register_inspectable::<PlayerMovement>()
            .register_inspectable::<PlayerDash>()
            .register_inspectable::<PlayerSwing>()
            .register_inspectable::<Ball>()
            .register_inspectable::<BallBounce>()
            .register_inspectable::<CourtRegion>()
            .add_startup_system(test_setup)
            .add_system(test_system);
    }
}

fn test_setup(mut commands: Commands) {}

fn test_system(mut path_q: Query<&mut Path>, time: ScaledTime) {}
