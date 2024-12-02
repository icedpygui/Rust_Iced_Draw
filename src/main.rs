//! This example showcases an interactive `Canvas` for drawing Bézier curves.
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use colors::{get_rgba_from_canvas_draw_color, DrawCanvasColor};
use iced::keyboard::key;
use iced::widget::{button, column, container, 
    pick_list, radio, row, text, text_input, vertical_space};
use iced::{event, keyboard, Color, Element, Event, Point, Radians, Subscription, Theme, Vector};

use serde::{Deserialize, Serialize};

mod draw_canvas;
use draw_canvas::{get_vertical_angle_of_vector, Arc, Bezier, CanvasWidget, Circle, DrawCurve, DrawMode, Line, PolyLine, Polygon, RightTriangle, Widget};
mod colors;


pub fn main() -> iced::Result {
    iced::application("Drawing Tool - Iced", Example::update, Example::view)
        .theme(|_| Theme::CatppuccinMocha)
        .subscription(Example::subscription)
        .antialiasing(true)
        .centered()
        .run()
}

#[derive(Default)]
struct Example {
    state: draw_canvas::State,
    curves: Vec<DrawCurve>,
}

#[derive(Debug, Clone)]
enum Message {
    AddCurve(DrawCurve),
    Clear,
    DeleteLast,
    ModeSelected(String),
    RadioSelected(Widget),
    Event(Event),
    Load,
    Save,
    ColorSelected(String),
    PolyInput(String),
}

