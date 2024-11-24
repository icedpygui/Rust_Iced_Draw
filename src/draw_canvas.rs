
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
                selected_poly_points: 4,
                selected_poly_points_str: "".to_string(),
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
                                                update_draw_mode(
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
                                        let edit_curve_index = edit_curve_index.clone();
                                        let mut new_widget: CanvasWidget = 
                                                edit_widget_points(
                                                    widget.clone(), 
                                                    cursor_position, 
                                                    edit_point_index.clone(), 
                                                    edit_mid_point.clone()
                                                );
                                        new_widget = 
                                            update_draw_mode(
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
                                                2.0,
                                                self.state.draw_mode,
                                            );
                                        let (widget, _) = set_widget_point(&selected_widget, cursor_position);
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
                                                        edit_curve_index: None,
                                                    })
                                                },
                                                CanvasWidget::Circle(circle) => { 
                                                    Some(DrawCurve {
                                                        widget: CanvasWidget::Circle(circle),
                                                        edit_curve_index: None,
                                                    })
                                                },
                                                CanvasWidget::Line(line) => {
                                                    Some(DrawCurve {
                                                        widget: CanvasWidget::Line(line),
                                                        edit_curve_index: None,
                                                    })
                                                },
                                                CanvasWidget::PolyLine(pl) => {
                                                    Some(DrawCurve{
                                                        widget: CanvasWidget::PolyLine(pl),
                                                        edit_curve_index: None,
                                                    })
                                                },
                                                CanvasWidget::Polygon(mut pg) => {
                                                    pg.points = 
                                                        build_polygon(
                                                            pg.mid_point, 
                                                            pg.pg_point, 
                                                            pg.poly_points,
                                                            pg.degrees
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
                                            // closest widget is found
                                            // update the widget in case the 
                                            // edit dropdown was selected first
                                            // then other changes selected
                                            let selected_widget = 
                                                update_widget(
                                                    self.curves[index.unwrap()].widget.clone(), 
                                                    self.state.selected_poly_points, 
                                                    self.state.selected_color,
                                                    2.0,
                                                    self.state.draw_mode
                                                );

                                            *program_state = Some(Pending::Rotate {
                                                widget: selected_widget,
                                                edit_curve_index: index,
                                                step_count: 0.0,
                                                degrees: 0.0,
                                            });
                                            // returning DrawCurve so that the curve
                                            // being editied will not show after the refresh
                                            // The pending process will show the curve
                                            // until its finsihed.

                                            // update the curve to indicate that
                                            // it is in rotation mode.
                                            let widget = self.curves[index.unwrap()].widget.clone();
                                            
                                            let new_widget = 
                                                update_draw_mode(
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
                                    Some(Pending::Rotate {
                                        widget,
                                        edit_curve_index,
                                        step_count:_,
                                        degrees,
                                    }) => {

                                        let rotated_widget = 
                                            update_rotated_widget(
                                                widget,
                                                degrees,
                                            );

                                        let edit_curve_index = edit_curve_index.clone();
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
                    mouse::Event::WheelScrolled { delta } => {
                        match self.state.draw_mode {
                            DrawMode::Rotate => {
                                match program_state {
                                    None => None,
                                    
                                    Some(Pending::Rotate { 
                                        widget,
                                        edit_curve_index,
                                        step_count,
                                        degrees:_,   
                                    }) => {

                                        let delta = match delta {
                                            mouse::ScrollDelta::Lines { x:_, y } => y,
                                            mouse::ScrollDelta::Pixels { x:_, y } => y,
                                        };
                                        let step = 360.0/60.0;
                                        let step_count = *step_count + delta;
                                        let theta: f32 = PI/180.0 * step * delta;
                                        let mut degrees = step_count * step;
                                        
                                        degrees = degrees % 360.0;

                                        degrees = if degrees <= 0.0 {
                                            let mut degs = 360.0 + degrees;
                                            if degs == 360.0 {
                                                degs = 0.0;
                                            }
                                            degs
                                        } else {
                                            degrees
                                        };

                                        let widget = 
                                            rotate_geometry(widget, theta);
                                        
                                        *program_state = Some(Pending::Rotate{
                                            widget,
                                            edit_curve_index: *edit_curve_index,
                                            step_count,
                                            degrees,
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
                            (Some(build_bezier_path(
                            bz, 
                            bz.draw_mode, 
                            None, 
                            None, 
                            false)), 
                            Some(bz.color), Some(bz.width))
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
                            (Some(build_line_path(
                                line, 
                                line.draw_mode, 
                                None, 
                                None, 
                                false)), 
                                Some(line.color), Some(line.width))
                        }
                    },
                    CanvasWidget::PolyLine(pl) => {
                        // skip if being editied or rotated
                        if pl.draw_mode == DrawMode::Edit  || 
                            pl.draw_mode == DrawMode::Rotate {
                            (None, None, None)
                        } else {
                            (Some(build_polyline_path(
                                pl, 
                                pl.draw_mode, 
                                None, 
                                None, 
                                false)), 
                                Some(pl.color), Some(pl.width))
                        }
                    },
                    CanvasWidget::Polygon(pg) => {
                        // skip if being editied or rotated
                        if pg.draw_mode == DrawMode::Edit  || 
                            pg.draw_mode == DrawMode::Rotate {
                            (None, None, None)
                        } else {
                            (Some(build_polygon_path(
                                pg, 
                                pg.draw_mode, 
                                None, 
                                None, 
                                false)), 
                                Some(pg.color), Some(pg.width))
                        }
                    }
                    CanvasWidget::RightTriangle(tr) => {
                        // skip if being editied or rotated
                        if tr.draw_mode == DrawMode::Edit  || 
                            tr.draw_mode == DrawMode::Rotate {
                            (None, None, None)
                        } else {
                            (Some(build_right_triangle_path(
                                tr, 
                                tr.draw_mode, 
                                None, 
                                None, 
                                false)), 
                                Some(tr.color), Some(tr.width))
                        }
                    },
                    CanvasWidget::None => (None, None, None),
                };
            
            if path.is_some() {
                frame.stroke(
                &path.unwrap(),
                Stroke::default()
                    .with_width(width.unwrap())
                    .with_color(color.unwrap()),
                );
            }
        }

    }
}



#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Pending {
    N {
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
        step_count: f32,
        degrees: f32,
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

        if let Some(pending_cursor) = cursor.position_in(bounds) {
            // This draw happens when the mouse is moved and the state is none.
            match self {
                Pending::N { 
                    widget, 
                } => {
                    let (path, color, width, degrees, mid_point) = match widget {
                        CanvasWidget::Bezier(bz) => {
                            let path = 
                                build_bezier_path(
                                    bz, 
                                    DrawMode::New, 
                                    Some(pending_cursor),
                                    None,
                                    false,
                                );
                            (path, bz.color, bz.width, None, None)
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
                            (path, cir.color, cir.width, None, None)
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
                            (path, line.color, line.width, None, None)
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
                            (path, pl.color, pl.width, None, None)
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
                            
                            (path, pg.color, pg.width, Some(pg.degrees), Some(pg.mid_point))
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
                            (path, r_tr.color, r_tr.width, None, None)
                        },
                        _ => (Path::new(|_| {}), Color::TRANSPARENT, 0.0, None, None)
                    };

                    if degrees.is_some() {
                        let degrees = format!("{:.prec$}", degrees.unwrap(), prec = 1);
                        let mid_point = mid_point.unwrap();
                        let position = Point::new(mid_point.x-5.0, mid_point.y-5.0);
                        frame.fill_text(canvas::Text {
                            position: position,
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
                Pending::EditSecond { 
                    widget,
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
                Pending::EditThird { 
                    widget,
                    edit_curve_index:_, 
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
                Pending::Rotate {
                    widget,
                    edit_curve_index: _,
                    step_count: _,
                    degrees, 
                } => {
                    let (path, color, width, mid_point) = match widget {
                        CanvasWidget::Bezier(bz) => {
                            let path = 
                                build_bezier_path(
                                    bz, 
                                    DrawMode::Rotate, 
                                    None,
                                    None, 
                                    false,
                                );
                            (path, bz.color, bz.width, bz.mid_point)
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
                            (path, cir.color, cir.width, cir.center)
                        },
                            CanvasWidget::Line(line) => {
                            let path = 
                                build_line_path(
                                    line, 
                                    DrawMode::Rotate, 
                                    None,
                                    None,
                                    false,
                                );
                            (path, line.color, line.width, line.mid_point)
                        },
                        CanvasWidget::PolyLine(pl) => {
                            let path = 
                                build_polyline_path(
                                    pl, 
                                    DrawMode::Rotate, 
                                    None,
                                    None,
                                    false,
                                );
                            (path, pl.color, pl.width, pl.mid_point)
                        },
                        CanvasWidget::Polygon(pg) => {
                            let path = 
                                build_polygon_path(
                                    pg, 
                                    DrawMode::Rotate, 
                                    None,
                                    None,
                                    false,
                                );
                            (path, pg.color, pg.width, pg.mid_point)
                        },
                        CanvasWidget::RightTriangle(tr) => {
                            let path = 
                                build_right_triangle_path(
                                    tr, 
                                    DrawMode::Rotate, 
                                    None,
                                    None,
                                    false,
                                );
                            (path, tr.color, tr.width, tr.mid_point)
                        },
                        CanvasWidget::None => {
                            (Path::new(|_| {}), Color::TRANSPARENT, 0.0, Point::default())
                        }
                    };
                    let degrees = format!("{:.prec$}", degrees, prec = 1);
                    let position = Point::new(mid_point.x-5.0, mid_point.y-5.0);
                    frame.fill_text(canvas::Text {
                        position: position,
                        color: Color::WHITE,
                        size: 10.0.into(),
                        content: degrees,
                        ..canvas::Text::default()
                    });
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
    pub draw_mode: DrawMode,
}

#[derive(Debug, Clone, Default)]
pub struct PolyLine {
    pub points: Vec<Point>,
    pub poly_points: usize,
    pub mid_point: Point,
    pub color: Color,
    pub width: f32,
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

        let distance = match &draw_curve.widget {
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
        };

    distance

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
                        degrees: &mut f32,
                    ) -> CanvasWidget {
    match widget {
        CanvasWidget::None => CanvasWidget::None,
        CanvasWidget::Bezier(bz) => {
            bz.draw_mode = DrawMode::DrawAll;
            CanvasWidget::Bezier(bz.clone())
        },
        CanvasWidget::Circle(cir) => {
            cir.draw_mode = DrawMode::DrawAll;
            CanvasWidget::Circle(cir.clone())
        },
        CanvasWidget::Line(ln) => {
            ln.draw_mode = DrawMode::DrawAll;
            CanvasWidget::Line(ln.clone())
        },
        CanvasWidget::PolyLine(pl) => {
            pl.draw_mode = DrawMode::DrawAll;
            CanvasWidget::PolyLine(pl.clone())
        },
        CanvasWidget::Polygon(pg) => {
            pg.draw_mode = DrawMode::DrawAll;
            pg.degrees = *degrees;
            CanvasWidget::Polygon(pg.clone())
        },
        CanvasWidget::RightTriangle(tr) => {
            tr.draw_mode = DrawMode::DrawAll;
            CanvasWidget::RightTriangle(tr.clone())
        },
    }
}

fn update_draw_mode(widget: CanvasWidget, 
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
fn set_widget_point(widget: &CanvasWidget, cursor_position: Point) -> (CanvasWidget, bool) {
    match widget {
        CanvasWidget::None => (CanvasWidget::None, true),
        CanvasWidget::Bezier(bezier) => {
            let mut bz = bezier.clone();
            bz.points.push(cursor_position);
            let finished = if bz.points.len() == 3 {
                bz.mid_point = get_mid_geometry(&bz.points, Widget::Bezier);
                bz.draw_mode = DrawMode::DrawAll;
                true
            } else {
                false
            };
            if finished {
                bz.draw_mode = DrawMode::DrawAll;
            }
            (CanvasWidget::Bezier(bz), finished)
        },
        CanvasWidget::Circle(circle) => {
            let mut cir = circle.clone();
            let finished = if cir.center == Point::default() {
                cir.center = cursor_position;
                false
            } else {
                cir.radius = cir.center.distance(cursor_position);
                cir.circle_point = cursor_position;
                true
            };
            if finished {
                cir.draw_mode = DrawMode::DrawAll;
            }
            (CanvasWidget::Circle(cir), finished)
        },
        CanvasWidget::Line(line) => {
            let mut ln = line.clone();
            ln.points.push(cursor_position);

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
            pl.points.push(cursor_position);
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
                pg.mid_point = cursor_position;
                false
            } else {
                pg.pg_point = cursor_position;
                true
            };
            if finished {
                pg.draw_mode = DrawMode::DrawAll;
                let v1 = Point { x: pg.mid_point.x, y: 10.0 };
                let v2 = Point{x: pg.mid_point.x, y: cursor_position.y};
                pg.degrees = angle_of_vectors(v1, v2, true)
            }
            (CanvasWidget::Polygon(pg), finished)
        },
        CanvasWidget::RightTriangle(right_triangle) => {
            let mut rt = right_triangle.clone();
            rt.points.push(cursor_position);
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
                bz.mid_point = get_mid_geometry(&bz.points, Widget::Bezier);
            } else if mid_point {
                bz.points = 
                    translate_geometry(
                        bz.points, 
                        cursor,
                        bz.mid_point, 
                        );
                bz.mid_point = cursor;
            }
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
        Widget::RightTriangle => {
            let x = (pts[0].x + pts[1].x + pts[2].x)/3.0;
            let y = (pts[0].y + pts[1].y + pts[2].y)/3.0;
            Point {x, y}
        },
        Widget::None => Point::default(),
    }
    
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

fn rotate_geometry(widget: &mut CanvasWidget, theta: f32) -> CanvasWidget {

    match widget {
        CanvasWidget::None => CanvasWidget::None,
        CanvasWidget::Bezier(bz) => {
            let mut bz = bz.clone();
            bz.points = rotate_widget(bz.points, bz.mid_point, theta);
            CanvasWidget::Bezier(bz.clone())
        },
        CanvasWidget::Circle(cir) => {
            let mut cir = cir.clone();
            cir.circle_point = rotate_widget(vec![cir.circle_point], cir.center, theta)[0];
            CanvasWidget::Circle(cir.clone())
        },
        CanvasWidget::Line(line) => {
            let mut line = line.clone();
            line.points = rotate_widget(line.points, line.mid_point, theta);
            CanvasWidget::Line(line.clone())
        },
        CanvasWidget::PolyLine(pl) => {
            let mut pl = pl.clone();
            pl.points = rotate_widget(pl.points, pl.mid_point, theta);
            CanvasWidget::PolyLine(pl.clone())
        },
        CanvasWidget::Polygon(pg) => {
            let mut pg = pg.clone();
            pg.points = rotate_widget(pg.points, pg.mid_point, theta);
            CanvasWidget::Polygon(pg.clone())
        },
        CanvasWidget::RightTriangle(tr) => {
            let mut tr = tr.clone();
            tr.points = rotate_widget(tr.points, tr.mid_point, theta);
            CanvasWidget::RightTriangle(tr)
        },
    }

}

// To rotate a point (x, y) around a center point (cx, cy) by an angle θ, 
// the formula for the rotated coordinates (x', y') is: 
// x' = (x - cx) * cos(θ) - (y - cy) * sin(θ) + cx and 
// y' = (x - cx) * sin(θ) + (y - cy) * cos(θ) + cy; 
// where (x, y) is the original point, (cx, cy) is the center of rotation, 
//and θ is the rotation angle in radians. 
fn rotate_widget(points: Vec<Point>, center: Point, theta: f32) -> Vec<Point> {
    let mut new_points = vec![];
    for point in points.iter() {
        let x_new = (point.x - center.x) * theta.cos() - (point.y - center.y) * theta.sin() + center.x;
        let y_new = (point.x - center.x) * theta.sin() + (point.y - center.y) * theta.cos() + center.y;

        new_points.push(Point { x: x_new, y: y_new })
    }
    
    new_points
}

fn angle_of_vectors(v1: Point, v2: Point, degrees: bool) -> f32 {
    let mag_1 = (v1.x*v1.x + v1.y*v1.y).sqrt();
    let mag_2 = (v2.x*v2.x + v2.y*v2.y).sqrt();

    let dot = (v1.x*v2.x) + (v1.y*v2.y);
    if degrees {
        (dot/(mag_1*mag_2)).acos()*180.0/PI
    } else {
        (dot/(mag_1*mag_2)).acos()
    }
    
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

    let mut pts = rotate_widget(points.clone(), mid_point, degrees);
    pts.push(pts[0]);
    pts

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
                let mut pts = bz.points.clone();
                let mut mid_point = bz.mid_point;

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
                    mid_point = get_mid_geometry(&pts, Widget::Bezier);
                }

                p.move_to(pts[0]);
                p.quadratic_curve_to(pts[2], pts[1]);
                
                for pt in pts {
                    p.circle(pt, 3.0);
                }
                p.circle(mid_point, 3.0);
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
                p.move_to(bz.points[0]);
                p.quadratic_curve_to(bz.points[2], bz.points[1]);
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
                ) -> Path {
    Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                p.move_to(line.points[0]);
                p.line_to(line.points[1]);
            },
            DrawMode::Edit => {
                let mut pts = line.points.clone();
                let mut mid_point = line.mid_point;

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

                p.move_to(pts[0]);
                p.line_to(pts[1]);
                p.circle(pts[0], 3.0);
                p.circle(pts[1], 3.0);
                p.circle(mid_point, 3.0);
            },
            DrawMode::New => {
                p.move_to(line.points[0]);
                p.line_to(pending_cursor.unwrap());
            },
            DrawMode::Rotate => {
                p.move_to(line.points[0]);
                p.line_to(line.points[1]);
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
                let mut mid_point = pg.mid_point;
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
                let cursor = pending_cursor.unwrap();
                let v1 = Point{x: pg.mid_point.x, y:10.0};
                let v2 = Point{ x: pg.mid_point.x, y: cursor.y };
                let degrees = angle_of_vectors(v1, v2, true);
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
            },
            DrawMode::Rotate => {
                for (index, point) in pg.points.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*point);
                    } else {
                        p.line_to(*point);
                    }
                }
            },
        }
    })
}

fn build_right_triangle_path(tr: &RightTriangle, 
                            draw_mode: DrawMode, 
                            pending_cursor: Option<Point>,
                            edit_point_index: Option<usize>, 
                            edit_mid_point: bool,
                        ) -> Path {
    Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                p.move_to(tr.points[0]);
                p.line_to(tr.points[1]);
                p.line_to(tr.points[2]);
                p.line_to(tr.points[0]);
            },
            DrawMode::Edit => {
                let mut mid_point = tr.mid_point;
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
