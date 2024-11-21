
use std::f32::consts::PI;
// use std::sync::{Mutex, MutexGuard};

use iced::{mouse, Color, Size};
use iced::widget::canvas::event::{self, Event};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Element, Fill, Point, Renderer, Theme};

#[derive(Debug, Clone, Default)]
pub enum CanvasWidget {
    #[default]
    None,
    Bezier(Bezier),
    Circle(Circle),
    Line(Line),
    PolyLine(PolyLine),
    Polygon(Polygon),
    RightTriangle(RightTriangle),
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
    pub draw_mode: DrawMode,
    pub draw_width: f32,
    pub edit_widget: CanvasWidget,
    pub edit_widget_index: Option<usize>,
    pub rotation: bool,
    pub escape_pressed: bool,
    pub selected_widget: CanvasWidget,
    pub selected_color: Color,
    pub selected_widget_index: usize,
    pub selected_poly_points: usize,
    pub first_left_click: bool,
    pub second_left_click: bool,
    pub first_right_click: bool,
    pub second_right_click: bool,
}

impl Default for State {
    fn default() -> Self {
        Self { 
                cache: canvas::Cache::default(),
                canvas_mode: CanvasMode::None,
                draw_mode: DrawMode::DrawAll,
                draw_width: 2.0,
                edit_widget_index: None,
                edit_widget: CanvasWidget::None,
                rotation: false,
                escape_pressed: false,
                selected_widget: CanvasWidget::None,
                selected_color: Color::WHITE,
                selected_widget_index: 0,
                selected_poly_points: 5,
                first_left_click: false,
                second_left_click: false,
                first_right_click: false,
                second_right_click: false,
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

    pub fn make_selection(&mut self, selection: CanvasWidget) {
            self.selected_widget = selection;
    }

    pub fn set_indexes(&mut self, indexes: usize) {
        self.selected_widget_index = indexes;
    }

    pub fn set_color(&mut self, color: Color) {
            self.selected_color = color;
        }

    pub fn set_width(&mut self, width: f32) {
        self.draw_width = width;
    }
    pub fn set_poly_points(&mut self, points: usize) {
        self.selected_poly_points = points;
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
        bounds: iced::Rectangle,
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
                        match self.state.draw_mode {
                            DrawMode::DrawAll => {
                                None
                            },
                            DrawMode::Edit => {
                                match program_state {
                                    // First mouse click sets the state of the first Pending point
                                    // return a none since no Curve yet
                                    None => {
                                        let mut index: Option<usize> = None;
                                        for (idx, curve) in self.curves.iter().enumerate() {
                                            let found = find_widget(&curve.widget, cursor_position);
                                            if found {
                                                index = Some(idx);
                                                break;
                                            }
                                        }
                                        if index.is_some() {
                                            *program_state = Some(Pending::Edit {
                                                widget: self.curves[index.unwrap()].widget.clone(),
                                                first_click: true,
                                                second_click: false,
                                                edit_curve_index: None,
                                                edit_point_index: None,
                                                edit_mid_point: false,
                                            });
                                            Some(DrawCurve {
                                                widget: self.curves[index.unwrap()].widget.clone(),
                                                first_click: true,
                                                rotation: false,
                                                angle: 0.0,
                                            })
                                        } else {
                                            None
                                        }
                                    },
                                    // The second click is a Some() since it was created above
                                    // The pending is carrying the previous info
                                    Some(Pending::Edit { 
                                        widget,
                                        first_click,
                                        second_click,
                                        edit_curve_index:_,
                                        edit_point_index,
                                        edit_mid_point, 
                                    }) => {
                                        // after first click, look for closest point to edit in selected widget
                                        if *first_click {
                                            let (point_index, mid_point, ) = 
                                                find_closest_point_index(cursor_position, &widget);

                                            *program_state = Some(Pending::Edit {
                                                widget: widget.clone(),
                                                first_click: false,
                                                second_click: true,
                                                edit_curve_index: None,
                                                edit_point_index: point_index,
                                                edit_mid_point: mid_point,
                                            });
                                            None
                                        } else {
                                            *program_state = None;
                                            let new_widget: CanvasWidget = 
                                                    edit_widget_points(
                                                        widget.clone(), 
                                                        cursor_position, 
                                                        *edit_point_index, 
                                                        *edit_mid_point
                                                    );

                                            Some(DrawCurve {
                                                widget: new_widget,
                                                first_click: true,
                                                rotation: false,
                                                angle: 0.0,
                                            })
                                        } else {
                                            None
                                        }
                                    },
                                    _ => None,
                                }
                            },
                            DrawMode::New => {
                                match program_state {
                                    // First mouse click sets the state of the first Pending point
                                    // return a none since no Curve yet
                                    None => {
                                        let (widget, _) = set_widget_point(&self.state.selected_widget, cursor_position);
                                        
                                        *program_state = Some(Pending::N {
                                            widget,
                                        });
                                        None
                                    },
                                    // The second click is a Some() since it was created above
                                    // The pending is carrying the previous info
                                    Some(Pending::N { 
                                            widget, 
                                    }) => {
                                        let (widget, completed) = set_widget_point(widget, cursor_position);
                                        
                                        // if completed, we return the Curve and set the state to none
                                        // if not, then this is repeated until completed.
                                        if completed {
                                            *program_state = None;
                                            match widget {
                                                CanvasWidget::None => {
                                                    None
                                                },
                                                CanvasWidget::Bezier(bezier) => {
                                                    Some(DrawCurve {
                                                        widget: CanvasWidget::Bezier(bezier),
                                                        first_click: false,
                                                        rotation: false,
                                                        angle: 0.0,
                                                    })
                                                },
                                                CanvasWidget::Circle(circle) => { 
                                                    Some(DrawCurve {
                                                        widget: CanvasWidget::Circle(circle),
                                                        first_click: false,
                                                        rotation: false,
                                                        angle: 0.0,
                                                    })
                                                },
                                                CanvasWidget::Line(line) => {
                                                    Some(DrawCurve {
                                                        widget: CanvasWidget::Line(line),
                                                        first_click: false,
                                                        rotation: false,
                                                        angle: 0.0,
                                                    })
                                                },
                                                CanvasWidget::PolyLine(pl) => {
                                                    Some(DrawCurve{
                                                        widget: CanvasWidget::PolyLine(pl),
                                                        first_click: false,
                                                        rotation: false,
                                                        angle: 0.0,
                                                    })
                                                },
                                                CanvasWidget::Polygon(pg) => {
                                                    Some(DrawCurve {
                                                        widget:CanvasWidget::Polygon(pg),
                                                        first_click: false,
                                                        rotation: false,
                                                        angle: 0.0,
                                                    })
                                                },
                                                CanvasWidget::RightTriangle(r_tr) => {
                                                    Some(DrawCurve {
                                                        widget: CanvasWidget::RightTriangle(r_tr),
                                                        first_click: false,
                                                        rotation: false,
                                                        angle: 0.0,
                                                    })
                                                },
                                            }
                                            
                                        } else {
                                            *program_state = Some(Pending::N {
                                                widget,
                                            });
                                            None
                                        }
                                    },
                                    _ => None,
                                }
                            },
                            DrawMode::Rotate => {
                                None
                            },
                        }
                        // if self.state.edit_widget_index.is_some() {
                        
                                
                        // } else {
                            // adding an new curve
                            
                        // }
                    },
                    // mouse::Event::ButtonPressed(mouse::Button::Right) => {
                    //     if self.state.edit_widget_index.is_none() {
                    //         *program_state = None;
                    //         return (event::Status::Ignored, None)
                    //     }

                    //     match program_state {
                    //         // First mouse click sets the state of the first Pending point
                    //         // return a none since no Curve yet
                    //         None => {
                    //             *program_state = Some(Pending::Rotation { 
                    //                 widget: self.state.edit_widget.curve_type,
                    //                 points: self.state.edit_widget.points.clone(),
                    //                 poly_points: self.state.edit_widget.poly_points,
                    //                 mid_point: self.state.edit_widget.mid_point,
                    //                 step: 0.0,
                    //                 step_count: 0,
                    //                 angle: self.state.edit_widget.angle,
                    //                 first_click: true,
                    //                 color: self.state.edit_widget.color, 
                    //                 width: self.state.edit_widget.width,
                    //             });
                    //             Some(DrawCurve {
                    //                 curve_type: self.state.edit_widget.curve_type,
                    //                 points: self.state.edit_widget.points.clone(),
                    //                 poly_points: self.state.edit_widget.poly_points,
                    //                 mid_point: Point::default(),
                    //                 first_click: true,
                    //                 rotation: true,
                    //                 angle: self.state.edit_widget.angle,
                    //                 color: self.state.edit_widget.color,
                    //                 width: self.state.edit_widget.width,
                    //             })
                    //         },
                    //         // The second click is a Some() since it was created above
                    //         // The pending is carrying the previous info
                    //         Some(Pending::Rotation {
                    //             widget: curve_type,
                    //             points,
                    //             poly_points,
                    //             mid_point,
                    //             step: _,
                    //             step_count: _,
                    //             angle,
                    //             first_click: false,
                    //             color,
                    //             width,
                    //         }) => {
                    //             // update draw curve
                    //             let points = points.clone();
                    //             let curve_type = curve_type.clone();
                    //             let poly_points = poly_points.clone();
                    //             let mid_point = mid_point.clone();
                    //             let angle = angle.clone();
                    //             let color = color.clone();
                    //             let width = width.clone();

                    //             *program_state = None;

                    //             Some(DrawCurve {
                    //                 curve_type,
                    //                 points,
                    //                 poly_points,
                    //                 mid_point,
                    //                 first_click: false,
                    //                 rotation: false,
                    //                 angle,
                    //                 color,
                    //                 width,
                    //             })
                    //         },
                    //         _ => None,
                    //     }
                    // },
                    // mouse::Event::WheelScrolled { delta } => {
                    //     let scroll_direction = match delta {
                    //         mouse::ScrollDelta::Lines { x: _, y } => {
                    //             y
                    //         },
                    //         mouse::ScrollDelta::Pixels { x: _, y } => {
                    //             y
                    //         },
                    //     };
                    //     if !self.state.rotation {
                    //         *program_state = None;
                    //         return (event::Status::Ignored, None)
                    //     }
                    //     match program_state {
                    //         Some(Pending::Rotation {
                    //             widget: curve_type,
                    //             points,
                    //             poly_points,
                    //             mid_point,
                    //             step: _,
                    //             step_count,
                    //             angle: _,
                    //             first_click: _,
                    //             color,
                    //             width,
                    //         }) => {
                                
                    //             let step = PI/10.0;
                                
                    //             let step_angle = step * scroll_direction;

                    //             let mut step_count = *step_count + scroll_direction as i32;

                    //             if step_count > 19 {
                    //                 step_count = 0;
                    //             }
                    //             if step_count < 0 {
                    //                 step_count = 19;
                    //             }

                    //             let angle = step_count as f32/10.0 * step_angle;
                                
                    //             let points = rotate_geometry(points, *mid_point, &step_angle);

                    //             *program_state = Some(Pending::Rotation {
                    //                 widget: *curve_type,
                    //                 points,
                    //                 poly_points: *poly_points,
                    //                 mid_point: *mid_point, 
                    //                 step,
                    //                 step_count,
                    //                 angle, 
                    //                 first_click: false,
                    //                 color: *color,
                    //                 width: *width,
                    //             });
                    //             None
                    //         }
                    //         _ => None,
                    //     }
                    // }
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
        bounds: iced::Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let content =
            self.state.cache.draw(renderer, bounds.size(), |frame| {
                DrawCurve::draw_all(self.curves, frame, theme, self.state.edit_widget_index);

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
        bounds: iced::Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}


// fn edit_curve_first_click(curve_type: DrawCurve, cursor_position: Point) 
//     -> DrawCurve {
//     // The first click loads up the Curve
//     // since we're in edit mode, cursor position used.
//     match curve_type {
//         CanvasWidget::Bezier(bezier) => {
//             let index = find_closest_point_index(cursor_position, CanvasWidget::Bezier(bezier));
           
//         }
//     }
    
//     // either a point in the curve or the mid point will be assigned to
//     // the cursor position
//     let (edit_point_index, edit_mid_point) = 
//         find_closest_point_index(cursor_position, 
//                                 edit_mid_point.unwrap(), 
//                                 &pts,
//                                 curve_type,);
    
//     // ensures the right triangle stays aligned
//     if curve_type == CanvasWidget::RightTriangle {
//         if pts.len() > 1 {
//         pts[1].x = pts[0].x;
//         }
//         if pts.len() > 2 {
//             pts[2].y = pts[1].y;
//         }
//     }
//     // Since new points are generated using the cursor position,
//     // normally you would need to recalc the center position
//     // but since the point cuicle is not shown during movement,
//     // no need at this time.
//     if edit_mid_point.is_some() {
//         mid_point = edit_mid_point.unwrap();
//     }
//     (pts, mid_point, edit_point_index, edit_mid_point)

// }

// fn edit_curve_second_click(state: &State,
//                             cursor_position: Point,
//                             mut points: Vec<Point>, 
//                             edit_point_index: Option<usize>,
//                             ) -> (Vec<Point>, Point) {
    
//     // Since points_to_move was found using closest point,
//     // point_to_edit pointed to it therefore skip when some()
//     let curve_type = state.edit_widget.curve_type;

//     let (mut pts, mid_point) = if edit_point_index.is_some() {
//         points[edit_point_index.clone().unwrap()] = cursor_position;
//         // recalculate mid_point
//         let mid_point = get_mid_geometry(&points, curve_type);
//         (points, mid_point)
//     }  else {
//         let mid_point = cursor_position;
//         (translate_geometry(points, cursor_position, curve_type),
//         mid_point)
//     };
    
//     if curve_type == CanvasWidget::RightTriangle {
//         if pts.len() > 1 {
//         pts[1].x = pts[0].x;
//         }
//         if pts.len() > 2 {
//             pts[2].y = pts[1].y;
//         }
//     }
//     (pts, mid_point)

// }

#[derive(Debug, Clone, Default)]
pub struct DrawCurve {
    pub widget: CanvasWidget,
    pub first_click: bool,
    pub rotation: bool,
    pub angle: f32,
}

impl DrawCurve {
    fn draw_all(curves: &[DrawCurve], frame: &mut Frame, _theme: &Theme, 
                curve_to_edit: Option<usize>,) {
        // This draw only occurs at the completion of the widget(update occurs) and cache is cleared
        
        // increment_draw_curve_counter();
        
        for (index, draw_curve) in curves.iter().enumerate() {
            // if first click, skip the curve to be edited so that it 
            // will not be seen until the second click.  Otherwise is shows
            // during editing because there is no way to refresh
            // The pending routine will diplay the curve
            let edit_circles = 
                if !draw_curve.first_click && 
                    curve_to_edit.is_some() && 
                    index == curve_to_edit.unwrap() {
                    true
                } else {
                    false
            };
            if edit_circles {
                 // skiping index being edited
                continue;
            }
            
            let (path, color, width) = 
                match &draw_curve.widget {
                    CanvasWidget::Bezier(bz) => {
                        (build_bezier_path(bz, edit_circles, None), &bz.color, &bz.width)
                    },
                    CanvasWidget::Circle(cir) => {
                        (build_circle_path(cir, edit_circles, None), &cir.color, &cir.width)
                    },
                    CanvasWidget::Line(line) => {
                        (build_line_path(line, edit_circles, None), &line.color, &line.width)
                    },
                    CanvasWidget::PolyLine(pl) => {
                        (build_polyline_path(pl, edit_circles, None), &pl.color, &pl.width)
                    },
                    CanvasWidget::Polygon(pg) => {
                        (build_polygon_path(pg, edit_circles, None), &pg.color, &pg.width)
                    }
                    CanvasWidget::RightTriangle(r_tr) => {
                        (build_right_triangle_path(r_tr, edit_circles, None), &r_tr.color, &r_tr.width)
                    },
                    CanvasWidget::None => (Path::new(|_| {}), &Color::BLACK, &0.0),
                };

            frame.stroke(
                &path,
                Stroke::default()
                    .with_width(*width)
                    .with_color(*color),
            );
        }

    }
}



#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Pending {
    N {
        widget: CanvasWidget, 
    },
    Edit {
        widget: CanvasWidget, 
        first_click: bool,
        second_click: bool, 
        edit_curve_index: Option<usize>,
        edit_point_index: Option<usize>,
        edit_mid_point: bool,
        },
    Rotation {
        widget: CanvasWidget,
        step: f32,
        step_count: i32,
        angle: f32,
    }
}

impl Pending {
    fn draw(
        &self,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        cursor: mouse::Cursor,
    ) -> Geometry {
        let _ = theme;
        let mut frame = Frame::new(renderer, bounds.size());

        if let Some(pending_cursor) = cursor.position_in(bounds) {
            // This draw happens when the mouse is moved and the state is none.
            match self {
                Pending::N { 
                    widget, 
                } => {
                    let (path, color, width) = match widget {
                        CanvasWidget::Bezier(bz) => {
                            let path = 
                                build_bezier_path(
                                    bz, 
                                    DrawMode::New, 
                                    Some(pending_cursor),
                                    None,
                                    false,
                                );
                            (path, bz.color, bz.width)
                        },
                        CanvasWidget::Circle(cir) => {
                            let path = 
                                build_circle_path(
                                    cir, 
                                    DrawMode::New, 
                                    Some(pending_cursor),
                                    None,
                                    false,
                                );
                            (path, cir.color, cir.width)
                        },
                        CanvasWidget::Line(line) => {
                            let path = 
                                build_line_path(
                                    line, 
                                    DrawMode::New, 
                                    Some(pending_cursor),
                                    None,
                                    false,
                                );
                            (path, line.color, line.width)
                        },
                        // return points as they are set
                        CanvasWidget::PolyLine(pl) => {
                            let path = 
                                build_polyline_path(
                                    pl, 
                                    DrawMode::New, 
                                    Some(pending_cursor),
                                    None,
                                    false,
                                );
                            (path, pl.color, pl.width)
                        },
                        CanvasWidget::Polygon(pg) => {
                            let path = 
                                build_polygon_path(
                                    pg, 
                                    DrawMode::New, 
                                    Some(pending_cursor),
                                    None,
                                    false,
                                );
                            (path, pg.color, pg.width)
                        },
                        CanvasWidget::RightTriangle(r_tr) => {
                            let path = 
                                build_right_triangle_path(
                                    r_tr, 
                                    DrawMode::New, 
                                    Some(pending_cursor),
                                    None,
                                    false,
                                );
                            (path, r_tr.color, r_tr.width)
                        },
                        _ => (Path::new(|_| {}), Color::TRANSPARENT, 0.0)
                    };
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(width)
                            .with_color(color),
                    );
                },
                Pending::Edit { 
                    widget,
                    first_click: _,
                    second_click: _ ,
                    edit_curve_index: _, 
                    edit_point_index, 
                    edit_mid_point,  } => {
                    let (path, color, width) = match widget {
                        CanvasWidget::None => {
                            (Path::new(|_| {}), Color::TRANSPARENT, 0.0)
                        },
                        CanvasWidget::Bezier(bz) => {
                            let path = 
                                build_bezier_path(
                                    bz, 
                                    DrawMode::Edit, 
                                    Some(pending_cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );
                            (path, bz.color, bz.width)
                        },
                        CanvasWidget::Circle(cir) => {
                            let path = 
                                build_circle_path(
                                    cir, 
                                    DrawMode::Edit, 
                                    Some(pending_cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );
                            (path, cir.color, cir.width)
                        },
                        CanvasWidget::Line(line) => {
                            let path = 
                                build_line_path(
                                    line, 
                                    DrawMode::Edit, 
                                    Some(pending_cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );
                            (path, line.color, line.width)
                        },
                        CanvasWidget::PolyLine(pl) => {
                            let path = 
                                build_polyline_path(
                                    pl, 
                                    DrawMode::Edit, 
                                    Some(pending_cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );
                            (path, pl.color, pl.width)
                        },
                        CanvasWidget::Polygon(pg) => {
                            let path = 
                                build_polygon_path(
                                    pg, 
                                    DrawMode::Edit, 
                                    Some(pending_cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );
                            (path, pg.color, pg.width)
                        },
                        CanvasWidget::RightTriangle(r_tr) => {
                            let path = 
                                build_right_triangle_path(
                                    r_tr, 
                                    DrawMode::Edit, 
                                    Some(pending_cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );
                            (path, r_tr.color, r_tr.width)
                        },
                    };
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(width)
                            .with_color(color),
                    );
                },
                Pending::Rotation {
                    widget,
                    step: _,
                    step_count: _,
                    angle: _, 
                } => {
                    // let (path, color, width) = match widget {
                    //     CanvasWidget::None => {
                    //         (Path::new(|_| {}), Color::TRANSPARENT, 0.0)
                    //     },
                    //     CanvasWidget::Bezier(bz) => {
                    //         let path = 
                    //             build_bezier_path(
                    //                 bz, 
                    //                 DrawMode::Rotate, 
                    //                 None,
                    //                 None,
                    //                 false,
                    //             );
                    //         (path, bz.color, bz.width)
                    //     },
                    //     CanvasWidget::Circle(cir) => {
                    //         let path = 
                    //             build_circle_path(
                    //                 cir, 
                    //                 DrawMode::Rotate, 
                    //                 None,
                    //                 None,
                    //                 false,
                    //             );
                    //         (path, cir.color, cir.width)
                    //     },
                    //     CanvasWidget::Line(line) => {
                    //         let path = 
                    //             build_line_path(
                    //                 line, 
                    //                 DrawMode::Rotate, 
                    //                 None,
                    //                 None,
                    //                 false,
                    //             );
                    //         (path, line.color, line.width)
                    //     },
                    //     CanvasWidget::PolyLine(pl) => {
                    //         let path = 
                    //             build_polyline_path(
                    //                 pl, 
                    //                 DrawMode::Rotate, 
                    //                 None,
                    //                 None,
                    //                 false,
                    //             );
                    //         (path, pl.color, pl.width)
                    //     },
                    //     CanvasWidget::Polygon(pg) => {
                    //         let path = 
                    //             build_polygon_path(
                    //                 pg, 
                    //                 DrawMode::Rotate, 
                    //                 None,
                    //                 None,
                    //                 false,
                    //             );
                    //         (path, pg.color, pg.width)
                    //     },
                    //     CanvasWidget::RightTriangle(r_tr) => {
                    //         let path = 
                    //             build_right_triangle_path(
                    //                 r_tr, 
                    //                 DrawMode::Rotate, 
                    //                 None,
                    //                 None,
                    //                 false,
                    //             );
                    //         (path, r_tr.color, r_tr.width)
                    //     },
                    // };
                    // frame.stroke(
                    //     &path,
                    //     Stroke::default()
                    //         .with_width(width)
                    //         .with_color(color),
                    // );
                },
            };
        }

        frame.into_geometry()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Bezier {
    pub points: Vec<Point>,
    pub mid_point: Point,
    pub color: Color,
    pub width: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Circle {
    pub center: Point,
    pub circle_point: Point,
    pub radius: f32,
    pub color: Color,
    pub width: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Line {
    pub points: Vec<Point>,
    pub mid_point: Point,
    pub color: Color,
    pub width: f32,
}

#[derive(Debug, Clone, Default)]
pub struct PolyLine {
    pub points: Vec<Point>,
    pub poly_points: usize,
    pub mid_point: Point,
    pub color: Color,
    pub width: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Polygon {
    pub points: Vec<Point>,
    pub poly_points: usize,
    pub mid_point: Point,
    pub pg_point: Point,
    pub color: Color,
    pub width: f32,
}

#[derive(Debug, Clone, Default)]
pub struct RightTriangle {
    pub points: Vec<Point>,
    pub mid_point: Point,
    pub color: Color,
    pub width: f32,
}

pub enum Widget {
    Bezier,
    Circle,
    Line,
    PolyLine,
    Polygon,
    RightTriangle,
    Triangle
}

#[derive(Clone, Copy, Debug, PartialEq, Eq,)]
pub enum DrawMode {
    DrawAll,
    Edit,
    New,
    Rotate,
}
// fn point_in_circle(point: Point, cursor: Point) -> bool {
//     let dist = point.distance(cursor);
//     if dist < 5.0 {
//         true
//     } else {
//         false
//     }
// }

fn find_widget(widget: &CanvasWidget, cursor_position: Point) -> bool {
    false
}
// Adds a cursor position to the points then determines 
// if finish by returning the widget and the boolean
fn set_widget_point(widget: &CanvasWidget, cursor_position: Point) -> (CanvasWidget, bool) {
    match widget {
        CanvasWidget::None => (),
        CanvasWidget::Bezier(bezier) => {
            bezier.points.push(cursor_position);
            let finished = if bezier.points.len() == 3 {
                bezier.mid_point = get_mid_geometry(&bezier.points, Widget::Bezier);
                true
            } else {
                false
            };
            (CanvasWidget::Bezier(*bezier), finished)
        },
        CanvasWidget::Circle(circle) => {
            let finished = if circle.center != Point::default() {
                circle.center = cursor_position;
                false
            } else {
                circle.radius = circle.center.distance(cursor_position);
                true
            };
            (CanvasWidget::Circle(*circle), finished)
        },
        CanvasWidget::Line(line) => {
            line.points.push(cursor_position);
            let finished = if line.points.len() == 2 {
                line.mid_point = get_mid_point(line.points[0], line.points[1]);
                true
            } else {
                false
            };
            (CanvasWidget::Line(*line), finished)
        },
        CanvasWidget::PolyLine(poly_line) => {
            poly_line.points.push(cursor_position);
            let finished = if poly_line.points.len() == poly_line.poly_points {
                poly_line.mid_point = get_mid_geometry(&poly_line.points, Widget::PolyLine);
                true
            } else {
                false
            };
            (CanvasWidget::PolyLine(*poly_line), finished)
        },
        CanvasWidget::Polygon(polygon) => {
            polygon.points.push(cursor_position);
            let finished = if polygon.points.len() == polygon.poly_points {
                // close the polygon
                polygon.points.push(polygon.points[0]);
                polygon.mid_point = get_mid_geometry(&polygon.points, Widget::Polygon);
                true
            } else {
                false
            };
            (CanvasWidget::Polygon(*polygon), finished)
        },
        CanvasWidget::RightTriangle(right_triangle) => {
            right_triangle.points.push(cursor_position);
            if right_triangle.points.len() > 1 {
            right_triangle.points[1].x = right_triangle.points[0].x;
            }
            if right_triangle.points.len() > 2 {
                right_triangle.points[2].y = right_triangle.points[1].y;
            }
            let finished = if right_triangle.points.len() == 3 {
                // close the triangle
                right_triangle.points.push(right_triangle.points[0]);
                right_triangle.mid_point = get_mid_geometry(&right_triangle.points, Widget::RightTriangle);
                true
            } else {
                false
            };
            (CanvasWidget::RightTriangle(*right_triangle), finished)
        },
    }
}

fn edit_widget_points(widget: CanvasWidget, 
                        cursor: Point, 
                        index: Option<usize>, 
                        mid_point: bool,
                    ) -> CanvasWidget {
    match widget {
        CanvasWidget::None => {
            CanvasWidget::None
        },
        CanvasWidget::Bezier(mut bz) => {
            if index.is_some() {
                bz.points[index.unwrap()] = cursor;
                bz.mid_point = get_mid_geometry(&bz.points, Widget::Bezier);
            } else if mid_point {
                bz.mid_point = cursor;
                bz.points = translate_geometry(bz.points, cursor, Widget::Bezier)
            }
            CanvasWidget::Bezier(bz)
        },
        CanvasWidget::Circle(mut cir) => {
            if index.is_some() {
                cir.circle_point = cursor;
                cir.radius = cir.center.distance(cursor);
            } else if mid_point {
                cir.center = cursor;
                let mut points = vec![cir.center, cir.circle_point];
                points = translate_geometry(points, cursor, Widget::Circle);
                cir.center = points[0];
                cir.circle_point = points[1];
            }

            CanvasWidget::Circle(cir)
        },
        CanvasWidget::Line(mut line) => {
            if index.is_some() {
                line.points[index.unwrap()] = cursor;
            } else if mid_point {
                line.mid_point = cursor;
                line.points = translate_geometry(line.points.clone(), cursor, Widget::Line);
            }

            CanvasWidget::Line(line)
        },
        CanvasWidget::PolyLine(mut pl) => {
            if index.is_some() {
                pl.points[index.unwrap()] = cursor;
            } else if mid_point {
                pl.mid_point = cursor;
                pl.points = translate_geometry(pl.points.clone(), cursor, Widget::PolyLine);
            }

            CanvasWidget::PolyLine(pl)
        },
        CanvasWidget::Polygon(mut pg) => {
            if index.is_some() {
                pg.pg_point = cursor;
                pg.points = build_polygon(pg.mid_point, pg.pg_point, pg.poly_points)
            } else if mid_point {
                let mut pts = vec![pg.mid_point, pg.pg_point];
                pts = translate_geometry(pts, cursor, Widget::Polygon);
                pg.points = build_polygon(pts[0], pts[1], pg.poly_points);
                pg.mid_point = pts[0];
                pg.pg_point = pts[1];
            }

            CanvasWidget::Polygon(pg)
        },
        CanvasWidget::RightTriangle(mut r_tr) => {
            if index.is_some() {
                r_tr.points[index.unwrap()] = cursor;
            } else if mid_point {
                r_tr.mid_point = cursor;
                r_tr.points = translate_geometry(r_tr.points.clone(), cursor, Widget::RightTriangle);
            }

            CanvasWidget::RightTriangle(r_tr)
        },
    }
}

fn find_closest_point_index(cursor: Point, 
                            widget: &CanvasWidget, ) 
                            -> (Option<usize>, bool) {

    let mut distance: f32 = 1000000.0;
    let mut point_index = 0;

    match widget {
        CanvasWidget::None => (None, false),
        CanvasWidget::Bezier(bezier) => {
            for (idx, point) in bezier.points.iter().enumerate() {
                let dist = cursor.distance(*point);
                if  dist < distance {
                    point_index = idx;
                    distance = dist;
                }
            };
            
            let mid_dist = bezier.mid_point.distance(cursor);
            if mid_dist < distance {
                (None, true)
            } else {
                (Some(point_index), false)
            }
        },
        CanvasWidget::Circle(cir) => {
            let cir_center = cursor.distance(cir.center);
            let cir_point = cursor.distance(cir.circle_point);
            if cir_center <= cir_point {
                (None, true)
            } else {
                (Some(1), false)
            }
        } 
        CanvasWidget::Line(line) => {
            let pts = line.points.clone();
            pts.push(line.mid_point);
            for (idx, point) in line.points.iter().enumerate() {
                let dist = cursor.distance(*point);
                if  dist < distance {
                    point_index = idx;
                    distance = dist;
                }
            };
            
            if point_index == 2 {
                (None, true)
            } else {
                (Some(point_index), false)
            }
        },
        CanvasWidget::PolyLine(poly_line) => {
            for (idx, point) in poly_line.points.iter().enumerate() {
                let dist = cursor.distance(*point);
                if  dist < distance {
                    point_index = idx;
                    distance = dist;
                }
            };
            
            let mid_dist = poly_line.mid_point.distance(cursor);
            if mid_dist < distance {
                (None, true)
            } else {
                (Some(point_index), false)
            }
        },
        CanvasWidget::Polygon(pg) => {
            let pg_center = cursor.distance(pg.points[0]);
            let pg_point = cursor.distance(pg.points[1]);
            if pg_center <= pg_point {
                (None, true)
            } else {
                (Some(1), false)
            }
        },
        CanvasWidget::RightTriangle(tr) {
            for (idx, point) in tr.points.iter().enumerate() {
                let dist = cursor.distance(*point);
                if  dist < distance {
                    point_index = idx;
                    distance = dist;
                }
            };
            
            let mid_dist = tr.mid_point.distance(cursor);
            if mid_dist < distance {
                (None, true)
            } else {
                (Some(point_index), false)
            }
        },

    }
    
}

fn get_mid_point(pt1: Point, pt2: Point) -> Point {
    Point {x: (pt1.x + pt2.x) / 2.0, y: (pt1.y + pt2.y) / 2.0 }
}

pub fn get_mid_geometry(pts: &Vec<Point>, curve_type: Widget) -> Point {

    match curve_type {
        Widget::Bezier => {
            let x = (pts[0].x + pts[1].x + pts[2].x)/3.0;
            let y = (pts[0].y + pts[1].y + pts[2].y)/3.0;
            Point {x, y}
        },
        Widget::Circle => {
            // return the center point
            pts[0]
        },
        Widget::Line => {
            get_mid_point(pts[0], pts[1])
        },
        Widget::PolyLine => {
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
        Widget::Polygon => {
            // return the center point
            pts[0]
        },
        Widget::Rectangle => {
            get_mid_point(pts[0], pts[1])
        },
        Widget::RightTriangle => {
            let x = (pts[0].x + pts[1].x + pts[2].x)/3.0;
            let y = (pts[0].y + pts[1].y + pts[2].y)/3.0;
            Point {x, y}
        },
        Widget::Triangle => {
            let x = (pts[0].x + pts[1].x + pts[2].x)/3.0;
            let y = (pts[0].y + pts[1].y + pts[2].y)/3.0;
            Point {x, y}
        },
    }
    
}

fn translate_geometry(pts: Vec<Point>, 
                        new_center: Point, 
                        curve_type: Widget) 
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

// To rotate a point (x, y) around a center point (cx, cy) by an angle θ, 
// the formula for the rotated coordinates (x', y') is: 
// x' = (x - cx) * cos(θ) - (y - cy) * sin(θ) + cx and 
// y' = (x - cx) * sin(θ) + (y - cy) * cos(θ) + cy; 
// where (x, y) is the original point, (cx, cy) is the center of rotation, 
//and θ is the rotation angle in radians. 
fn rotate_geometry(points: &Vec<Point>, center: Point, theta: &f32) -> Vec<Point> {

    let mut new_points = vec![];
    for point in points.iter() {
        let x_new = (point.x - center.x) * theta.cos() - (point.y - center.y) * theta.sin() + center.x;
        let y_new = (point.x - center.x) * theta.sin() + (point.y - center.y) * theta.cos() + center.y;

        new_points.push(Point { x: x_new, y: y_new })
    }
    
    new_points
     
}

fn get_rectangle_size(top_left: Point, bottom_right: Point) -> Size {
    let width = (top_left.x - bottom_right.x).abs();
    let height = (top_left.y - bottom_right.y).abs();
    Size::new(width, height)
}

fn build_polygon(mid_point: Point, point: Point, num_points: usize) -> Vec<Point> {
    let angle = 2.0 * PI / num_points as f32;
    let radius = mid_point.distance(point);
    let mut points = vec![];
    for i in 0..num_points {
        let x = mid_point.x + radius * (i as f32 * angle).sin();
        let y = mid_point.y + radius * (i as f32 * angle).cos();
        points.push(Point::new(x, y));
    }
    points
}

fn build_bezier_path(bz: &Bezier, 
                    draw_mode: DrawMode, 
                    pending_cursor: Option<Point>,
                    edit_point_index: Option<usize>, 
                    edit_mid_point: bool,
                ) -> Path {
    Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                p.move_to(bz.points[0]);
                p.quadratic_curve_to(bz.points[2], bz.points[1]);
            },
            DrawMode::Edit => {
                let points = if edit_mid_point.is_some() {
                    translate_geometry(bz.points.clone(), edit_mid_point.unwrap(), Widget::Bezier)
                } else if edit_point_index.is_some() {
                    let mut pts = bz.points.clone();
                    pts[edit_point_index.unwrap()] = pending_cursor.unwrap();
                    pts
                } else {
                    bz.points.clone()
                };
                p.move_to(points[0]);
                p.quadratic_curve_to(points[2], points[1]);
                
                for pt in points {
                    p.circle(pt, 3.0);
                }
                if edit_mid_point.is_some() {
                    p.circle(edit_mid_point.unwrap(), 3.0);
                } else {
                    p.circle(bz.mid_point, 3.0);
                }
            },
            DrawMode::New => {
                if bz.points.len() == 2 {
                    p.move_to(bz.points[0]);
                    p.quadratic_curve_to(pending_cursor.unwrap(), bz.points[1]);
                } else {
                    if bz.points.len() == 1 {
                        p.move_to(bz.points[0]);
                        p.line_to(pending_cursor.unwrap());
                    }
                }
            },
            DrawMode::Rotate => {

            },
        }
    })

}

fn build_circle_path(cir: &Circle, 
                    draw_mode: DrawMode, 
                    pending_cursor: Option<Point>,
                    edit_point_index: Option<usize>, 
                    edit_mid_point: bool,
                ) -> Path {
    Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                p.circle(cir.center, cir.radius);
            },
            DrawMode::Edit => {
                let center = if edit_mid_point.is_some() {
                    edit_mid_point.unwrap()
                } else {
                    cir.center
                };
                let radius = if edit_point_index.is_some() {
                    center.distance(to)
                } else {
                    cir.radius
                };
                p.circle(center, radius);
                p.circle(center, 3.0);
                p.circle(circle_point, 3.0);
            },
            DrawMode::New => {
                let circle_point = pending_cursor.unwrap();
                let radius = cir.center.distance(circle_point);
                p.circle(cir.center, radius);
            },
            DrawMode::Rotate => {

            },
        }
    })
}

fn build_line_path(line: &Line, 
                    draw_mode: DrawMode, 
                    pending_cursor: Option<Point>,
                    edit_point_index: Option<usize>, 
                    edit_mid_point: bool,
                ) -> Path {
    Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                p.move_to(line.points[0]);
                p.line_to(line.points[1]);
            },
            DrawMode::Edit => {
                p.move_to(line.points[0]);
                p.line_to(line.points[1]);
                p.circle(line.mid_point, 3.0);
            },
            DrawMode::New => {
                p.move_to(line.points[0]);
                p.line_to(pending_cursor.unwrap());
            },
            DrawMode::Rotate => {

            },
        }
    })
}

fn build_polyline_path(pl: &PolyLine, 
                        draw_mode: DrawMode, 
                        pending_cursor: Option<Point>,
                        edit_point_index: Option<usize>, 
                        edit_mid_point: bool,
                    ) -> Path {
    Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                for (index, point) in pl.points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                }
            },
            DrawMode::Edit => {
                for (index, point) in pl.points.iter().enumerate() {
                    p.circle(*point, 3.0);
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                    p.line_to(pending_cursor.unwrap());
                }
            },
            DrawMode::New => {
                for (index, point) in pl.points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                    p.line_to(pending_cursor.unwrap());
                }
            },
            DrawMode::Rotate => {
                for (index, point) in pl.points.iter().enumerate() {

                }
            },
        }
    })
}

