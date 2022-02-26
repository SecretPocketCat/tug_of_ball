use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
use bevy_input::ActionInput;
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};
use heron::rapier_plugin::PhysicsWorld;
use heron::*;

use crate::{player::{PlayerSwing, ActionStatus, PlayerMovement, Player}, PlayerInput, InputAxis};

#[derive(Default, Component, Inspectable)]
pub struct Ball {
    dir: Vec2,
}

pub struct BallPlugin;
impl Plugin for BallPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_startup_system(setup)
            .add_system(movement)
            .add_system(handle_collisions);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("icon.png"),
        sprite: Sprite {
            color: Color::BLACK,
            custom_size: Some(Vec2::splat(30.)),
            ..Default::default()
        },
        ..Default::default()
    }).insert(Ball::default())
    // .insert(RigidBody::KinematicPositionBased)
    // .insert(CollisionShape::Sphere {
    //     radius: 15.,
    // })
    .insert(Name::new("Ball"));
}

fn movement(
    mut query: Query<(&mut Ball, &mut Transform)>,
    time: ScaledTime,
) {
    for (mut p, mut t) in query.iter_mut() {
        t.translation += p.dir.to_vec3() * 800. * time.scaled_delta_seconds();
    }
}

// todo: 'dashing swing'?
fn handle_collisions(
    input: Res<PlayerInput>,
    mut ball_q: Query<(&mut Ball, &Transform)>,
    mut player_q: Query<(&Player, &mut PlayerSwing)>,
    phys: PhysicsWorld,
    time: ScaledTime,
) {
    for (mut ball, ball_t) in ball_q.iter_mut() {
        // offset on Z if no movement to prevent shape_cast from panicking
        let end_offset = if ball.dir == Vec2::ZERO { Vec2::splat(0.01).to_vec3() } else { ball.dir.to_vec3() * time.scaled_delta_seconds() };
        if let Some(hit) = phys.shape_cast(&CollisionShape::Sphere { radius: 90. }, ball_t.translation, Quat::IDENTITY, ball_t.translation + end_offset) {
            if let Ok((player, mut swing)) = player_q.get_mut(hit.entity) {
                if let ActionStatus::Active(_) = swing.status {
                    if !swing.timer.finished() {
                        swing.start_cooldown();
                        let dir = input.get_xy_axes(player.id, &InputAxis::X, &InputAxis::Y);
                        ball.dir = dir;
                    }
                }
            }
        }
    }
}

