use bevy::prelude::*;
use bevy_inspector_egui::{WorldInspectorPlugin, RegisterInspectable};
use crate::{player::{Player, PlayerMovement, PlayerDash, PlayerSwing}, ball::{Ball, BallBounce}};

pub struct DebugPlugin;
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_plugin(WorldInspectorPlugin::new())
            .register_inspectable::<Player>()
            .register_inspectable::<PlayerMovement>()
            .register_inspectable::<PlayerDash>()
            .register_inspectable::<PlayerSwing>()
            .register_inspectable::<Ball>()
            .register_inspectable::<BallBounce>();
    }
}
