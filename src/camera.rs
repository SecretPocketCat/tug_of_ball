use crate::{
    animation::asymptotic_smoothing_with_delta_time,
    ball::Ball,
    level::CourtSettings,
    player::{Player, PLAYER_SIZE},
    reset::Persistent,
    score::Score,
};
use bevy::{prelude::*, window::WindowResized};
use bevy_time::{ScaledTime, ScaledTimeDelta};
use std::ops::{Add, Mul};

pub const BASE_VIEW_WIDTH: f32 = 1920.;
pub const BASE_VIEW_HEIGHT: f32 = 1080.;
pub const MIN_SIZE_MULT: f32 = 0.4;
pub const START_MULT: f32 = 1.0;

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(TargetCamScale {
            base_scale: 1.,
            focus_scale: 1.,
            view: Default::default(),
        })
        .add_startup_system(setup)
        .add_system(on_window_resize)
        .add_system(update_focus_scale)
        .add_system(scale_projection)
        .add_system(follow_focus_point);
    }
}

#[derive(Component)]
struct MainCam;

struct TargetCamScale {
    base_scale: f32,
    focus_scale: f32,
    view: Vec2,
}

fn setup(mut commands: Commands) {
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(Persistent)
        .insert(MainCam);
    commands
        .spawn_bundle(UiCameraBundle::default())
        .insert(Persistent);
}

fn follow_focus_point(
    mut cam_q: Query<&mut Transform, With<MainCam>>,
    ball_q: Query<&Transform, (With<Ball>, Without<MainCam>, Without<Player>)>,
    player_q: Query<(&Player, &Transform), (Without<Ball>, Without<MainCam>)>,
    time: ScaledTime,
    score: Res<Score>,
) {
    if let Ok(mut cam_t) = cam_q.get_single_mut() {
        let mut focus = Vec2::ZERO;
        let mut focus_mult = Vec2::new(0.1, 0.05);

        if let Some(is_left) = score.left_has_won {
            if let Some((_, player_t)) = player_q.iter().find(|(p, ..)| p.is_left() == is_left) {
                focus = player_t.translation.truncate();
                focus_mult = Vec2::splat(0.5);
            }
        } else {
            if let Ok(ball_t) = ball_q.get_single() {
                focus = ball_t.translation.truncate();
            }
        }

        let target_pos = Vec3::new(
            focus.x * focus_mult.x,
            focus.y * focus_mult.y,
            cam_t.translation.z,
        );
        cam_t.translation = asymptotic_smoothing_with_delta_time(
            cam_t.translation,
            target_pos,
            0.05,
            time.scaled_delta_seconds(),
        );
    }
}

fn scale_projection(
    mut cam_q: Query<&mut OrthographicProjection, With<MainCam>>,
    cam_scale: Res<TargetCamScale>,
    time: ScaledTime,
    mut court: ResMut<CourtSettings>,
) {
    if let Ok(mut cam_proj) = cam_q.get_single_mut() {
        let scale = cam_scale.base_scale * cam_scale.focus_scale;
        cam_proj.scale = asymptotic_smoothing_with_delta_time(
            cam_proj.scale,
            scale,
            0.035,
            time.scaled_delta_seconds(),
        );

        court.view = cam_scale.view * scale;
    }
}

fn update_focus_scale(
    player_q: Query<&GlobalTransform, With<Player>>,
    mut cam_scale: ResMut<TargetCamScale>,
) {
    let mut x = 0.;
    let mut y = 0.;

    for p_t in player_q.iter() {
        let pos_abs = p_t.translation.abs();
        if pos_abs.x > x {
            x = pos_abs.x;
        }

        if pos_abs.y > y {
            y = pos_abs.y;
        }
    }

    let width_scale = ((x + 100.) / (BASE_VIEW_WIDTH / 2.0)).clamp(1., 2.);
    let height_scale = ((y + 60.) / (BASE_VIEW_HEIGHT / 2.0)).clamp(1., 1.75);
    cam_scale.focus_scale = width_scale.max(height_scale);
}

fn on_window_resize(
    mut evr_resize: EventReader<WindowResized>,
    mut cam_scale: ResMut<TargetCamScale>,
) {
    for ev in evr_resize.iter() {
        if ev.id.is_primary() {
            cam_scale.base_scale = (BASE_VIEW_WIDTH / ev.width).max(BASE_VIEW_HEIGHT / ev.height);
            cam_scale.view = Vec2::new(ev.width, ev.height);
        }
    }
}
