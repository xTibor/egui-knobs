use std::f32::consts::TAU;

use eframe::egui::Ui;
use eframe::emath::{almost_equal, lerp, Pos2, Rot2, Vec2};
use eframe::epaint::{Color32, Shape, Stroke};
use itertools::Itertools;

// ----------------------------------------------------------------------------

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum KnobOrientation {
    Right,
    Bottom,
    Left,
    Top,
    Custom(f32),
}

impl KnobOrientation {
    pub fn rot2(&self) -> Rot2 {
        match *self {
            Self::Right => Rot2::from_angle(TAU * 0.00),
            Self::Bottom => Rot2::from_angle(TAU * 0.25),
            Self::Left => Rot2::from_angle(TAU * 0.50),
            Self::Top => Rot2::from_angle(TAU * 0.75),
            Self::Custom(angle) => Rot2::from_angle(angle),
        }
    }
}

// ----------------------------------------------------------------------------

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum KnobDirection {
    Clockwise,
    Counterclockwise,
}

impl KnobDirection {
    pub fn to_float(&self) -> f32 {
        match *self {
            Self::Clockwise => 1.0,
            Self::Counterclockwise => -1.0,
        }
    }
}

// ----------------------------------------------------------------------------

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum KnobMode {
    Signed,
    Unsigned,
    SpinAround,
}

// ----------------------------------------------------------------------------

/// A polar function defining the shape of a knob widget.
type KnobShapeFn<'a> = Box<dyn 'a + Fn(f32) -> f32>;

pub enum KnobShape<'a> {
    Circle,
    Square,
    Squircle(f32),
    Polygon(usize),
    SuperPolygon(usize, f32),
    Custom(KnobShapeFn<'a>),
}

impl KnobShape<'_> {
    const RESOLUTION: usize = 32;

    pub fn eval(&self, theta: f32) -> f32 {
        match self {
            KnobShape::Circle => 1.0,
            KnobShape::Square => (1.0 / theta.cos().abs()).min(1.0 / theta.sin().abs()),
            KnobShape::Squircle(factor) => {
                assert!(*factor > 0.0, "squircle factor must be positive");
                let a = theta.cos().abs().powf(*factor);
                let b = theta.sin().abs().powf(*factor);
                1.0 / (a + b).powf(1.0 / *factor)
            }
            KnobShape::Polygon(n) => {
                assert!(*n >= 3, "polygon must have at least 3 sides");
                1.0 / ((*n as f32 / 2.0 * theta).cos().asin() * 2.0 / *n as f32).cos()
            }
            KnobShape::SuperPolygon(n, factor) => {
                assert!(*n >= 3, "polygon must have at least 3 sides");
                assert!(
                    (0.0..=2.0).contains(factor),
                    "polygon factor must be between 0.0 and 2.0"
                );

                (((0.25 * (*n as f32) * theta).cos()).abs().powf(*factor)
                    + ((0.25 * (*n as f32) * theta).sin()).abs().powf(*factor))
                .powf(-1.0 / *factor)
            }
            KnobShape::Custom(callback) => callback(theta),
        }
    }

    pub fn paint_shape(
        &self,
        ui: &mut Ui,
        center: Pos2,
        radius: f32,
        fill: Color32,
        stroke: Stroke,
        rotation: Rot2,
    ) {
        let outline_points = (0..Self::RESOLUTION)
            .map(move |i| {
                let angle = (i as f32 / Self::RESOLUTION as f32) * TAU;
                let shape_radius = self.eval(angle - (rotation * Vec2::RIGHT).angle());
                center + Vec2::angled(angle) * radius * shape_radius
            })
            .collect_vec();

        // https://github.com/emilk/egui/issues/513
        outline_points
            .iter()
            .circular_tuple_windows()
            .for_each(|(point_1, point_2)| {
                ui.painter().add(Shape::convex_polygon(
                    vec![center, *point_1, *point_2],
                    fill,
                    Stroke::new(1.0, fill),
                ));
            });

        ui.painter().add(Shape::closed_line(outline_points, stroke));
    }

    pub fn paint_arc(
        &self,
        ui: &mut Ui,
        center: Pos2,
        inner_radius: f32,
        outer_radius: f32,
        start_angle: f32,
        end_angle: f32,
        fill: Color32,
        stroke: Stroke,
        rotation: Rot2,
    ) {
        // NOTE: convex_polygon() is broken, spews rendering artifacts all over
        //   the window when it tries to render degenerate polygons:
        //     ∃(P1,P2) ∈ Poly (dist(P1,P2) ≈ 0)

        // HACK: convex_polygon() workaround
        if almost_equal(start_angle, end_angle, 0.001) {
            let shape_radius = self.eval(start_angle - (rotation * Vec2::RIGHT).angle());

            ui.painter().add(Shape::line_segment(
                [
                    center + Vec2::angled(start_angle) * inner_radius * shape_radius,
                    center + Vec2::angled(start_angle) * outer_radius * shape_radius,
                ],
                stroke,
            ));
            return;
        }

        let generate_arc_points = |radius| {
            (0..=Self::RESOLUTION).map(move |i| {
                let angle = lerp(start_angle..=end_angle, i as f32 / Self::RESOLUTION as f32);
                let shape_radius = self.eval(angle - (rotation * Vec2::RIGHT).angle());
                center + Vec2::angled(angle) * radius * shape_radius
            })
        };

        // HACK: convex_polygon() workaround
        let inner_radius = inner_radius.max(0.1);

        let outer_arc = generate_arc_points(outer_radius).collect::<Vec<_>>();
        let inner_arc = generate_arc_points(inner_radius).collect::<Vec<_>>();

        // https://github.com/emilk/egui/issues/513
        outer_arc
            .iter()
            .zip(inner_arc.iter())
            .tuple_windows()
            .for_each(|((outer_1, inner_1), (outer_2, inner_2))| {
                ui.painter().add(Shape::convex_polygon(
                    vec![*outer_1, *inner_1, *inner_2, *outer_2],
                    fill,
                    Stroke::new(1.0, fill),
                ));
            });

        let outline_points: Vec<Pos2> = outer_arc
            .iter()
            .chain(inner_arc.iter().rev())
            .cloned()
            .collect();

        ui.painter().add(Shape::closed_line(outline_points, stroke));

        // TODO: Remove hacks and paint the arc with a single call:
        // Shape::concave_polygon(
        //     outline_points, // outer_arc.chain(inner_arc.rev())
        //     fill,
        //     stroke,
        // )
    }
}

// ----------------------------------------------------------------------------

/// Wrap angle to `(0..TAU)` range.
pub fn normalized_angle_unsigned_excl(angle: f32) -> f32 {
    ((angle % TAU) + TAU) % TAU
}

/// Wrap angle to `(0..=TAU)` range.
pub fn normalized_angle_unsigned_incl(angle: f32) -> f32 {
    if angle < 0.0 {
        ((angle % TAU) + TAU) % TAU
    } else if angle > TAU {
        angle % TAU
    } else {
        angle
    }
}
