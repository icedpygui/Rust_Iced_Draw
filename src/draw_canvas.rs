
use std::collections::HashMap;
use std::f32::consts::PI;
use iced::keyboard::Key;
use iced::widget::canvas::path::arc::Elliptical;
use iced::widget::container::Id;
use iced::widget::text::{LineHeight, Shaping};
use iced::{alignment, mouse, Color, Font, Pixels, Radians, Vector};
use iced::widget::canvas::event::{self, Event};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Element, Fill, Point, Renderer, Theme};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub enum CanvasWidget {
    #[default]
    None,
    Arc(Arc),
    Bezier(Bezier),
    Circle(Circle),
    Ellipse(Ellipse),
    Line(Line),
    PolyLine(PolyLine),
    Polygon(Polygon),
    RightTriangle(RightTriangle),
    Text(Text),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq,)]
pub enum DrawMode {
    #[default]
    DrawAll,
    Edit,
    New,
    Rotate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq,)]
pub enum DrawStatus {
    Inprogress,
    Completed,
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
pub struct CanvasState {
    cache: canvas::Cache,
    pub curves: HashMap<Id, CanvasWidget>,
    pub draw_mode: DrawMode,
    pub edit_widget_id: Option<Id>,
    pub escape_pressed: bool,
    pub selected_radio_widget: Option<Widget>,
    pub selected_color: Color,
    pub selected_color_str: Option<String>,
    pub selected_poly_points: usize,
    pub selected_poly_points_str: String,
    pub selected_step_degrees: f32,
    pub selected_width: f32,
    pub selected_width_str: String,
    pub timer_event_enabled: bool,
    pub timer_duration: u64,
    pub elapsed_time: u64,
    pub blink: bool,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self { 
                cache: canvas::Cache::default(),
                curves: HashMap::new(),
                draw_mode: DrawMode::DrawAll,
                edit_widget_id: None,
                escape_pressed: false,
                selected_radio_widget: None,
                selected_color: Color::WHITE,
                selected_color_str: Some("White".to_string()),
                selected_poly_points: 3,
                selected_poly_points_str: "".to_string(),
                selected_step_degrees: 6.0,
                selected_width: 2.0,
                selected_width_str: "".to_string(),
                timer_event_enabled: true,
                timer_duration: 1000,
                elapsed_time: 0,
                blink: false,
             }
        }
}

impl CanvasState {
    pub fn view<'a>(&'a self, curves: &'a HashMap<Id, CanvasWidget>) -> Element<'a, CanvasWidget> {
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
    state: &'a CanvasState,
    curves: &'a HashMap<Id, CanvasWidget>,
}

impl<'a> canvas::Program<CanvasWidget> for DrawPending<'a> {
    type State = Option<Pending>;

