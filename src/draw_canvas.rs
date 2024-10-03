
use iced::{mouse, Color, Size};
use iced::widget::canvas::event::{self, Event};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Element, Fill, Point, Rectangle, Renderer, Theme};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Choice {
    #[default]
    None,
    Bezier,
    Circle,
    Line,
    Rectangle,
    RightTriangle,
    Triangle,
}
#[derive(Debug)]
pub struct State {
    cache: canvas::Cache,
    pub curves: Vec<DrawCurve>,
    pub selection: Choice,
    pub escape_pressed: bool,
    pub curve_to_edit: Option<usize>,
    pub edit_points: Vec<Point>,
    pub selected_color_str: Option<String>,
    pub selected_color: Color,
    pub draw_width: f32,
}

impl Default for State {
    fn default() -> Self {
        Self { 
                cache: canvas::Cache::default(),
                curves: vec![],
                selection: Choice::None,
                escape_pressed: false,
                curve_to_edit: None,
                edit_points: vec![],
                selected_color_str: None,
                selected_color: Color::WHITE,
                draw_width: 2.0,
             }
        }
}

impl State {
    pub fn view<'a>(&'a self, curves: &'a [DrawCurve]) -> Element<'a, DrawCurve> {
        Canvas::new(DrawPending {
            state: self,
            curves,
        })
        .width(Fill)
        .height(Fill)
        .into()
    }

    pub fn request_redraw(&mut self) {
        self.cache.clear();
    }
}

struct DrawPending<'a> {
    state: &'a State,
    curves: &'a [DrawCurve],
}

impl<'a> canvas::Program<DrawCurve> for DrawPending<'a> {
    type State = Option<Pending>;

