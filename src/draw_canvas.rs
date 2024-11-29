
use std::f32::consts::PI;
// use std::sync::{Mutex, MutexGuard};
use iced::{mouse, Color};
use iced::widget::canvas::event::{self, Event};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Element, Fill, Point, Renderer, Theme};
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq,)]
pub enum DrawMode {
    #[default]
    DrawAll,
    Edit,
    New,
    Rotate,
}

// used to display text widget
impl DrawMode {
    pub fn string(&self) -> String {
        match &self {
            DrawMode::DrawAll => "DrawAll".to_string(),
            DrawMode::New => "New".to_string(),
            DrawMode::Edit => "Edit".to_string(),
            DrawMode::Rotate => "Rotate".to_string(),
        }
    }

    pub fn to_enum(s: String) -> Self {
        match s.as_str() {
            "DrawAll" | "drawall" | "Drawall" => DrawMode::DrawAll,
            "Edit" | "edit" => DrawMode::Edit,
            "New" | "new" => DrawMode::New,
            "Rotate" | "rotate" => DrawMode::Rotate,
            _ => DrawMode::DrawAll,
        }
    }
}

#[derive(Debug)]
pub struct State {
    cache: canvas::Cache,
    pub draw_mode: DrawMode,
    pub draw_width: f32,
    pub edit_widget_index: Option<usize>,
    pub escape_pressed: bool,
    pub selected_widget: CanvasWidget,
    pub selected_radio_widget: Option<Widget>,
    pub selected_color: Color,
    pub selected_color_str: Option<String>,
    pub selected_poly_points: usize,
    pub selected_poly_points_str: String,
    pub selected_step_degrees: f32,
}

impl Default for State {
    fn default() -> Self {
        Self { 
                cache: canvas::Cache::default(),
                draw_mode: DrawMode::DrawAll,
                draw_width: 2.0,
                edit_widget_index: None,
                escape_pressed: false,
                selected_widget: CanvasWidget::None,
                selected_radio_widget: None,
                selected_color: Color::WHITE,
                selected_color_str: None,
                selected_poly_points: 3,
                selected_poly_points_str: "".to_string(),
                selected_step_degrees: 6.0,
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
                                    // edit consists of 3 clicks
                                    // 1 - find closest widget
                                    // 2 - find closest point
                                    // 3 - finish
                                    None => {
                                        let mut closest = 1_000_000_f32;
                                        let mut index = None;
                                        for (idx, curve) in self.curves.iter().enumerate() {
                                            let distance: f32 = get_distance_to_mid_point(curve, cursor_position);
                                            if distance < closest {
                                                index = Some(idx);
                                                closest = distance;
                                            }
                                        }
                                        
                                        if index.is_some() {
                                            let selected_widget = 
                                                self.curves[index.unwrap()].widget.clone();
                                            
                                            *program_state = Some(Pending::EditSecond {
                                                widget: selected_widget,
                                                edit_curve_index: index,
                                                edit_point_index: None, 
                                                edit_mid_point: false,
                                            });
                                            // returning DrawCurve so that the curve
                                            // being editied will not show after the refresh
                                            // The pending process will show the curve
                                            // until its finsihed.

                                            // update date the curve to indicate that
                                            // it is in edit mode.  Just using same method
                                            // so redunant info.
                                            let widget = self.curves[index.unwrap()].widget.clone();
                                            
                                            let new_widget = 
                                                set_widget_draw_mode(
                                                    widget, 
                                                    self.state.draw_mode,
                                                );

                                            Some(DrawCurve {
                                                widget: new_widget,
                                                edit_curve_index: index,
                                            })
                                        } else {
                                            None
                                        }
                                    },
                                    // The second click is a Some() since it was created above
                                    // The pending is carrying the previous info
                                    // This second click will find the point
                                    // and replace with cursor
                                    Some(Pending::EditSecond { 
                                        widget,
                                        edit_curve_index,
                                        edit_point_index: _, 
                                        edit_mid_point: _,
                                    }) => {
                                        // Find for closest point to edit in selected widget
                                        // which might be either a mid point(translate) or 
                                        // curve point (move point).
                                        let widget = widget.clone();
                                        let (point_index, mid_point) = 
                                            find_closest_point_index(&widget, cursor_position);
                                        
                                        *program_state = Some(Pending::EditThird {
                                            widget,
                                            edit_curve_index: *edit_curve_index,
                                            edit_point_index: point_index,
                                            edit_mid_point: mid_point,
                                        });
                                        None
                                    },
                                    // The third click will send back the DrawCurve
                                    // with the finally updated curve
                                    Some(Pending::EditThird { 
                                        widget,
                                        edit_curve_index,
                                        edit_point_index,
                                        edit_mid_point, 
                                    }) => {
                                        let edit_curve_index = *edit_curve_index;
                                        let mut new_widget: CanvasWidget = 
                                                edit_widget_points(
                                                    widget.clone(), 
                                                    cursor_position, 
                                                    *edit_point_index, 
                                                    *edit_mid_point
                                                );
                                        new_widget = 
                                            set_widget_draw_mode(
                                                new_widget, 
                                                DrawMode::DrawAll,
                                            );
                                        
                                        *program_state = None;
                                        Some(DrawCurve {
                                            widget: new_widget,
                                            edit_curve_index,
                                        })
                                    },
                                    _ => None,
                                }
                            },
                            DrawMode::New => {
                                match program_state {
                                    // First mouse click sets the state of the first Pending point
                                    // return a none since no Curve yet
                                    None => {
                                        // in case the poly points, color, and width have changed since 
                                        // the widget selected
                                        let selected_widget = 
                                            update_widget(
                                                self.state.selected_widget.clone(), 
                                                self.state.selected_poly_points,
                                                self.state.selected_color,
                                                self.state.draw_width,
                                                self.state.draw_mode,
                                            );
                                        let (widget, _) = set_widget_point(&selected_widget, cursor_position);
                                        *program_state = Some(Pending::New {
                                            widget,
                                        });
                                        None
                                    },
                                    // The second click is a Some() since it was created above
                                    // The pending is carrying the previous info
                                    Some(Pending::New { 
                                            widget, 
                                    }) => {

                                        let (widget, completed) = 
                                            set_widget_point(widget, cursor_position);
                                        
                                        // if completed, we return the Curve and set the state to none
                                        // if not, then this is repeated until completed.
                                        if completed {
                                            *program_state = None;
                                            match widget {
                                                CanvasWidget::None => {
                                                    None
                                                },
                                                CanvasWidget::Bezier(mut bz) => {
                                                    bz.mid_point = get_mid_point(bz.points[0], bz.points[1]);
                                                    Some(DrawCurve {
                                                        widget: CanvasWidget::Bezier(bz),
                                                        edit_curve_index: None,
                                                    })
                                                },
                                                CanvasWidget::Circle(circle) => { 
                                                    Some(DrawCurve {
                                                        widget: CanvasWidget::Circle(circle),
                                                        edit_curve_index: None,
                                                    })
                                                },
                                                CanvasWidget::Line(mut ln) => {
                                                    // degree is angle rotation around mid point 
                                                    let degrees = 
                                                        get_angle_of_vectors(
                                                            ln.points[0],
                                                            ln.points[1], 
                                                        );
                                                    ln.degrees = degrees;

                                                    Some(DrawCurve {
                                                        widget: CanvasWidget::Line(ln),
                                                        edit_curve_index: None,
                                                    })
                                                },
                                                CanvasWidget::PolyLine(mut pl) => {
                                                    let (slope, intercept) =
                                                        get_linear_regression(&pl.points);
                                                    
                                                    let line = 
                                                        get_line_from_slope_intercept(
                                                            &pl.points, 
                                                            slope, 
                                                            intercept
                                                        );
                                                    pl.mid_point = get_mid_point(line.0, line.1);
                                                    pl.degrees = 
                                                        get_angle_of_vectors(
                                                            pl.mid_point,
                                                            line.1,
                                                        );

                                                    Some(DrawCurve{
                                                        widget: CanvasWidget::PolyLine(pl),
                                                        edit_curve_index: None,
                                                    })
                                                },
                                                CanvasWidget::Polygon(mut pg) => {
                                                    pg.pg_point = cursor_position;
                                                    let degrees = 
                                                        get_angle_of_vectors(
                                                            pg.mid_point, 
                                                            cursor_position, 
                                                            );

                                                    pg.degrees = degrees;
                                                    pg.points = 
                                                        build_polygon(
                                                            pg.mid_point, 
                                                            pg.pg_point, 
                                                            pg.poly_points,
                                                            pg.degrees,
                                                        );
                                                    
                                                    Some(DrawCurve {
                                                        widget:CanvasWidget::Polygon(pg),
                                                        edit_curve_index: None,
                                                    })
                                                },
                                                CanvasWidget::RightTriangle(r_tr) => {
                                                    Some(DrawCurve {
                                                        widget: CanvasWidget::RightTriangle(r_tr),
                                                        edit_curve_index: None,
                                                    })
                                                },
                                            }
                                            
                                        } else {
                                            *program_state = Some(Pending::New {
                                                widget,
                                            });
                                            None
                                        }
                                    },
                                    _ => None,
                                }
                            },
                            DrawMode::Rotate => {
                                match program_state {
                                    // rotation consists of 2 clicks
                                    // 1 - find closest widget
                                    //  - move mouse wheel
                                    // 2 - click to finish
                                    None => {
                                        let mut closest = 1_000_000_f32;
                                        let mut index = None;
                                        for (idx, curve) in self.curves.iter().enumerate() {
                                            let distance: f32 = get_distance_to_mid_point(curve, cursor_position);
                                            if distance < closest {
                                                index = Some(idx);
                                                closest = distance;
                                            }
                                        }
                                        
                                        if index.is_some() {

                                            let selected_widget = self.curves[index.unwrap()].widget.clone();

                                            // The widget needs to be in DrawAll initially, 
                                            // in order to display it in pending
                                            // However, the below return of the draw curve 
                                            // the widget need to ne in the rotate method.
                                            let widget = 
                                                set_widget_draw_mode(
                                                    selected_widget, 
                                                    DrawMode::Rotate,
                                                );
                                            
                                            *program_state = Some(Pending::Rotate {
                                                widget: widget.clone(),
                                                edit_curve_index: index,
                                                step_degrees: self.state.selected_step_degrees,
                                                degrees: get_widget_degrees(&widget),
                                            });

                                            // returning DrawCurve so that the curve
                                            // being editied will not show after the refresh
                                            // The pending process will show the curve
                                            // until its finsihed.
                                            // let widget = 
                                            //     set_widget_draw_mode(
                                            //         widget, 
                                            //         DrawMode::Rotate,
                                            //     );
                                            Some(DrawCurve {
                                                widget,
                                                edit_curve_index: index,
                                            })
                                        } else {
                                            None
                                        }
                                    },
                                    // After the final rotation completed
                                    Some(Pending::Rotate {
                                        widget,
                                        edit_curve_index,
                                        step_degrees: _,
                                        degrees,
                                    }) => {
                                        let rotated_widget = 
                                            update_rotated_widget(
                                                widget,
                                                degrees.unwrap(),
                                                0.0,
                                                DrawMode::DrawAll,
                                            );

                                        let edit_curve_index = *edit_curve_index;

                                        *program_state = None;

                                        Some(DrawCurve {
                                            widget: rotated_widget,
                                            edit_curve_index,
                                        })
                                    },
                                    _ => None,
                                }
                            },
                        }
                    },
                    mouse::Event::WheelScrolled { delta} => {
                        match self.state.draw_mode {
                            DrawMode::Rotate => {
                                match program_state {
                                    None => None,
                                    
                                    Some(Pending::Rotate { 
                                        widget,
                                        edit_curve_index,
                                        step_degrees,
                                        degrees,  
                                    }) => {
                                        let delta = match delta {
                                            mouse::ScrollDelta::Lines { x:_, y } => y,
                                            mouse::ScrollDelta::Pixels { x:_, y } => y,
                                        };
                                        
                                        let mut degrees = degrees.unwrap() + *step_degrees * delta;
                                        
                                        degrees %= 360.0;

                                        // Setting the widget draw_mode at each mouse wheel
                                        // since it was set to DrawAll initially.
                                        // Otherwise needed to have another pending type
                                        // and duplicate a lot of code.  Had to clone anyway.
                                        
                                        let widget = update_rotated_widget(widget, degrees, *step_degrees*delta, DrawMode::Rotate);
                                        *program_state = Some(Pending::Rotate{
                                            widget,
                                            edit_curve_index: *edit_curve_index,
                                            step_degrees: *step_degrees,
                                            degrees: Some(degrees),
                                        });
                                        None
                                        
                                    },
                                    _ => None,
                                }
                            },
                            _ => None,
                        }
                    },
                    _ => None,
                };
                (event::Status::Captured, message)
            },
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
                DrawCurve::draw_all(self.curves, frame, theme);

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