fn build_polygon_path(pg: &Polygon, 
                        draw_mode: DrawMode, 
                        pending_cursor: Option<Point>,
                        edit_point_index: Option<usize>, 
                        edit_mid_point: bool,
                    ) -> Path {
    Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                let points = &pg.points;
                for (index, point) in points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                }
                p.line_to(points[0]);
            },
            DrawMode::Edit => {
                let points = 
                    build_polygon(pg.mid_point, pending_cursor.unwrap(), pg.poly_points);
                for (index, point) in points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                }
                p.line_to(points[0]);
                p.circle(pg.mid_point, 3.0);
                p.circle(pg.pg_point, 3.0);
            },
            DrawMode::New => {
                let points = 
                    build_polygon(pg.mid_point, pending_cursor.unwrap(), pg.poly_points);
                for (index, point) in points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                }
            },
            DrawMode::Rotate => {

            },
        }
    })
}

fn build_rectangle_path(rect: &Rectangle, 
                        draw_mode: DrawMode, 
                        pending_cursor: Option<Point>,
                        edit_point_index: Option<usize>, 
                        edit_mid_point: bool,
                    ) -> Path {
    Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                p.rectangle(rect.top_left, rect.size);
            },
            DrawMode::Edit => {
                let size = if rect.size == Size::default() {
                    get_rectangle_size(rect.top_left, pending_cursor.unwrap())
                } else {
                    rect.size
                };
                p.rectangle(rect.top_left, size);

                p.circle(rect.top_left, 3.0);
                p.circle(rect.bottom_right, 3.0);
                p.circle(rect.mid_point, 3.0);
            },
            DrawMode::New => 
            {
                let size: Size = get_rectangle_size(rect.top_left, pending_cursor.unwrap());
                p.rectangle(rect.top_left, size);
            },
            DrawMode::Rotate => {

            },
        }
    })
}

