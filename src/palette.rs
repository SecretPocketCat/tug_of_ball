use bevy::prelude::*;
use bevy_input::*;
use bevy_prototype_lyon::prelude::{DrawMode, FillMode, StrokeMode};
use bevy_tweening::{
    lens::{SpriteColorLens, TextColorLens},
    Animator, EaseFunction, Tween, TweeningType,
};
use rand::random;

use crate::{
    input::{InputAction, PlayerInput},
    level::Court,
    trail::Trail,
};

const COURT_STROKE_WIDTH: f32 = 10.;

pub struct PalettePlugin;
impl Plugin for PalettePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(on_palette_changed)
            .add_system(on_sprite_added)
            .add_system(on_text_added)
            .add_system(on_trail_added)
            .add_system(on_court_added)
            .add_system(handle_input)
            .insert_resource(if random::<bool>() {
                CLAY_PALETTE
            } else {
                GRASS_PALETTE
            });
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct RgbColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}
impl RgbColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn new_with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl From<RgbColor> for Color {
    fn from(col: RgbColor) -> Self {
        Color::rgba_u8(col.r, col.g, col.b, col.a)
    }
}

pub struct Palette {
    background: RgbColor,
    court: RgbColor,
    court_lines: RgbColor,
    court_pickets: RgbColor,
    ball: RgbColor,
    ball_trail: RgbColor,
    player: RgbColor,
    player_aim: RgbColor,
    player_face: RgbColor,
    player_charge: RgbColor,
    score_text: RgbColor,
    shadow: RgbColor,
}

impl Palette {
    pub fn get_color(&self, col: &PaletteColor) -> Color {
        match col {
            PaletteColor::Background => self.background.into(),
            PaletteColor::Court => self.court.into(),
            PaletteColor::CourtLines => self.court_lines.into(),
            PaletteColor::CourtPost => self.court_pickets.into(),
            PaletteColor::Ball => self.ball.into(),
            PaletteColor::BallTrail => self.ball_trail.into(),
            PaletteColor::Player => self.player.into(),
            PaletteColor::PlayerAim => self.player_aim.into(),
            PaletteColor::PlayerFace => self.player_face.into(),
            PaletteColor::PlayerCharge => self.player_charge.into(),
            PaletteColor::Text => self.score_text.into(),
            PaletteColor::Shadow => self.shadow.into(),
        }
    }
}

// based on
// https://lospec.com/palette-list/en4
pub const GRASS_PALETTE: Palette = Palette {
    background: RgbColor::new(32, 40, 61),
    court: RgbColor::new(66, 110, 93),
    court_lines: RgbColor::new(251, 247, 243),
    court_pickets: RgbColor::new(109, 141, 138),
    ball: RgbColor::new(229, 176, 131),
    ball_trail: RgbColor::new(246, 237, 205),
    player: RgbColor::new(251, 247, 243),
    player_aim: RgbColor::new(251, 247, 243),
    player_face: RgbColor::new(32, 40, 61),
    player_charge: RgbColor::new(109, 141, 138),
    score_text: RgbColor::new(251, 247, 243),
    shadow: RgbColor::new_with_alpha(0, 8, 24, 80),
};

// based on
// https://lospec.com/palette-list/pastel-qt
pub const CLAY_PALETTE: Palette = Palette {
    background: RgbColor::new(101, 80, 87),
    court: RgbColor::new(226, 169, 126),
    court_lines: RgbColor::new(246, 237, 205),
    court_pickets: RgbColor::new(203, 129, 117),
    ball: RgbColor::new(109, 141, 138),
    ball_trail: RgbColor::new(168, 200, 166),
    player: RgbColor::new(246, 237, 205),
    player_aim: RgbColor::new(246, 237, 205),
    player_face: RgbColor::new(101, 80, 87),
    player_charge: RgbColor::new(203, 129, 117),
    score_text: RgbColor::new(246, 237, 205),
    shadow: RgbColor::new_with_alpha(22, 12, 0, 80),
};

#[derive(Component, Clone, Copy)]
pub enum PaletteColor {
    Background,
    Court,
    CourtLines,
    CourtPost,
    Ball,
    BallTrail,
    Player,
    PlayerAim,
    PlayerFace,
    PlayerCharge,
    Text,
    Shadow,
}

fn on_palette_changed(
    mut commands: Commands,
    palette: Res<Palette>,
    sprite_q: Query<(Entity, &PaletteColor, Option<&Sprite>, Option<&Text>)>,
) {
    if palette.is_changed() {
        for (e, col, sprite, text) in sprite_q.iter() {
            if let Some(sprite) = sprite {
                commands.entity(e).insert(Animator::new(Tween::new(
                    EaseFunction::QuadraticInOut,
                    TweeningType::Once,
                    std::time::Duration::from_millis(1000),
                    SpriteColorLens {
                        start: sprite.color,
                        end: palette.get_color(col),
                    },
                )));
            } else if let Some(text) = text {
                commands.entity(e).insert(Animator::new(Tween::new(
                    EaseFunction::QuadraticInOut,
                    TweeningType::Once,
                    std::time::Duration::from_millis(1000),
                    TextColorLens {
                        start: text.sections[0].style.color,
                        end: palette.get_color(col),
                        section: 0,
                    },
                )));
            }
        }
    }
}

fn on_sprite_added(
    palette: Res<Palette>,
    mut q: Query<(&PaletteColor, &mut Sprite), Added<Sprite>>,
) {
    for (col, mut sprite) in q.iter_mut() {
        sprite.color = palette.get_color(col);
    }
}

fn on_text_added(palette: Res<Palette>, mut q: Query<(&PaletteColor, &mut Text), Added<Text>>) {
    for (col, mut text) in q.iter_mut() {
        text.sections[0].style.color = palette.get_color(col);
    }
}

fn on_trail_added(palette: Res<Palette>, mut q: Query<&mut DrawMode, Added<Trail>>) {
    for mut draw_mode in q.iter_mut() {
        *draw_mode = DrawMode::Fill(FillMode::color(palette.get_color(&PaletteColor::BallTrail)));
    }
}

fn on_court_added(palette: Res<Palette>, mut q: Query<&mut DrawMode, With<Court>>) {
    for mut draw_mode in q.iter_mut() {
        *draw_mode = DrawMode::Outlined {
            fill_mode: FillMode::color(palette.get_color(&PaletteColor::Court)),
            outline_mode: StrokeMode::new(
                palette.get_color(&PaletteColor::CourtLines),
                COURT_STROKE_WIDTH,
            ),
        };
    }
}

fn handle_input(mut palette: ResMut<Palette>, input: Res<PlayerInput>) {
    for id in 0..=4 {
        if input.just_pressed(id, InputAction::ChangePalette) {
            let is_grass = palette.background == GRASS_PALETTE.background;
            *palette = if is_grass {
                CLAY_PALETTE
            } else {
                GRASS_PALETTE
            };

            break;
        }
    }
}