    fn update(
        &self,
        program_state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (event::Status, Option<DrawCurve>) {
        let Some(cursor_position) = cursor.position_in(bounds) else {
            return (event::Status::Ignored, None);
        };
        
        match event {
            Event::Mouse(mouse_event) => {
                if self.state.escape_pressed {
                    *program_state = None;
                    return (event::Status::Ignored, None)
                }
                
                let message = match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        let color = Some(self.state.selected_color);
                        let width = self.state.draw_width;

                        if self.state.curve_to_edit.is_some() {
                            if program_state.is_none() {
                                for (index, point) in self.state.edit_points.iter().enumerate() {
                                    if point_in_circle(*point, cursor_position) {
                                        let curve = self.state.curves[self.state.curve_to_edit.unwrap()];
                                    
                                        *program_state = Some(Pending::Edit { curve_type: self.state.selection, 
                                                                                edit_index: Some(index),
                                                                                from: curve.from, 
                                                                                to: curve.to, 
                                                                                control: curve.control,
                                                                                color,
                                                                                width });
                                    }
                                }
                            }
                        }

                        match self.state.selection {
                            Choice::None => None,
                            Choice::Bezier => {
                                pending_bezier(program_state, cursor_position, color, width)
                            },
                            Choice::Circle => {
                                pending_circle(program_state, cursor_position, color, width)
                            },
                            Choice::Line => {
                                pending_line(program_state, cursor_position, color, width)
                            },
                            Choice::Rectangle => {
                                pending_rectangle(program_state, cursor_position, color, width)
                            },
                            Choice::Triangle => {
                                pending_triangle(program_state, cursor_position, color, width)
                            },
                            Choice::RightTriangle => {
                                pending_right_triangle(program_state, cursor_position, color, width)
                            },
                        }
                    }
                    mouse::Event::ButtonReleased(mouse::Button::Left) => {
                        let mut curve = None;
                        if program_state.is_some() && self.state.curve_to_edit.is_some() {
                            let st = program_state.unwrap();
                            
                            match st {
                                Pending::Edit { curve_type, edit_index,
                                                mut from, mut to, mut 
                                                control,
                                                color,
                                                width } => {
                                    match edit_index {
                                        Some(0) => from = cursor_position,
                                        Some(1) => to = cursor_position,
                                        Some(2) => control = Some(cursor_position),
                                        None => (),
                                        _ => (),
                                    }
                                    *program_state = None;
                                    curve = Some(DrawCurve {
                                            curve_type,
                                            from,
                                            to,
                                            control,
                                            color,
                                            width,
                                        })

                                },
                                _ =>(),
                            }
                            
                        }
                        
                        curve
                    }
                    _ => None,
                };

                (event::Status::Captured, message)
            }
            _ => (event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let content =
            self.state.cache.draw(renderer, bounds.size(), |frame| {
                DrawCurve::draw_all(self.curves, frame, theme, self.state.curve_to_edit);

                frame.stroke(
                    &Path::rectangle(Point::ORIGIN, frame.size()),
                    Stroke::default()
                        .with_width(2.0)
                        .with_color(theme.palette().text),
                );
            });

        if let Some(pending) = state {
            vec![content, pending.draw(renderer, theme, bounds, cursor)]
        } else {
            vec![content]
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DrawCurve {
    pub curve_type: Choice,
    pub from: Point,
    pub to: Point,
    pub control: Option<Point>,
    pub color: Option<Color>,
    pub width: f32,
}

impl DrawCurve {
    fn draw_all(curves: &[DrawCurve], frame: &mut Frame, theme: &Theme, curve_to_edit: Option<usize>) {

        for (index, curve) in curves.iter().enumerate() {
            match curve.curve_type {
                Choice::None => {
                    ()
                },
                Choice::Bezier => {
                    let path = Path::new(|p| {
                        if curve_to_edit.is_some() && curve_to_edit == Some(index) {
                            p.circle(curve.from, 2.0);
                            p.circle(curve.to, 2.0);
                            p.circle(curve.control.unwrap(), 2.0);
                        }
                        p.move_to(curve.from);
                        p.quadratic_curve_to(curve.control.unwrap(), curve.to);
                    });
                    let color = match curve.color {
                        Some(c) => c,
                        None => theme.palette().text
                    };
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(color),
                    );
                },
                Choice::Circle => {
                    let path = Path::new(|p| {
                        let radius = curve.from.distance(curve.to);
                        p.circle(curve.from, radius);
                    });
                    let color = match curve.color {
                        Some(c) => c,
                        None => theme.palette().text
                    };
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(color),
                    );
                },
                Choice::Line => {
                    let path = Path::new(|p| {
                        p.move_to(curve.from);
                        p.line_to(curve.to);
                    });
                    let color = match curve.color {
                        Some(c) => c,
                        None => theme.palette().text
                    };
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(color),
                    );
                },
                Choice::Rectangle => {
                    let width = (curve.to.x-curve.from.x).abs();
                    let height = (curve.to.y-curve.from.y).abs();
                    let size = Size{ width, height };

                    let top_left = if curve.from.x < curve.to.x && curve.from.y > curve.to.y {
                        // top right
                        Point{ x: curve.from.x, y: curve.from.y-height }
                    } else if curve.from.x > curve.to.x && curve.from.y > curve.to.y {
                        // top_left
                        Point{x: curve.from.x-width, y: curve.to.y}
                    } else if curve.from.x > curve.to.x  && curve.from.y < curve.to.y {
                        // bottom left
                        Point{ x: curve.to.x, y: curve.from.y }
                    } else if curve.from.x < curve.to.x  && curve.from.y < curve.to.y {
                        // bottom right
                        curve.from
                    } else {
                        curve.to
                    };
                    let path = Path::new(|p| {
                        p.rectangle(top_left, size);
                    });
                    let color = match curve.color {
                        Some(c) => c,
                        None => theme.palette().text
                    };
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(color),
                    );
                },
                Choice::Triangle => {
                    let path = Path::new(|p| {
                        p.move_to(curve.from);
                        p.line_to(curve.to);
                        p.line_to(curve.control.unwrap());
                        p.line_to(curve.from);
                    });
                    let color = match curve.color {
                        Some(c) => c,
                        None => theme.palette().text
                    };
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(color),
                    );
                },
                Choice::RightTriangle => {
                    let path = Path::new(|p| {
                        p.move_to(curve.from);
                        p.line_to(curve.to);
                        p.line_to(curve.control.unwrap());
                        p.line_to(curve.from);
                    });
                    let color = match curve.color {
                        Some(c) => c,
                        None => theme.palette().text
                    };
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(color),
                    );
                },
            }
        }

    }
}


fn pending_bezier(state: &mut Option<Pending>, cursor_position: Point, color: Option<Color>, width: f32) -> Option<DrawCurve> {
    match *state {
        None => {
            *state = Some(Pending::One {
                curve_type: Choice::Bezier,
                from: cursor_position,
                color,
                width,
            });
            
            None
        }
        Some(Pending::One { curve_type: Choice::Bezier,
                            from,
                            color,
                            width, }) => {
            *state = Some(Pending::Two {
                                curve_type: Choice::Bezier,
                                from,
                                to: cursor_position,
                                color,
                                width,
                                });
            
            None
        }
        Some(Pending::Two { curve_type: Choice::Bezier, 
                            from, 
                            to,
                            color,
                            width, }) => {
            *state = None;

            Some(DrawCurve {
                curve_type: Choice::Bezier,
                from,
                to,
                control: Some(cursor_position),
                color,
                width,
            })
        }
        Some(Pending::Edit {curve_type: Choice::Bezier,
                            edit_index, 
                            from, 
                            to,
                            control,
                            color,
                            width }) => {
                               
            *state = Some(Pending::Edit {
                curve_type: Choice::Bezier,
                edit_index,
                from,
                to,
                control,
                color,
                width,
            });

            None
        }
        _ => None
    }
}