#[derive(Debug, Clone, Default)]
pub struct DrawCurve {
    pub widget: CanvasWidget,
    // pub first_click: bool,
    pub edit_curve_index: Option<usize>,
}

impl DrawCurve {
    fn draw_all(curves: &[DrawCurve], frame: &mut Frame, _theme: &Theme) {
        // This draw only occurs at the completion of the widget(update occurs) and cache is cleared
        
        for draw_curve in curves.iter() {
            // if first click, skip the curve to be edited so that it 
            // will not be seen until the second click.  Otherwise is shows
            // during editing because there is no way to refresh
            // The pending routine will diplay the curve

            let (path, color, width) = 
                match &draw_curve.widget {
                    CanvasWidget::Bezier(bz) => {
                        // skip if being editied or rotated
                        if bz.draw_mode == DrawMode::Edit || 
                            bz.draw_mode == DrawMode::Rotate {
                            (None, None, None)
                        } else {
                            let (path, _, _) = 
                                build_bezier_path(
                                bz, 
                                bz.draw_mode, 
                                None, 
                                None, 
                                false,
                                None,
                            );

                            (Some(path), Some(bz.color), Some(bz.width))
                        }
                    },
                    CanvasWidget::Circle(cir) => {
                        // skip if being editied or rotated
                        if cir.draw_mode == DrawMode::Edit  || 
                            cir.draw_mode == DrawMode::Rotate {
                            (None, None, None)
                        } else {
                            (Some(build_circle_path(
                                cir, 
                                cir.draw_mode,
                                None, 
                                None, 
                                false)), 
                                Some(cir.color), Some(cir.width))
                        }
                    },
                    CanvasWidget::Line(line) => {
                        // skip if being editied or rotated
                        if line.draw_mode == DrawMode::Edit  || 
                            line.draw_mode == DrawMode::Rotate {
                            (None, None, None)
                        } else {
                            let (path, _, _) = build_line_path(
                                line, 
                                line.draw_mode, 
                                None, 
                                None, 
                                false,
                                );

                            (Some(path), Some(line.color), Some(line.width))
                        }
                    },
                    CanvasWidget::PolyLine(pl) => {
                        // skip if being editied or rotated
                        if pl.draw_mode == DrawMode::Edit  || 
                            pl.draw_mode == DrawMode::Rotate {
                            (None, None, None)
                        } else {
                            let (path, _, _) = build_polyline_path(
                                pl, 
                                pl.draw_mode, 
                                None, 
                                None, 
                                false,
                            );
                            (Some(path), Some(pl.color), Some(pl.width))
                        }
                    },
                    CanvasWidget::Polygon(pg) => {
                        // skip if being editied or rotated
                        if pg.draw_mode == DrawMode::Edit  || 
                            pg.draw_mode == DrawMode::Rotate {
                            (None, None, None)
                        } else {
                            let (path, _, _) = 
                            build_polygon_path(
                                pg, 
                                pg.draw_mode, 
                                None, 
                                None, 
                                false);
                                
                            (Some(path), Some(pg.color), Some(pg.width))
                        }
                    }
                    CanvasWidget::RightTriangle(tr) => {
                        // skip if being editied or rotated
                        if tr.draw_mode == DrawMode::Edit  || 
                            tr.draw_mode == DrawMode::Rotate {
                            (None, None, None)
                        } else {
                            let (path, _, _) = build_right_triangle_path(
                                tr, 
                                tr.draw_mode, 
                                None, 
                                None, 
                                false,
                            );
                                
                            (Some(path), Some(tr.color), Some(tr.width))
                        }
                    },
                    CanvasWidget::None => (None, None, None),
                };
                
                if let Some(path) = path { frame.stroke(
                    &path,
                    Stroke::default()
                    .with_width(width.unwrap())
                    .with_color(color.unwrap()),
                    ) }
        }

    }
}



