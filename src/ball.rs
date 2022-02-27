use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
use bevy_input::ActionInput;
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};
use heron::rapier_plugin::PhysicsWorld;
use heron::*;

use crate::{player::{PlayerSwing, ActionStatus, PlayerMovement, Player}, PlayerInput, InputAxis, wall::Wall, WIN_WIDTH};

const BALL_SIZE: f32 = 30.;

#[derive(Default, Component, Inspectable)]
pub struct Ball {
    dir: Vec2,
    size: f32,
}

#[derive(Default, Component, Inspectable)]
pub struct BallBounce {
    gravity: f32,
    velocity: f32,
    max_velocity: f32,
}

pub struct BallPlugin;
impl Plugin for BallPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_startup_system(setup)
            .add_system(movement)
            .add_system(bounce)
            .add_system_to_stage(CoreStage::PostUpdate, handle_collisions);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let bounce = commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("icon.png"),
        sprite: Sprite {
            color: Color::YELLOW,
            custom_size: Some(Vec2::splat(BALL_SIZE)),
            ..Default::default()
        },
        transform: Transform::from_xyz(0., 0., 2.),
        ..Default::default()
        }).insert(BallBounce {
            gravity: -380.,
            max_velocity: 150.,
            ..Default::default()
        })
        .id();

    let shadow = commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("icon.png"),
        sprite: Sprite {
            color: Color::BLACK,
            custom_size: Some(Vec2::splat(BALL_SIZE * 0.7)),
            ..Default::default()
        },
        ..Default::default()
        }).id();

    commands.spawn()
        .insert(Transform::from_xyz( -WIN_WIDTH / 2. + 250., 0., 0.))
        .insert(GlobalTransform::default())
        .insert(Ball {
            size: BALL_SIZE,
            ..Default::default()
        })
        .insert(RigidBody::KinematicPositionBased)
        .insert(CollisionShape::Sphere {
            radius: 15.,
        })
        .insert(Name::new("Ball"))
        .add_child(bounce)
        .add_child(shadow);
}

fn movement(
    mut query: Query<(&mut Ball, &mut Transform)>,
    time: ScaledTime,
) {
    for (mut ball, mut t) in query.iter_mut() {
        if ball.dir == Vec2::ZERO {
            continue;
        }

        let speed = ball.dir.length();

        if speed < 0.025 {
            ball.dir = Vec2::ZERO;
            return;
        }

        // very simple drag
        let drag_mult = if speed < 0.25 { 1. } else { 0.35 };
        ball.dir *= 1. - drag_mult * time.scaled_delta_seconds();

        // move
        t.translation += ball.dir.to_vec3() * 800. * time.scaled_delta_seconds();
    }
}

fn get_bounce_velocity(dir_len: f32, max_velocity: f32) -> f32 {
    // todo: non-linear
    dir_len.sqrt().min(1.) * max_velocity
}

fn bounce(
    mut bounce_query: Query<(&mut BallBounce, &mut Transform, &Parent)>,
    mut ball_q: Query<&mut Ball>,
    time: ScaledTime,
) {
    for (mut ball_bounce, mut t, p) in bounce_query.iter_mut() {
        let mut ball = ball_q.get_mut(p.0).unwrap();

        if ball.dir == Vec2::ZERO {
            continue;
        }

        ball_bounce.velocity += ball_bounce.gravity * time.scaled_delta_seconds();
        t.translation.y += ball_bounce.velocity * time.scaled_delta_seconds();

        if t.translation.y <= 0. {
            ball_bounce.velocity = get_bounce_velocity(ball.dir.length(), ball_bounce.max_velocity);
        }

        // let dir_len = ball.dir.length();
        // // ball_bounce.velocity += -30. * time.scaled_delta_seconds();
        
        // if ball_bounce.velocity <= 0. {
        //     ball_bounce.velocity = get_bounce_duration(dir_len);
        // }

        // t.translation.y += (ball_bounce.velocity - ball_bounce.gravity) * time.scaled_delta_seconds();
    }
}

// todo: 'dashing swing'?
fn handle_collisions(
    mut coll_events: EventReader<CollisionEvent>,
    input: Res<PlayerInput>,
    mut ball_q: Query<(&mut Ball, &Children)>,
    mut ball_bounce_q: Query<&mut BallBounce>,
    mut player_q: Query<(&Player, &PlayerMovement, &mut PlayerSwing)>,
    wall_q: Query<&Sprite, With<Wall>>,
) {
    for ev in coll_events.iter() {
        if ev.is_started() {
            let mut ball;
            let other_e;
            let bounce_e;
            let (entity_1, entity_2) = ev.rigid_body_entities();
            if let Ok(b) = ball_q.get_mut(entity_1) {
                ball = b.0;
                bounce_e = b.1.iter().nth(0).unwrap();
                other_e = entity_2;
            } else if let Ok(b) = ball_q.get_mut(entity_2) {
                ball = b.0;
                bounce_e = b.1.iter().nth(0).unwrap();
                other_e = entity_1;
            } else {
                continue;
            }

            let mut ball_bounce = ball_bounce_q.get_mut(bounce_e.clone()).unwrap();

            if let Ok((player, movement, mut swing)) = player_q.get_mut(other_e) {
                if let ActionStatus::Active(ball_speed_multiplier) = swing.status {
                    if !swing.timer.finished() {
                        swing.start_cooldown();
                        // todo: limit angle to roughly 45deg?
                        let mut dir = input.get_xy_axes(player.id, &InputAxis::X, &InputAxis::Y);

                        if dir == Vec2::ZERO {
                            dir = movement.last_dir;
                        }

                        ball.dir = dir * ball_speed_multiplier;
                        ball_bounce.velocity = get_bounce_velocity(dir.length(), ball_bounce.max_velocity);
                    }
                }
            }
            else if let Ok(wall_sprite) = wall_q.get(other_e) {
                let size = wall_sprite.custom_size.unwrap();
                let is_hor = size.x > size.y;
                let x = if is_hor { 1. } else { -1. }; 
                ball.dir *= Vec2::new(x, -x);
            }
        }
    }
}