fn pending_circle(state: &mut Option<Pending>, cursor_position: Point, color: Option<Color>, width: f32) -> Option<DrawCurve> {
    match *state {
        None => {
            *state = Some(Pending::One {
                curve_type: Choice::Circle,
                from: cursor_position,
                color,
                width
            });

            None
        }
        Some(Pending::One { curve_type: Choice::Circle, 
                            from,
                            color,
                            width }) => {
            *state = None;

            Some(DrawCurve {
                curve_type: Choice::Circle,
                from,
                to: cursor_position,
                control: None,
                color,
                width,
            })
        }
        Some(Pending::Two { curve_type: Choice::Circle, from: _, to: _, color: _, width: _ }) => {
            *state = None;
            None
        }
        _ => None
        
    }
}

fn pending_line(state: &mut Option<Pending>, cursor_position: Point, color: Option<Color>, width: f32) -> Option<DrawCurve> {
    match *state {
        None => {
            *state = Some(Pending::One {
                curve_type: Choice::Line,
                from: cursor_position,
                color, 
                width
            });

            None
        }
        Some(Pending::One { curve_type: Choice::Line, 
                            from, 
                            color, 
                            width }) => {
            *state = None;

            Some(DrawCurve {
                curve_type: Choice::Line,
                from,
                to: cursor_position,
                control: None,
                color,
                width,
            })
        }
        Some(Pending::Two { curve_type: Choice::Line, from: _, to: _ , color: _, width: _ }) => {
            *state = None;
            None
        }
        _ => None
        
    }
}

fn pending_rectangle(state: &mut Option<Pending>, cursor_position: Point, color: Option<Color>, width: f32) -> Option<DrawCurve> {
    match *state {
        None => {
            *state = Some(Pending::One {
                curve_type: Choice::Rectangle,
                from: cursor_position,
                color,
                width,
            });

            None
        }
        Some(Pending::One { curve_type: Choice::Rectangle, 
                            from , color, width}) => {
            *state = None;

            Some(DrawCurve {
                curve_type: Choice::Rectangle,
                from,
                to: cursor_position,
                control: None,
                color,
                width,
            })
        }
        Some(Pending::Two { curve_type: Choice::Rectangle, from: _, to: _ , color: _, width: _}) => {
            *state = None;
            None
        }
        _ => None
        
    }
}

fn pending_triangle(state: &mut Option<Pending>, cursor_position: Point, color: Option<Color>, width: f32) -> Option<DrawCurve> {
    match *state {
        None => {
            *state = Some(Pending::One {
                curve_type: Choice::Triangle,
                from: cursor_position,
                color,
                width,
            });

            None
        }
        Some(Pending::One { curve_type: Choice::Triangle,
                            from , color, width}) => {
            *state = Some(Pending::Two {
                curve_type: Choice::Triangle,
                from,
                to: cursor_position,
                color,
                width,
            });

            None
        }
        Some(Pending::Two { curve_type: Choice::Triangle, 
                            from, 
                            to,
                            color,
                            width, }) => {
            *state = None;

            Some(DrawCurve {
                curve_type: Choice::Triangle,
                from,
                to: to,
                control: Some(cursor_position),
                color,
                width,
            })
        }
        _ => None
    }
}

fn pending_right_triangle(state: &mut Option<Pending>, cursor_position: Point, color: Option<Color>, width: f32) -> Option<DrawCurve> {
    match *state {
        None => {
            *state = Some(Pending::One {
                curve_type: Choice::Triangle,
                from: cursor_position,
                color,
                width,
            });

            None
        }
        Some(Pending::One { curve_type: Choice::Triangle,
                            from,
                            color,
                            width, }) => {
            *state = Some(Pending::Two {
                curve_type: Choice::Triangle,
                from,
                to: cursor_position,
                color,
                width,
            });

            None
        }
        Some(Pending::Two { curve_type: Choice::Triangle, 
                            from, 
                            to,
                            color,
                            width, }) => {
            *state = None;

            Some(DrawCurve {
                curve_type: Choice::Triangle,
                from,
                to,
                control: Some(cursor_position),
                color,
                width,
            })
        }
        _ => None
    }
}

#[derive(Debug, Clone, Copy)]
enum Pending {
    One { curve_type: Choice, from: Point, color: Option<Color>, width: f32 },
    Two { curve_type: Choice, from: Point, to: Point, color: Option<Color>, width: f32 },
    Edit {curve_type: Choice, edit_index: Option<usize>, from: Point, to: Point, control: Option<Point>, color: Option<Color>, width: f32 },
}

