use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
use bevy_input::ActionInput;
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};

use crate::{InputAction, InputAxis};

#[derive(Inspectable, Clone, Copy)]
enum DashStatus {
    Ready,
    Dashing(Vec2),
    Cooldown,
}

#[derive(Component, Inspectable)]
pub struct Player {
    id: usize,
    speed: f32,
    dash_speed: f32,
    dash_status: DashStatus,
    dash_duration_sec: f32,
    dash_cooldown_sec: f32,
    #[inspectable(ignore)]
    dash_timer: Timer,
    last_movement_dir: Vec2, 
}

impl Default for Player {
    fn default() -> Self {
        Self {
            id: 999,
            speed: 800.,
            dash_speed: 2600.,
            dash_duration_sec: 0.2,
            dash_cooldown_sec: 0.25,
            dash_status: DashStatus::Ready,
            dash_timer: Default::default(),
            last_movement_dir: Default::default(),
        }
    }
}

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_startup_system(setup)
            .add_system(movement)
            .add_system(swing);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for i in 1..=2 {
        let x = if i == 1 { -200. } else { 200. }; 
        commands.spawn_bundle(SpriteBundle {
            texture: asset_server.load("icon.png"),
            transform: Transform::from_xyz(x, 0., 0.),
            sprite: Sprite {
                color: if i == 1 { Color::RED } else { Color::MIDNIGHT_BLUE },
                custom_size: Some(Vec2::splat(80.)),
                ..Default::default()
            },
            ..Default::default()
        }).insert(Player {
            id: i,
            last_movement_dir: if x < 0. { Vec2::X } else { -Vec2::X },
            ..Default::default()
        })
        .insert(Name::new("Player"));
    }
}

// todo: movement easing
// possibly during dash as well
fn movement(
    input: Res<ActionInput<InputAction, InputAxis>>,
    mut query: Query<(&mut Player, &mut Transform)>,
    time: ScaledTime,
) {
    for (mut p, mut t) in query.iter_mut() {
        let dir = input.get_xy_axes(p.id, &InputAxis::X, &InputAxis::Y);
        let mut move_by = (dir * p.speed * time.scaled_delta_seconds()).to_vec3();

        if input.just_pressed(p.id, InputAction::Dash) {
            if let DashStatus::Ready = p.dash_status {
                p.dash_status = DashStatus::Dashing(if dir != Vec2::ZERO { dir } else { p.last_movement_dir });
                p.dash_timer = Timer::from_seconds(p.dash_duration_sec, false);
            }
        }

        match p.dash_status {
            DashStatus::Cooldown => {
                p.dash_timer.tick(time.scaled_delta());
    
                if p.dash_timer.just_finished() {
                    p.dash_status = DashStatus::Ready;
                }
            },
            DashStatus::Dashing(dir) => {
                p.dash_timer.tick(time.scaled_delta());
    
                if p.dash_timer.just_finished() {
                    p.dash_status = DashStatus::Cooldown;
                    p.dash_timer = Timer::from_seconds(p.dash_cooldown_sec, false);
                }
                else {
                    move_by = (dir * p.dash_speed * time.scaled_delta_seconds()).to_vec3();
                }
            },
            _ => {},
        }

        if move_by.truncate() != Vec2::ZERO {
            t.translation += move_by;
            p.last_movement_dir = move_by.truncate().normalize_or_zero();
        }
    }
}

fn swing(
    input: Res<ActionInput<InputAction, InputAxis>>,
    mut query: Query<&Player>,
) {
    for player in query.iter_mut() {
        if input.held(player.id, InputAction::Swing) {
            info!("swing!");
        }
    }
}
