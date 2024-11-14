
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

// used to display text widget
impl CanvasMode {
    pub fn string(&self) -> String {
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
    pub first_click: bool,
    pub edit_draw_curve: DrawCurve,
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
                first_click: false,
                curve_to_edit: None,
                draw_width: 2.0,
                edit_draw_curve: DrawCurve::default(),
                escape_pressed: false,
                poly_points: 5,
                selection: IpgCanvasWidget::None,
                selected_color_str: Some("White".to_string()),
                selected_color: Color::WHITE,
                selected_curve_index: 0,
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
        let selection = if self.state.curve_to_edit.is_some() {
            self.state.edit_draw_curve.curve_type
        } else {
            self.state.selection
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
                            match program_state {
                                None => {
                                    // The first click loads up the Curve
                                    // since we're in edit mode, cursor position used.
                                    let mut pts = self.state.edit_draw_curve.points.clone();
                                    let mut point_for_moving: Option<Point> = Some(get_mid_geometry(&pts, selection));
                                    // push the mid_point onto pts for find close()
                                    // then remove to use.
                                    pts.push(point_for_moving.unwrap());
                                    let closest_index = 
                                        find_closest_point_index(cursor_position, &pts);
                                    pts[closest_index] = cursor_position;
                                    if closest_index == pts.len() {
                                        point_for_moving = Some(pts.remove(pts.len()-1));
                                    } else {
                                        point_for_moving = None;
                                    }
                                    
                                    if selection == IpgCanvasWidget::Triangle {
                                        if pts.len() > 1 {
                                        pts[1].x = pts[0].x;
                                        }
                                        if pts.len() > 2 {
                                            pts[2].y = pts[1].y;
                                    }
                                    }
                                    
                                    *program_state = Some(Pending::Edit {
                                        curve_type: selection,
                                        first_click: true,
                                        curve_to_edit: self.state.curve_to_edit,
                                        point_to_edit: closest_index,
                                        point_for_moving,
                                        points: pts.clone(),
                                        poly_points: self.state.edit_draw_curve.poly_points,
                                        color: self.state.edit_draw_curve.color,
                                        width: self.state.edit_draw_curve.width,
                                    });
                                    Some(DrawCurve {
                                        curve_type: selection,
                                        points: pts,
                                        poly_points: self.state.edit_draw_curve.poly_points,
                                        point_for_moving,
                                        first_click: true,
                                        color: self.state.edit_draw_curve.color,
                                        width: self.state.edit_draw_curve.width,
                                    })
                                },
                                // The second click is a Some() since it was created above
                                // The pending is carrying the previous info
                                Some(Pending::Edit { 
                                        curve_type,
                                        first_click: _, 
                                        curve_to_edit: _,
                                        point_to_edit,
                                        point_for_moving,
                                        points,
                                        poly_points,
                                        color,
                                        width,
                                }) => {
                                    // we clone here because if we don't the state cannot be 
                                    // set to none because it would be borrowed if we use it.
                                    let mut pts = points.clone();
                                    // Since points_to_move was found using closest point,
                                    // point_to_edit pointed to it therefore skip when some()
                                    pts = if point_for_moving.is_none() {
                                        pts[*point_to_edit] = cursor_position;
                                        pts
                                    }  else {
                                        translate_geometry(pts, cursor_position, *curve_type)
                                    };
                                    
                                    let color = color.clone();
                                    let width = width.clone();
                                    let poly_points = poly_points.clone();
                                    let point_for_moving = None;

                                    if selection == IpgCanvasWidget::RightTriangle {
                                        if pts.len() > 1 {
                                        pts[1].x = pts[0].x;
                                        }
                                        if pts.len() > 2 {
                                            pts[2].y = pts[1].y;
                                        }
                                    }
                                    // second click ends the editing and returns to the 
                                    // main update() AddCurve 
                                    *program_state = None;
                                    Some(DrawCurve {
                                        curve_type: selection,
                                        points: pts,
                                        poly_points,
                                        point_for_moving,
                                        first_click: false,
                                        color,
                                        width,
                                    })
                                },
                                _ => None,
                            }
                                
                        } else {
                            // adding an new curve
                            match program_state {
                                // First mouse click sets the state of the first Pending point
                                // return a none since no Curve yet
                                None => {
                                    *program_state = Some(Pending::N {
                                        curve_type: selection,
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
                                    let point_for_moving = None;

                                    if selection == IpgCanvasWidget::RightTriangle {
                                        if pts.len() > 1 {
                                        pts[1].x = pts[0].x;
                                        }
                                        if pts.len() > 2 {
                                            pts[2].y = pts[1].y;
                                        }
                                    }
                                    
                                    let count = if selection == IpgCanvasWidget::PolyLine {
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
                                            curve_type: selection,
                                            points: pts,
                                            poly_points,
                                            point_for_moving,
                                            first_click: false,
                                            color,
                                            width,
                                        })
                                    } else {
                                        *program_state = Some(Pending::N {
                                            curve_type: selection,
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
                DrawCurve::draw_all(self.curves, frame, theme, self.state.curve_to_edit, self.state.first_click);

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

#[derive(Debug, Clone, Default)]
pub struct DrawCurve {
    pub curve_type: IpgCanvasWidget,
    pub points: Vec<Point>,
    pub poly_points: usize,
    pub point_for_moving: Option<Point>,
    pub first_click: bool,
    pub color: Color,
    pub width: f32,
}

impl DrawCurve {
    fn draw_all(curves: &[DrawCurve], frame: &mut Frame, _theme: &Theme, 
                curve_to_edit: Option<usize>,
                first_click: bool) {
        // This draw only occurs at the completion of the widget(update occurs) and if cache is cleared
        for (index, curve) in curves.iter().enumerate() {
            if curve_to_edit.is_some() && 
                index == curve_to_edit.unwrap() && 
                (curve.first_click || first_click){
                // skip after first click
                dbg!("first click");
                continue;
            } else if curve_to_edit.is_some() && 
                    index == curve_to_edit.unwrap() {
                    let path = Path::new(|p| {
                        for point in curve.points.iter() {
                            p.circle(point.clone(), 2.0);
                        }
                        if curve.point_for_moving.is_some() {
                            p.circle(curve.point_for_moving.unwrap(), 3.0);
                        }
                    });
    
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(curve.color),
                    );
            }
            
            match curve.curve_type {
                IpgCanvasWidget::None => {
                    ()
                },
                IpgCanvasWidget::Bezier => {
                    let path = Path::new(|p| {
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
                        for (index, point) in curve.points.iter().enumerate() {
                            if index == 0 {
                                p.move_to(point.clone());
                            } else {
                                p.line_to(point.clone());
                            }
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
                        if curve_to_edit.is_some() && curve_to_edit == Some(index) {
                            p.circle(curve.points[0], 2.0);
                            p.circle(curve.points[1], 2.0);
                        }
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
    N {curve_type: IpgCanvasWidget, 
        count: usize, 
        points: Vec<Point>, 
        poly_points: usize, 
        color: Color, 
        width: f32},

    Edit {curve_type: IpgCanvasWidget, 
            first_click: bool, 
            curve_to_edit: Option<usize>,
            point_to_edit: usize,
            point_for_moving: Option<Point>, 
            points: Vec<Point>, 
            poly_points: usize, 
            color: Color, 
            width: f32 },
}

impl Pending {
    fn draw(
        &self,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Geometry {
        let _ = theme;
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

                    match curve_type {
                        IpgCanvasWidget::Bezier => {
                            // if 2 points are set, use the cursor position for the control
                            if points.len() == count-1 {
                                let path = Path::new(|p| {
                                    p.move_to(points[0]);
                                    p.quadratic_curve_to(cursor_position, points[1]);
                                });
                            
                                frame.stroke(
                                    &path,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            
                            // if only one point is set, just draw a line bewteen the point and the cursor point
                            } else if points.len() == count-2 {
                                let line = Path::line(points[0], cursor_position);
                                frame.stroke(
                                    &line,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            }
                        },
                        IpgCanvasWidget::Circle => {
                            // if only one point set, draw circle using cursor point
                            if points.len() == count-1 {
                                let radius = points[0].distance(cursor_position);
                                let line = Path::circle(points[0], radius);
                                frame.stroke(
                                    &line,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            }
                        },
                        IpgCanvasWidget::Line => {
                            // if only one point set, draw a line using the cursor
                            if points.len() == count-1 {
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
                        IpgCanvasWidget::PolyLine => {
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
                        },
                        IpgCanvasWidget::Polygon => {
                            let n = poly_points.clone();
                            let angle = 0.0-PI/n as f32;
                            let center = points[0];
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
                            
                        },
                        IpgCanvasWidget::Rectangle => {
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
                            
                        },
                        IpgCanvasWidget::Triangle => {
                            // if points are not set yet, just draw the lines.
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
                        },
                        IpgCanvasWidget::RightTriangle => {
                            let mut pts = points.clone();
                            // if points are not set yet, just draw the lines,
                            // using the cursor as the next point.
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
                            
                        },
                        _=> ()
                    };
                },
                Pending::Edit { 
                    curve_type,
                    first_click: _, 
                    curve_to_edit: _,
                    point_to_edit,
                    point_for_moving, 
                    points, 
                    poly_points, 
                    color, 
                    width } => {
                        match curve_type {
                            IpgCanvasWidget::Bezier=> {
                                let mut pts = points.clone();
                                // Since the point to edit included the point_to_move
                                // this needs to be skipped if some().
                                if point_for_moving.is_none() {
                                    pts[*point_to_edit] = cursor_position;
                                }
                                
                                let path = Path::new(|p| {
                                if point_for_moving.is_some() {
                                    pts = translate_geometry(pts, 
                                                            cursor_position, 
                                                            IpgCanvasWidget::Bezier);
                                }
                                p.move_to(pts[0]);
                                p.quadratic_curve_to(pts[2], pts[1]);
                                });
                                
                                frame.stroke(
                                    &path,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            },
                            IpgCanvasWidget::Circle => {
                                let mut pts = points.clone();
                                pts[*point_to_edit] = cursor_position;
                                let radius = pts[0].distance(pts[1]);
                                let path = Path::circle(points[0], radius);
                                frame.stroke(
                                    &path,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            },
                            IpgCanvasWidget::Line => {
                                let mut pts = points.clone();
                                pts[*point_to_edit] = cursor_position;
                                let path = Path::line(pts[0], pts[1]);
                                frame.stroke(
                                    &path,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            },
                            IpgCanvasWidget::PolyLine => {
                                let mut pts = points.clone();
                                pts[*point_to_edit] = cursor_position;
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
                            },
                            IpgCanvasWidget::Polygon => {
                                let mut pts = points.clone();
                                pts[*point_to_edit] = cursor_position;
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
                                points.push(pts[0]);
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
                                
                            },
                            IpgCanvasWidget::Rectangle => {
                                let mut pts = points.clone();
                                pts[*point_to_edit] = cursor_position;
                                let rect_width = (pts[0].x-pts[1].x).abs();
                                let height = (pts[0].y-pts[1].y).abs();
                                
                                let top_left = if pts[0].x < pts[1].x && pts[0].y > pts[1].y {
                                    // top right
                                    Some(Point{ x: pts[0].x, y: pts[0].y-height })
                                } else if pts[0].x > pts[1].x && pts[0].y > pts[1].y {
                                    //  top left
                                    Some(Point{x: points[0].x-rect_width, y: pts[1].y})
                                } else if pts[0].x > pts[1].x  && pts[0].y < pts[1].y {
                                    // bottom left
                                    Some(Point{ x: pts[1].x, y: pts[0].y })
                                } else if pts[1].x > pts[0].x && pts[1].y > pts[0].y {
                                    // bottom right
                                    Some(pts[0])
                                } else {
                                    None
                                };

                                let path = if top_left.is_some() {
                                        let size = Size{ width: rect_width, height };
                                    Path::rectangle(top_left.unwrap(), size)
                                    } else {
                                        Path::line(points[0], cursor_position)
                                    };
                                frame.stroke(
                                &path,
                                Stroke::default()
                                    .with_width(*width)
                                    .with_color(*color),
                                )
                                
                            },
                            IpgCanvasWidget::Triangle => {
                                let mut pts = points.clone();
                                pts[*point_to_edit] = cursor_position;
                                pts.push(pts[0]);
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
                            },
                            IpgCanvasWidget::RightTriangle => {
                                let mut pts = points.clone();
                                pts[*point_to_edit] = cursor_position;
                                
                                if pts.len() > 1 {
                                    pts[1].x = pts[0].x;
                                }
                                if pts.len() > 2 {
                                    pts[2].y = pts[1].y;
                                }
                                pts.push(pts[0]);
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
                                
                            },
                            _ => (),
                        }
                    }
            };
        }

        frame.into_geometry()
    }
}

// fn point_in_circle(point: Point, cursor: Point) -> bool {
//     let dist = point.distance(cursor);
//     if dist < 5.0 {
//         true
//     } else {
//         false
//     }
// }

fn find_closest_point_index(cursor: Point, points: &Vec<Point>) -> usize {
    let mut closest_distance: f32 = 1000000.0;
    let mut closest_index = 0;
    for (index, point) in points.iter().enumerate() {
        let distance = cursor.distance(*point);
        if  distance < closest_distance {
            closest_index = index;
            closest_distance = distance;
        }
    }   
    closest_index
}

fn get_mid_point(pt1: Point, pt2: Point) -> Point {
    Point {x: (pt1.x + pt2.x) / 2.0, y: (pt1.y + pt2.y) / 2.0 }
}

fn get_mid_geometry(pts: &Vec<Point>, curve_type: IpgCanvasWidget) -> Point {

    match curve_type {
        IpgCanvasWidget::Bezier => {
            // calculate the mid between first and second points 
            // then the mid pt for the mid and control.
            let mid = get_mid_point(pts[0], pts[1]);
            get_mid_point(mid, pts[2])
        },
        IpgCanvasWidget::Circle | IpgCanvasWidget::Polygon=> {
            // return the center point
            pts[0]
        },
        IpgCanvasWidget::Line | IpgCanvasWidget::Rectangle => {
            get_mid_point(pts[0], pts[1])
        },
        IpgCanvasWidget::PolyLine => {
            let index = (pts.len() as i32 / 2) as i32 as usize;
            let (pt1, pt2) = if pts.len() % 2 == 0 {
                (pts[index-1], pts[index])
            } else {
                
                let mid1 = get_mid_point(pts[index-1], pts[index]);
                let mid2 = get_mid_point(pts[index], pts[index+1]);
                (mid1, mid2)
            };
              
            get_mid_point(pt1, pt2)
        },
        IpgCanvasWidget::RightTriangle | IpgCanvasWidget::Triangle=> {
            let x = (pts[0].x + pts[1].x + pts[2].x)/3.0;
            let y = (pts[0].y + pts[1].y + pts[2].y)/3.0;
            Point {x, y}
        },
        IpgCanvasWidget::None => Point { x: 0.0, y: 0.0 },
    }
    
}

#[test]
fn test_find_closest_point_index() {
    let cursor = Point {
        x: 150.0,
        y: 160.0,
    };
    let points  = vec![
        Point {
            x: 126.422,
            y: 94.05606,
        },
        Point {
            x: 169.99988,
            y: 162.48233,
        },
        Point {
            x: 234.4834,
            y: 101.7688,
        },
    ];
    for i in 0..3 {
        dbg!(cursor.distance(points[i]));
    }
    
    let closest = find_closest_point_index(cursor, &points);
    dbg!(closest);
}

fn translate_geometry(pts: Vec<Point>, 
                        new_center: Point, 
                        curve_type: IpgCanvasWidget) 
                        -> Vec<Point> {
    let old_center = get_mid_geometry(&pts, curve_type);
    let mut new_pts = vec![];
    let dist_x = new_center.x - old_center.x;
    let dist_y = new_center.y - old_center.y;
    for pt in pts.iter() {
        new_pts.push(Point{x: pt.x + dist_x, y: pt.y + dist_y})
    }

    new_pts
}


// #[derive(Debug)]
// pub struct Counter {
//     pub counter: u64, 
// }

// pub static COUNTER: Mutex<Counter> = Mutex::new(Counter {
//     counter: 0,
// });

// pub fn access_counter() -> MutexGuard<'static, Counter> {
//     COUNTER.lock().unwrap()
// }

// pub fn reset_counter() {
//     let mut cnt = access_counter();
//     cnt.counter = 0;
//     drop(cnt);
// }

// pub fn increment_counter() -> u64 {
//     let mut counter = 0;
//     let mut cnt = access_counter();
//     cnt.counter += 1;
//     counter = cnt.counter;
//     drop(cnt);
//     counter
// }