fn build_right_triangle_path(r_tr: &RightTriangle, 
                            draw_mode: DrawMode, 
                            pending_cursor: Option<Point>,
                            edit_point_index: Option<usize>, 
                            edit_mid_point: bool,
                        ) -> Path {
    Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                p.move_to(r_tr.points[0]);
                for point in r_tr.points.iter() {
                    p.line_to(*point);
                }
            },
            DrawMode::Edit => {
                let mut cursor = pending_cursor.unwrap();
                p.move_to(r_tr.points[0]);
                cursor.x = r_tr.points[1].x;
                p.line_to(cursor);
                p.line_to(r_tr.points[0]);
                
                for point in r_tr.points.iter() {
                    p.line_to(*point);
                    p.circle(*point, 3.0);
                }
            },
            DrawMode::New => {
                let mut cursor = pending_cursor.unwrap();
                p.move_to(r_tr.points[0]);
                if r_tr.points.len() == 1 {
                    cursor.y = r_tr.points[0].y;
                    p.line_to(cursor);
                } else if r_tr.points.len() == 2 {
                    cursor.x = r_tr.points[1].x;
                    p.line_to(cursor);
                    p.line_to(r_tr.points[0]);
                }
                for point in r_tr.points.iter() {
                    p.line_to(*point);
                }
            },
            DrawMode::Rotate => {
                
            },
        }
    })
}

fn build_triangle_path(tr: &Triangle, 
                        draw_mode: DrawMode, 
                        pending_cursor: Option<Point>,
                        edit_point_index: Option<usize>, 
                        edit_mid_point: bool,
                    ) -> Path {
    Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                for (index, point) in tr.points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                }
            },
            DrawMode::Edit => {
                for (index, point) in tr.points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                        p.circle(*point, 3.0);
                    } else {
                        p.line_to(*point);
                        p.circle(*point, 3.0);
                    }
                }
            },
            DrawMode::New => {
                for (index, point) in tr.points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                    p.line_to(pending_cursor.unwrap());
                }
            },
            DrawMode::Rotate => {
                
            },
        }
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