#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Pending {
    New {
        widget: CanvasWidget, 
    },
    EditSecond {
        widget: CanvasWidget, 
        edit_curve_index: Option<usize>,
        edit_point_index: Option<usize>, 
        edit_mid_point: bool,
        },
    EditThird {
        widget: CanvasWidget, 
        edit_curve_index: Option<usize>,
        edit_point_index: Option<usize>,
        edit_mid_point: bool,
        },
    Rotate {
        widget: CanvasWidget,
        edit_curve_index: Option<usize>,
        step_degrees: f32,
        degrees: Option<f32>,
    },
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

        if let Some(cursor) = cursor.position_in(bounds) {
            // This draw happens when the mouse is moved and the state is none.
            match self {
                Pending::New { 
                    widget, 
                } => {
                    let (path, color, width, degrees, mid_point) = match widget {
                        CanvasWidget::Bezier(bz) => {
                            let (path, degrees, _) = 
                                build_bezier_path(
                                    bz, 
                                    DrawMode::New, 
                                    Some(cursor),
                                    None,
                                    false,
                                    None,
                                );
                                
                            (path, bz.color, bz.width, Some(degrees), Some(bz.points[0]))
                        },
                        CanvasWidget::Circle(cir) => {
                            let path = 
                                build_circle_path(
                                    cir, 
                                    DrawMode::New, 
                                    Some(cursor),
                                    None,
                                    false,
                                );
                            (path, cir.color, cir.width, None, None)
                        },
                        CanvasWidget::Line(line) => {
                            let (path, degrees, _) = 
                                build_line_path(
                                    line, 
                                    DrawMode::New, 
                                    Some(cursor),
                                    None,
                                    false,
                                );
                            (path, line.color, line.width, Some(degrees), Some(line.points[0]))
                        },
                        // return points as they are set
                        CanvasWidget::PolyLine(pl) => {
                            let (path, degrees, mid_point) = 
                                build_polyline_path(
                                    pl, 
                                    DrawMode::New, 
                                    Some(cursor),
                                    None,
                                    false,
                                );
                            (path, pl.color, pl.width, Some(degrees), Some(mid_point))
                        },
                        CanvasWidget::Polygon(pg) => {
                            let (path, degrees, mid_point) = 
                                build_polygon_path(
                                    pg,
                                    DrawMode::New, 
                                    Some(cursor),
                                    None,
                                    false,
                                );
                            
                            (path, pg.color, pg.width, Some(degrees), Some(mid_point))
                        },
                        CanvasWidget::RightTriangle(r_tr) => {
                            let (path, degrees, mid_point) = 
                                build_right_triangle_path(
                                    r_tr, 
                                    DrawMode::New, 
                                    Some(cursor),
                                    None,
                                    false,
                                );
                            (path, r_tr.color, r_tr.width, Some(degrees), Some(mid_point))
                        },
                        _ => (Path::new(|_| {}), Color::TRANSPARENT, 0.0, None, None)
                    };

                    if degrees.is_some() {
                        let degrees = format!("{:.prec$}", degrees.unwrap(), prec = 1);
                        let mid_point = mid_point.unwrap();
                        let position = Point::new(mid_point.x-10.0, mid_point.y-20.0);
                        frame.fill_text(canvas::Text {
                            position,
                            color: Color::WHITE,
                            size: 10.0.into(),
                            content: degrees,
                            ..canvas::Text::default()
                        });
                    }

                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(width)
                            .with_color(color),
                    );
                },
                Pending::EditSecond{
                    widget,
                    edit_curve_index:_, 
                    edit_point_index, 
                    edit_mid_point
                } | Pending::EditThird { 
                    widget,
                    edit_curve_index:_, 
                    edit_point_index, 
                    edit_mid_point,  
                } => {

                    let (path, color, width, degrees, mid_point) = match widget {
                        CanvasWidget::None => {
                            (Path::new(|_| {}), Color::TRANSPARENT, 0.0, None, Point::default())
                        },
                        CanvasWidget::Bezier(bz) => {
                            let (path, degrees, mid_point) = 
                                build_bezier_path(
                                    bz, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                    None,
                                );
                           
                            (path, bz.color, bz.width, Some(degrees), mid_point)
                        },
                        CanvasWidget::Circle(cir) => {
                            let path = 
                                build_circle_path(
                                    cir, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );
                            (path, cir.color, cir.width, None, cir.center)
                        },
                        CanvasWidget::Line(line) => {
                            let (path, degrees, mid_point) = 
                                build_line_path(
                                    line, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );
                            
                            (path, line.color, line.width, Some(degrees), mid_point)
                        },
                        CanvasWidget::PolyLine(pl) => {
                            let (path, degrees, mid_point) = 
                                build_polyline_path(
                                    pl, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );
                            (path, pl.color, pl.width, Some(degrees), mid_point)
                        },
                        CanvasWidget::Polygon(pg) => {
                            let (path, degrees, mid_point) = 
                                build_polygon_path(
                                    pg, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );
                            (path, pg.color, pg.width, Some(degrees), mid_point)
                        },
                        CanvasWidget::RightTriangle(tr) => {
                            let (path, degrees, mid_point) = 
                                build_right_triangle_path(
                                    tr, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );
                            (path, tr.color, tr.width, Some(degrees), mid_point)
                        },
                    };

                    if degrees.is_some() {
                        let degrees = format!("{:.prec$}", degrees.unwrap(), prec = 1);
                        let position = Point::new(mid_point.x-10.0, mid_point.y-20.0);
                        frame.fill_text(canvas::Text {
                            position,
                            color: Color::WHITE,
                            size: 10.0.into(),
                            content: degrees,
                            ..canvas::Text::default()
                        });
                    }

                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(width)
                            .with_color(color),
                    );
                },
                
                Pending::Rotate {
                    widget,
                    edit_curve_index: _,
                    step_degrees: _,
                    degrees, 
                } => {
                    let (path, color, width, mid_point, pending_degrees) = match widget {
                        CanvasWidget::Bezier(bz) => {
                            let (path, pending_degrees, _) = 
                                build_bezier_path(
                                    bz, 
                                    bz.draw_mode,
                                    None,
                                    None, 
                                    false,
                                    *degrees,
                                );
                            (path, bz.color, bz.width, bz.mid_point, Some(pending_degrees))
                        },
                        CanvasWidget::Circle(cir) => {
                        let path = 
                            build_circle_path(
                                cir, 
                                DrawMode::Rotate, 
                                None,
                                None,
                                false,
                            );
                            (path, cir.color, cir.width, cir.center, None)
                        },
                            CanvasWidget::Line(line) => {
                            let (path, _, _) = 
                                build_line_path(
                                    line, 
                                    DrawMode::Rotate, 
                                    None,
                                    None,
                                    false,
                                );
                            (path, line.color, line.width, line.mid_point, None)
                        },
                        CanvasWidget::PolyLine(pl) => {
                            let (path, _, _) = 
                                build_polyline_path(
                                    pl, 
                                    DrawMode::Rotate, 
                                    None,
                                    None,
                                    false,
                                );
                            (path, pl.color, pl.width, pl.mid_point, None)
                        },
                        CanvasWidget::Polygon(pg) => {
                            let (path, _, _) = 
                                build_polygon_path(
                                    pg, 
                                    DrawMode::Rotate, 
                                    None,
                                    None,
                                    false,
                                );
                            (path, pg.color, pg.width, pg.mid_point, None)
                        },
                        CanvasWidget::RightTriangle(tr) => {
                            let (path, _, _) = 
                                build_right_triangle_path(
                                    tr, 
                                    DrawMode::Rotate, 
                                    None,
                                    None,
                                    false,
                                );
                            (path, tr.color, tr.width, tr.mid_point, None)
                        },
                        CanvasWidget::None => {
                            (Path::new(|_| {}), Color::TRANSPARENT, 0.0, Point::default(), None)
                        }
                    };

                    if pending_degrees.is_some() {
                        let degrees = format!("{:.prec$}", pending_degrees.unwrap(), prec = 1);
                        let position = Point::new(mid_point.x-10.0, mid_point.y-20.0);

                        frame.fill_text(canvas::Text {
                            position,
                            color: Color::WHITE,
                            size: 10.0.into(),
                            content: degrees,
                            ..canvas::Text::default()
                        });
                    }

                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(width)
                            .with_color(color),
                    );
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
    pub degrees: f32,
    pub draw_mode: DrawMode,
}

