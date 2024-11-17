
use std::f32::consts::PI;
// use std::sync::{Mutex, MutexGuard};

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
    None,
    New,
    Edit,
}

// used to display text widget
impl CanvasMode {
    pub fn string(&self) -> String {
        match &self {
            CanvasMode::None => "None".to_string(),
            CanvasMode::New => "New".to_string(),
            CanvasMode::Edit => "Edit".to_string(),
        }
    }

    pub fn to_enum(s: String) -> Self {
        match s.as_str() {
            "Edit" | "edit" => CanvasMode::Edit,
            "New" | "new" => CanvasMode::New,
            _ => CanvasMode::None,
        }
    }
}

#[derive(Debug)]
pub struct State {
    cache: canvas::Cache,
    pub canvas_mode: CanvasMode,
    pub draw_width: f32,
    pub edit_draw_curve: DrawCurve,
    pub edit_curve_index: Option<usize>,
    pub rotation: bool,
    pub escape_pressed: bool,
    pub poly_points: usize,
    pub curve_type: IpgCanvasWidget,
    pub selected_color_str: Option<String>,
    pub selected_color: Color,
    pub selected_curve_index: usize,
}

impl Default for State {
    fn default() -> Self {
        Self { 
                cache: canvas::Cache::default(),
                canvas_mode: CanvasMode::None,
                draw_width: 2.0,
                edit_curve_index: None,
                edit_draw_curve: DrawCurve::default(),
                rotation: false,
                escape_pressed: false,
                poly_points: 5,
                curve_type: IpgCanvasWidget::None,
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
            self.curve_type = selection;
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
        
        match event {
            Event::Mouse(mouse_event) => {
                if self.state.escape_pressed {
                    *program_state = None;
                    return (event::Status::Ignored, None)
                }
                
                let message = match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        if self.state.edit_curve_index.is_some() {
                            match program_state {
                                None => {
                                    let (pts, 
                                        mid_point, 
                                        edit_point_index, 
                                        edit_mid_point) = 
                                            edit_curve_first_click(self.state, cursor_position);

                                    *program_state = Some(Pending::Edit {
                                        curve_type: self.state.edit_draw_curve.curve_type,
                                        first_click: true,
                                        edit_curve_index: self.state.edit_curve_index,
                                        edit_point_index,
                                        edit_mid_point,
                                        points: pts.clone(),
                                        poly_points: self.state.edit_draw_curve.poly_points,
                                        mid_point,
                                        color: self.state.edit_draw_curve.color,
                                        width: self.state.edit_draw_curve.width,
                                    });
                                    
                                    Some(DrawCurve {
                                        curve_type: self.state.edit_draw_curve.curve_type,
                                        points: pts,
                                        poly_points: self.state.edit_draw_curve.poly_points,
                                        mid_point,
                                        first_click: true,
                                        rotation: false,
                                        color: self.state.edit_draw_curve.color,
                                        width: self.state.edit_draw_curve.width,
                                    })
                                },
                                // The second click is a Some() since it was created above
                                // The pending is carrying the previous info
                                Some(Pending::Edit { 
                                        curve_type: _,
                                        first_click: _, 
                                        edit_curve_index: _,
                                        edit_point_index,
                                        edit_mid_point: _,
                                        points,
                                        poly_points,
                                        mid_point: _,
                                        color,
                                        width,
                                }) => {
                                    let (pts, mid_point) = 
                                        edit_curve_second_click(&self.state, 
                                                                cursor_position,
                                                                points.clone(), 
                                                                edit_point_index.clone(),
                                                                );

                                    // Need to clone otherwise Program_state borrowed
                                    let poly_points = poly_points.clone();
                                    let color = color.clone();
                                    let width = width.clone();
                                    *program_state = None;
                                    Some(DrawCurve {
                                        curve_type: self.state.edit_draw_curve.curve_type,
                                        points: pts,
                                        poly_points,
                                        mid_point,
                                        first_click: false,
                                        rotation: false,
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
                                        curve_type: self.state.curve_type,
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
                                    let curve_type = self.state.curve_type;
                                    let mut pts = points.clone();
                                    pts.push(cursor_position);
                                    let color = color.clone();
                                    let width = width.clone();
                                    let poly_points = poly_points.clone();

                                    if curve_type == IpgCanvasWidget::RightTriangle {
                                        if pts.len() > 1 {
                                        pts[1].x = pts[0].x;
                                        }
                                        if pts.len() > 2 {
                                            pts[2].y = pts[1].y;
                                        }
                                    }
                                    
                                    let count = if curve_type == IpgCanvasWidget::PolyLine {
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
                                            curve_type: curve_type,
                                            points: pts,
                                            poly_points,
                                            mid_point: Point::default(),
                                            first_click: false,
                                            rotation: false,
                                            color,
                                            width,
                                        })
                                    } else {
                                        *program_state = Some(Pending::N {
                                            curve_type: curve_type,
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
                    },
                    mouse::Event::ButtonPressed(mouse::Button::Right) => {
                        if self.state.edit_curve_index.is_none() {
                            *program_state = None;
                            return (event::Status::Ignored, None)
                        }

                        match program_state {
                            // First mouse click sets the state of the first Pending point
                            // return a none since no Curve yet
                            None => {
                                *program_state = Some(Pending::Rotation { 
                                    points: self.state.edit_draw_curve.points.clone(), 
                                    scroll_count: 0.0,
                                    first_click: true,
                                });
                                Some(DrawCurve {
                                    curve_type: self.state.edit_draw_curve.curve_type,
                                    points: self.state.edit_draw_curve.points.clone(),
                                    poly_points: self.state.edit_draw_curve.poly_points,
                                    mid_point: Point::default(),
                                    first_click: true,
                                    rotation: true,
                                    color: self.state.edit_draw_curve.color,
                                    width: self.state.edit_draw_curve.width,
                                })
                            },
                            // The second click is a Some() since it was created above
                            // The pending is carrying the previous info
                            Some(Pending::Rotation { 
                                points, 
                                scroll_count: _,
                                first_click: false,
                            }) => {
                                // after pushing on the point, we check to see if the count matches
                                // if so then we return the Curve and set the state to none
                                // if not, then this is repeated until the count is equaled.
                                let points = points.clone();
                                *program_state = None;
                                Some(DrawCurve {
                                    curve_type: self.state.curve_type,
                                    points,
                                    poly_points: self.state.poly_points,
                                    mid_point: Point::default(),
                                    first_click: false,
                                    rotation: false,
                                    color: self.state.selected_color,
                                    width: self.state.draw_width,
                                })
                            },
                            _ => None,
                        }
                    },
                    mouse::Event::WheelScrolled { delta } => {
                        if self.state.rotation {
                            dbg!(delta);
                        }
                        *program_state = None;
                        return (event::Status::Ignored, None)
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
                DrawCurve::draw_all(self.curves, frame, theme, self.state.edit_curve_index);

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


fn edit_curve_first_click(state: &State, cursor_position: Point) 
    -> (Vec<Point>, Point, Option<usize>, Option<Point>) {
    // The first click loads up the Curve
    // since we're in edit mode, cursor position used.
    let curve_type = state.edit_draw_curve.curve_type;
    let mut pts = state.edit_draw_curve.points.clone();
    let mut mid_point = state.edit_draw_curve.mid_point.clone();
    let edit_mid_point: Option<Point> = Some(mid_point);
    
    // either a point in the curve or the mid point will be assigned to
    // the cursor position
    let (edit_point_index, edit_mid_point) = 
        find_closest_point_index(cursor_position, 
                                edit_mid_point.unwrap(), 
                                &pts,
                                curve_type,);
    
    // ensures the right triangle stays aligned
    if curve_type == IpgCanvasWidget::RightTriangle {
        if pts.len() > 1 {
        pts[1].x = pts[0].x;
        }
        if pts.len() > 2 {
            pts[2].y = pts[1].y;
        }
    }
    // Since new points are generated using the cursor position,
    // normally you would need to recalc the center position
    // but since the point cuicle is not shown during movement,
    // no need at this time.
    if edit_mid_point.is_some() {
        mid_point = edit_mid_point.unwrap();
    }
    (pts, mid_point, edit_point_index, edit_mid_point)

}

fn edit_curve_second_click(state: &State,
                            cursor_position: Point,
                            mut points: Vec<Point>, 
                            edit_point_index: Option<usize>,
                            ) -> (Vec<Point>, Point) {
    
    // Since points_to_move was found using closest point,
    // point_to_edit pointed to it therefore skip when some()
    let curve_type = state.edit_draw_curve.curve_type;

    let (mut pts, mid_point) = if edit_point_index.is_some() {
        points[edit_point_index.clone().unwrap()] = cursor_position;
        // recalculate mid_point
        let mid_point = get_mid_geometry(&points, curve_type);
        (points, mid_point)
    }  else {
        let mid_point = cursor_position;
        (translate_geometry(points, cursor_position, curve_type),
        mid_point)
    };
    
    if curve_type == IpgCanvasWidget::RightTriangle {
        if pts.len() > 1 {
        pts[1].x = pts[0].x;
        }
        if pts.len() > 2 {
            pts[2].y = pts[1].y;
        }
    }
    (pts, mid_point)

}

#[derive(Debug, Clone, Default)]
pub struct DrawCurve {
    pub curve_type: IpgCanvasWidget,
    pub points: Vec<Point>,
    pub poly_points: usize,
    pub mid_point: Point,
    pub first_click: bool,
    pub rotation: bool,
    pub color: Color,
    pub width: f32,
}

impl DrawCurve {
    fn draw_all(curves: &[DrawCurve], frame: &mut Frame, _theme: &Theme, 
                curve_to_edit: Option<usize>,) {
        // This draw only occurs at the completion of the widget(update occurs) and cache is cleared
        
        // increment_draw_curve_counter();
        
        for (index, curve) in curves.iter().enumerate() {
            // if first click, skip the curve to be edited so that it 
            // will not be seen until the second click.  Otherwise is shows
            // during editing because there is no way to refresh
            // The pending routine will diplay the curve
            if !curve.first_click {
                // if in edit mode put a small circle at each point
                if curve_to_edit.is_some() && 
                    index == curve_to_edit.unwrap() {
                    let path = Path::new(|p| {
                        for point in curve.points.iter() {
                            p.circle(point.clone(), 2.0);
                        }
                        p.circle(curve.mid_point, 3.0);
                    });

                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(curve.width)
                            .with_color(curve.color),
                    );
                }
            } else {
                // skiping index being edited
                continue;
            }
            
            let path = 
                match curve.curve_type {
                    IpgCanvasWidget::Bezier => {
                        build_bezier_path(&curve.points, None, 0)
                    },
                    IpgCanvasWidget::Circle => {
                        build_circle_path(&curve.points, None, 0)
                    },
                    IpgCanvasWidget::Line => {
                        build_line_path(&curve.points, None, 0)
                    },
                    IpgCanvasWidget::PolyLine => {
                        build_polyline_path(&curve.points, None, 0)
                    },
                    IpgCanvasWidget::Polygon => {
                        build_polygon_path(&curve.points, curve.poly_points, None, 0)
                    }
                    IpgCanvasWidget::Rectangle => {
                        build_rectangle_path(&curve.points, None, 0)
                    },
                    IpgCanvasWidget::Triangle => {
                        build_triangle_path(&curve.points, None, 0)
                    },
                    IpgCanvasWidget::RightTriangle => {
                        build_triangle_path(&curve.points, None, 0)
                    },
                    _ => Path::new(|_| {}),
                };

            frame.stroke(
                &path,
                Stroke::default()
                    .with_width(curve.width)
                    .with_color(curve.color),
            );
        }

    }
}



#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Pending {
    N {
        curve_type: IpgCanvasWidget, 
        count: usize, 
        points: Vec<Point>, 
        poly_points: usize, 
        color: Color, 
        width: f32
    },
    Edit {
        curve_type: IpgCanvasWidget, 
        first_click: bool, 
        edit_curve_index: Option<usize>,
        edit_point_index: Option<usize>,
        edit_mid_point: Option<Point>,
        mid_point: Point, 
        points: Vec<Point>, 
        poly_points: usize, 
        color: Color, 
        width: f32 
        },
    Rotation {
        points: Vec<Point>,
        scroll_count: f32,
        first_click: bool,
    }
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
                    edit_curve_index: _,
                    edit_point_index,
                    edit_mid_point, 
                    points, 
                    poly_points,
                    mid_point: _, 
                    color, 
                    width } => {
                        match curve_type {
                            IpgCanvasWidget::Bezier=> {
                                let mut pts = points.clone();
                                if edit_point_index.is_some() {
                                    pts[edit_point_index.unwrap()] = cursor_position;
                                }
                                if edit_mid_point.is_some() {
                                    pts = translate_geometry(pts, 
                                            cursor_position, 
                                            IpgCanvasWidget::Bezier);
                                }
                                
                                let path = Path::new(|p| {
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
                                if edit_point_index.is_some(){
                                    pts[edit_point_index.unwrap()] = cursor_position;
                                }
                                if edit_mid_point.is_some() {
                                    pts = translate_geometry(pts, 
                                            cursor_position, 
                                            IpgCanvasWidget::Circle);
                                }

                                let radius = pts[0].distance(pts[1]);
                                
                                let path = Path::circle(pts[0], radius);

                                frame.stroke(
                                    &path,
                                    Stroke::default()
                                        .with_width(*width)
                                        .with_color(*color),
                                );
                            },
                            IpgCanvasWidget::Line => {
                                let mut pts = points.clone();
                                if edit_point_index.is_some(){
                                    pts[edit_point_index.unwrap()] = cursor_position;
                                }
                                if edit_mid_point.is_some() {
                                    pts = translate_geometry(pts, 
                                            cursor_position, 
                                            IpgCanvasWidget::Line);
                                }

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
                                if edit_point_index.is_some(){
                                    pts[edit_point_index.unwrap()] = cursor_position;
                                }
                                if edit_mid_point.is_some() {
                                        pts = translate_geometry(pts, 
                                            cursor_position, 
                                            IpgCanvasWidget::PolyLine);
                                }

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
                                if edit_point_index.is_some(){
                                    pts[edit_point_index.unwrap()] = cursor_position;
                                }
                                if edit_mid_point.is_some() {
                                    pts = translate_geometry(pts, 
                                            cursor_position, 
                                            IpgCanvasWidget::Polygon);
                                }
                                
                                let n = poly_points.clone();
                                let angle = 0.0-PI/n as f32;
                                let center = pts[0];
                                let radius = center.distance(pts[1]) as f32;
                                let mut points = vec![];
                                let pi_2_n = 2.0*PI/n as f32;
                                for i in 0..n {
                                    let x = center.x as f32 + radius * (pi_2_n * i as f32 - angle).sin();
                                    let y = center.y as f32 + radius * (pi_2_n * i as f32 - angle).cos();
                                    points.push(Point { x: x as f32, y: y as f32 });
                                }
                                // close the polygon
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
                                let mut pts = points.clone();
                                if edit_point_index.is_some(){
                                    pts[edit_point_index.unwrap()] = cursor_position;
                                }
                                if edit_mid_point.is_some() {
                                    pts = translate_geometry(pts, 
                                            cursor_position, 
                                            IpgCanvasWidget::Rectangle);
                                }
                                
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
                                if edit_point_index.is_some(){
                                    pts[edit_point_index.unwrap()] = cursor_position;
                                }
                                if edit_mid_point.is_some() {
                                    pts = translate_geometry(pts, 
                                            cursor_position, 
                                            IpgCanvasWidget::Triangle);
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
                            IpgCanvasWidget::RightTriangle => {
                                let mut pts = points.clone();
                                if edit_point_index.is_some(){
                                    pts[edit_point_index.unwrap()] = cursor_position;
                                }
                                if edit_mid_point.is_some() {
                                    pts = translate_geometry(pts, 
                                            cursor_position, 
                                            IpgCanvasWidget::RightTriangle);
                                }
                                
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
                Pending::Rotation { points, scroll_count, first_click: _ } => {
                    let _new_points = rotate_geometry(points, scroll_count);
                },
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

fn find_closest_point_index(cursor: Point, 
                            mid_point: Point, 
                            points: &Vec<Point>,
                            curve_type: IpgCanvasWidget, ) 
                            -> (Option<usize>, Option<Point>) {
    match curve_type {
        IpgCanvasWidget::None => (None, None),
        IpgCanvasWidget::Bezier | IpgCanvasWidget::Line |
        IpgCanvasWidget::PolyLine | IpgCanvasWidget::Rectangle |
        IpgCanvasWidget::RightTriangle | IpgCanvasWidget::Triangle => {
            let mut distance: f32 = 1000000.0;
            let mut point_index = 0;
            for (idx, point) in points.iter().enumerate() {
                let dist = cursor.distance(*point);
                if  dist < distance {
                    point_index = idx;
                    distance = dist;
                }
            };
            
            let mid_dist = mid_point.distance(cursor);
            if mid_dist < distance {
                (None, Some(cursor))
            } else {
                (Some(point_index), None)
            }
        },
        IpgCanvasWidget::Circle | IpgCanvasWidget::Polygon => {
            let cur_mid = cursor.distance(points[0]);
            let cur_circle = cursor.distance(points[1]);
            if cur_mid <= cur_circle {
                (None, Some(cursor))
            } else {
                (Some(1), None)
            }
        },
    }
    
}

fn get_mid_point(pt1: Point, pt2: Point) -> Point {
    Point {x: (pt1.x + pt2.x) / 2.0, y: (pt1.y + pt2.y) / 2.0 }
}

pub fn get_mid_geometry(pts: &Vec<Point>, curve_type: IpgCanvasWidget) -> Point {

    match curve_type {
        IpgCanvasWidget::Bezier => {
            let x = (pts[0].x + pts[1].x + pts[2].x)/3.0;
            let y = (pts[0].y + pts[1].y + pts[2].y)/3.0;
            Point {x, y}
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

fn rotate_geometry(points: &Vec<Point>, theta: &f32) -> Vec<Point> {

    let mut new_points = vec![];
    for point in points.iter() {
        let x_new = point.x * theta.cos() - point.y * theta.sin();
        let y_new = point.x * theta.sin() + point.y * theta.cos();

        new_points.push(Point { x: x_new, y: y_new })
    }
    
    new_points
     
}

fn build_bezier_path(points: &Vec<Point>, cursor: Option<Point>, position: usize) -> Path {
    let mut points = points.clone();
    if cursor.is_some() {
        points[position] = cursor.unwrap();
    }
    Path::new(|p| {
        p.move_to(points[0]);
        p.quadratic_curve_to(points[2], points[1]);
    })
}

fn build_circle_path(points: &Vec<Point>, cursor: Option<Point>, position: usize) -> Path {
    let mut points = points.clone();
    if cursor.is_some() {
        points[position] = cursor.unwrap();
    }
    Path::new(|p| {
        p.circle(points[0], points[0].distance(points[1]));
    })
}

fn build_line_path(points: &Vec<Point>, cursor: Option<Point>, position: usize) -> Path {
    let mut points = points.clone();
    if cursor.is_some() {
        points[position] = cursor.unwrap();
    }
    Path::new(|p| {
        p.move_to(points[0]);
        p.line_to(points[1]);
    })
}

fn build_polyline_path(points: &Vec<Point>, cursor: Option<Point>, position: usize) -> Path {
    let mut points = points.clone();
    if cursor.is_some() {
        points[position] = cursor.unwrap();
    }
    Path::new(|p| {
        for (index, point) in points.iter().enumerate() {
            if index == 0 {
                p.move_to(point.clone());
            } else {
                p.line_to(point.clone());
            }
        }
    })
}

fn build_polygon_path(points: &Vec<Point>, poly_points: usize, cursor: Option<Point>, position: usize) -> Path {
    let mut points = points.clone();
    if cursor.is_some() {
        points[position] = cursor.unwrap();
    }
    let n = poly_points;
    let angle = 0.0-PI/n as f32;
    let center = points[0];
    let to = points[1];
    let radius = center.distance(to) as f32;
    let mut points = vec![];
    let pi_2_n = 2.0*PI/n as f32;

    for i in 0..n {
        let x = center.x as f32 + radius * (pi_2_n * i as f32 - angle).sin();
        let y = center.y as f32 + radius * (pi_2_n * i as f32 - angle).cos();
        points.push(Point { x: x as f32, y: y as f32 });
    }
    points.push(points[0]);

    Path::new(|p| {
        p.move_to(points[0]);
        for point in points.iter() {
            p.line_to(point.clone());
        }
    })
}

fn build_rectangle_path(points: &Vec<Point>, cursor: Option<Point>, position: usize) -> Path {
    let mut points = points.clone();
    if cursor.is_some() {
        points[position] = cursor.unwrap();
    }
    let width = (points[1].x-points[0].x).abs();
    let height = (points[1].y-points[0].y).abs();
    let size = Size{ width, height };

    let top_left = if points[0].x < points[1].x && points[0].y > points[1].y {
        // top right
        Point{ x: points[0].x, y: points[0].y-height }
    } else if points[0].x > points[1].x && points[0].y > points[1].y {
        // top_left
        Point{x: points[0].x-width, y: points[1].y}
    } else if points[0].x > points[1].x  && points[0].y < points[1].y {
        // bottom left
        Point{ x: points[1].x, y: points[0].y }
    } else if points[0].x < points[1].x  && points[0].y < points[1].y {
        // bottom right
        points[0]
    } else {
        points[1]
    };
    
    Path::new(|p| {
        p.rectangle(top_left, size);
    })
}

fn build_triangle_path(points: &Vec<Point>, cursor: Option<Point>, position: usize) -> Path {
    let mut points = points.clone();
    if cursor.is_some() {
        points[position] = cursor.unwrap();
    }
    Path::new(|p| {
        p.move_to(points[0]);
        p.line_to(points[1]);
        p.line_to(points[2]);
        p.line_to(points[0]);
    })
}


// #[derive(Debug)]
// pub struct Counter {
//     pub counter_draw_curve: u64,
//     pub counter_draw_pending_left_mouse: u64,
// }

// pub static COUNTER: Mutex<Counter> = Mutex::new(Counter {
//     counter_draw_curve: 0,
//     counter_draw_pending_left_mouse: 0,
// });

// pub fn access_counter() -> MutexGuard<'static, Counter> {
//     COUNTER.lock().unwrap()
// }

// pub fn reset_counter() {
//     let mut cnt = access_counter();
//     cnt.counter_draw_curve = 0;
//     drop(cnt);
// }

// pub fn increment_draw_curve_counter() {
//     let mut cnt = access_counter();
//     cnt.counter_draw_curve += 1;
//     println!("DrawCurve_draw_all() - {}", cnt.counter_draw_curve);
//     drop(cnt);

// }

// pub fn increment_counter_draw_pending_left_mouse() {
//     let mut cnt = access_counter();
//     cnt.counter_draw_pending_left_mouse += 1;
//     println!("DrawPending-> update()-> mouse left- {}", cnt.counter_draw_pending_left_mouse);
//     drop(cnt);
// }
