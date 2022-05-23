use std::f32::consts::TAU;

use eframe::egui::{self, Ui};
use eframe::emath::{normalized_angle, pos2, vec2, Align2};
use eframe::epaint::{FontFamily, FontId, Shape};

use crate::common::{normalized_angle_unsigned, KnobMode};

pub struct CompassLabels<'a>(pub [&'a str; 4]);

pub fn compass_knob(
    ui: &mut Ui,
    mode: KnobMode,
    value: &mut f32,
    width: f32,
    height: f32,
    labels: CompassLabels,
    spread: f32,
    snap_angle: Option<f32>,
    shift_snap_angle: Option<f32>,
    min: Option<f32>,
    max: Option<f32>,
) -> egui::Response {
    let desired_size = egui::vec2(width, height);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

    let constraint_value = |mut new_value| {
        if mode == KnobMode::Signed {
            new_value = normalized_angle(new_value);
        }

        if mode == KnobMode::Unsigned {
            new_value = normalized_angle_unsigned(new_value);
        }

        if let Some(min) = min {
            new_value = new_value.max(min);
        }

        if let Some(max) = max {
            new_value = new_value.min(max);
        }

        new_value
    };

    if response.dragged() {
        let new_value = *value - response.drag_delta().x / rect.width() * spread;
        *value = constraint_value(new_value);
        response.mark_changed();
    }

    if response.drag_released() {
        if let Some(angle) = if ui.input().modifiers.shift_only() {
            shift_snap_angle
        } else {
            snap_angle
        } {
            assert!(angle > 0.0, "non-positive snap angles are not supported");
            let new_value = (*value / angle).round() * angle;
            *value = constraint_value(new_value);
            response.mark_changed();
        }
    }

    let map_angle_to_screen =
        |angle: f32| rect.center().x - (*value - angle) * (rect.width() / spread);

    if ui.is_rect_visible(rect) {
        let visuals = *ui.style().interact(&response);

        ui.painter().rect(
            rect,
            visuals.rounding,
            ui.style().visuals.extreme_bg_color,
            ui.style().visuals.noninteractive().fg_stroke,
        );

        ui.set_clip_rect(rect);

        ui.painter().add(Shape::convex_polygon(
            vec![
                rect.center(),
                rect.center() - vec2(height / 6.0, height / 4.0),
                rect.center() - vec2(-height / 6.0, height / 4.0),
            ],
            visuals.bg_fill,
            visuals.fg_stroke,
        ));

        ui.painter().text(
            rect.center_top(),
            Align2::CENTER_TOP,
            format!("{:.0}°", value.to_degrees()),
            FontId::new(height / 4.0, FontFamily::Proportional),
            visuals.text_color(),
        );

        let left_degrees =
            (((*value - (spread / 2.0)).to_degrees() / 10.0).floor() * 10.0) as isize;
        let right_degrees =
            (((*value + (spread / 2.0)).to_degrees() / 10.0).ceil() * 10.0) as isize;

        for degree in (left_degrees..=right_degrees).step_by(10) {
            let tick_x = map_angle_to_screen((degree as f32).to_radians());

            let tick_height = if degree % 90 == 0 {
                1.0
            } else if degree % 30 == 0 {
                0.75
            } else {
                0.5
            };

            ui.painter().line_segment(
                [
                    pos2(tick_x, rect.top() + height * 0.5),
                    pos2(
                        tick_x,
                        rect.top() + height * 0.5 + height * 0.25 * tick_height,
                    ),
                ],
                ui.style().visuals.noninteractive().fg_stroke,
            );

            if degree % 90 == 0 {
                ui.painter().text(
                    pos2(tick_x, rect.bottom()),
                    Align2::CENTER_BOTTOM,
                    labels.0[((((degree / 90) % 4) + 4) % 4) as usize],
                    FontId::new(height / 4.0, FontFamily::Proportional),
                    ui.style().visuals.text_color(),
                );
            }
        }

        let paint_stop = |angle: f32| {
            let stop_x = map_angle_to_screen(angle);

            ui.painter().line_segment(
                [pos2(stop_x, rect.top()), pos2(stop_x, rect.bottom())],
                ui.style().visuals.noninteractive().fg_stroke,
            );
        };

        if let Some(min) = min {
            paint_stop(min);
        }

        if let Some(max) = max {
            paint_stop(max);
        }
    }

    response
}