#[derive(Debug, Clone, Default)]
pub struct Circle {
    pub center: Point,
    pub circle_point: Point,
    pub radius: f32,
    pub color: Color,
    pub width: f32,
    pub draw_mode: DrawMode,
}

#[derive(Debug, Clone, Default)]
pub struct Line {
    pub points: Vec<Point>,
    pub mid_point: Point,
    pub color: Color,
    pub width: f32,
    pub degrees: f32,
    pub draw_mode: DrawMode,
}

#[derive(Debug, Clone, Default)]
pub struct PolyLine {
    pub points: Vec<Point>,
    pub poly_points: usize,
    pub mid_point: Point,
    pub color: Color,
    pub width: f32,
    pub degrees: f32,
    pub draw_mode: DrawMode,
}

#[derive(Debug, Clone, Default)]
pub struct Polygon {
    pub points: Vec<Point>,
    pub poly_points: usize,
    pub mid_point: Point,
    pub pg_point: Point,
    pub color: Color,
    pub width: f32,
    pub degrees: f32,
    pub draw_mode: DrawMode,
}

#[derive(Debug, Clone, Default)]
pub struct RightTriangle {
    pub points: Vec<Point>,
    pub mid_point: Point,
    pub color: Color,
    pub width: f32,
    pub degrees: f32,
    pub draw_mode: DrawMode,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq,)]
pub enum Widget {
    None,
    Bezier,
    Circle,
    Line,
    PolyLine,
    Polygon,
    RightTriangle,
}

fn get_distance_to_mid_point(draw_curve: &DrawCurve, cursor: Point) -> f32 {

        match &draw_curve.widget {
            CanvasWidget::None => 1_000_000_f32,
            CanvasWidget::Bezier(bz) => {
                cursor.distance(bz.mid_point)
            },
            CanvasWidget::Circle(cir) => {
                cursor.distance(cir.center)
            },
            CanvasWidget::Line(line) => {
                cursor.distance(line.mid_point)
            },
            CanvasWidget::PolyLine(pl) => {
                cursor.distance(pl.mid_point)
            },
            CanvasWidget::Polygon(pg) => {
                cursor.distance(pg.mid_point)
            },
            CanvasWidget::RightTriangle(tr) => {
                cursor.distance(tr.mid_point)
            },
        }

}

fn update_widget(widget: CanvasWidget, 
                poly_points: usize, 
                selected_color: Color,
                selected_width: f32,
                draw_mode: DrawMode) 
                -> CanvasWidget {
    match widget {
        CanvasWidget::None => {
            CanvasWidget::None
        },
        CanvasWidget::Bezier(mut bz) => {
            bz.color = selected_color;
            bz.width = selected_width;
            bz.draw_mode = draw_mode;
            CanvasWidget::Bezier(bz)
        },
        CanvasWidget::Circle(mut cir) => {
            cir.color = selected_color;
            cir.width = selected_width;
            cir.draw_mode = draw_mode;
            CanvasWidget::Circle(cir)
        },
        CanvasWidget::Line(mut ln) => {
            ln.color = selected_color;
            ln.width = selected_width;
            ln.draw_mode = draw_mode;
            CanvasWidget::Line(ln)
        },
        CanvasWidget::PolyLine(mut pl) => {
            pl.color = selected_color;
            pl.width = selected_width;
            pl.poly_points = poly_points;
            pl.draw_mode = draw_mode;
            CanvasWidget::PolyLine(pl)
        },
        CanvasWidget::Polygon(mut pg) => {
            pg.color = selected_color;
            pg.width = selected_width;
            pg.poly_points = poly_points;
            pg.draw_mode = draw_mode;
            CanvasWidget::Polygon(pg)
        },
        CanvasWidget::RightTriangle(mut tr) => {
            tr.color = selected_color;
            tr.width = selected_width;
            tr.draw_mode = draw_mode;
            CanvasWidget::RightTriangle(tr)
        },
    }
}

fn update_rotated_widget(widget: &mut CanvasWidget,
                        degrees: f32, 
                        step_degrees: f32,
                        draw_mode: DrawMode,
                    ) -> CanvasWidget {
    match widget {
        CanvasWidget::None => CanvasWidget::None,
        CanvasWidget::Bezier(bz) => {
            bz.draw_mode = draw_mode;
            bz.degrees = degrees;
            bz.points = rotate_geometry(&bz.points, &bz.mid_point, &step_degrees, Widget::Bezier);
            CanvasWidget::Bezier(bz.clone())
        },
        CanvasWidget::Circle(cir) => {
            cir.draw_mode = DrawMode::DrawAll;
            CanvasWidget::Circle(cir.clone())
        },
        CanvasWidget::Line(ln) => {
            ln.draw_mode = DrawMode::DrawAll;
            ln.degrees = degrees;
            CanvasWidget::Line(ln.clone())
        },
        CanvasWidget::PolyLine(pl) => {
            pl.draw_mode = DrawMode::DrawAll;
            pl.degrees = degrees;
            CanvasWidget::PolyLine(pl.clone())
        },
        CanvasWidget::Polygon(pg) => {
            pg.draw_mode = DrawMode::DrawAll;
            pg.degrees = degrees;
            CanvasWidget::Polygon(pg.clone())
        },
        CanvasWidget::RightTriangle(tr) => {
            tr.draw_mode = DrawMode::DrawAll;
            tr.degrees = degrees;
            CanvasWidget::RightTriangle(tr.clone())
        },
    }
}