    fn update(
        &self,
        program_state: &mut Self::State,
        event: Event,
        bounds: iced::Rectangle,
        cursor: mouse::Cursor,
    ) -> (event::Status, Option<CanvasWidget>) {
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
                                        let widget_opt = find_closest_widget(self.curves, cursor_position);
                                        
                                        let selected_widget = 
                                            match widget_opt {
                                                Some(w) => w,
                                                None => return (event::Status::Ignored, None),
                                            };

                                        // set draw_mode to indicate being edited
                                        let widget = 
                                            set_widget_mode_or_status(
                                                selected_widget, 
                                                Some(DrawMode::Edit),
                                                Some(DrawStatus::Inprogress),
                                            );
                                        *program_state = Some(Pending::EditSecond {
                                            widget: widget.clone(),
                                        });
                                        // returning DrawCurve so that the curve
                                        // being editied will not show after the refresh
                                        // The pending process will show the curve
                                        // until its finsihed.

                                        Some(widget)
                                    },
                                    // The second click is a Some() since it was created above
                                    // The pending is carrying the previous info
                                    // This second click will find the point
                                    // and replace with cursor
                                    Some(Pending::EditSecond { 
                                        widget,
                                    }) => {
                                        // Find for closest point to edit in selected widget
                                        // which might be either a mid point(translate) or 
                                        // curve point (move point).
                                        let widget = widget.clone();
                                        let (point_index, mid_point, other_point) = 
                                            find_closest_point_index(&widget, cursor_position);
                                        
                                        *program_state = Some(Pending::EditThird {
                                            widget,
                                            edit_point_index: point_index,
                                            edit_mid_point: mid_point,
                                            edit_other_point: other_point,
                                        });
                                        None
                                    },
                                    // The third click will send back the DrawCurve
                                    // with the finally updated curve
                                    Some(Pending::EditThird { 
                                        widget,
                                        edit_point_index,
                                        edit_mid_point,
                                        edit_other_point, 
                                    }) => {

                                        let new_widget: CanvasWidget = 
                                                update_edited_widget(
                                                    widget.clone(), 
                                                    cursor_position, 
                                                    *edit_point_index, 
                                                    *edit_mid_point,
                                                    *edit_other_point,
                                                    DrawStatus::Completed,
                                                );
                                        
                                        *program_state = None;
                                        Some(new_widget)
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
                                        if self.state.selected_radio_widget.is_none() {
                                            return (event::Status::Ignored, None)
                                        }
                                        let selected_widget = 
                                            add_new_widget(
                                                self.state.selected_radio_widget.unwrap(), 
                                                self.state.selected_poly_points,
                                                self.state.selected_color,
                                                self.state.selected_width,
                                                self.state.draw_mode,
                                            );

                                        let (widget, _) = 
                                            set_widget_point(
                                                &selected_widget, 
                                                cursor_position,
                                            );
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
                                        
                                        // if completed, we return the CanvasWidget and set the state to none
                                        // if not, then this is repeated until completed.
                                        if completed {
                                            *program_state = None;
                                            complete_new_widget(widget, cursor_position)
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
                                        let widget_opt = find_closest_widget(self.curves, cursor_position);
                                        
                                        let selected_widget = 
                                            match widget_opt {
                                                Some(w) => w,
                                                None => return (event::Status::Ignored, None),
                                            };
                                        
                                        // The widget needs to be in DrawAll initially, 
                                        // in order to display it in pending
                                        // However, the below return of the draw curve 
                                        // the widget need to ne in the rotate method.
                                        let widget = 
                                            set_widget_mode_or_status(
                                                selected_widget, 
                                                Some(DrawMode::Rotate),
                                                Some(DrawStatus::Inprogress),
                                            );
                                        
                                        *program_state = Some(Pending::Rotate {
                                            widget: widget.clone(),
                                            step_degrees: self.state.selected_step_degrees,
                                            degrees: get_widget_degrees(&widget),
                                        });

                                        // returning CanvasWidget so that the curve
                                        // being editied will not show after the refresh
                                        // The pending process will show the curve
                                        // until its finsihed.
                                        Some(widget)
                                    },
                                    // After the final rotation completed
                                    Some(Pending::Rotate {
                                        widget,
                                        step_degrees: _,
                                        degrees: _,
                                    }) => {
                                        let (rotated_widget, _) = 
                                            update_rotated_widget(
                                                widget,
                                                0.0,
                                                Some(DrawStatus::Completed),
                                            );

                                        *program_state = None;

                                        Some(rotated_widget)
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
                                        step_degrees,
                                        degrees: _,  
                                    }) => {
                                        let delta = match delta {
                                            mouse::ScrollDelta::Lines { x:_, y } => y,
                                            mouse::ScrollDelta::Pixels { x:_, y } => y,
                                        };

                                        // Setting the widget draw_mode at each mouse wheel
                                        // since it was set to DrawAll initially.
                                        // Otherwise needed to have another pending type
                                        // and duplicate a lot of code.  Had to clone anyway.
                                        let (widget, degrees) = 
                                            update_rotated_widget(
                                                widget, 
                                                *step_degrees*delta,
                                                None, 
                                            );
                                        
                                        *program_state = Some(Pending::Rotate{
                                            widget,
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
            Event::Keyboard(key_event) => {
                let message = match key_event {
                    iced::keyboard::Event::KeyPressed { 
                        key:_, 
                        modified_key:_, 
                        physical_key:_, 
                        location:_, 
                        modifiers:_, 
                        text:_ } => None,
                    iced::keyboard::Event::KeyReleased {key: _, location:_, modifiers:_ } => None,
                    iced::keyboard::Event::ModifiersChanged(_) => None,
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
            self.state.cache.draw(renderer, bounds.size(), 
                            |frame| {
                DrawCurve::draw_all(self.curves, self.state.blink, self.state.elapsed_time, frame, theme);

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
}

impl DrawCurve {
    fn draw_all(curves: &HashMap<Id, CanvasWidget>, blink: bool, time: u64, frame: &mut Frame, _theme: &Theme) {
        // This draw only occurs at the completion of the 
        // widget(update occurs) and cache is cleared
        
        for (_id, widget) in curves.iter() {
            // if first click, skip the curve to be edited so that it 
            // will not be seen until the second click.  Otherwise is shows
            // during editing because there is no way to refresh
            // The pending routine will diplay the curve

            let (path, color, width) = 
                match &widget {
                    CanvasWidget::Arc(arc) => {
                        // skip if being editied or rotated
                        if arc.status == DrawStatus::Inprogress {
                            (None, None, None)
                        } else {
                            let (path, _, _,_,_,_) = 
                                build_arc_path(
                                arc, 
                                arc.draw_mode, 
                                None, 
                                None, 
                                false,
                            );

                            (Some(path), Some(arc.color), Some(arc.width))
                        }
                    },
                    CanvasWidget::Bezier(bz) => {
                        // skip if being editied or rotated
                        if bz.status == DrawStatus::Inprogress {
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
                        if cir.status== DrawStatus::Inprogress {
                            (None, None, None)
                        } else {
                            let path = 
                                build_circle_path(
                                    cir, 
                                    cir.draw_mode,
                                    None, 
                                    None, 
                                    false
                                );
                            (Some(path), Some(cir.color), Some(cir.width))
                        }
                    },
                    CanvasWidget::Ellipse(ell) => {
                        // skip if being editied or rotated
                        if ell.status == DrawStatus::Inprogress {
                            (None, None, None)
                        } else {
                            let path = 
                                build_ellipse_path(
                                    ell, 
                                    ell.draw_mode,
                                    None, 
                                    None, 
                                    false,
                                );
                            (Some(path), Some(ell.color), Some(ell.width))
                        }
                    },
                    CanvasWidget::Line(line) => {
                        // skip if being editied or rotated
                        if line.status == DrawStatus::Inprogress {
                            (None, None, None)
                        } else {
                            let (path, _, _) = build_line_path(
                                line, 
                                line.draw_mode, 
                                None, 
                                None, 
                                false,
                                None,
                                );

                            (Some(path), Some(line.color), Some(line.width))
                        }
                    },
                    CanvasWidget::PolyLine(pl) => {
                        // skip if being editied or rotated
                        if pl.status == DrawStatus::Inprogress {
                            (None, None, None)
                        } else {
                            let (path, _, _) = build_polyline_path(
                                pl, 
                                pl.draw_mode, 
                                None, 
                                None, 
                                false,
                                false,
                                None,
                            );
                            (Some(path), Some(pl.color), Some(pl.width))
                        }
                    },
                    CanvasWidget::Polygon(pg) => {
                        // skip if being editied or rotated
                        if pg.status == DrawStatus::Inprogress {
                            (None, None, None)
                        } else {
                            let (path, _, _) = 
                            build_polygon_path(
                                pg, 
                                pg.draw_mode, 
                                None,  
                                false,
                                false,
                                None,
                            );
                                
                            (Some(path), Some(pg.color), Some(pg.width))
                        }
                    }
                    CanvasWidget::RightTriangle(tr) => {
                        // skip if being editied or rotated
                        if tr.status == DrawStatus::Inprogress {
                            (None, None, None)
                        } else {
                            let (path, _, _, _) = build_right_triangle_path(
                                tr, 
                                tr.draw_mode, 
                                None, 
                                None, 
                                false,
                                false,
                                None,
                            );
                                
                            (Some(path), Some(tr.color), Some(tr.width))
                        }
                    },
                    CanvasWidget::Text(txt) => {
                        dbg!(time, blink);
                        frame.fill_text(build_text_path (
                            txt,
                            txt.draw_mode,
                            None,
                            0.0,
                            blink,
                        ));
                        (None, Some(txt.color), None)
                        
                        
                    }
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
        },
    EditThird {
        widget: CanvasWidget, 
        edit_point_index: Option<usize>,
        edit_mid_point: bool,
        edit_other_point: bool,
        },
    Rotate {
        widget: CanvasWidget,
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
                    let (path, 
                        color, 
                        width,
                        mid_point, 
                        degrees_left,
                        degrees_center,
                        ) = 
                    match widget {
                        CanvasWidget::Arc(arc) => {
                            let (path, _, 
                                _, 
                                _, 
                                degrees_left,
                                degrees_center) = 
                                build_arc_path(
                                    arc, 
                                    DrawMode::New, 
                                    Some(cursor),
                                    None,
                                    false,
                                );
                            (path, arc.color, arc.width, Some(arc.points[0]), degrees_left, degrees_center)
                        },
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
                                
                            (path, bz.color, bz.width, Some(bz.points[0]), None, Some(degrees))
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
                            (path, cir.color, cir.width, None, None, None)
                        },
                        CanvasWidget::Ellipse(ell) => {
                            let path = 
                                build_ellipse_path(
                                    ell, 
                                    DrawMode::New, 
                                    Some(cursor),
                                    None,
                                    false,
                                );
                            (path, ell.color, ell.width, Some(ell.points[0]), None, None)
                        }
                        CanvasWidget::Line(line) => {
                            let (path, degrees, _) = 
                                build_line_path(
                                    line, 
                                    DrawMode::New, 
                                    Some(cursor),
                                    None,
                                    false,
                                    None,
                                );
                            (path, line.color, line.width, Some(line.points[0]), Some(degrees), None)
                        },
                        CanvasWidget::Polygon(pg) => {
                            let (path, degrees, mid_point) = 
                                build_polygon_path(
                                    pg,
                                    DrawMode::New, 
                                    Some(cursor),
                                    false,
                                    false,
                                    None,
                                );
                            
                            (path, pg.color, pg.width, Some(mid_point), Some(degrees), None)
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
                                    false,
                                    None,
                                );
                            (path, pl.color, pl.width, Some(mid_point), Some(degrees), None)
                        },
                        CanvasWidget::RightTriangle(r_tr) => {
                            let (path, degrees, mid_point, _) = 
                                build_right_triangle_path(
                                    r_tr, 
                                    DrawMode::New, 
                                    Some(cursor),
                                    None,
                                    false,
                                    false,
                                    None,
                                );
                            (path, r_tr.color, r_tr.width, Some(mid_point), Some(degrees), None)
                        },
                        CanvasWidget::Text(txt) => {
                            frame.fill_text(build_text_path(
                                    txt, 
                                    DrawMode::New, 
                                    Some(cursor), 
                                    0.0,
                                    false,
                                ));
                                
                            (Path::new(|_| {}), Color::TRANSPARENT, 0.0, None, None, None)
                        }
                        _ => (Path::new(|_| {}), Color::TRANSPARENT, 0.0, None, None, None)
                    };

                    if degrees_center.is_some() {
                        let degrees = format!("{:.prec$}", degrees_center.unwrap(), prec = 1);
                        let mid_point = mid_point.unwrap();
                        let position_center = Point::new(mid_point.x-10.0, mid_point.y-20.0);
                        frame.fill_text(canvas::Text {
                            position: position_center,
                            color: Color::WHITE,
                            size: 10.0.into(),
                            content: degrees,
                            horizontal_alignment: alignment::Horizontal::Center,
                            vertical_alignment: alignment::Vertical::Center,
                            ..canvas::Text::default()
                        });
                    }
                    if degrees_left.is_some() {
                        let degrees = format!("{:.prec$}", degrees_left.unwrap(), prec = 1);
                        let mid_point = mid_point.unwrap();
                        let position_left = Point::new(mid_point.x-30.0, mid_point.y-10.0);
                        frame.fill_text(canvas::Text {
                            position: position_left,
                            color: Color::WHITE,
                            size: 10.0.into(),
                            content: degrees,
                            horizontal_alignment: alignment::Horizontal::Center,
                            vertical_alignment: alignment::Vertical::Center,
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
                } => {
                    let (path, color, width) = 
                        match widget {
                            CanvasWidget::None => {
                                (Path::new(|_| {}), Color::TRANSPARENT, 0.0)
                            },
                            CanvasWidget::Arc(arc) => {
                                let (path, _, _, _,_,_) = 
                                build_arc_path(
                                    arc, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    None,
                                    false,
                                );

                                (path, arc.color, arc.width)
                            },
                            CanvasWidget::Bezier(bz) => {
                                let (path, _, _) = 
                                build_bezier_path(
                                    bz, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    None, 
                                    false,
                                    None,
                                );
                           
                                (path, bz.color, bz.width)
                            },
                            CanvasWidget::Circle(cir) => {
                                let path = 
                                build_circle_path(
                                    cir, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    None, 
                                    false,
                                );
                                (path, cir.color, cir.width)
                            },
                            CanvasWidget::Ellipse(ell) => {
                                let path = 
                                build_ellipse_path(
                                    ell, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    None, 
                                    false,
                                );
                                (path, ell.color, ell.width)
                            },
                            CanvasWidget::Line(line) => {
                                let (path, _, _) = 
                                build_line_path(
                                    line, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    None, 
                                    false,
                                    None,
                                );
                            
                                (path, line.color, line.width)
                            },
                            CanvasWidget::Polygon(pg) => {
                                let (path, _, _) = 
                                build_polygon_path(
                                    pg, 
                                    DrawMode::Edit, 
                                    Some(cursor), 
                                    false,
                                    false,
                                    None,
                                );
                                (path, pg.color, pg.width)
                            },
                            CanvasWidget::PolyLine(pl) => {
                                let (path, _, _) = 
                                    build_polyline_path(
                                        pl, 
                                        DrawMode::Edit, 
                                        Some(cursor),
                                        None, 
                                        false,
                                        false,
                                        None,
                                    );
                                (path, pl.color, pl.width)
                            },
                            CanvasWidget::RightTriangle(tr) => {
                                let (path, _, _, _) = 
                                build_right_triangle_path(
                                    tr, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    None, 
                                    false,
                                    false,
                                    None,
                                );
                                (path, tr.color, tr.width)
                            },
                            CanvasWidget::Text(_txt) => {
                                (Path::new(|_| {}), Color::TRANSPARENT, 0.0)
                            }
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
                    edit_point_index, 
                    edit_mid_point, 
                    edit_other_point, 
                } => {

                    let (path, 
                        color, 
                        width, 
                        mid_point, 
                        degrees_left,
                        degrees_center,
                        ) = match widget {

                        CanvasWidget::None => {
                            (Path::new(|_| {}), Color::TRANSPARENT, 0.0, Point::default(), None, None)
                        },
                        CanvasWidget::Arc(arc) => {
                            let (path, 
                                mid_point, 
                                _, 
                                _,
                                degrees_left,
                                degrees_center,
                                ) = 
                                build_arc_path(
                                    arc, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );

                            (path, arc.color, arc.width, mid_point, degrees_left, degrees_center)
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
                           
                            (path, bz.color, bz.width, mid_point, None, Some(degrees))
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
                            (path, cir.color, cir.width, cir.center, None, None)
                        },
                        CanvasWidget::Ellipse(ell) => {
                            let path = 
                                build_ellipse_path(
                                    ell, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                );
                            (path, ell.color, ell.width, ell.center, None, None)
                        },
                        CanvasWidget::Line(line) => {
                            let (path, degrees, mid_point) = 
                                build_line_path(
                                    line, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                    None,
                                );
                            
                            (path, line.color, line.width, mid_point, None, Some(degrees))
                        },
                        CanvasWidget::Polygon(pg) => {
                            let (path, degrees, mid_point) = 
                                build_polygon_path(
                                    pg, 
                                    DrawMode::Edit, 
                                    Some(cursor), 
                                    *edit_mid_point,
                                    *edit_other_point,
                                    None,
                                );
                            (path, pg.color, pg.width, mid_point, None, Some(degrees))
                        },
                        CanvasWidget::PolyLine(pl) => {
                            let (path, degrees, mid_point) = 
                                build_polyline_path(
                                    pl, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                    *edit_other_point,
                                    None,
                                );
                            (path, pl.color, pl.width, mid_point, None, Some(degrees))
                        },
                        CanvasWidget::RightTriangle(tr) => {
                            let (path, degrees, mid_point, _) = 
                                build_right_triangle_path(
                                    tr, 
                                    DrawMode::Edit, 
                                    Some(cursor),
                                    *edit_point_index, 
                                    *edit_mid_point,
                                    *edit_other_point,
                                    None,
                                );
                            (path, tr.color, tr.width, mid_point, None, Some(degrees))
                        },
                        CanvasWidget::Text(_txt) => {
                            (Path::new(|_| {}), Color::TRANSPARENT, 0.0, Point::default(), None, None)
                        }
                    };

                    if degrees_left.is_some() {
                        let degrees = format!("{:.prec$}", degrees_left.unwrap(), prec = 1);
                        let position = Point::new(mid_point.x-30.0, mid_point.y-10.0);
                        frame.fill_text(canvas::Text {
                            position,
                            color: Color::WHITE,
                            size: 10.0.into(),
                            content: degrees,
                            horizontal_alignment: alignment::Horizontal::Center,
                            vertical_alignment: alignment::Vertical::Center,
                            ..canvas::Text::default()
                        });
                    }

                    if degrees_center.is_some() {
                        let degrees = format!("{:.prec$}", degrees_center.unwrap(), prec = 1);
                        let position = Point::new(mid_point.x-10.0, mid_point.y-20.0);
                        frame.fill_text(canvas::Text {
                            position,
                            color: Color::WHITE,
                            size: 10.0.into(),
                            content: degrees,
                            horizontal_alignment: alignment::Horizontal::Center,
                            vertical_alignment: alignment::Vertical::Center,
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
                    step_degrees: _,
                    degrees, 
                } => {
                    let (path, 
                        color, 
                        width, 
                        mid_point, 
                        degrees_left,
                        degrees_center,
                    ) = match widget {
                        CanvasWidget::Arc(arc) => {
                            let (path, 
                                _, 
                                _, 
                                _, 
                                degrees_left,
                                degrees_center,) = 
                                build_arc_path(
                                    arc, 
                                    arc.draw_mode,
                                    None,
                                    None, 
                                    false,
                                );

                            (path, arc.color, arc.width, arc.mid_point, degrees_left, degrees_center)
                        },
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
                            (path, bz.color, bz.width, bz.mid_point, None, Some(pending_degrees))
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
                            (path, cir.color, cir.width, cir.center, None, None)
                        },
                        CanvasWidget::Ellipse(ell) => {
                            let path = 
                                build_ellipse_path(
                                    ell, 
                                    DrawMode::Rotate, 
                                    None,
                                    None,
                                    false,
                                );
                                (path, ell.color, ell.width, ell.center, None, Some(to_degrees(&ell.rotation.0)))
                            },
                        CanvasWidget::Line(line) => {
                            let (path, pending_degrees, _) = 
                                build_line_path(
                                    line, 
                                    line.draw_mode, 
                                    None,
                                    None,
                                    false,
                                    *degrees,
                                );
                            (path, line.color, line.width, line.mid_point, None, Some(pending_degrees))
                        },
                        CanvasWidget::Polygon(pg) => {
                            let (path, pending_degrees, _) = 
                                build_polygon_path(
                                    pg, 
                                    pg.draw_mode, 
                                    None,
                                    false,
                                    false,
                                    *degrees,
                                );
                            (path, pg.color, pg.width, pg.mid_point, None, Some(pending_degrees))
                        },
                        CanvasWidget::PolyLine(pl) => {
                            let (path, pending_degrees, _) = 
                                build_polyline_path(
                                    pl, 
                                    DrawMode::Rotate, 
                                    None,
                                    None,
                                    false,
                                    false,
                                    *degrees,
                                );
                            (path, pl.color, pl.width, pl.mid_point, None, Some(pending_degrees))
                        },
                        CanvasWidget::RightTriangle(tr) => {
                            let (path, pending_degrees, _, _) = 
                                build_right_triangle_path(
                                    tr, 
                                    DrawMode::Rotate, 
                                    None,
                                    None,
                                    false,
                                    false,
                                    *degrees,
                                );
                            (path, tr.color, tr.width, tr.mid_point, None, Some(pending_degrees))
                        },
                        CanvasWidget::Text(_txt) => {
                            (Path::new(|_| {}), Color::TRANSPARENT, 0.0, Point::default(), None, None)
                        }
                        CanvasWidget::None => {
                            (Path::new(|_| {}), Color::TRANSPARENT, 0.0, Point::default(), None, None)
                        }
                    };

                    if degrees_left.is_some() {
                        let degrees = format!("{:.prec$}", degrees_left.unwrap(), prec = 1);
                        let position = Point::new(mid_point.x-30.0, mid_point.y-10.0);
                        frame.fill_text(canvas::Text {
                            position,
                            color: Color::WHITE,
                            size: 10.0.into(),
                            content: degrees,
                            horizontal_alignment: alignment::Horizontal::Center,
                            vertical_alignment: alignment::Vertical::Center,
                            ..canvas::Text::default()
                        });
                    }

                    if degrees_center.is_some() {
                        let degrees = format!("{:.prec$}", degrees_center.unwrap(), prec = 1);
                        let position = Point::new(mid_point.x-10.0, mid_point.y-20.0);
                        frame.fill_text(canvas::Text {
                            position,
                            color: Color::WHITE,
                            size: 10.0.into(),
                            content: degrees,
                            horizontal_alignment: alignment::Horizontal::Center,
                            vertical_alignment: alignment::Vertical::Center,
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

#[derive(Debug, Clone)]
pub struct Arc {
    pub id: Id,
    pub points: Vec<Point>,
    pub mid_point: Point,
    pub radius: f32,
    pub color: Color,
    pub width: f32,
    pub start_angle: Radians,
    pub end_angle: Radians,
    pub draw_mode: DrawMode,
    pub status: DrawStatus,
}

#[derive(Debug, Clone)]
pub struct Bezier {
    pub id: Id,
    pub points: Vec<Point>,
    pub mid_point: Point,
    pub color: Color,
    pub width: f32,
    pub degrees: f32,
    pub draw_mode: DrawMode,
    pub status: DrawStatus,
}

#[derive(Debug, Clone)]
pub struct Circle {
    pub id: Id,
    pub center: Point,
    pub circle_point: Point,
    pub radius: f32,
    pub color: Color,
    pub width: f32,
    pub draw_mode: DrawMode,
    pub status: DrawStatus,
}

#[derive(Debug, Clone)]
pub struct Ellipse {
    pub id: Id,
    pub points: Vec<Point>,
    pub center: Point,
    pub radii: Vector,
    pub rotation: Radians,
    pub color: Color,
    pub width: f32,
    pub draw_mode: DrawMode,
    pub status: DrawStatus,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub id: Id,
    pub points: Vec<Point>,
    pub mid_point: Point,
    pub color: Color,
    pub width: f32,
    pub degrees: f32,
    pub draw_mode: DrawMode,
    pub status: DrawStatus,
}

#[derive(Debug, Clone)]
pub struct PolyLine {
    pub id: Id,
    pub points: Vec<Point>,
    pub poly_points: usize,
    pub mid_point: Point,
    pub pl_point: Point,
    pub color: Color,
    pub width: f32,
    pub degrees: f32,
    pub draw_mode: DrawMode,
    pub status: DrawStatus,
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub id: Id,
    pub points: Vec<Point>,
    pub poly_points: usize,
    pub mid_point: Point,
    pub pg_point: Point,
    pub color: Color,
    pub width: f32,
    pub degrees: f32,
    pub draw_mode: DrawMode,
    pub status: DrawStatus,
}

#[derive(Debug, Clone)]
pub struct RightTriangle {
    pub id: Id,
    pub points: Vec<Point>,
    pub mid_point: Point,
    pub tr_point: Point,
    pub color: Color,
    pub width: f32,
    pub degrees: f32,
    pub draw_mode: DrawMode,
    pub status: DrawStatus,
}

#[derive(Debug, Clone)]
pub struct Text {
    pub id: Id,
    pub content: String,
    pub position: Point,
    pub color: Color,
    pub size: Pixels,
    pub line_height: LineHeight,
    pub font: Font,
    pub horizontal_alignment: alignment::Horizontal,
    pub vertical_alignment: alignment::Vertical,
    pub shaping: Shaping,
    pub degrees: f32,
    pub draw_mode: DrawMode,
    pub status: DrawStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq,)]
pub enum Widget {
    None,
    Arc,
    Bezier,
    Circle,
    Ellipse,
    Line,
    PolyLine,
    Polygon,
    RightTriangle,
    Text,
}

fn add_new_widget(widget: Widget, 
                    poly_points: usize, 
                    color: Color,
                    width: f32,
                    draw_mode: DrawMode) 
                    -> CanvasWidget {
    match widget {
        Widget::None => {
            CanvasWidget::None
        },
        Widget::Arc => {
            CanvasWidget::Arc(
                Arc {
                    id: Id::unique(),
                    points: vec![],
                    mid_point: Point::default(),
                    radius: 0.0,
                    color,
                    width,
                    start_angle: Radians::PI,
                    end_angle: Radians::PI,
                    draw_mode,
                    status: DrawStatus::Inprogress,
                })
        
        },
        Widget::Bezier => {
            CanvasWidget::Bezier(
                Bezier { 
                    id: Id::unique(),
                    points: vec![],
                    mid_point: Point::default(),
                    color, 
                    width, 
                    degrees: 0.0, 
                    draw_mode,
                    status: DrawStatus::Inprogress,
                }
            )
        },
        Widget::Circle => {
            CanvasWidget::Circle(
                Circle {
                    id: Id::unique(),
                    center: Point::default(),
                    circle_point: Point::default(),
                    radius: 0.0,
                    color,
                    width,
                    draw_mode,
                    status: DrawStatus::Inprogress,
                }
            )
        },
        Widget::Ellipse => {
            CanvasWidget::Ellipse(
                Ellipse {
                    id: Id::unique(),
                    points: vec![],
                    center: Point::default(),
                    radii: Vector{x: 0.0, y: 0.0},
                    rotation: Radians(0.0),
                    color,
                    width,
                    draw_mode,
                    status: DrawStatus::Inprogress,
                }
            )
        },
        Widget::Line => {
            CanvasWidget::Line(
                Line {
                    id: Id::unique(),
                    points: vec![],
                    mid_point: Point::default(),
                    color,
                    width,
                    degrees: 0.0,
                    draw_mode,
                    status: DrawStatus::Inprogress,
                }
            )
        },
        Widget::PolyLine => {
            CanvasWidget::PolyLine(
                PolyLine {
                    id: Id::unique(),
                    points: vec![],
                    poly_points,
                    mid_point: Point::default(),
                    pl_point: Point::default(),
                    color,
                    width,
                    degrees: 0.0,
                    draw_mode,
                    status: DrawStatus::Inprogress,
                }
            )
        },
        Widget::Polygon => {
            CanvasWidget::Polygon(
                Polygon {
                    id: Id::unique(),
                    points: vec![],
                    poly_points,
                    mid_point: Point::default(),
                    pg_point: Point::default(),
                    color,
                    width,
                    degrees: 0.0,
                    draw_mode,
                    status: DrawStatus::Inprogress,
                }
            )
        },
        Widget::RightTriangle => {
            CanvasWidget::RightTriangle(
                RightTriangle {
                    id: Id::unique(),
                    points: vec![],
                    mid_point: Point::default(),
                    tr_point: Point::default(),
                    color,
                    width,
                    degrees: 0.0,
                    draw_mode,
                    status: DrawStatus::Inprogress,
                }
            )
        },
        Widget::Text => {
            CanvasWidget::Text(
                Text {
                    id: Id::unique(),
                    content: String::new(),
                    position: Point::default(),
                    color,
                    size: Pixels(16.0),
                    line_height: LineHeight::Relative(1.2),
                    font: Font::default(),
                    horizontal_alignment: alignment::Horizontal::Left,
                    vertical_alignment: alignment::Vertical::Top,
                    shaping: Shaping::Basic,
                    degrees: 0.0,
                    draw_mode,
                    status: DrawStatus::Inprogress,
                }
            )
        },
    }
}

fn complete_new_widget(widget: CanvasWidget, cursor: Point) -> Option<CanvasWidget> {
    match widget {
        CanvasWidget::None => {
            None
        },
        CanvasWidget::Arc(arc) => {
            Some(CanvasWidget::Arc(arc))
        },
        CanvasWidget::Bezier(mut bz) => {
            bz.mid_point = 
                get_mid_point(
                    bz.points[0], 
                    bz.points[1]
                );
            Some(CanvasWidget::Bezier(bz))
        },
        CanvasWidget::Circle(cir) => { 
            Some(CanvasWidget::Circle(cir))
        },
        CanvasWidget::Ellipse(mut ell) => {
            ell.center = ell.points[0];
            let vx = ell.points[1].distance(ell.center);
            let vy = cursor.distance(ell.center);
            ell.radii = Vector{ x: vx, y: vy };
            Some(CanvasWidget::Ellipse(ell))
        },
        CanvasWidget::Line(mut ln) => {
            // degree is angle rotation around mid point 
            let degrees = 
                get_horizontal_angle_of_vector(
                    ln.points[0],
                    ln.points[1], 
                );
            ln.degrees = degrees;

            Some(CanvasWidget::Line(ln))
        },
        CanvasWidget::Polygon(mut pg) => {
            pg.pg_point = cursor;
            let degrees = 
                get_horizontal_angle_of_vector(
                    pg.mid_point, 
                    cursor, 
                    );

            pg.degrees = degrees;
            pg.points = 
                build_polygon(
                    pg.mid_point, 
                    pg.pg_point, 
                    pg.poly_points,
                    pg.degrees,
                );
            
            Some(CanvasWidget::Polygon(pg))
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
            pl.mid_point = 
                get_mid_point(
                    line.0, 
                    line.1);
            pl.pl_point = 
                Point::new(
                    pl.mid_point.x + 100.0, 
                    pl.mid_point.y
                );
            pl.degrees = 
                get_horizontal_angle_of_vector(
                    pl.mid_point,
                    pl.pl_point,
                );
            
            Some(CanvasWidget::PolyLine(pl))
        },
        CanvasWidget::RightTriangle(mut tr) => {
            tr.mid_point = tr.points[1];
            let trans_pts = translate_geometry(&tr.points, Point::default(), tr.points[1]);
            let opp = Point::new(-trans_pts[2].x, -trans_pts[2].y);
            tr.tr_point = Point::new(opp.x+tr.points[1].x, opp.y+tr.points[1].y);
            if tr.points[1].x > tr.points[2].x {
                tr.degrees = 180.0;
            } else {
                tr.degrees = 0.0;
            }
            
            Some(CanvasWidget::RightTriangle(tr))
        },
        CanvasWidget::Text(mut txt) => {
            txt.degrees = 0.0;
            Some(CanvasWidget::Text(txt))
        }
    }
}

fn update_edited_widget(widget: CanvasWidget,
                        cursor: Point, 
                        index: Option<usize>, 
                        mid_point: bool,
                        other_point: bool,
                        status: DrawStatus,
                    ) -> CanvasWidget {
    match widget {
        CanvasWidget::None => {
            CanvasWidget::None
        },
        CanvasWidget::Arc(mut arc) => {
            if index.is_some() {
                arc.points[index.unwrap()] = cursor;
                if index.unwrap() == 1 {
                    arc.radius = arc.mid_point.distance(cursor);
                    arc.start_angle = get_angle_of_vectors(
                            arc.mid_point, 
                            Point::new(-arc.mid_point.x, arc.mid_point.y), 
                            cursor) + Radians::PI;
                    arc.end_angle = 
                            get_angle_of_vectors(
                                arc.mid_point, 
                                cursor, 
                                arc.points[2]) + arc.start_angle;
                }
                if index.unwrap() == 2 {
                    arc.end_angle = 
                        get_angle_of_vectors(
                            arc.points[0], 
                            arc.points[1], 
                            cursor
                        ) + arc.start_angle;
                }
                // calc the end_angle point        
                let r = arc.radius;
                let b = arc.end_angle.0;
                let point_b = Point::new(r*b.cos(), r*b.sin());
                arc.points[2] = translate_geometry(&vec![point_b], arc.mid_point, Point::default())[0];

            } else if mid_point {
                arc.points = 
                    translate_geometry(
                        &arc.points, 
                        cursor,
                        arc.mid_point, 
                        );
                arc.mid_point = cursor;
            }
            arc.status = status;
            CanvasWidget::Arc(arc)
        },
        CanvasWidget::Bezier(mut bz) => {
            if index.is_some() {
                bz.points[index.unwrap()] = cursor;
                bz.mid_point = get_mid_point(bz.points[0], bz.points[1]);
            } else if mid_point {
                bz.points = 
                    translate_geometry(
                        &bz.points, 
                        cursor,
                        bz.mid_point, 
                        );
                bz.mid_point = cursor;
            }
            let degrees = 
                get_horizontal_angle_of_vector(
                    bz.points[0],
                    bz.points[1], 
                );
            bz.degrees = degrees;
            bz.status = status;
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
                        &points, 
                        cursor,
                        cir.center,
                    );
                cir.center = cursor;
                cir.circle_point = points[0];
            }
            cir.status = status;
            CanvasWidget::Circle(cir)
        },
        CanvasWidget::Ellipse(mut ell) => {
           if mid_point {
                let points = 
                    translate_geometry(
                        &ell.points, 
                        cursor,
                        ell.center,
                    );
                ell.center = cursor;
                ell.points = points;
            }
            if index == Some(1) {
                let p1 = Point::new(cursor.x, ell.center.y);
                let vx = p1.distance(ell.center);
                let vy = ell.points[2].distance(ell.center);
                ell.points[1] = p1;
                dbg!(&p1);
                ell.radii = Vector{ x: vx, y: vy };
            } else if index == Some(2) {
                let p2 = Point::new(ell.center.x, cursor.y);
                let vx = ell.points[1].distance(ell.center);
                let vy = p2.distance(ell.center);
                ell.points[2] = p2;
                ell.radii = Vector{ x: vx, y: vy };
            }

            ell.status = status;
            CanvasWidget::Ellipse(ell)
        },
        CanvasWidget::Line(mut line) => {
            if index.is_some() {
                line.points[index.unwrap()] = cursor;
                line.mid_point = get_mid_point(line.points[0], line.points[1]);
            } else if mid_point {
                line.points = 
                    translate_geometry(
                        &line.points, 
                        cursor,
                        line.mid_point, 
                        );
                line.mid_point = cursor;
            }

            let degrees = 
                get_horizontal_angle_of_vector(
                    line.points[0],  
                    line.points[1], 
                );
            line.degrees = degrees;
            line.status = status;
            CanvasWidget::Line(line)
        },
        CanvasWidget::Polygon(mut pg) => {
            if other_point {
                pg.pg_point = cursor;
                pg.degrees = get_horizontal_angle_of_vector(pg.mid_point, cursor);
                pg.points = 
                    build_polygon(
                        pg.mid_point, 
                        pg.pg_point, 
                        pg.poly_points,
                        pg.degrees,
                );
            } else if mid_point {
                let trans_pts = 
                    translate_geometry(
                        &vec![pg.pg_point], 
                        cursor,
                        pg.mid_point, 
                    );
                pg.points = 
                    build_polygon(
                        cursor, 
                        trans_pts[0], 
                        pg.poly_points,
                        pg.degrees,
                    );
                pg.mid_point = cursor;
                pg.pg_point = trans_pts[0];
            }
            pg.status = status;
            CanvasWidget::Polygon(pg)
        },
        CanvasWidget::PolyLine(mut pl) => {
            if index.is_some() {
                pl.points[index.unwrap()] = cursor;
                let mid_point = 
                    get_mid_geometry(
                        &pl.points, 
                        Widget::PolyLine
                    );
                pl.pl_point = 
                    translate_geometry(
                        &vec![pl.pl_point], 
                        mid_point, 
                        pl.mid_point
                    )[0];
                pl.mid_point = mid_point;
                pl.degrees = 
                    get_horizontal_angle_of_vector(
                        pl.mid_point, 
                        pl.pl_point
                    );
            }  else if mid_point {
                let mut pts = pl.points.clone();
                pts.push(pl.pl_point);
                pts = 
                    translate_geometry(
                        &pts, 
                        cursor,
                        pl.mid_point, 
                    );
                pl.mid_point = cursor;
                pl.pl_point = pts.pop().unwrap();
                pl.points = pts;
            } else if other_point {
                let degrees = get_horizontal_angle_of_vector(pl.mid_point, cursor);
                let step_degrees = degrees-pl.degrees;
                pl.points = rotate_geometry(&pl.points, &pl.mid_point, &step_degrees, Widget::PolyLine);
                pl.pl_point = cursor;
                pl.degrees = degrees;
            }
            pl.status = status;
            CanvasWidget::PolyLine(pl)
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
                let mid = get_mid_point(tr.points[1], tr.points[2]);
                let dist_b_mid = Point::new(mid.x-tr.points[2].x, mid.y-tr.points[2].y);
                tr.tr_point = Point::new(tr.points[2].x+dist_b_mid.x, tr.points[2].y+dist_b_mid.y);
            } else if mid_point {
                let mut pts = tr.points.clone();
                pts.push(tr.tr_point);
                pts = 
                    translate_geometry(
                        &pts, 
                        cursor,
                        tr.mid_point, 
                    );
                tr.mid_point = cursor;
                tr.tr_point = pts.pop().unwrap();
                tr.points = pts;
            } else if other_point {
                let degrees = get_horizontal_angle_of_vector(tr.mid_point, cursor);
                let step_degrees = degrees-tr.degrees;
                tr.points = rotate_geometry(&tr.points, &tr.mid_point, &step_degrees, Widget::RightTriangle);
                tr.tr_point = cursor;
                tr.degrees = degrees;
            }
            tr.status = status;
            CanvasWidget::RightTriangle(tr)
        },
        CanvasWidget::Text(txt) => {
            CanvasWidget::Text(txt)
        }
    }
}

fn update_rotated_widget(widget: &mut CanvasWidget, 
                        step_degrees: f32,
                        status: Option<DrawStatus>,
                    ) -> (CanvasWidget, f32) {
    match widget {
        CanvasWidget::None => (CanvasWidget::None, 0.0),
        CanvasWidget::Arc(arc) => {
            arc.points = rotate_geometry(&arc.points, &arc.mid_point, &step_degrees, Widget::Arc);
            arc.start_angle = 
                get_angle_of_vectors(
                    arc.points[0], 
                    Point::new(-arc.points[0].x, arc.points[0].y), 
                    arc.points[1]) + Radians::PI;
            arc.end_angle = 
                get_angle_of_vectors(
                    arc.points[0], 
                    arc.points[1], 
                    arc.points[2]) + arc.start_angle;

            // calc the end_angle point        
            let r = arc.radius;
            let b = arc.end_angle.0;
            let point_b = Point::new(r*b.cos(), r*b.sin());

            arc.points[2] = translate_geometry(&vec![point_b], arc.mid_point, Point::default())[0];
            
            if status.is_some() {
                arc.status = status.unwrap();
            }
            (CanvasWidget::Arc(arc.clone()), Radians::into(arc.start_angle))
        },
        CanvasWidget::Bezier(bz) => {
            bz.points = rotate_geometry(&bz.points, &bz.mid_point, &step_degrees, Widget::Bezier);
            bz.degrees = get_horizontal_angle_of_vector(bz.mid_point, bz.points[1]);
            if status.is_some() {
                bz.status = status.unwrap();
            }
            (CanvasWidget::Bezier(bz.clone()), bz.degrees)
        },
        CanvasWidget::Circle(cir) => {
            (CanvasWidget::Circle(cir.clone()), 0.0)
        },
        CanvasWidget::Ellipse(ell) => {
            let rads = to_radians(&step_degrees) + ell.rotation.0;
            ell.rotation = Radians(rads);
            if status.is_some() {
                ell.status = status.unwrap();
            }
            (CanvasWidget::Ellipse(ell.clone()), to_degrees(&rads))
        },
        CanvasWidget::Line(ln) => {
            ln.points = rotate_geometry(&ln.points, &ln.mid_point, &step_degrees, Widget::Line);
            ln.degrees = get_horizontal_angle_of_vector(ln.mid_point, ln.points[1]);
            if status.is_some() {
                ln.status = status.unwrap();
            }
            (CanvasWidget::Line(ln.clone()), ln.degrees)
        },
        CanvasWidget::Polygon(pg) => {
            pg.points = rotate_geometry(&pg.points, &pg.mid_point, &step_degrees, Widget::Polygon);
            pg.pg_point = rotate_geometry(&[pg.pg_point], &pg.mid_point, &step_degrees, Widget::Line)[0];
            pg.degrees = get_horizontal_angle_of_vector(pg.mid_point, pg.pg_point);
            if status.is_some() {
                pg.status = status.unwrap();
            }
            (CanvasWidget::Polygon(pg.clone()), pg.degrees)
        },
        CanvasWidget::PolyLine(pl) => {
            let mut pts = pl.points.clone();
            pts.push(pl.pl_point);
            pts = rotate_geometry(&pts, &pl.mid_point, &step_degrees, Widget::PolyLine);
            pl.pl_point = pts.pop().unwrap();
            pl.points = pts;
            pl.degrees = get_horizontal_angle_of_vector(pl.mid_point, pl.pl_point);
            if status.is_some() {
                pl.status = status.unwrap();
            }
            (CanvasWidget::PolyLine(pl.clone()), pl.degrees)
        },
        CanvasWidget::RightTriangle(tr) => {
            let mut pts = tr.points.clone();
            pts.push(tr.tr_point);
            pts = rotate_geometry(&pts, &tr.mid_point, &step_degrees, Widget::RightTriangle);
            tr.tr_point = pts.pop().unwrap();
            tr.points = pts;
            tr.degrees = get_horizontal_angle_of_vector(tr.mid_point, tr.tr_point);
            if status.is_some() {
                tr.status = status.unwrap();
            }
            (CanvasWidget::RightTriangle(tr.clone()), tr.degrees)
        },
        CanvasWidget::Text(txt) => {
            (CanvasWidget::Text(txt.clone()), 0.0)
        }
    }
}

fn add_keypress(widget: &mut CanvasWidget, modified: Key) -> Option<CanvasWidget> {
    match widget {
        CanvasWidget::Text(txt) => {
            match modified.as_ref() {
                Key::Named(_) => (),
                Key::Character(c) => {
                    txt.content.push_str(c);
                },
                Key::Unidentified => (),
            }
            
            Some(CanvasWidget::Text(txt.clone()))
        },
        _ => None
    }
}

pub fn set_widget_mode_or_status(widget: CanvasWidget, 
                    mode: Option<DrawMode>,
                    status: Option<DrawStatus>,
                    ) -> CanvasWidget {
    match widget {
        CanvasWidget::None => {
            CanvasWidget::None
        },
        CanvasWidget::Arc(mut arc) => {
            if mode.is_some() {
                arc.draw_mode = mode.unwrap();
            }
            if status.is_some() {
                arc.status = status.unwrap();
            }
            CanvasWidget::Arc(arc)
        },
        CanvasWidget::Bezier(mut bz) => {
            if mode.is_some() {
                bz.draw_mode = mode.unwrap();
            }
            if status.is_some() {
                bz.status = status.unwrap();
            }
            CanvasWidget::Bezier(bz)
        },
        CanvasWidget::Circle(mut cir) => {
            if mode.is_some() {
                cir.draw_mode = mode.unwrap();
            }
            if status.is_some() {
                cir.status = status.unwrap();
            }
            CanvasWidget::Circle(cir)
        },
        CanvasWidget::Ellipse(mut ell) => {
            if mode.is_some() {
                ell.draw_mode = mode.unwrap();
            }
            if status.is_some() {
                ell.status = status.unwrap();
            }
            CanvasWidget::Ellipse(ell)
        },
        CanvasWidget::Line(mut ln) => {
            if mode.is_some() {
                ln.draw_mode = mode.unwrap();
            }
            if status.is_some() {
                ln.status = status.unwrap();
            }
            CanvasWidget::Line(ln)
        },
        CanvasWidget::PolyLine(mut pl) => {
            if mode.is_some() {
                pl.draw_mode = mode.unwrap();
            }
            if status.is_some() {
                pl.status = status.unwrap();
            }
            CanvasWidget::PolyLine(pl)
        },
        CanvasWidget::Polygon(mut pg) => {
            if mode.is_some() {
                pg.draw_mode = mode.unwrap();
            }
            if status.is_some() {
                pg.status = status.unwrap();
            }
            CanvasWidget::Polygon(pg)
        },
        CanvasWidget::RightTriangle(mut tr) => {
            if mode.is_some() {
                tr.draw_mode = mode.unwrap();
            }
            if status.is_some() {
                tr.status = status.unwrap();
            }
            CanvasWidget::RightTriangle(tr)
        },
        CanvasWidget::Text(mut txt) => {
            if mode.is_some() {
                txt.draw_mode = mode.unwrap();
            }
            if status.is_some() {
                txt.status = status.unwrap();
            }
            CanvasWidget::Text(txt)
        }
    }
}

// Adds a cursor position to the points then determines 
// if finish by returning the widget and the boolean
fn set_widget_point(widget: &CanvasWidget, cursor: Point) -> (CanvasWidget, bool) {
    match widget {
        CanvasWidget::None => (CanvasWidget::None, true),
        CanvasWidget::Arc(arc) => {
            let mut arc = arc.clone();
            arc.points.push(cursor);

            let finished = match arc.points.len() {
                1 => {
                    arc.mid_point = arc.points[0];
                    false
                },
                2 => {
                    arc.radius = arc.points[0].distance(arc.points[1]);
                    arc.start_angle = 
                        get_angle_of_vectors(
                            arc.points[0], 
                            Point::new(-arc.points[0].x, arc.points[0].y), 
                            arc.points[1]) + Radians::PI;
                    false
                },
                3 => {
                    arc.end_angle = 
                        get_angle_of_vectors(
                            arc.points[0], 
                            arc.points[1], 
                            cursor) + arc.start_angle;
                    // calc the end_angle point        
                    let r = arc.radius;
                    let b = arc.end_angle.0;
                    let point_b = Point::new(r*b.cos(), r*b.sin());
                    arc.points[2] = translate_geometry(&vec![point_b], arc.mid_point, Point::default())[0];
                    true
                },
                _ => false
            };

            (CanvasWidget::Arc(arc), finished)
        },
        CanvasWidget::Bezier(bezier) => {
            let mut bz = bezier.clone();
            bz.points.push(cursor);

            if bz.points.len() == 2 {
                bz.degrees = get_horizontal_angle_of_vector(bz.points[0], bz.points[1]);
            }
            let finished = if bz.points.len() == 3 {
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
            
            (CanvasWidget::Circle(cir), finished)
        },
        CanvasWidget::Ellipse(ell) => {
            let mut ell = ell.clone();
            let finished = if ell.points.len() == 0 {
                ell.points.push(cursor);
                false
            } else if ell.points.len() == 1 {
                let p1 = Point::new(cursor.x, ell.points[0].y);
                ell.points.push(p1);
                false
            } else if ell.points.len() == 2 {
                let p2 = Point::new(ell.points[0].x, cursor.y);
                ell.points.push(p2);
                true
            } else {
                false
            };
            
            (CanvasWidget::Ellipse(ell), finished)
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
                pg.degrees = get_horizontal_angle_of_vector(pg.mid_point, pg.pg_point)
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
            
            (CanvasWidget::RightTriangle(rt), finished)
        },
        CanvasWidget::Text(text) => {
            let mut txt = text.clone();
            
            let finished = if txt.position == Point::default() {
                txt.position = cursor;
                false
            } else {
                true
            };
            
            (CanvasWidget::Text(txt), finished)
        }
    }
}

fn find_closest_widget(curves: &HashMap<Id, CanvasWidget>, cursor: Point) -> Option<CanvasWidget> {
    let mut closest = f32::INFINITY;
    let mut closest_id = None;
    for (id, cw) in curves.iter() {
        let distance: f32 = get_distance_to_mid_point(cw, cursor);
        if distance < closest {
            closest = distance;
            closest_id = Some(id);
        }
    }

    let dc_opt = 
        match closest_id {
            Some(id) => curves.get(id),
            None => None,
        };

    match dc_opt {
        Some(widget) => Some(widget.clone()),
        None => None,
    }
}

// returns a bool if mid_point and an optional usize 
// if a point in points.
fn find_closest_point_index(widget: &CanvasWidget,
                            cursor: Point, 
                            ) -> (Option<usize>, bool, bool) {

    let mut point_dist: f32 = f32::INFINITY;
    let mut point_index = 0;

    match widget {
        CanvasWidget::None => (None, false, false),
        CanvasWidget::Arc(arc) => {
            for (idx, point) in arc.points.iter().enumerate() {
                // skip first point since its a mid_point too.
                if idx == 0 {
                    continue;
                } else {
                    let dist = cursor.distance(*point);
                    if  dist < point_dist {
                        point_index = idx;
                        point_dist = dist;
                    }
                }
                
            };
            
            let mid_dist = arc.mid_point.distance(cursor);

            if mid_dist < point_dist {
                (None, true, false)
            } else {
                (Some(point_index), false, false)
            }
        },
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
                (None, true, false)
            } else {
                (Some(point_index), false, false)
            }
        },
        CanvasWidget::Circle(cir) => {
            let center_dist = cursor.distance(cir.center);
            let point_dist = cursor.distance(cir.circle_point);
            if center_dist < point_dist {
                (None, true, false)
            } else {
                (Some(1), false, false)
            }
        }
        CanvasWidget::Ellipse(ell) => {
            let center_dist = cursor.distance(ell.center);
            let point_1_dist = cursor.distance(ell.points[1]);
            let point_2_dist = cursor.distance(ell.points[2]);
            if center_dist < point_1_dist && center_dist < point_2_dist {
                (None, true, false)
            } else if point_1_dist < point_2_dist {
                (Some(1), false, false)
            } else {
                (Some(2), false, false)
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
                (None, true, false)
            } else {
                (Some(point_index), false, false)
            }
        },
        CanvasWidget::Polygon(pg) => {
            let pg_center = cursor.distance(pg.mid_point);
            let pg_point = cursor.distance(pg.pg_point);
            if pg_center <= pg_point {
                (None, true, false)
            } else {
                (None, false, true)
            }
        },
        CanvasWidget::PolyLine(pl) => {
            for (idx, point) in pl.points.iter().enumerate() {
                let dist = cursor.distance(*point);
                if  dist < point_dist {
                    point_index = idx;
                    point_dist = dist;
                }
            };
            
            let mid_dist = pl.mid_point.distance(cursor);
            let pl_pt_dist = pl.pl_point.distance(cursor);

            if point_dist < mid_dist && point_dist < pl_pt_dist {
                (Some(point_index), false, false)
            } else if mid_dist < pl_pt_dist {
                (None, true, false)
            } else {
                (None, false, true)
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
            let tr_pt_dist = tr.tr_point.distance(cursor);

            if point_dist < mid_dist && point_dist < tr_pt_dist {
                (Some(point_index), false, false)
            } else if mid_dist < tr_pt_dist {
                (None, true, false)
            } else {
                (None, false, true)
            }
        },
        CanvasWidget::Text(_txt) => {
            (None, true, false)
        }
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

pub fn get_widget_id(widget: &CanvasWidget) -> Id {
    match widget {
        CanvasWidget::None => Id::new("None"),
        CanvasWidget::Arc(arc) => arc.id.clone(),
        CanvasWidget::Bezier(bz) => bz.id.clone(),
        CanvasWidget::Circle(cir) => cir.id.clone(),
        CanvasWidget::Ellipse(ell) => ell.id.clone(),
        CanvasWidget::Line(line) => line.id.clone(),
        CanvasWidget::PolyLine(pl) => pl.id.clone(),
        CanvasWidget::Polygon(pg) => pg.id.clone(),
        CanvasWidget::RightTriangle(tr) => tr.id.clone(),
        CanvasWidget::Text(txt) => txt.id.clone(),
    }
}

fn get_widget_degrees(widget: &CanvasWidget) -> Option<f32> {
    match widget {
        CanvasWidget::None => Some(0.0),
        CanvasWidget::Arc(arc) => Some(Radians::into(arc.start_angle)),
        CanvasWidget::Bezier(bezier) => Some(bezier.degrees),
        CanvasWidget::Circle(_circle) => Some(0.0),
        CanvasWidget::Ellipse(_ell) => Some(0.0),
        CanvasWidget::Line(line) => Some(line.degrees),
        CanvasWidget::PolyLine(poly_line) => Some(poly_line.degrees),
        CanvasWidget::Polygon(polygon) => Some(polygon.degrees),
        CanvasWidget::RightTriangle(right_triangle) => Some(right_triangle.degrees),
        CanvasWidget::Text(txt) => Some(txt.degrees),
    }
}

pub fn get_draw_mode_and_status(widget: &CanvasWidget) -> (DrawMode, DrawStatus) {
    match widget {
        CanvasWidget::None => (DrawMode::DrawAll, DrawStatus::Completed),
        CanvasWidget::Arc(arc) => (arc.draw_mode, arc.status),
        CanvasWidget::Bezier(bz) => (bz.draw_mode, bz.status),
        CanvasWidget::Circle(cir) => (cir.draw_mode, cir.status),
        CanvasWidget::Ellipse(ell) => (ell.draw_mode, ell.status),
        CanvasWidget::Line(ln) => (ln.draw_mode, ln.status),
        CanvasWidget::PolyLine(pl) => (pl.draw_mode, pl.status),
        CanvasWidget::Polygon(pg) => (pg.draw_mode, pg.status),
        CanvasWidget::RightTriangle(tr) => (tr.draw_mode, tr.status),
        CanvasWidget::Text(txt) => (txt.draw_mode, txt.status),
    }
}

fn get_distance_to_mid_point(widget: &CanvasWidget, cursor: Point) -> f32 {

        match &widget {
            CanvasWidget::None => f32::INFINITY,
            CanvasWidget::Arc(arc) => {
                cursor.distance(arc.mid_point)
            },
            CanvasWidget::Bezier(bz) => {
                cursor.distance(bz.mid_point)
            },
            CanvasWidget::Circle(cir) => {
                cursor.distance(cir.center)
            },
            CanvasWidget::Ellipse(ell) => {
                cursor.distance(ell.center)
            },
            CanvasWidget::Line(line) => {
                cursor.distance(line.mid_point)
            },
            CanvasWidget::Polygon(pg) => {
                cursor.distance(pg.mid_point)
            },
            CanvasWidget::PolyLine(pl) => {
                cursor.distance(pl.mid_point)
            },
            CanvasWidget::RightTriangle(tr) => {
                cursor.distance(tr.mid_point)
            },
            CanvasWidget::Text(txt) => {
                cursor.distance(txt.position)
            }
        }

}

pub fn get_mid_geometry(pts: &[Point], curve_type: Widget) -> Point {
    match curve_type {
        Widget::Arc => {
            get_mid_point(pts[0], pts[1])
        }
        Widget::Bezier => {
            get_mid_point(pts[0], pts[1])
        },
        Widget::Circle => {
            // return the center point
            pts[0]
        },
        Widget::Ellipse => {
            // return the center point
            pts[0]
        }
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
        Widget::Text => {
            pts[0]
        }
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

fn translate_geometry(pts: &Vec<Point>, 
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
fn rotate_geometry(
                    points: &[Point], 
                    mid_point: &Point, 
                    step_degrees: &f32, 
                    widget: Widget,
                    ) -> Vec<Point> {
    match widget {
        Widget::None => vec![],
        _ => {
            let theta = to_radians(step_degrees);
            let mut new_points = vec![];
            for point in points.iter() {
                let x_new = (point.x - mid_point.x) * theta.cos() - 
                                (point.y - mid_point.y) * theta.sin() + 
                                mid_point.x;
                let y_new = (point.x - mid_point.x) * theta.sin() + 
                                (point.y - mid_point.y) * theta.cos() + 
                                mid_point.y;

                new_points.push(Point { x: x_new, y: y_new })
            }
            
            new_points
        }
    }
}

// The first point is used to create a horizontal vector
pub fn get_horizontal_angle_of_vector(center: Point, p2: Point) -> f32 {
    let p1 = Point::new(center.x-10.0, center.y);
    let pts = 
        translate_geometry(
            &vec![p1, p2], 
            Point::default(), 
            center,
        );

    let angle = ((pts[0].y).atan2(pts[0].x) -
                        (pts[1].y).atan2(pts[1].x)) * -1.0;

    // Since beyond pi, angle goes negative
    let new_angle = if angle < 0.0 {
        2.0 * PI + angle
    } else {
        angle
    };

    to_degrees(&new_angle)
}

pub fn get_angle_of_vectors(center: Point, p1: Point, p2: Point) -> Radians {

    let pts = 
        translate_geometry(
            &vec![p1, p2], 
            Point::default(), 
            center,
        );

    let angle = ((pts[0].y).atan2(pts[0].x) -
                        (pts[1].y).atan2(pts[1].x)) * -1.0;
    
    // Since beyond pi, angle goes negative
    let new_angle = if angle < 0.0 {
        2.0 * PI + angle
    } else {
        angle
    };

    Radians::from(new_angle)
}

fn to_degrees(radians_f32: &f32) -> f32 {
    radians_f32 * 180.0/PI
}

fn to_radians(degrees: &f32) -> f32 {
    degrees * PI/180.0
}

fn build_polygon(mid_point: Point, pg_point: Point, poly_points: usize, mut degrees: f32) -> Vec<Point> {
    
    let angle = 2.0 * PI / poly_points as f32;
    let radius = mid_point.distance(pg_point);
    let mut points = vec![];
    for i in 0..poly_points {
        let x = mid_point.x + radius * (i as f32 * angle).sin();
        let y = mid_point.y + radius * (i as f32 * angle).cos();
        points.push(Point::new(x, y));
    }
    
    degrees += 180.0;
    let mut pts = rotate_geometry(&points, &mid_point, &degrees, Widget::Polygon);
    pts.push(pts[0]);
    pts

}

fn build_arc_path(arc: &Arc, 
                    draw_mode: DrawMode, 
                    pending_cursor: Option<Point>,
                    edit_point_index: Option<usize>, 
                    edit_mid_point: bool,
                    ) -> (Path, Point, Radians, Radians, Option<f32>, Option<f32>) {

    let mut pts = arc.points.clone();
    let mut mid_point = arc.mid_point;
    let mut start_angle = arc.start_angle;
    let mut end_angle = arc.end_angle;
    let mut radius = arc.radius;
    let mut degrees_left = None;
    let mut degrees_center = None;

    let path = Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                let new_arc = 
                    canvas::path::Arc{ 
                        center: arc.mid_point, 
                        radius: arc.radius, 
                        start_angle: arc.start_angle, 
                        end_angle: arc.end_angle, 
                    };
                p.arc(new_arc);
            },
            DrawMode::Edit => {
                pts = arc.points.clone();
                let cursor = pending_cursor.unwrap();

                if edit_mid_point {
                    pts = translate_geometry(
                        &pts, 
                        cursor,
                        mid_point, 
                        );
                    mid_point = cursor;
                } 
                if edit_point_index.is_some() && edit_point_index != Some(0) {
                    pts[edit_point_index.unwrap()] = cursor;
                    if edit_point_index == Some(1) {
                        radius = mid_point.distance(cursor);

                        start_angle = get_angle_of_vectors(
                            mid_point, 
                            Point::new(-mid_point.x, mid_point.y), 
                            cursor) + Radians::PI;
                    
                        end_angle = 
                            get_angle_of_vectors(
                                mid_point, 
                                cursor, 
                                pts[2]) + start_angle;
                        degrees_left = 
                            Some(get_horizontal_angle_of_vector(
                                mid_point, 
                                cursor, 
                            ));
                    } else if edit_point_index == Some(2) {
                        end_angle = 
                            get_angle_of_vectors(
                                mid_point, 
                                pts[1], 
                                cursor) + start_angle;
                        degrees_center = Some(to_degrees(&(end_angle.0-start_angle.0)));
                    }

                    // calc the end_angle point        
                    let r = radius;
                    let b = end_angle.0;
                    let point_b = Point::new(r*b.cos(), r*b.sin());
                    pts[2] = translate_geometry(&vec![point_b], mid_point, Point::default())[0];
                    
                }
                p.move_to(pts[0]);
                p.line_to(pts[1]);
                
                p.circle(mid_point, 3.0);
                p.circle(pts[1], 3.0);
                p.circle(pts[2], 3.0);
                
                let edit_arc = canvas::path::Arc{ 
                                            center: mid_point, 
                                            radius, 
                                            start_angle, 
                                            end_angle, 
                                        };
                p.arc(edit_arc);
            },
            DrawMode::New => {
                let cursor = pending_cursor.unwrap();
                let pts_len = arc.points.len();
                
                if pts_len == 1 {
                    p.move_to(arc.points[0]);
                    p.line_to(cursor);
                    start_angle = get_angle_of_vectors(
                            arc.points[0], 
                            Point::new(-arc.points[0].x, arc.points[0].y), 
                            cursor);
                    degrees_left = 
                        Some(get_horizontal_angle_of_vector(
                        mid_point, 
                        cursor, 
                    ));
                }
                if pts_len == 2 {
                    p.move_to(arc.points[0]);
                    p.line_to(arc.points[1]);
                    
                    end_angle = 
                        get_angle_of_vectors(
                            arc.points[0], 
                            arc.points[1], 
                            cursor) + arc.start_angle;

                    let radius = arc.points[0].distance(arc.points[1]);
                    let new_arc = canvas::path::Arc{ 
                                            center: arc.points[0], 
                                            radius, 
                                            start_angle: arc.start_angle, 
                                            end_angle, 
                                        };
                    p.arc(new_arc);

                    degrees_left = 
                        Some(get_horizontal_angle_of_vector(
                        mid_point, 
                        pts[1], 
                    ));
                    degrees_center = Some(to_degrees(&(end_angle.0-start_angle.0)));
                };
            },
            DrawMode::Rotate => {
                let rotated_arc = 
                    canvas::path::Arc{ 
                        center: arc.points[0], 
                        radius, 
                        start_angle: arc.start_angle, 
                        end_angle, 
                    };

                degrees_left = 
                    Some(get_horizontal_angle_of_vector(
                        mid_point, 
                        pts[1], 
                    ));

                p.arc(rotated_arc);
                p.move_to(arc.points[0]);
                p.line_to(arc.points[1]);
                p.move_to(arc.points[0]);
                p.line_to(arc.points[2]);
                p.circle(arc.mid_point, 3.0);
            },
        }
    });
    
    (path, mid_point, start_angle, end_angle, degrees_left, degrees_center)

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
                        &pts, 
                        pending_cursor.unwrap(),
                        mid_point, 
                        );
                    mid_point = pending_cursor.unwrap();
                } 
                if edit_point_index.is_some() {
                    pts[edit_point_index.unwrap()] = pending_cursor.unwrap();
                    mid_point = get_mid_point(pts[0], pts[1]);
                    
                    degrees = 
                        get_horizontal_angle_of_vector(
                            mid_point, 
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
                        get_horizontal_angle_of_vector(
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
                        &vec![cir_point], 
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

fn build_ellipse_path(ell: &Ellipse, 
                        draw_mode: DrawMode, 
                        pending_cursor: Option<Point>,
                        edit_point_index: Option<usize>, 
                        edit_mid_point: bool,
                    ) -> Path {
    Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                p.ellipse(Elliptical{ 
                    center: ell.center, 
                    radii: ell.radii, 
                    rotation: ell.rotation, 
                    start_angle: Radians(0.0), 
                    end_angle: Radians(2.0*PI) 
                });
            },
            DrawMode::Edit => {
                let mut center = ell.center;
                let mut radii = ell.radii;
                let mut p1 = ell.points[1];
                let mut p2 = ell.points[2];

                if edit_mid_point {
                    let points = translate_geometry(
                        &ell.points, 
                        pending_cursor.unwrap(),
                        center,
                    );
                    center = pending_cursor.unwrap();
                    p1 = points[1];
                    p2 = points[2];
                }

                if edit_point_index.is_some() {
                    let cursor = pending_cursor.unwrap();
                    if edit_point_index == Some(1) {
                        let vx = cursor.distance(center);
                        let vy = ell.points[2].distance(center);
                        p1 = Point::new(cursor.x, center.y);
                        radii = Vector{x: vx, y: vy};
                    } else {
                        let vx = ell.points[1].distance(center);
                        let vy = cursor.distance(center);
                        p2 = Point::new(center.x, cursor.y);
                        radii = Vector{x: vx, y: vy};
                    }
                }

                p.circle(center, 3.0);
                p.circle(p1, 3.0);
                p.circle(p2, 3.0);
                p.ellipse(Elliptical{ 
                    center, 
                    radii, 
                    rotation: ell.rotation, 
                    start_angle: Radians(0.0), 
                    end_angle: Radians(2.0*PI) 
                });
            },
            DrawMode::New => {
                let cursor = pending_cursor.unwrap();
                if ell.points.len() > 0 {
                    p.move_to(ell.points[0]);

                }
                if ell.points.len() == 0 {
                    p.circle(cursor, 3.0);
                } else if ell.points.len() == 1 {
                    let p1 = Point::new(cursor.x, ell.points[0].y);
                    p.line_to(p1);
                    let radius = p1.distance(ell.points[0]);
                    p.circle(ell.points[0], radius);
                } else if ell.points.len() == 2 {
                    p.line_to(ell.points[1]);
                    let p2 = Point::new(ell.points[0].x, cursor.y);
                    p.line_to(p2);
                    let vx = ell.points[1].distance(ell.points[0]);
                    let vy = p2.distance(ell.points[0]);
                    p.ellipse(Elliptical{ 
                        center: ell.points[0], 
                        radii: Vector{x: vx, y: vy}, 
                        rotation: Radians(0.0), 
                        start_angle: Radians(0.0), 
                        end_angle: Radians(2.0*PI) 
                    });
                }
                
            },
            DrawMode::Rotate => {
                let vx = ell.points[1].distance(ell.center);
                let vy = ell.points[2].distance(ell.center);
                p.ellipse(Elliptical{ 
                        center: ell.center, 
                        radii: Vector{x: vx, y: vy}, 
                        rotation: ell.rotation, 
                        start_angle: Radians(0.0), 
                        end_angle: Radians(2.0*PI) 
                    });
                p.circle(ell.center, 3.0);
            },
        }
    })
}

fn build_line_path(line: &Line, 
                    draw_mode: DrawMode, 
                    pending_cursor: Option<Point>,
                    edit_point_index: Option<usize>, 
                    edit_mid_point: bool,
                    degrees: Option<f32>,
                    ) -> (Path, f32, Point) {

    let mut degrees = match degrees {
        Some(d) => d,
        None => line.degrees,
    };
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
                        &pts, 
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
                    get_horizontal_angle_of_vector(
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
                    get_horizontal_angle_of_vector(
                        line.points[0], 
                        pending_cursor.unwrap(), 
                    );
            },
            DrawMode::Rotate => {
                p.move_to(line.points[0]);
                p.line_to(line.points[1]);
                p.circle(mid_point, 3.0);
            },
        }
    });

    (path, degrees, mid_point)

}

fn build_polygon_path(pg: &Polygon, 
                        draw_mode: DrawMode, 
                        pending_cursor: Option<Point>,
                        edit_mid_point: bool,
                        edit_other_point: bool,
                        degrees: Option<f32>,
                        ) -> (Path, f32, Point) {

    let mut degrees = match degrees {
        Some(d) => d,
        None => pg.degrees,
    };
    let mut mid_point = pg.mid_point;
    let mut pg_point = pg.pg_point;

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
                if edit_mid_point {
                    pg_point = translate_geometry(
                        &vec![pg.pg_point], 
                        pending_cursor.unwrap(),
                        pg.mid_point, 
                    )[0];
                    mid_point = pending_cursor.unwrap();
                } 
                if edit_other_point {
                    pg_point = pending_cursor.unwrap();
                    degrees = get_horizontal_angle_of_vector(pg.mid_point, pending_cursor.unwrap());
                }
                
                let pts = 
                    build_polygon(
                        mid_point, 
                        pg_point, 
                        pg.poly_points,
                        degrees
                    );
                
                for (index, pt) in pts.iter().enumerate() {
                    if index == 0 {
                        p.move_to(*pt);
                    } else {
                        p.line_to(*pt);
                    }
                }
                p.line_to(pts[0]);
                p.move_to(mid_point);
                p.line_to(pg_point);
                p.circle(mid_point, 3.0);
                p.circle(pg_point, 3.0);
            },
            DrawMode::New => {
                degrees = get_horizontal_angle_of_vector(pg.mid_point, pending_cursor.unwrap());

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
                p.circle(points[0], 3.0);
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
                p.line_to(pg.pg_point);
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
                        edit_other_point: bool,
                        degrees: Option<f32>,
                        ) -> (Path, f32, Point) {

    let mut degrees = match degrees {
        Some(d) => d,
        None => pl.degrees,
    };
    let mut pts = pl.points.clone();
    let mut mid_point = pl.mid_point;
    let mut pl_point = pl.pl_point;

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
                if edit_mid_point {
                    pts.push(pl_point);
                    pts = translate_geometry(
                        &pts, 
                        pending_cursor.unwrap(),
                        mid_point, 
                    );
                    pl_point = pts.pop().unwrap();
                    mid_point = pending_cursor.unwrap();
                } 
                if edit_point_index.is_some() {
                    pts[edit_point_index.unwrap()] = pending_cursor.unwrap();
                    mid_point = get_mid_geometry(&pts, Widget::PolyLine);
                    pl_point = translate_geometry(
                                    &vec![pl_point], 
                                    mid_point, 
                                    pl.mid_point,
                                )[0];
                }
                if edit_other_point {
                    degrees = get_horizontal_angle_of_vector(pl.mid_point, pending_cursor.unwrap());
                    let step_degrees = degrees-pl.degrees;
                    pts = rotate_geometry(&pts, &mid_point, &step_degrees, Widget::PolyLine);
                    pl_point = pending_cursor.unwrap();

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
                p.move_to(mid_point);
                p.line_to(pl_point);
                p.circle(pl_point, 3.0);
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
                // let (slope, intercept) = get_linear_regression(&pl.points);
                // let(p1, p2) = get_line_from_slope_intercept(&pl.points, slope, intercept);
                // mid_point = get_mid_point(p1, p2);
                // degrees = get_vertical_angle_of_vector(mid_point, p2);
                p.move_to(pl.mid_point);
                p.line_to(pl.pl_point);
                p.circle(mid_point, 3.0);
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
                            edit_other_point: bool,
                            degrees: Option<f32>,
                        ) -> (Path, f32, Point, Point) {

    let mut pts = tr.points.clone();
    let mut mid_point = tr.mid_point;
    let mut tr_point = tr.tr_point;
    let mut degrees = match degrees {
        Some(d) => d,
        None => tr.degrees,
    };

    let path = Path::new(|p| {
        match draw_mode {
            DrawMode::DrawAll => {
                p.move_to(tr.points[0]);
                p.line_to(tr.points[1]);
                p.line_to(tr.points[2]);
                p.line_to(tr.points[0]);
            },
            DrawMode::Edit => {
                pts.push(tr_point);
                if edit_mid_point {
                    pts = translate_geometry(
                        &pts, 
                        pending_cursor.unwrap(),
                        mid_point, 
                    );
                    tr_point = pts.pop().unwrap();
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
                    let mid = get_mid_point(pts[1], pts[2]);
                    let dist_b_mid = Point::new(mid.x-pts[2].x, mid.y-pts[2].y);
                    tr_point = Point::new(
                                    tr.points[2].x+dist_b_mid.x, 
                                    tr.points[2].y+dist_b_mid.y);
                }
                if edit_other_point {
                    degrees = 
                        get_horizontal_angle_of_vector(
                            tr.mid_point, 
                            pending_cursor.unwrap()
                        );
                    let step_degrees = degrees-tr.degrees;
                    pts = rotate_geometry(
                            &pts, 
                            &mid_point, 
                            &step_degrees, 
                            Widget::RightTriangle
                        );
                    tr_point = pending_cursor.unwrap();
                }

                p.move_to(pts[0]);
                p.line_to(pts[1]);
                p.line_to(pts[2]);
                p.line_to(pts[0]);
                p.move_to(mid_point);
                p.line_to(tr_point);

                p.circle(pts[0], 3.0);
                p.circle(pts[1], 3.0);
                p.circle(pts[2], 3.0);
                p.circle(mid_point, 3.0);
                p.circle(tr_point, 3.0);
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

                p.move_to(tr.mid_point);
                p.line_to(tr.tr_point);

                p.circle(tr.mid_point, 3.0);
                p.circle(tr.tr_point, 3.0);
            },
        }
    });

    (path, degrees, mid_point, tr_point)

}

fn build_text_path (txt: &Text, 
                    draw_mode: DrawMode, 
                    _pending_cursor: Option<Point>,
                    _degrees: f32,
                    blink: bool,
                    ) -> canvas::Text {

    match draw_mode {
        DrawMode::DrawAll => {
            canvas::Text {
                content: txt.content.clone(),
                position: txt.position,
                color: txt.color,
                size: txt.size,
                line_height: txt.line_height,
                font: txt.font,
                horizontal_alignment: txt.horizontal_alignment,
                vertical_alignment: txt.vertical_alignment,
                shaping: txt.shaping,
            }
        },
        DrawMode::Edit => {
            canvas::Text {
                content: txt.content.clone(),
                position: txt.position,
                color: txt.color,
                size: txt.size,
                line_height: txt.line_height,
                font: txt.font,
                horizontal_alignment: txt.horizontal_alignment,
                vertical_alignment: txt.vertical_alignment,
                shaping: txt.shaping,
            }
        },
        DrawMode::New => {
            let mut content = txt.content.clone();
            if blink {
                dbg!("pushing |");
                content.push_str("|");
            };
            dbg!(&content);
            canvas::Text {
                content,
                position: txt.position,
                color: txt.color,
                size: txt.size,
                line_height: txt.line_height,
                font: txt.font,
                horizontal_alignment: txt.horizontal_alignment,
                vertical_alignment: txt.vertical_alignment,
                shaping: txt.shaping,
            }
        },
        DrawMode::Rotate => {
            canvas::Text {
                content: txt.content.clone(),
                position: txt.position,
                color: txt.color,
                size: txt.size,
                line_height: txt.line_height,
                font: txt.font,
                horizontal_alignment: txt.horizontal_alignment,
                vertical_alignment: txt.vertical_alignment,
                shaping: txt.shaping,
            }
        },
    }

                  
}






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
    let degrees = get_horizontal_angle_of_vector(center, p2);
    dbg!(degrees);

    let p2 = Point::new(20.0, 10.0);
    let degrees = get_horizontal_angle_of_vector(center, p2);
    dbg!(degrees);

    let p2 = Point::new(0.0, -10.0);
    let degrees = get_horizontal_angle_of_vector(center, p2);
    dbg!(degrees);

    let p2 = Point::new(-20.0, 0.0);
    let degrees = get_horizontal_angle_of_vector(center, p2);
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
