use bevy::prelude::*;
use bevy_inspector_egui::{WorldInspectorPlugin, RegisterInspectable};
use bevy_prototype_lyon::prelude::{PathBuilder, GeometryBuilder, StrokeMode, DrawMode, StrokeOptions, LineCap, LineJoin, Path};
use bevy_time::ScaledTime;
use crate::{player::{Player, PlayerMovement, PlayerDash, PlayerSwing, PlayerScore}, ball::{Ball, BallBounce}, level::CourtRegion};

pub struct DebugPlugin;
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_plugin(WorldInspectorPlugin::new())
            .register_inspectable::<Player>()
            .register_inspectable::<PlayerMovement>()
            .register_inspectable::<PlayerDash>()
            .register_inspectable::<PlayerSwing>()
            .register_inspectable::<PlayerScore>()
            .register_inspectable::<Ball>()
            .register_inspectable::<BallBounce>()
            .register_inspectable::<CourtRegion>()
            .add_startup_system(test_setup)
            .add_system(test_system);
    }
}

fn test_setup(mut commands: Commands) {
}

fn test_system(
    mut path_q: Query<&mut Path>,
    time: ScaledTime,
) {
}