fn set_widget_draw_mode(widget: CanvasWidget, 
                    draw_mode: DrawMode,
                    ) -> CanvasWidget {
    match widget {
        CanvasWidget::None => {
            CanvasWidget::None
        },
        CanvasWidget::Bezier(mut bz) => {
            bz.draw_mode = draw_mode;
            CanvasWidget::Bezier(bz)
        },
        CanvasWidget::Circle(mut cir) => {
            cir.draw_mode = draw_mode;
            CanvasWidget::Circle(cir)
        },
        CanvasWidget::Line(mut ln) => {
            ln.draw_mode = draw_mode;
            CanvasWidget::Line(ln)
        },
        CanvasWidget::PolyLine(mut pl) => {
            pl.draw_mode = draw_mode;
            CanvasWidget::PolyLine(pl)
        },
        CanvasWidget::Polygon(mut pg) => {
            pg.draw_mode = draw_mode;
            CanvasWidget::Polygon(pg)
        },
        CanvasWidget::RightTriangle(mut tr) => {
            tr.draw_mode = draw_mode;
            CanvasWidget::RightTriangle(tr)
        },
    }

}

// Adds a cursor position to the points then determines 
// if finish by returning the widget and the boolean
fn set_widget_point(widget: &CanvasWidget, cursor: Point) -> (CanvasWidget, bool) {
    match widget {
        CanvasWidget::None => (CanvasWidget::None, true),
        CanvasWidget::Bezier(bezier) => {

            let mut bz = bezier.clone();
            bz.points.push(cursor);

            if bz.points.len() == 2 {
                bz.degrees = get_angle_of_vectors(bz.points[0], bz.points[1]);
            }
            let finished = if bz.points.len() == 3 {
                // degrees won't change with this last point
                bz.draw_mode = DrawMode::DrawAll;
                true
            } else {
                false
            };
            
            (CanvasWidget::Bezier(bz), finished)
        },
        CanvasWidget::Circle(circle) => {
            let mut cir = circle.clone();
            let finished = if cir.center == Point::default() {
                cir.center = cursor;
                false
            } else {
                cir.radius = cir.center.distance(cursor);
                cir.circle_point = cursor;
                true
            };
            if finished {
                cir.draw_mode = DrawMode::DrawAll;
            }
            (CanvasWidget::Circle(cir), finished)
        },
        CanvasWidget::Line(line) => {
            let mut ln = line.clone();
            ln.points.push(cursor);

            let finished = if ln.points.len() == 2 {
                ln.mid_point = get_mid_point(ln.points[0], ln.points[1]);
                true
            } else {
                false
            };
            if finished {
                ln.draw_mode = DrawMode::DrawAll;
            }
            (CanvasWidget::Line(ln), finished)
        },
        CanvasWidget::PolyLine(poly_line) => {
            let mut pl = poly_line.clone();
            pl.points.push(cursor);
            let finished = if pl.points.len() == pl.poly_points {
                pl.mid_point = get_mid_geometry(&pl.points, Widget::PolyLine);
                true
            } else {
                false
            };
            if finished {
                pl.draw_mode = DrawMode::DrawAll;
            }
            (CanvasWidget::PolyLine(pl), finished)
        },
        CanvasWidget::Polygon(polygon) => {
            let mut pg = polygon.clone();
            let finished = if pg.mid_point == Point::default() {
                pg.mid_point = cursor;
                false
            } else {
                pg.pg_point = cursor;
                true
            };
            if finished {
                pg.draw_mode = DrawMode::DrawAll;
                pg.degrees = get_angle_of_vectors(pg.mid_point, pg.pg_point)
            }
            (CanvasWidget::Polygon(pg), finished)
        },
        CanvasWidget::RightTriangle(right_triangle) => {
            let mut rt = right_triangle.clone();
            rt.points.push(cursor);
            if rt.points.len() > 1 {
            rt.points[1].x = rt.points[0].x;
            }
            if rt.points.len() > 2 {
                rt.points[2].y = rt.points[1].y;
            }
            let finished = if rt.points.len() == 3 {
                // close the triangle
                rt.points.push(right_triangle.points[0]);
                rt.mid_point = get_mid_geometry(&rt.points, Widget::RightTriangle);
                true
            } else {
                false
            };
            if finished {
                rt.draw_mode = DrawMode::DrawAll;
            }
            (CanvasWidget::RightTriangle(rt), finished)
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
                bz.mid_point = get_mid_point(bz.points[0], bz.points[1]);
            } else if mid_point {
                bz.points = 
                    translate_geometry(
                        bz.points, 
                        cursor,
                        bz.mid_point, 
                        );
                bz.mid_point = cursor;
            }
            let degrees = 
                get_angle_of_vectors(
                    bz.points[0],
                    bz.points[1], 
                );
            bz.degrees = degrees;

            CanvasWidget::Bezier(bz)
        },
        CanvasWidget::Circle(mut cir) => {
            if index.is_some() {
                cir.circle_point = cursor;
                cir.radius = cir.center.distance(cursor);
            } else if mid_point {
                let mut points = vec![cir.circle_point];
                points = 
                    translate_geometry(
                        points, 
                        cursor,
                        cir.center,
                    );
                cir.center = cursor;
                cir.circle_point = points[0];
            }

            CanvasWidget::Circle(cir)
        },
        CanvasWidget::Line(mut line) => {
            if index.is_some() {
                line.points[index.unwrap()] = cursor;
                line.mid_point = get_mid_point(line.points[0], line.points[1]);
            } else if mid_point {
                line.points = 
                    translate_geometry(
                        line.points.clone(), 
                        cursor,
                        line.mid_point, 
                        );
                line.mid_point = cursor;
            }

            let degrees = 
                get_angle_of_vectors(
                    line.points[0],  
                    line.points[1], 
                );
            line.degrees = degrees;

            CanvasWidget::Line(line)
        },
        CanvasWidget::PolyLine(mut pl) => {
            if index.is_some() {
                pl.points[index.unwrap()] = cursor;
                pl.mid_point = get_mid_geometry(&pl.points, Widget::PolyLine);
            } else if mid_point {
                pl.points = 
                    translate_geometry(
                        pl.points.clone(), 
                        cursor,
                        pl.mid_point, 
                        );
                pl.mid_point = cursor;
            }

            CanvasWidget::PolyLine(pl)
        },
        CanvasWidget::Polygon(mut pg) => {
            if index.is_some() {
                pg.pg_point = cursor;
                pg.points = 
                    build_polygon(
                        pg.mid_point, 
                        pg.pg_point, 
                        pg.poly_points,
                        pg.degrees,
                );
            } else if mid_point {
                let mut pts = vec![pg.pg_point];
                pts = 
                    translate_geometry(
                        pts, 
                        cursor,
                        pg.mid_point, 
                    );
                pg.points = 
                    build_polygon(
                        cursor, 
                        pts[0], 
                        pg.poly_points,
                        pg.degrees,
                    );
                pg.mid_point = cursor;
                pg.pg_point = pts[0];
            }

            CanvasWidget::Polygon(pg)
        },
        CanvasWidget::RightTriangle(mut tr) => {
            if index.is_some() {
                let index = index.unwrap();
                if index == 0 {
                    tr.points[index].y = cursor.y;
                }
                if index == 1 {
                    tr.points[1].y = cursor.y;
                    tr.points[2].y = cursor.y;
                }
                if index == 2 {
                    tr.points[2].x = cursor.x;
                }
                tr.mid_point = get_mid_geometry(&tr.points, Widget::RightTriangle);
            } else if mid_point {
                tr.points = 
                    translate_geometry(
                        tr.points.clone(), 
                        cursor,
                        tr.mid_point, 
                    );
                tr.mid_point = cursor;
            }

            CanvasWidget::RightTriangle(tr)
        },
    }
}

