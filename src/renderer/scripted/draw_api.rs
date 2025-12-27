//! Drawing command queue and Rhai API registration.
//!
//! Provides the bridge between Rhai scripts and nannou's Draw API.

use nannou::prelude::*;
use rhai::{Dynamic, Engine};
use std::cell::RefCell;
use std::rc::Rc;

/// A queued drawing command from a script
#[derive(Clone, Debug)]
pub enum DrawCommand {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
    },
    Ellipse {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        stroke: f32,
        color: [f32; 4],
    },
    Text {
        x: f32,
        y: f32,
        content: String,
        size: i64,
        color: [f32; 4],
    },
}

impl DrawCommand {
    /// Execute this command using nannou's Draw API
    pub fn execute(&self, draw: &Draw) {
        match self {
            DrawCommand::Rect { x, y, w, h, color } => {
                draw.rect()
                    .x_y(*x, *y)
                    .w_h(*w, *h)
                    .rgba(color[0], color[1], color[2], color[3]);
            }
            DrawCommand::Ellipse { x, y, w, h, color } => {
                draw.ellipse()
                    .x_y(*x, *y)
                    .w_h(*w, *h)
                    .rgba(color[0], color[1], color[2], color[3]);
            }
            DrawCommand::Line {
                x1,
                y1,
                x2,
                y2,
                stroke,
                color,
            } => {
                draw.line()
                    .start(pt2(*x1, *y1))
                    .end(pt2(*x2, *y2))
                    .weight(*stroke)
                    .rgba(color[0], color[1], color[2], color[3]);
            }
            DrawCommand::Text {
                x,
                y,
                content,
                size,
                color,
            } => {
                draw.text(content)
                    .x_y(*x, *y)
                    .font_size(*size as u32)
                    .rgba(color[0], color[1], color[2], color[3]);
            }
        }
    }
}

/// Type alias for the shared command queue
pub type CommandQueue = Rc<RefCell<Vec<DrawCommand>>>;

/// Register all drawing functions on the Rhai engine
pub fn register_draw_api(engine: &mut Engine, commands: CommandQueue) {
    // rect(x, y, w, h, r, g, b, a)
    let cmds = commands.clone();
    engine.register_fn(
        "rect",
        move |x: f64, y: f64, w: f64, h: f64, r: f64, g: f64, b: f64, a: f64| {
            cmds.borrow_mut().push(DrawCommand::Rect {
                x: x as f32,
                y: y as f32,
                w: w as f32,
                h: h as f32,
                color: [r as f32, g as f32, b as f32, a as f32],
            });
        },
    );

    // ellipse(x, y, w, h, r, g, b, a)
    let cmds = commands.clone();
    engine.register_fn(
        "ellipse",
        move |x: f64, y: f64, w: f64, h: f64, r: f64, g: f64, b: f64, a: f64| {
            cmds.borrow_mut().push(DrawCommand::Ellipse {
                x: x as f32,
                y: y as f32,
                w: w as f32,
                h: h as f32,
                color: [r as f32, g as f32, b as f32, a as f32],
            });
        },
    );

    // line(x1, y1, x2, y2, stroke, r, g, b, a)
    let cmds = commands.clone();
    engine.register_fn(
        "line",
        move |x1: f64, y1: f64, x2: f64, y2: f64, stroke: f64, r: f64, g: f64, b: f64, a: f64| {
            cmds.borrow_mut().push(DrawCommand::Line {
                x1: x1 as f32,
                y1: y1 as f32,
                x2: x2 as f32,
                y2: y2 as f32,
                stroke: stroke as f32,
                color: [r as f32, g as f32, b as f32, a as f32],
            });
        },
    );

    // text(x, y, content, size, r, g, b, a)
    let cmds = commands.clone();
    engine.register_fn(
        "text",
        move |x: f64, y: f64, content: String, size: i64, r: f64, g: f64, b: f64, a: f64| {
            cmds.borrow_mut().push(DrawCommand::Text {
                x: x as f32,
                y: y as f32,
                content,
                size,
                color: [r as f32, g as f32, b as f32, a as f32],
            });
        },
    );

    // hsla(h, s, l, a) -> [r, g, b, a]
    // Manual HSL to RGB conversion
    engine.register_fn("hsla", |h: f64, s: f64, l: f64, a: f64| -> rhai::Array {
        let h = h as f32;
        let s = s as f32;
        let l = l as f32;

        let (r, g, b) = if s == 0.0 {
            (l, l, l)
        } else {
            let q = if l < 0.5 {
                l * (1.0 + s)
            } else {
                l + s - l * s
            };
            let p = 2.0 * l - q;

            let hue_to_rgb = |p: f32, q: f32, mut t: f32| -> f32 {
                if t < 0.0 { t += 1.0; }
                if t > 1.0 { t -= 1.0; }
                if t < 1.0 / 6.0 {
                    return p + (q - p) * 6.0 * t;
                }
                if t < 1.0 / 2.0 {
                    return q;
                }
                if t < 2.0 / 3.0 {
                    return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
                }
                p
            };

            (
                hue_to_rgb(p, q, h + 1.0 / 3.0),
                hue_to_rgb(p, q, h),
                hue_to_rgb(p, q, h - 1.0 / 3.0),
            )
        };

        vec![
            Dynamic::from(r as f64),
            Dynamic::from(g as f64),
            Dynamic::from(b as f64),
            Dynamic::from(a),
        ]
    });
}

/// Register math utility functions
pub fn register_math_api(engine: &mut Engine) {
    engine.register_fn("sin", |x: f64| x.sin());
    engine.register_fn("cos", |x: f64| x.cos());
    engine.register_fn("tan", |x: f64| x.tan());
    engine.register_fn("abs", |x: f64| x.abs());
    engine.register_fn("sqrt", |x: f64| x.sqrt());
    engine.register_fn("pow", |x: f64, y: f64| x.powf(y));
    engine.register_fn("floor", |x: f64| x.floor());
    engine.register_fn("ceil", |x: f64| x.ceil());
    engine.register_fn("min", |x: f64, y: f64| x.min(y));
    engine.register_fn("max", |x: f64, y: f64| x.max(y));
    engine.register_fn("clamp", |x: f64, min: f64, max: f64| x.clamp(min, max));

    // Random functions
    engine.register_fn("rand", || rand::random::<f64>());
    engine.register_fn("rand_range", |min: f64, max: f64| {
        min + rand::random::<f64>() * (max - min)
    });

    // Constants
    engine.register_fn("pi", || std::f64::consts::PI);
    engine.register_fn("tau", || std::f64::consts::TAU);
}