impl Example {
    fn update(&mut self, message: Message) {
        match message {
            Message::AddCurve(draw_curve) => {
                if draw_curve.edit_curve_index.is_some() && 
                    !self.curves.is_empty(){
                    self.curves[draw_curve.edit_curve_index.unwrap()] = draw_curve.clone();
                } else {
                    self.curves.push(draw_curve);
                }
                
                self.state.request_redraw();
            }
            Message::Clear => {
                self.state = draw_canvas::State::default();
                self.curves.clear();
            }
            Message::DeleteLast => {
                if self.curves.is_empty() {
                    return
                }
                self.curves.remove(self.curves.len()-1);
                self.state.request_redraw();
            }
            Message::ModeSelected(mode) => {
                let mode = DrawMode::to_enum(mode.clone());
                match mode {
                    DrawMode::DrawAll => {
                        self.state.edit_widget_index = None;
                        self.state.draw_mode = DrawMode::DrawAll;
                    },
                    DrawMode::Edit => {
                        if self.curves.is_empty() {
                            return
                        }
                        self.state.edit_widget_index = Some(0);
                        self.state.draw_mode = DrawMode::Edit;
                    },
                    DrawMode::New => {
                        self.state.edit_widget_index = None;
                        self.state.draw_mode = DrawMode::New;
                    },
                    DrawMode::Rotate => {
                        self.state.edit_widget_index = None;
                        self.state.draw_mode = DrawMode::Rotate;
                    },
                }
                
                self.state.request_redraw();
            },
            Message::RadioSelected(choice) => {
                match choice {
                    Widget::Arc => {
                        self.state.selected_widget  = 
                            CanvasWidget::Arc(
                                Arc {
                                    points: vec![],
                                    mid_point: Point::default(),
                                    radius: 0.0,
                                    color: self.state.selected_color,
                                    width: self.state.draw_width,
                                    degrees: 0.0,
                                    draw_mode: self.state.draw_mode,
                                }
                            );
                        self.state.selected_radio_widget = Some(Widget::Arc);
                    },
                    Widget::Bezier => {
                        self.state.selected_widget  = 
                            CanvasWidget::Bezier(
                                Bezier {
                                    points: vec![],
                                    mid_point: Point::default(),
                                    color: self.state.selected_color,
                                    width: self.state.draw_width,
                                    degrees: 0.0,
                                    draw_mode: self.state.draw_mode,
                                }
                            );
                        self.state.selected_radio_widget = Some(Widget::Bezier);
                    },
                    Widget::Circle => {
                        self.state.selected_widget = 
                            CanvasWidget::Circle(
                                Circle {
                                    center: Point::default(),
                                    circle_point: Point::default(),
                                    radius: 0.0,
                                    color: self.state.selected_color,
                                    width: self.state.draw_width,
                                    draw_mode: self.state.draw_mode,
                                }
                            );
                        self.state.selected_radio_widget = Some(Widget::Circle);
                    },
                    Widget::Ellipse => {
                        self.state.selected_widget = 
                            CanvasWidget::Ellipse(
                                draw_canvas::Ellipse {
                                    center: Point::default(),
                                    color: self.state.selected_color,
                                    width: self.state.draw_width,
                                    draw_mode: self.state.draw_mode,
                                    points: vec![],
                                    ell_point: Point::default(),
                                    radii: Vector::ZERO,
                                    rotation: Radians::PI,
                                    start_angle: Radians::PI,
                                    end_angle: Radians::PI,
                                    degrees: 0.0,
                                }
                            );
                        self.state.selected_radio_widget = Some(Widget::Circle);
                    },
                    Widget::Line => {
                        self.state.selected_widget = 
                            CanvasWidget::Line(
                                Line {
                                    points: vec![],
                                    mid_point: Point::default(),
                                    color: self.state.selected_color,
                                    width: self.state.draw_width,
                                    degrees: 0.0,
                                    draw_mode: self.state.draw_mode,
                                }
                            );
                        self.state.selected_radio_widget = Some(Widget::Line);
                    },
                    Widget::PolyLine => {
                        self.state.selected_widget = 
                            CanvasWidget::PolyLine(
                                PolyLine {
                                    points: vec![],
                                    poly_points: self.state.selected_poly_points,
                                    mid_point: Point::default(),
                                    pl_point: Point::default(),
                                    color: self.state.selected_color,
                                    width: self.state.draw_width,
                                    degrees: 0.0,
                                    draw_mode: self.state.draw_mode,
                                }
                            );
                        self.state.selected_radio_widget = Some(Widget::PolyLine);
                    },
                    Widget::Polygon => {
                        self.state.selected_widget = 
                            CanvasWidget::Polygon(
                                Polygon {
                                    points: vec![],
                                    poly_points: self.state.selected_poly_points,
                                    mid_point: Point::default(),
                                    pg_point: Point::default(),
                                    color: self.state.selected_color,
                                    width: self.state.draw_width,
                                    degrees: 0.0,
                                    draw_mode: self.state.draw_mode,
                                }
                            );
                        self.state.selected_radio_widget = Some(Widget::Polygon);
                    },
                    Widget::RightTriangle => {
                        self.state.selected_widget = 
                            CanvasWidget::RightTriangle(
                                RightTriangle { 
                                    points: vec![], 
                                    mid_point: Point::default(),
                                    tr_point: Point::default(), 
                                    color: self.state.selected_color,
                                    width: self.state.draw_width,
                                    degrees: 0.0,
                                    draw_mode: self.state.draw_mode,
                                }
                            );
                        self.state.selected_radio_widget = Some(Widget::RightTriangle);
                    },
                    Widget::None => (),
                } 
            },
            Message::Event(Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            })) => { 
                self.state.escape_pressed = true;
            },
            Message::Event(Event::Keyboard(keyboard::Event::KeyReleased {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            })) => { 
                self.state.escape_pressed = false;
            },
            Message::Event(_) => (),
            Message::Load => {
                let path = Path::new("./resources/data.json");
                let data = fs::read_to_string(path).expect("Unable to read file");
                let widgets = serde_json::from_str(&data).expect("Unable to parse");
                self.curves = import_widgets(widgets);
                self.state.request_redraw();
            }
            Message::Save => {
                let path = Path::new("./resources/data.json");
                let widgets = convert_to_export(&self.curves);
                let _ = save(path, &widgets);
            }
            Message::ColorSelected(color_str) => {
                let f: DrawCanvasColor = match color_str.as_str() {
                    "Primary" => DrawCanvasColor::PRIMARY,
                    "Secondary" => DrawCanvasColor::SECONDARY,
                    "Success" => DrawCanvasColor::SUCCESS,
                    "Danger" => DrawCanvasColor::DANGER,
                    _ => DrawCanvasColor::WHITE,
                };
                self.state.selected_color = Color::from(get_rgba_from_canvas_draw_color(f));
            },
            Message::PolyInput(input) => {
                // little error checking
                self.state.selected_poly_points_str = input.clone();
                if !input.is_empty() {
                    self.state.selected_poly_points = input.parse().unwrap();
                } else {
                    self.state.selected_poly_points = 4; //default
                }
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {

        event::listen().map(Message::Event)

    }

    fn view(&self) -> Element<Message> {
        let clear_btn = 
            button(
                "Clear")
                .on_press(Message::Clear)
                .into();

        let arc = 
            radio(
                "Arc",
                Widget::Arc,
                self.state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let biezer = 
            radio(
                "Beizer",
                Widget::Bezier,
                self.state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let circle = 
            radio(
                "Circle",
                Widget::Circle,
                self.state.selected_radio_widget,
                Message::RadioSelected,
                ).into();
        
        let eliptical = 
            radio(
                "Beizer",
                Widget::Bezier,
                self.state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let line = 
            radio(
                "Line",
                Widget::Line,
                self.state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let polygon = 
            radio(
                "Polygon",
                Widget::Polygon,
                self.state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let polyline = 
            radio(
                "PolyLine",
                Widget::PolyLine,
                self.state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let r_triangle = 
            radio(
                "Right Triangle",
                Widget::RightTriangle,
                self.state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let mode = self.state.draw_mode.string();

        let draw_mode = 
            text(format!("Mode = {}", mode))
            .into();

        let del_last = 
            button("Delete Last")
                .on_press(Message::DeleteLast)
                .into();

        let color_opt = 
            [
            "Primary".to_string(),
            "Secondary".to_string(),
            "Success".to_string(),
            "Danger".to_string(),
            "White".to_string(),
            ];

        let colors = 
            pick_list(
                color_opt, 
                self.state.selected_color_str.clone(), 
                Message::ColorSelected).into();

        let widths = text(format!("widths = {}", 2.0)).into();

        let poly_pts_input = 
            text_input("Poly Points(3)", 
                        &self.state.selected_poly_points_str)
                .on_input(Message::PolyInput)
                .into();

        let mode_options = 
            vec![
                "None".to_string(), 
                "New".to_string(), 
                "Edit".to_string(), 
                "Rotate".to_string()
                ];

        let mode = 
        pick_list(
            mode_options, 
            Some(self.state.draw_mode.string()), 
            Message::ModeSelected).into();

        let save = 
            button("Save")
                .padding(5.0)
                .on_press(Message::Save)
                .into();

        let load = 
            button("Load")
                .padding(5.0)
                .on_press(Message::Load)
                .into();
        
        let load_save_row = 
            row(vec![load, save])
                .spacing(5.0)
                .into();

        let instructions = 
            text("Start:\n Select a curve.\n\nDraw:\nUse left mouse button, click and move move then click again.\n\nCancel Draw:\nHold down esc and press left mouse button to cancel drawing.").into();
         
        let col = 
            column(vec![
            clear_btn,
            arc, 
            biezer, 
            circle,
            eliptical, 
            line,
            polygon,
            polyline,
            r_triangle,
            draw_mode,
            mode,
            poly_pts_input,
            load_save_row,
            colors,
            widths,
            del_last,
            vertical_space().height(50.0).into(),
            instructions,
            ])
            .width(150.0)
            .spacing(10.0)
            .padding(10.0)
            .into();

        

        let draw =  
            container(self.state
                .view(&self.curves)
                .map(Message::AddCurve))
                .into();
        
        row(vec![col, draw]).into()
    }

}

pub fn save(path: impl AsRef<Path>, data: &impl Serialize) -> std::io::Result<()> {
    let mut w = BufWriter::new(File::create(path).expect("unable to create file"));
    serde_json::to_writer_pretty(&mut w, data).expect("unable to format data");
    w.write_all(b"\n").expect("unable to append to buffer");
    w.flush().expect("unable to flush buffer");
    Ok(())
}

// iced Point does not derive any serialization 
// so had to use own version for saving data.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ExportPoint{
    x: f32,
    y: f32,
}

impl ExportPoint {
    fn convert(point: &Point) -> Self {
        ExportPoint {x: point.x, y: point.y}
    }

    pub fn distance(&self, to: Self) -> f32
    {
        let a = self.x - to.x;
        let b = self.y - to.y;

        a.hypot(b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ExportColor {
    /// Red component, 0.0 - 1.0
    pub r: f32,
    /// Green component, 0.0 - 1.0
    pub g: f32,
    /// Blue component, 0.0 - 1.0
    pub b: f32,
    /// Transparency, 0.0 - 1.0
    pub a: f32,
}

impl ExportColor {
    pub const fn from_rgba(color: &Color) -> ExportColor {
        ExportColor { r: color.r, g: color.g, b: color.b, a: color.a }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportWidget {
    pub name: Widget,
    pub points: Vec<ExportPoint>,
    pub poly_points: usize,
    pub mid_point: ExportPoint,
    pub other_point: ExportPoint,
    pub color: ExportColor,
    pub width: f32,
}

#[allow(clippy::redundant_closure)]
fn import_widgets(widgets: Vec<ExportWidget>) -> Vec<DrawCurve> {
    
    let mut vec_dc = vec![];

    for widget in widgets.iter() {
        let points: Vec<Point> = widget.points.iter().map(|p| convert_to_point(p)).collect();
        let mid_point = convert_to_point(&widget.mid_point);
        let other_point = convert_to_point(&widget.other_point);
        let color = convert_to_color(&widget.color);
        let width = widget.width;
        let draw_mode = DrawMode::DrawAll;

        match widget.name {
            Widget::None => {
                vec_dc.push(DrawCurve{
                    widget: CanvasWidget::None,
                    edit_curve_index: None,
                })
            },
            Widget::Arc => {
                let point = points[1].clone();
                let arc = Arc {
                    points: points,
                    mid_point,
                    radius: 0.0,
                    color,
                    width,
                    degrees: get_vertical_angle_of_vector(mid_point, point),
                    draw_mode,
                };
                vec_dc.push(DrawCurve {
                    widget: CanvasWidget::Arc(arc),
                    edit_curve_index: None,
                });
            },
            Widget::Bezier => {
                let point = points[1].clone();
                let bz = Bezier {
                    points: points,
                    mid_point,
                    color,
                    width,
                    degrees: get_vertical_angle_of_vector(mid_point, point),
                    draw_mode,
                };
                vec_dc.push(DrawCurve {
                    widget: CanvasWidget::Bezier(bz),
                    edit_curve_index: None,
                });
            },
            Widget::Circle => {
                let cir = Circle {
                    center: mid_point,
                    circle_point: convert_to_point(&widget.points[0]),
                    radius: widget.mid_point.distance(widget.points[0]),
                    color,
                    width,
                    draw_mode,
                };
                vec_dc.push(DrawCurve {
                    widget: CanvasWidget::Circle(cir),
                    edit_curve_index: None,
                });
            },
            Widget::Ellipse => {
                let cir = Circle {
                    center: mid_point,
                    circle_point: convert_to_point(&widget.points[0]),
                    radius: widget.mid_point.distance(widget.points[0]),
                    color,
                    width,
                    draw_mode,
                };
                vec_dc.push(DrawCurve {
                    widget: CanvasWidget::Circle(cir),
                    edit_curve_index: None,
                });
            },
            Widget::Line => {
                let point = points[1].clone();
                let ln = Line {
                    points,
                    mid_point,
                    color,
                    width,
                    degrees: get_vertical_angle_of_vector(mid_point, point),
                    draw_mode,
                };
                vec_dc.push(DrawCurve {
                    widget: CanvasWidget::Line(ln),
                    edit_curve_index: None,
                });
            },
            Widget::Polygon => {
                let pg = Polygon {
                    points,
                    poly_points: widget.poly_points,
                    mid_point,
                    pg_point: other_point,
                    color,
                    width,
                    degrees: get_vertical_angle_of_vector(mid_point, other_point),
                    draw_mode,
                };
                vec_dc.push(DrawCurve {
                    widget: CanvasWidget::Polygon(pg),
                    edit_curve_index: None,
                });
            },
            Widget::PolyLine => {
                let pl = PolyLine {
                    points,
                    poly_points: widget.poly_points,
                    mid_point,
                    pl_point: other_point,
                    color,
                    width,
                    degrees: get_vertical_angle_of_vector(mid_point, other_point),
                    draw_mode,
                };
                vec_dc.push(DrawCurve {
                    widget: CanvasWidget::PolyLine(pl),
                    edit_curve_index: None,
                });
            },
            Widget::RightTriangle => {
                let point = points[0].clone();
                let tr = RightTriangle {
                    points,
                    mid_point,
                    tr_point: other_point,
                    color,
                    width,
                    degrees: get_vertical_angle_of_vector(mid_point, point),
                    draw_mode,
                };
                vec_dc.push(DrawCurve {
                    widget: CanvasWidget::RightTriangle(tr),
                    edit_curve_index: None,
                });
            },
        }
    }

    vec_dc

}

fn convert_to_export(curves: &[DrawCurve]) -> Vec<ExportWidget> {
    let mut widgets = vec![];
    for curve in curves.iter() {
        widgets.push(curve.widget.clone())
    }   
    
    let mut export = vec![];

    for widget in widgets.iter() {

        let (name, 
            points, 
            mid_point,
            other_point, 
            poly_points, 
            color, 
            width, 
            ) = 
            match widget {
                CanvasWidget::None => {
                    (Widget::None, &vec![], Point::default(), Point::default(), 0, Color::TRANSPARENT, 0.0,)
                },
                CanvasWidget::Arc(arc) => {
                    (Widget::Arc, &arc.points, arc.mid_point, Point::default(), 0, arc.color, arc.width)
                },
                CanvasWidget::Bezier(bz) => {
                    (Widget::Bezier, &bz.points, bz.mid_point, Point::default(), 0, bz.color, bz.width)
                },
                CanvasWidget::Circle(cir) => {
                    (Widget::Circle, &vec![cir.circle_point], cir.center, cir.circle_point, 0, cir.color, cir.width)
                },
                CanvasWidget::Ellipse(ell) => {
                    (Widget::Ellipse, &vec![ell.ell_point], ell.center, ell.ell_point, 0, ell.color, ell.width)
                },
                CanvasWidget::Line(ln) => {
                    (Widget::Line, &ln.points, ln.mid_point, Point::default(), 0, ln.color, ln.width)
                },
                CanvasWidget::Polygon(pg) => {
                    (Widget::Polygon, &pg.points, pg.mid_point, pg.pg_point, pg.poly_points, pg.color, pg.width)
                },
                CanvasWidget::PolyLine(pl) => {
                    (Widget::PolyLine, &pl.points, pl.mid_point, pl.pl_point, pl.poly_points, pl.color, pl.width)
                },
                CanvasWidget::RightTriangle(tr) => {
                    (Widget::RightTriangle, &tr.points, tr.mid_point, Point::default(), 3, tr.color, tr.width)
                },
        };

        let x_color = ExportColor::from_rgba(&color);
        let x_mid_pt = ExportPoint::convert(&mid_point);
        let x_other_point = ExportPoint::convert(&other_point);
        let mut x_points = vec![];
        for point in points.iter() {
            x_points.push(ExportPoint::convert(point));
        }
        
        export.push(
            ExportWidget{
                name,
                points: x_points,
                poly_points, 
                mid_point: x_mid_pt,
                other_point: x_other_point, 
                color: x_color, 
                width,  
            })
    }
    
    export

}

fn convert_to_point(point: &ExportPoint) -> Point {
    Point { x: point.x, y: point.y }
}

fn convert_to_color(color: &ExportColor) -> Color {
    Color::from_rgba(color.r, color.g, color.b, color.a)
}