fn find_closest_point_index(widget: &CanvasWidget,
                            cursor: Point, 
                            ) 
                            -> (Option<usize>, bool) {

    let mut point_dist: f32 = 1_000_000.0;
    let mut point_index = 0;

    match widget {
        CanvasWidget::None => (None, false),
        CanvasWidget::Bezier(bezier) => {
            for (idx, point) in bezier.points.iter().enumerate() {
                let dist = cursor.distance(*point);
                if  dist < point_dist {
                    point_index = idx;
                    point_dist = dist;
                }
            };
            
            let mid_dist = bezier.mid_point.distance(cursor);

            if mid_dist < point_dist {
                (None, true)
            } else {
                (Some(point_index), false)
            }
        },
        CanvasWidget::Circle(cir) => {
            let center_dist = cursor.distance(cir.center);
            let point_dist = cursor.distance(cir.circle_point);
            if center_dist < point_dist {
                (None, true)
            } else {
                (Some(1), false)
            }
        } 
        CanvasWidget::Line(line) => {
            for (idx, point) in line.points.iter().enumerate() {
                let dist = cursor.distance(*point);
                if  dist < point_dist {
                    point_index = idx;
                    point_dist = dist;
                }
            };
            
            let mid_dist = cursor.distance(line.mid_point);

            if mid_dist < point_dist {
                (None, true)
            } else {
                (Some(point_index), false)
            }
        },
        CanvasWidget::PolyLine(poly_line) => {
            for (idx, point) in poly_line.points.iter().enumerate() {
                let dist = cursor.distance(*point);
                if  dist < point_dist {
                    point_index = idx;
                    point_dist = dist;
                }
            };
            
            let mid_dist = poly_line.mid_point.distance(cursor);
            if mid_dist < point_dist {
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
        CanvasWidget::RightTriangle(tr) => {
            for (idx, point) in tr.points.iter().enumerate() {
                let dist = cursor.distance(*point);
                if  dist < point_dist {
                    point_index = idx;
                    point_dist = dist;
                }
            };
            
            let mid_dist = tr.mid_point.distance(cursor);
            if mid_dist < point_dist {
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

fn get_linear_regression(points: &[Point]) -> (f32, f32) {
    let mut sx: f64 = 0.0;
    let mut sy: f64 = 0.0;
    let mut sxx: f64 = 0.0;
    let mut sxy: f64 = 0.0;

    for point in points.iter() {
        sx += point.x as f64;
        sy += point.y as f64;
        sxx += point.x as f64 * point.x as f64;
        sxy += point.x as f64 * point.y as f64;
    }

    let n = points.len() as f64;
    let beta = (n*sxy-sx*sy) / (n*sxx - sx*sx);
    let alpha = (1.0/n * sy) - (beta*1.0/n*sx);

    (beta as f32, alpha as f32)

}

fn get_widget_degrees(widget: &CanvasWidget) -> Option<f32> {
    match widget {
        CanvasWidget::None => Some(0.0),
        CanvasWidget::Bezier(bezier) => Some(bezier.degrees),
        CanvasWidget::Circle(_circle) => Some(0.0),
        CanvasWidget::Line(line) => Some(line.degrees),
        CanvasWidget::PolyLine(poly_line) => Some(poly_line.degrees),
        CanvasWidget::Polygon(polygon) => Some(polygon.degrees),
        CanvasWidget::RightTriangle(right_triangle) => Some(right_triangle.degrees),
    }
}

fn get_widget_points(widget: &CanvasWidget) -> (Vec<Point>, Point, Widget) {
    match widget {
        CanvasWidget::None => (vec![], Point::default(), Widget::None),
        CanvasWidget::Bezier(bz) => (bz.points.clone(), bz.mid_point, Widget::Bezier),
        CanvasWidget::Circle(_) => (vec![], Point::default(), Widget::Circle),
        CanvasWidget::Line(ln) => (ln.points.clone(), ln.mid_point, Widget::Line),
        CanvasWidget::PolyLine(pl) => (pl.points.clone(), pl.mid_point, Widget::PolyLine),
        CanvasWidget::Polygon(pg) => (pg.points.clone(), pg.mid_point, Widget::Polygon),
        CanvasWidget::RightTriangle(tr) => (tr.points.clone(), tr.mid_point, Widget::RightTriangle),
    }
}

pub fn get_mid_geometry(pts: &[Point], curve_type: Widget) -> Point {

    match curve_type {
        Widget::Bezier => {
            get_mid_point(pts[0], pts[1])
        },
        Widget::Circle => {
            // return the center point
            pts[0]
        },
        Widget::Line => {
            get_mid_point(pts[0], pts[1])
        },
        Widget::PolyLine => {

            let (slope, intercept) = get_linear_regression(pts);

            let (p1, p2) = get_line_from_slope_intercept(pts, slope, intercept);

            get_mid_point(p1, p2)

        },
        Widget::Polygon => {
            // return the center point
            pts[0]
        },
        Widget::RightTriangle => {
            let x = (pts[0].x + pts[1].x + pts[2].x)/3.0;
            let y = (pts[0].y + pts[1].y + pts[2].y)/3.0;
            Point {x, y}
        },
        Widget::None => Point::default(),
    }
    
}

fn get_line_from_slope_intercept(points: &[Point], 
                                slope: f32, 
                                intercept: f32,
                                ) -> (Point, Point) {

    let mut small_x = 1_000_000_f32;
    let mut large_x = 0.0;
    let mut small_y = 1_000_000_f32;
    let mut large_y = 0.0;

    for point in points.iter() {
        if point.x < small_x {
            small_x = point.x;
        }
        if point.x > large_x {
            large_x = point.x;
        }
        if point.y < small_y {
            small_y = point.y;
        }
        if point.y > large_y {
            large_y = point.y;
        }
    }
 
    let ys = slope*small_x + intercept;
    let yl = slope*large_x + intercept; 
    
    (Point{x: small_x, y: ys}, Point{x: large_x, y: yl})  

}

fn translate_geometry(pts: Vec<Point>, 
                        new_center: Point,
                        old_center: Point, 
                        ) 
                        -> Vec<Point> {
    let mut new_pts = vec![];
    let dist_x = new_center.x - old_center.x;
    let dist_y = new_center.y - old_center.y;
    for pt in pts.iter() {
        new_pts.push(Point{x: pt.x + dist_x, y: pt.y + dist_y})
    }

    new_pts
}

// The degrees are adjusted based on how degrees where calulated for each widget.
fn rotate_geometry(points: &[Point], mid_point: &Point, degrees: &f32, widget: Widget) -> Vec<Point> {
    match widget {
        Widget::None => vec![],
        Widget::Bezier => {
            rotate_widget(points, mid_point, &degrees)
        },
        Widget::Circle => {
            rotate_widget(points, mid_point, degrees)
        },
        Widget::Line => {
            rotate_widget(points, mid_point, degrees)
        },
        Widget::PolyLine => {
            rotate_widget(points, mid_point, degrees)
        },
        Widget::Polygon => {
            rotate_widget(points, mid_point, degrees)
        },
        Widget::RightTriangle => {
            rotate_widget(points, mid_point, degrees)
        },
    }
}

// Rotates a widget by the given angle by an additive method, not to an absolute angle.
fn rotate_widget(points: &[Point], center: &Point, degrees: &f32) -> Vec<Point> {
    let theta = to_radians(degrees);
    let mut new_points = vec![];
    for point in points.iter() {
        let x_new = (point.x - center.x) * theta.cos() - (point.y - center.y) * theta.sin() + center.x;
        let y_new = (point.x - center.x) * theta.sin() + (point.y - center.y) * theta.cos() + center.y;

        new_points.push(Point { x: x_new, y: y_new })
    }
    
    new_points
}

// The first point is used to create a vertical vector and is used as the center
fn get_angle_of_vectors(center: Point, p2: Point) -> f32 {
    let p1 = Point::new(center.x, 10.0);
    let pts = translate_geometry(vec![p1, p2], Point::default(), center);

    let mut angle = (pts[0].y).atan2(pts[0].x) -
                        (pts[1].y).atan2(pts[1].x);
    angle += PI;
    // Since beyond pi, angle goes negative
    let new_angle = if angle < 0.0 {
        2.0 * PI + angle
    } else {
        angle
    };

    to_degrees(&new_angle)
}

fn to_degrees(radians: &f32) -> f32 {
    radians * 180.0/PI
}

fn to_radians(degrees: &f32) -> f32 {
    degrees * PI/180.0
}

fn build_polygon(mid_point: Point, point: Point, poly_points: usize, degrees: f32) -> Vec<Point> {
    
    let angle = 2.0 * PI / poly_points as f32;
    let radius = mid_point.distance(point);
    let mut points = vec![];
    for i in 0..poly_points {
        let x = mid_point.x + radius * (i as f32 * angle).sin();
        let y = mid_point.y + radius * (i as f32 * angle).cos();
        points.push(Point::new(x, y));
    }
    
    let mut pts = rotate_geometry(&points, &mid_point, &degrees, Widget::Polygon);
    pts.push(pts[0]);
    pts

}

fn build_bezier_path(bz: &Bezier, 
                    draw_mode: DrawMode, 
                    pending_cursor: Option<Point>,
                    edit_point_index: Option<usize>, 
                    edit_mid_point: bool,
                    degrees: Option<f32>,
                    ) -> (Path, f32, Point) {

    let mut degrees = match degrees {
        Some(d) => d,
        None => bz.degrees,
    };
    
    let mut mid_point = bz.mid_point;
    let path = Path::new(|p| {

        match draw_mode {
            DrawMode::DrawAll => {
                p.move_to(bz.points[0]);
                p.quadratic_curve_to(bz.points[2], bz.points[1]);
            },
            DrawMode::Edit => {
                let mut pts = bz.points.clone();

                if edit_mid_point {
                    pts = translate_geometry(
                        pts.clone(), 
                        pending_cursor.unwrap(),
                        mid_point, 
                        );
                    mid_point = pending_cursor.unwrap();
                } 
                if edit_point_index.is_some() {
                    pts[edit_point_index.unwrap()] = pending_cursor.unwrap();
                    mid_point = get_mid_point(pts[0], pts[1]);
                    
                    degrees = 
                        get_angle_of_vectors(
                            pts[0], 
                            pts[1], 
                        );
                }

                p.move_to(pts[0]);
                p.quadratic_curve_to(pts[2], pts[1]);
                
                for pt in pts {
                    p.circle(pt, 3.0);
                }

                p.circle(mid_point, 3.0);
            },
            DrawMode::New => {
                if bz.points.len() == 1 {
                    mid_point = 
                        get_mid_point(
                            bz.points[0], 
                            pending_cursor.unwrap()
                        );
                    degrees = 
                        get_angle_of_vectors(
                            bz.points[0],  
                            pending_cursor.unwrap(),
                        );
                    p.move_to(bz.points[0]);
                    p.line_to(pending_cursor.unwrap());
                }
                if bz.points.len() == 2 {
                    p.move_to(bz.points[0]);
                    p.quadratic_curve_to(pending_cursor.unwrap(), bz.points[1]);
                }
            },
            DrawMode::Rotate => {
                p.move_to(bz.points[0]);
                p.quadratic_curve_to(bz.points[2], bz.points[1]);
                p.move_to(bz.points[0]);
                p.line_to(bz.points[1]);
                p.circle(bz.mid_point, 3.0);
            },
        }
    });

    (path, degrees, mid_point)

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
                let mut center = cir.center;
                let mut cir_point = cir.circle_point;
                let mut radius = cir.radius;

                if edit_mid_point {
                    cir_point = translate_geometry(
                        vec![cir_point], 
                        pending_cursor.unwrap(),
                        center,
                    )[0];
                    center = pending_cursor.unwrap();
                }

                if edit_point_index.is_some() {
                    cir_point = pending_cursor.unwrap();
                    radius = center.distance(cir_point);
                }

                p.circle(center, radius);
                p.circle(center, 3.0);
                p.circle(cir_point, 3.0);
            },
            DrawMode::New => {
                let circle_point = pending_cursor.unwrap();
                let radius = cir.center.distance(circle_point);
                p.circle(cir.center, radius);
            },
            DrawMode::Rotate => {
                p.circle(cir.center, cir.radius);
            },
        }
    })
}

fn build_line_path(line: &Line, 
                    draw_mode: DrawMode, 
                    pending_cursor: Option<Point>,
                    edit_point_index: Option<usize>, 
                    edit_mid_point: bool,
                ) -> (Path, f32, Point) {

    let mut degrees = 0.0;
    let mut mid_point = line.mid_point;

    let path = Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                p.move_to(line.points[0]);
                p.line_to(line.points[1]);
            },
            DrawMode::Edit => {
                let mut pts = line.points.clone();

                if edit_mid_point {
                    pts = translate_geometry(
                        pts, 
                        pending_cursor.unwrap(),
                        mid_point,
                    );
                    mid_point = pending_cursor.unwrap();
                };

                if edit_point_index.is_some() {
                    pts[edit_point_index.unwrap()] = pending_cursor.unwrap();
                    mid_point = get_mid_point(pts[0], pts[1])
                }

                degrees = 
                    get_angle_of_vectors(
                        pts[0],  
                        pts[1], 
                    );

                p.move_to(pts[0]);
                p.line_to(pts[1]);
                p.circle(pts[0], 3.0);
                p.circle(pts[1], 3.0);
                p.circle(mid_point, 3.0);
            },
            DrawMode::New => {
                p.move_to(line.points[0]);
                p.line_to(pending_cursor.unwrap());

                degrees = 
                    get_angle_of_vectors(
                        line.points[0], 
                        pending_cursor.unwrap(), 
                    );
            },
            DrawMode::Rotate => {
                p.move_to(line.points[0]);
                p.line_to(line.points[1]);

                // rotates around the center
                degrees = 
                    get_angle_of_vectors(
                        line.mid_point,  
                        line.points[1], 
                    );
            },
        }
    });

    (path, degrees, mid_point)

}