impl Pending {
    fn draw(
        &self,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Geometry {
        let mut frame = Frame::new(renderer, bounds.size());

        if let Some(cursor_position) = cursor.position_in(bounds) {

            match *self {
                Pending::One { curve_type, from, color, width } => {
                    match curve_type {
                        Choice::None => (),
                        Choice::Bezier => {
                            let line = Path::line(from, cursor_position);
                            frame.stroke(
                            &line,
                            Stroke::default()
                                .with_width(width)
                                .with_color(color.unwrap()),
                            )
                        },
                        Choice::Circle => {
                            let radius = from.distance(cursor_position);
                            let circle = Path::circle(from, radius);
                            frame.stroke(
                            &circle,
                            Stroke::default()
                                .with_width(width)
                                .with_color(color.unwrap()),
                            )
                        },
                        Choice::Line => {
                            let line = Path::line(from, cursor_position);
                            frame.stroke(
                            &line,
                            Stroke::default()
                                .with_width(width)
                                .with_color(color.unwrap()),
                            )
                        },
                        Choice::Rectangle => {
                            let width = (cursor_position.x-from.x).abs();
                            let height = (cursor_position.y-from.y).abs();
                            
                            
                            let top_left = if from.x < cursor_position.x && from.y > cursor_position.y {
                                // top right
                                Some(Point{ x: from.x, y: from.y-height })
                            } else if from.x > cursor_position.x && from.y > cursor_position.y {
                                //  top left
                                Some(Point{x: from.x-width, y: cursor_position.y})
                            } else if from.x > cursor_position.x  && from.y < cursor_position.y {
                                // bottom left
                                Some(Point{ x: cursor_position.x, y: from.y })
                            } else if cursor_position.x > from.x && cursor_position.y > from.y {
                                // bottom right
                                Some(from)
                            } else {
                                None
                            };

                            let rect = if top_left.is_some() {
                                    let size = Size{ width, height };
                                Path::rectangle(top_left.unwrap(), size)
                                } else {
                                    Path::line(from, cursor_position)
                                };
                            frame.stroke(
                            &rect,
                            Stroke::default()
                                .with_width(width)
                                .with_color(color.unwrap()),
                            )
                            
                        },
                        Choice::Triangle => {
                            let tr = Path::line(from, cursor_position);
                            frame.stroke(
                            &tr,
                            Stroke::default()
                                .with_width(width)
                                .with_color(color.unwrap()),
                            )
                        },
                        Choice::RightTriangle => {
                            let rtr = Path::line(from, cursor_position);
                            frame.stroke(
                            &rtr,
                            Stroke::default()
                                .with_width(width)
                                .with_color(color.unwrap()),
                            )
                        },
                    };
                }
                Pending::Two { curve_type, from, to, color, width } => {
                    let curve = match curve_type {
                        Choice::None => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: None,
                                color,
                                width,
                            }
                        },
                        Choice::Bezier => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: Some(cursor_position),
                                color,
                                width,
                            }
                        },
                        Choice::Circle => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: None,
                                color,
                                width,
                            }
                        },
                        Choice::Line => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: None,
                                color,
                                width,
                            }
                        },
                        Choice::Rectangle => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: None,
                                color,
                                width,
                            }
                        },
                        Choice::Triangle => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: Some(cursor_position),
                                color,
                                width,
                            }
                        },
                        Choice::RightTriangle => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: Some(cursor_position),
                                color,
                                width,
                            }
                        },
                    };

                    DrawCurve::draw_all(&[curve], &mut frame, theme, None);
                }
                Pending::Edit { curve_type, 
                                edit_index, 
                                mut from, 
                                mut to, 
                                mut control, 
                                color, 
                                width } => {
                    match edit_index {
                        Some(0) => from = cursor_position,
                        Some(1) => to = cursor_position,
                        Some(2) => control = Some(cursor_position),
                        None => (),
                        _ => ()
                    }
                    let curve = match curve_type {
                        Choice::None => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: None,
                                color,
                                width,
                            }
                        },
                        Choice::Bezier => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control,
                                color,
                                width,
                            }
                        },
                        Choice::Circle => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: None,
                                color,
                                width,
                            }
                        },
                        Choice::Line => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: None,
                                color,
                                width,
                            }
                        },
                        Choice::Rectangle => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: None,
                                color,
                                width,
                            }
                        },
                        Choice::Triangle => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: Some(cursor_position),
                                color,
                                width,
                            }
                        },
                        Choice::RightTriangle => {
                            DrawCurve {
                                curve_type,
                                from,
                                to,
                                control: Some(cursor_position),
                                color,
                                width,
                            }
                        },
                    };
                    DrawCurve::draw_all(&[curve], &mut frame, theme, None);
                },
            };
        }

        frame.into_geometry()
    }
}

fn point_in_circle(point: Point, cursor: Point) -> bool {
    let dist = point.distance(cursor);
    if dist < 5.0 {
        true
    } else {
        false
    }
}