use std::time::Duration;

use bevy::{
    math::Vec2,
    prelude::*,
    sprite::{collide_aabb::collide, Sprite, SpriteBundle},
};
use bevy_extensions::Vec2Conversion;
use bevy_input::ActionState;
use bevy_inspector_egui::Inspectable;

use bevy_time::{ScaledTime, ScaledTimeDelta};
use bevy_tweening::lens::{TransformPositionLens, TransformRotationLens, TransformScaleLens};
use bevy_tweening::*;
use heron::*;
use interpolation::EaseFunction;

use crate::{
    animation::{inverse_lerp, TweenDoneAction},
    ball::{spawn_ball, Ball, BallBouncedEvt, BallStatus},
    extra::TransformBundle,
    input::PlayerInput,
    level::{CourtRegion, CourtSettings, InitialRegion, Net, NetOffset},
    palette::PaletteColor,
    physics::PhysLayer,
    render::{PLAYER_Z, SHADOW_Z},
    score::{add_point_to_score, Score},
    trail::FadeOutTrail,
    InputAction, InputAxis, WIN_HEIGHT, WIN_WIDTH,
};

pub struct PlayerControllerPlugin;
impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {}
}