fn build_polyline_path(pl: &PolyLine, 
                        draw_mode: DrawMode, 
                        pending_cursor: Option<Point>,
                        edit_point_index: Option<usize>, 
                        edit_mid_point: bool,
                    ) -> (Path, f32, Point) {

    let degrees = pl.degrees;
    let mut mid_point = pl.mid_point;

    let path = Path::new(|p| {
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
                let mut mid_point = pl.mid_point;
                let mut pts = pl.points.clone();

                if edit_mid_point {
                    pts = translate_geometry(
                        pts, 
                        pending_cursor.unwrap(),
                        mid_point, 
                    );
                    mid_point = pending_cursor.unwrap();
                } 
                if edit_point_index.is_some() {
                    pts[edit_point_index.unwrap()] = pending_cursor.unwrap();
                    mid_point = get_mid_geometry(&pts, Widget::PolyLine);
                }

                for (index, point) in pts.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                }
                for pt in pts.iter() {
                    p.circle(*pt, 3.0);
                }
                p.circle(mid_point, 3.0);
            },
            DrawMode::New => {
                for (index, point) in pl.points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                }
                p.line_to(pending_cursor.unwrap());
            },
            DrawMode::Rotate => {
                for (index, point) in pl.points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                }
                let (slope, intercept) = get_linear_regression(&pl.points);
                let(p1, p2) = get_line_from_slope_intercept(&pl.points, slope, intercept);
                mid_point = get_mid_point(p1, p2);
                p.move_to(p1);
                p.line_to(p2);
                p.circle(mid_point, 3.0);
            },
        }
    });

    (path, degrees, mid_point)

}

