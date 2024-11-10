
use std::f32::consts::PI;

use iced::{mouse, Color, Size};
use iced::widget::canvas::event::{self, Event};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Element, Fill, Point, Rectangle, Renderer, Theme};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IpgCanvasWidget {
    #[default]
    None,
    Bezier,
    Circle,
    Line,
    PolyLine,
    Polygon,
    Rectangle,
    RightTriangle,
    Triangle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq,)]
pub enum CanvasMode {
    Edit,
    Select,
}

impl CanvasMode {
    pub fn to_str(&self) -> String {
        match &self {
            CanvasMode::Edit => "Edit".to_string(),
            CanvasMode::Select => "Select".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct State {
    cache: canvas::Cache,
    pub canvas_mode: CanvasMode,
    pub curve_to_edit: Option<usize>,
    pub draw_width: f32,
    pub edit_points: Vec<Point>,
    pub escape_pressed: bool,
    pub poly_points: usize,
    pub selection: IpgCanvasWidget,
    pub selected_color_str: Option<String>,
    pub selected_color: Color,
    pub selected_curve_index: usize,
}

impl Default for State {
    fn default() -> Self {
        Self { 
                cache: canvas::Cache::default(),
                canvas_mode: CanvasMode::Select,
                selection: IpgCanvasWidget::None,
                selected_curve_index: 0,
                poly_points: 5,
                escape_pressed: false,
                curve_to_edit: None,
                edit_points: vec![],
                selected_color_str: Some("White".to_string()),
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

    pub fn make_selection(&mut self, selection: IpgCanvasWidget) {
            self.selection = selection;
    }

    pub fn set_indexes(&mut self, indexes: usize) {
        self.selected_curve_index = indexes;
    }

    pub fn set_color(&mut self, color: Color) {
            self.selected_color = color;
        }

    pub fn set_width(&mut self, width: f32) {
        self.draw_width = width;
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
        let (curve_type, widget_type) = match self.state.selection {
                                IpgCanvasWidget::None => ("".to_string(), IpgCanvasWidget::None),
                                IpgCanvasWidget::Bezier => ("bezier".to_string(), IpgCanvasWidget::Bezier),
                                IpgCanvasWidget::Circle => ("circle".to_string(), IpgCanvasWidget::Circle),  
                                IpgCanvasWidget::Line => ("line".to_string(), IpgCanvasWidget::Line),
                                IpgCanvasWidget::PolyLine => ("polyline".to_string(), IpgCanvasWidget::PolyLine),
                                IpgCanvasWidget::Rectangle => ("rectangle".to_string(), IpgCanvasWidget::Rectangle),
                                IpgCanvasWidget::Polygon => ("polygon".to_string(), IpgCanvasWidget::Polygon),
                                IpgCanvasWidget::RightTriangle => ("right_triangle".to_string(), IpgCanvasWidget::RightTriangle),
                                IpgCanvasWidget::Triangle => ("triangle".to_string(), IpgCanvasWidget::Triangle),
                            };
        match event {
            Event::Mouse(mouse_event) => {
                if self.state.escape_pressed {
                    *program_state = None;
                    return (event::Status::Ignored, None)
                }
                
                let message = match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        if self.state.curve_to_edit.is_some() {
                            
                        }


                        match program_state {
                            // First mouse click sets the state of the first Pending point
                            // return a none since no Curve yet
                            None => {
                                *program_state = Some(Pending::N {
                                    curve_type,
                                    count: self.state.selected_curve_index,
                                    points: vec![cursor_position],
                                    poly_points: self.state.poly_points,
                                    color: self.state.selected_color,
                                    width: self.state.draw_width,
                                });
                                None
                            },
                            // The second click is a Some() since it was created above
                            // The pending is carrying the previous info
                            Some(Pending::N { 
                                    curve_type: _, 
                                    count,
                                    points,
                                    poly_points,
                                    color,
                                    width,
                            }) => {
                                // we clone here because if we don't the state cannot be 
                                // set to none because it would be borrowed if we use it.
                                let mut pts = points.clone();
                                pts.push(cursor_position);
                                let color = color.clone();
                                let width = width.clone();
                                let poly_points = poly_points.clone();

                                if curve_type == "right_triangle" {
                                    if pts.len() > 1 {
                                    pts[1].x = pts[0].x;
                                    }
                                    if pts.len() > 2 {
                                        pts[2].y = pts[1].y;
                                    }
                                }
                                
                                let count = if curve_type == "polyline".to_string() {
                                    poly_points
                                } else {
                                    *count
                                };
                                // after pushing on the point, we check to see if the count matches
                                // if so then we return the Curve and set the state to none
                                // if not, then this is repeated until the count is equaled.
                                if pts.len() == count {
                                    *program_state = None;
                                    Some(DrawCurve {
                                        curve_type: widget_type,
                                        points: pts,
                                        poly_points,
                                        color,
                                        width,
                                    })
                                } else {
                                    *program_state = Some(Pending::N {
                                        curve_type,
                                        count,
                                        points: pts,
                                        poly_points,
                                        color,
                                        width,
                                    });
                                    None
                                }
                            },
                            _ => None,
                        }
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

#[derive(Debug, Clone)]
pub struct DrawCurve {
    pub curve_type: IpgCanvasWidget,
    pub points: Vec<Point>,
    pub poly_points: usize,
    pub color: Color,
    pub width: f32,
}

impl DrawCurve {
    fn draw_all(curves: &[DrawCurve], frame: &mut Frame, _theme: &Theme, curve_to_edit: Option<usize>) {
        // This draw only occrs at the completion of the widget
        for (index, curve) in curves.iter().enumerate() {
            match curve.curve_type {
                IpgCanvasWidget::None => {
                    ()
                },
                IpgCanvasWidget::Bezier => {
                    let path = Path::new(|p| {
                        if curve_to_edit.is_some() && curve_to_edit == Some(index) {
                            p.circle(curve.points[0], 2.0);
                            p.circle(curve.points[1], 2.0);
                            p.circle(curve.points[2], 2.0);
                        }
                        p.move_to(curve.points[0]);
                        p.quadratic_curve_to(curve.points[2], curve.points[1]);
                    });
                    
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(curve.color),
                    );
                },
                IpgCanvasWidget::Circle => {
                    let path = Path::new(|p| {
                        let radius = curve.points[0].distance(curve.points[1]);
                        p.circle(curve.points[0], radius);
                    });
                    
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(curve.color),
                    );
                },
                IpgCanvasWidget::Line => {
                    let path = Path::new(|p| {
                        p.move_to(curve.points[0]);
                        p.line_to(curve.points[1]);
                    });
                    
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(curve.color),
                    );
                },
                IpgCanvasWidget::PolyLine => {
                    let path = Path::new(|p| {
                        p.move_to(curve.points[0]);
                        for point in curve.points.iter() {
                            p.line_to(point.clone());
                        }
                    });

                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(curve.color),
                    );
                },
                IpgCanvasWidget::Polygon => {
                    let n = curve.poly_points;
                    let angle = 0.0-PI/n as f32;
                    let center = curve.points[0];
                    let to = curve.points[1];
                    let radius = center.distance(to) as f32;
                    let mut points = vec![];
                    let pi_2_n = 2.0*PI/n as f32;

                    for i in 0..n {
                        let x = center.x as f32 + radius * (pi_2_n * i as f32 - angle).sin();
                        let y = center.y as f32 + radius * (pi_2_n * i as f32 - angle).cos();
                        points.push(Point { x: x as f32, y: y as f32 });
                    }
                    points.push(points[0]);

                    let path = Path::new(|p| {
                        p.move_to(points[0]);
                        for point in points.iter() {
                            p.line_to(point.clone());
                        }
                    });

                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(curve.color),
                    );
                }
                IpgCanvasWidget::Rectangle => {
                    let width = (curve.points[1].x-curve.points[0].x).abs();
                    let height = (curve.points[1].y-curve.points[0].y).abs();
                    let size = Size{ width, height };

                    let top_left = if curve.points[0].x < curve.points[1].x && curve.points[0].y > curve.points[1].y {
                        // top right
                        Point{ x: curve.points[0].x, y: curve.points[0].y-height }
                    } else if curve.points[0].x > curve.points[1].x && curve.points[0].y > curve.points[1].y {
                        // top_left
                        Point{x: curve.points[0].x-width, y: curve.points[1].y}
                    } else if curve.points[0].x > curve.points[1].x  && curve.points[0].y < curve.points[1].y {
                        // bottom left
                        Point{ x: curve.points[1].x, y: curve.points[0].y }
                    } else if curve.points[0].x < curve.points[1].x  && curve.points[0].y < curve.points[1].y {
                        // bottom right
                        curve.points[0]
                    } else {
                        curve.points[1]
                    };
                    let path = Path::new(|p| {
                        p.rectangle(top_left, size);
                    });
                    
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(curve.color),
                    );
                },
                IpgCanvasWidget::Triangle => {
                    let path = Path::new(|p| {
                        p.move_to(curve.points[0]);
                        p.line_to(curve.points[1]);
                        p.line_to(curve.points[2]);
                        p.line_to(curve.points[0]);
                    });
                    
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(curve.color),
                    );
                },
                IpgCanvasWidget::RightTriangle => {
                    let path = Path::new(|p| {
                        p.move_to(curve.points[0]);
                        p.line_to(curve.points[1]);
                        p.line_to(curve.points[2]);
                        p.line_to(curve.points[0]);
                    });
                    
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(curve.color),
                    );
                },
            }
        }

    }
}

#[derive(Debug, Clone)]
enum Pending {
    N {curve_type: String, count: usize, points: Vec<Point>, poly_points: usize, color: Color, width: f32},
    Edit {curve_type: String, edit_index: Option<usize>, points: Vec<Point>, color: Color, width: f32 },
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
            // This draw happens when the mouse is moved and the state is none.
            match self {
                Pending::N { curve_type, 
                            count, 
                            points,
                             poly_points, 
                             color, 
                             width } => {
                    match curve_type.as_str() {
                        "bezier" => {
                            // if complete return a curve through draw_all
                            if points.len() == *count {
                            let mut pts = points.clone();
                            pts[count-1] = cursor_position;
                            let curve = DrawCurve {
                                curve_type: IpgCanvasWidget::Bezier,
                                points: pts,
                                poly_points: *poly_points,
                                color: *color,
                                width: *width,
                            };

                            DrawCurve::draw_all(&[curve], &mut frame, theme, None);

                            // if 2 points are set, use the cursor position for the control
                            } else if points.len() == count-1 {
                                let path = Path::new(|p| {
                                    p.move_to(points[0]);
                                    p.quadratic_curve_to(cursor_position, points[1]);
                                });
                            
                                frame.stroke(
                                    &path,
                                    Stroke::default()
                                        .with_width(2.0)
                                        .with_color(theme.palette().text),
                                );
                            
                            // if only one point is set, just draw a line bewteen the point and the cursor point
                            } else if points.len() == count-2 {
                                let line = Path::line(points[0], cursor_position);
                                frame.stroke(
                                    &line,
                                    Stroke::default()
                                        .with_width(2.0)
                                        .with_color(theme.palette().text),
                                );
                            }
                        },
                        "circle" => {
                            // if 2 points set, return a curve
                            if points.len() == *count {
                            let mut pts = points.clone();
                            pts[count-1] = cursor_position;
                            let curve = DrawCurve {
                                curve_type: IpgCanvasWidget::Circle,
                                points: pts,
                                poly_points: *poly_points,
                                color: *color,
                                width: *width,
                            };

                            DrawCurve::draw_all(&[curve], &mut frame, theme, None);

                            // if only one point set, draw circle using cursor point
                            } else if points.len() == count-1 {
                                let radius = points[0].distance(cursor_position);
                                let line = Path::circle(points[0], radius);
                                frame.stroke(
                                    &line,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            }
                        }
                        "line" => {
                            // if 2 points set, return a curve
                            if points.len() == *count {
                            let mut pts = points.clone();
                            pts[count-1] = cursor_position;
                            let curve = DrawCurve {
                                curve_type: IpgCanvasWidget::Line,
                                points: pts,
                                poly_points: *poly_points,
                                color: *color,
                                width: *width,
                            };

                            DrawCurve::draw_all(&[curve], &mut frame, theme, None);

                            // if only one point set, draw a line using the cursor
                            } else if points.len() == count-1 {
                                let line = Path::line(points[0], cursor_position);
                                frame.stroke(
                                    &line,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            }
                        },
                        // if all points set based on the poly_points, return the curve
                        "polyline" => {
                            if points.len() == *poly_points {
                            let mut pts = points.clone();
                            pts[count-1] = cursor_position;
                            let curve = DrawCurve {
                                curve_type: IpgCanvasWidget::PolyLine,
                                points: pts,
                                poly_points: *poly_points,
                                color: *color,
                                width: *width,
                            };

                            DrawCurve::draw_all(&[curve], &mut frame, theme, None);

                            // if points are not set yet, just draw the lines.
                            } else {
                                let path = Path::new(|p| {
                                    for index in 0..points.len() {
                                        if index > 0 {
                                            p.move_to(points[index-1]);
                                            p.line_to(points[index]);
                                        }
                                    }
                                    let len = points.len();
                                    p.move_to(points[len-1]);
                                    p.line_to(cursor_position);
                                });
                                frame.stroke(
                                    &path,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            }
                        },
                        "polygon" => {
                            let mut pts = points.clone();
                            if points.len() == *count {
                                pts[count-1] = cursor_position;
                            let curve = DrawCurve {
                                curve_type: IpgCanvasWidget::Polygon,
                                points: pts,
                                poly_points: *poly_points,
                                color: *color,
                                width: *width,
                            };

                            DrawCurve::draw_all(&[curve], &mut frame, theme, None);

                            // if points are not set yet, draw polygon with cursor point.
                            } else {
                                let n = poly_points.clone();
                                let angle = 0.0-PI/n as f32;
                                let center = pts[0];
                                let to = cursor_position;
                                let radius = center.distance(to) as f32;
                                let mut points = vec![];
                                let pi_2_n = 2.0*PI/n as f32;
                                for i in 0..n {
                                    let x = center.x as f32 + radius * (pi_2_n * i as f32 - angle).sin();
                                    let y = center.y as f32 + radius * (pi_2_n * i as f32 - angle).cos();
                                    points.push(Point { x: x as f32, y: y as f32 });
                                }
                                points.push(points[0]);
                                let path = Path::new(|p| {
                                    p.move_to(points[0]);
                                    for point in points.iter() {
                                        p.line_to(point.clone());
                                    }
                                });
                                frame.stroke(
                                    &path,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            }
                        },
                        "rectangle" => {
                            if points.len() == *count {
                            let mut pts = points.clone();
                            pts[count-1] = cursor_position;
                            let curve = DrawCurve {
                                curve_type: IpgCanvasWidget::Rectangle,
                                points: pts,
                                poly_points: *poly_points,
                                color: *color,
                                width: *width,
                            };

                            DrawCurve::draw_all(&[curve], &mut frame, theme, None);

                            // if points are not set yet, just draw the lines.
                            } else {
                                let rect_width = (cursor_position.x-points[0].x).abs();
                                let height = (cursor_position.y-points[0].y).abs();
                                
                                let top_left = if points[0].x < cursor_position.x && points[0].y > cursor_position.y {
                                    // top right
                                    Some(Point{ x: points[0].x, y: points[0].y-height })
                                } else if points[0].x > cursor_position.x && points[0].y > cursor_position.y {
                                    //  top left
                                    Some(Point{x: points[0].x-rect_width, y: cursor_position.y})
                                } else if points[0].x > cursor_position.x  && points[0].y < cursor_position.y {
                                    // bottom left
                                    Some(Point{ x: cursor_position.x, y: points[0].y })
                                } else if cursor_position.x > points[0].x && cursor_position.y > points[0].y {
                                    // bottom right
                                    Some(points[0])
                                } else {
                                    None
                                };

                                let rect = if top_left.is_some() {
                                        let size = Size{ width: rect_width, height };
                                    Path::rectangle(top_left.unwrap(), size)
                                    } else {
                                        Path::line(points[0], cursor_position)
                                    };
                                frame.stroke(
                                &rect,
                                Stroke::default()
                                    .with_width(*width)
                                    .with_color(*color),
                                )
                            }
                        },
                        "triangle" => {
                            if points.len() == *count {
                            let mut pts = points.clone();
                            pts[count-1] = cursor_position;
                            let curve = DrawCurve {
                                curve_type: IpgCanvasWidget::Triangle,
                                points: pts,
                                poly_points: *poly_points,
                                color: *color,
                                width: *width,
                            };

                            DrawCurve::draw_all(&[curve], &mut frame, theme, None);

                            // if points are not set yet, just draw the lines.
                            } else {
                                let path = Path::new(|p| {
                                    for index in 0..points.len() {
                                        if index > 0 {
                                            p.move_to(points[index-1]);
                                            p.line_to(points[index]);
                                        }
                                    }
                                    let len = points.len();
                                    p.move_to(points[len-1]);
                                    p.line_to(cursor_position);
                                });
                                frame.stroke(
                                    &path,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            }
                        },
                        "right_triangle" => {
                            let mut pts = points.clone();
                            if pts.len() > 1 {
                                pts[1].x = pts[0].x;
                            }
                            if pts.len() > 2 {
                                pts[2].y = pts[1].y;
                            }
                            if pts.len() == *count {
                                pts[count-1] = cursor_position;
                                let curve = DrawCurve {
                                    curve_type: IpgCanvasWidget::Triangle,
                                    points: pts,
                                    poly_points: *poly_points,
                                    color: *color,
                                    width: *width,
                                };

                                DrawCurve::draw_all(&[curve], &mut frame, theme, None);

                            // if points are not set yet, just draw the lines.
                            } else {
                                let mut c_pos = cursor_position;
                                if pts.len() == 1 {
                                    c_pos.x = pts[0].x;
                                }
                                if pts.len() == 2 {
                                    c_pos.y = pts[1].y;
                                }
                                pts.push(c_pos);
                                let path = Path::new(|p| {
                                    for index in 0..pts.len() {
                                        if index > 0 {
                                            p.move_to(pts[index-1]);
                                            p.line_to(pts[index]);
                                        }
                                    }
                                });
                                frame.stroke(
                                    &path,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            }
                        },
                        _ => ()
                    };
                },
                // 
                _ => ()
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