fn build_polygon_path(pg: &Polygon, 
                        draw_mode: DrawMode, 
                        pending_cursor: Option<Point>,
                        edit_point_index: Option<usize>, 
                        edit_mid_point: bool,
                    ) -> (Path, f32, Point) {

    let mut degrees = 0.0;
    let mut mid_point = pg.mid_point;

    let path = Path::new(|p| {
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
                let mut pg_point = pg.pg_point;

                if edit_mid_point {
                    pg_point = translate_geometry(
                        vec![pg.pg_point], 
                        pending_cursor.unwrap(),
                        pg.mid_point, 
                    )[0];
                    mid_point = pending_cursor.unwrap();
                } 
                if edit_point_index.is_some() {
                    pg_point = pending_cursor.unwrap();
                }

                let pts = 
                    build_polygon(
                        mid_point, 
                        pg_point, 
                        pg.poly_points,
                        pg.degrees
                    );

                for (index, pt) in pts.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*pt);
                    } else {
                        p.line_to(*pt);
                    }
                }
                p.line_to(pts[0]);
                p.circle(mid_point, 3.0);
                p.circle(pg_point, 3.0);
            },
            DrawMode::New => {
                degrees = get_angle_of_vectors(pg.mid_point, pending_cursor.unwrap());

                let points = 
                    build_polygon(
                        pg.mid_point, 
                        pending_cursor.unwrap(), 
                        pg.poly_points,
                        degrees,
                    );
                for (index, point) in points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                }
                p.move_to(pg.mid_point);
                p.line_to(pending_cursor.unwrap());
            },
            DrawMode::Rotate => {
                for (index, point) in pg.points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                }
                p.move_to(pg.mid_point);
                p.line_to(pg.points[2]);
                p.circle(pg.mid_point, 3.0);
            },
        }
    });

    (path, degrees, mid_point)

}

fn build_right_triangle_path(tr: &RightTriangle, 
                            draw_mode: DrawMode, 
                            pending_cursor: Option<Point>,
                            edit_point_index: Option<usize>, 
                            edit_mid_point: bool,
                        ) -> (Path, f32, Point) {

    let mut mid_point = tr.mid_point;
    let degrees = tr.degrees;

    let path = Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                p.move_to(tr.points[0]);
                p.line_to(tr.points[1]);
                p.line_to(tr.points[2]);
                p.line_to(tr.points[0]);
            },
            DrawMode::Edit => {
                let mut pts = tr.points.clone();

                if edit_mid_point {
                    pts = translate_geometry(
                        tr.points.clone(), 
                        pending_cursor.unwrap(),
                        mid_point, 
                    );
                    mid_point = pending_cursor.unwrap();
                } 
                if edit_point_index.is_some() {
                    let index = edit_point_index.unwrap();
                    let cursor = pending_cursor.unwrap();
                    if index == 0 {
                        pts[0].y = cursor.y
                    }
                    if index == 1 {
                        pts[1].y = cursor.y;
                        pts[2].y = cursor.y;
                    }
                    if index == 2 {
                        pts[2].x = cursor.x;
                }
                    mid_point = get_mid_geometry(&pts, Widget::RightTriangle)
                }

                p.move_to(pts[0]);
                p.line_to(pts[1]);
                p.line_to(pts[2]);
                p.line_to(pts[0]);

                p.circle(pts[0], 2.0);
                p.circle(pts[1], 2.0);
                p.circle(pts[2], 2.0);
                p.circle(mid_point, 3.0);
            },
            DrawMode::New => {
                let mut cursor = pending_cursor.unwrap();
                p.move_to(tr.points[0]);
                if tr.points.len() == 1 {
                    cursor.x = tr.points[0].x;
                    p.line_to(cursor);
                } else if tr.points.len() == 2 {
                    cursor.y = tr.points[1].y;
                    p.line_to(tr.points[1]);
                    p.line_to(cursor);
                }
            },
            DrawMode::Rotate => {
                p.move_to(tr.points[0]);
                p.line_to(tr.points[1]);
                p.line_to(tr.points[2]);
                p.line_to(tr.points[0]);
            },
        }
    });

    (path, degrees, mid_point)

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



#[test]
fn test_get_linear_regression() {
    let points: Vec<Point>= 
    vec![
    Point::new(1.47, 52.21),
    Point::new(1.50, 53.12),
    Point::new(1.52, 54.48),
    Point::new(1.55, 55.84),
    Point::new(1.57, 57.20),
    Point::new(1.60, 58.57),
    Point::new(1.63, 59.93),
    Point::new(1.65, 61.29),
    Point::new(1.68, 63.11),
    Point::new(1.70, 64.47),
    Point::new(1.73, 66.28),
    Point::new(1.75, 68.10),
    Point::new(1.78, 69.92),
    Point::new(1.80, 72.19),
    Point::new(1.83, 74.46),
    ];

    let (slope, intercept) = get_linear_regression(&points);

    assert_eq!(61.27219, slope);
    assert_eq!(-39.06196, intercept);

}

#[test]
fn test_get_line_from_slope_intercept() {
    let points = vec![Point::new(0.0, 100.0), Point::new(30.0, 30.0), Point::new(25.0, 50.0)];
    let (slope, intercept) = get_linear_regression(&points);
    let line_points = get_line_from_slope_intercept(&points, slope, intercept);
    println!("{:?} {:?}, {:?}",slope, intercept, line_points );
}

#[test]
fn test_get_angle() {
    //  all 4 quadrants
    let center = Point::new(0.0, 0.0);
    let p2 = Point::new(0.0, 10.0);
    let degrees = get_angle_of_vectors(center, p2);
    dbg!(degrees);

    let p2 = Point::new(20.0, 10.0);
    let degrees = get_angle_of_vectors(center, p2);
    dbg!(degrees);

    let p2 = Point::new(0.0, -10.0);
    let degrees = get_angle_of_vectors(center, p2);
    dbg!(degrees);

    let p2 = Point::new(-20.0, 0.0);
    let degrees = get_angle_of_vectors(center, p2);
    dbg!(degrees);
}

#[test]
fn test_rotate_geometry() {
    let mut points= vec![Point::new(0.0, 0.0), Point::new(0.0, 20.0)];
    let mid_point = Point::new(0.0, 0.0);
    let degrees = &6.0;
    for _ in 0..2 {
        points = rotate_geometry(&points.clone(), &mid_point, degrees, Widget::Line);
        dbg!(&points);
    }
    
}
