//! This example showcases an interactive `Canvas` for drawing Bézier curves.
use std::fmt::{self, Debug};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use colors::{get_rgba_from_canvas_draw_color, DrawCanvasColor};
use iced::keyboard::key;
use iced::widget::{button, column, container, pick_list, radio, row, text, text_input, vertical_space};
use iced::{event, keyboard, Color, Element, Event, Point, Subscription, Theme};

use serde::{Deserialize, Serialize};

mod draw_canvas;
use draw_canvas::{CanvasMode, DrawCurve, IpgCanvasWidget};
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
    AddCurve(draw_canvas::DrawCurve),
    Clear,
    DeleteLast,
    Edit,
    EditNext,
    RadioSelected(IpgCanvasWidget),
    Event(Event),
    Load,
    Save,
    ColorSelected(String),
    PolyInput(String),
}

impl Example {
    fn update(&mut self, message: Message) {
        match message {
            Message::AddCurve(curve) => {
                if self.state.curve_to_edit.is_some() {
                    self.curves[self.state.curve_to_edit.unwrap()] = curve;
                } else {
                    self.curves.push(curve);
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
            Message::Edit => {
                // edit button toggles mode
                if self.state.canvas_mode == CanvasMode::Edit {
                    self.state.canvas_mode = CanvasMode::Select;
                    self.state.curve_to_edit = None;
                    self.state.request_redraw();
                    return
                } else {
                    self.state.canvas_mode = CanvasMode::Edit;
                    self.state.curve_to_edit = Some(0);
                    self.state.edit_draw_curve = self.curves[0].clone();
                }
                
                if self.curves.is_empty() {
                    return
                }
                
                self.state.request_redraw();
            },
            Message::EditNext => {
                let idx = if self.state.curve_to_edit.is_none() {
                    return
                } else {
                    self.state.curve_to_edit.unwrap()
                };

                self.state.curve_to_edit = if idx > self.curves.len()-1 {
                    Some(0)
                } else {
                    Some(idx + 1)
                };
                self.state.edit_draw_curve = self.curves[self.state.curve_to_edit.unwrap()].clone();
                self.state.request_redraw();
            },
            Message::RadioSelected(choice) => {
                self.state.selection = choice;
                match choice {
                    IpgCanvasWidget::None => (),
                    IpgCanvasWidget::Bezier => {
                        self.state.make_selection(IpgCanvasWidget::Bezier);
                        self.state.set_indexes(3);
                        self.state.set_color(Color::WHITE);
                        self.state.set_width(2.0);
                    },
                    IpgCanvasWidget::Circle => {
                        self.state.make_selection(IpgCanvasWidget::Circle);
                        self.state.set_indexes(2);
                        self.state.set_color(Color::WHITE);
                        self.state.set_width(2.0);
                    },
                    IpgCanvasWidget::Line => {
                        self.state.make_selection(IpgCanvasWidget::Line);
                        self.state.set_indexes(2);
                        self.state.set_color(Color::WHITE);
                        self.state.set_width(2.0);
                    },
                    IpgCanvasWidget::PolyLine => {
                        self.state.make_selection(IpgCanvasWidget::PolyLine);
                        self.state.set_indexes(5);
                        self.state.set_color(Color::WHITE);
                        self.state.set_width(2.0);
                    },
                    IpgCanvasWidget::Polygon => {
                        self.state.make_selection(IpgCanvasWidget::Polygon);
                        self.state.set_indexes(2);
                        self.state.set_color(Color::WHITE);
                        self.state.set_width(2.0);
                    },
                    IpgCanvasWidget::Rectangle => {
                        self.state.make_selection(IpgCanvasWidget::Rectangle);
                        self.state.set_indexes(2);
                        self.state.set_color(Color::WHITE);
                        self.state.set_width(2.0);
                    },
                    IpgCanvasWidget::RightTriangle => {
                        self.state.make_selection(IpgCanvasWidget::RightTriangle);
                        self.state.set_indexes(3);
                        self.state.set_color(Color::WHITE);
                        self.state.set_width(2.0);
                    },
                    IpgCanvasWidget::Triangle => {
                        self.state.make_selection(IpgCanvasWidget::Triangle);
                        self.state.set_indexes(3);
                        self.state.set_color(Color::WHITE);
                        self.state.set_width(2.0);
                    },
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
                let curves = serde_json::from_str(&data).expect("Unable to parse");
                self.curves = convert_to_iced_point_color(curves);
                self.state.request_redraw();
            }
            Message::Save => {
                let path = Path::new("./resources/data.json");
                let curves = convert_to_draw_point_color(&self.curves);
                let _ = save(path, &curves);
            }
            Message::ColorSelected(color_str) => {
                let f: DrawCanvasColor = match color_str.as_str() {
                    "Primary" => DrawCanvasColor::PRIMARY,
                    "Secondary" => DrawCanvasColor::SECONDARY,
                    "Success" => DrawCanvasColor::SUCCESS,
                    "Danger" => DrawCanvasColor::DANGER,
                    _ => DrawCanvasColor::WHITE,
                };
                self.state.selected_color_str = Some(color_str);
                self.state.selected_color = Color::from(get_rgba_from_canvas_draw_color(f));
            },
            Message::PolyInput(input) => {
                // little error checking
                if input != "" {
                    self.state.poly_points = input.parse().unwrap();
                } else {
                    self.state.poly_points = 3;
                }
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {

        event::listen().map(Message::Event)

    }

    fn view(&self) -> Element<Message> {
        let clear_btn: Element<Message> = 
            button(
                "Clear")
                .on_press(Message::Clear)
                .into();

        let biezer: Element<Message> = 
            radio(
                "Beizer",
                IpgCanvasWidget::Bezier,
                Some(self.state.selection),
                Message::RadioSelected,
                ).into();

        let circle: Element<Message> = 
            radio(
                "Circle",
                IpgCanvasWidget::Circle,
                Some(self.state.selection),
                Message::RadioSelected,
                ).into();

        let line: Element<Message> = 
            radio(
                "Line",
                IpgCanvasWidget::Line,
                Some(self.state.selection),
                Message::RadioSelected,
                ).into();

        let polygon: Element<Message> = 
        radio(
            "Polygon",
            IpgCanvasWidget::Polygon,
            Some(self.state.selection),
            Message::RadioSelected,
            ).into();

        let polyline: Element<Message> = 
            radio(
                "PolyLine",
                IpgCanvasWidget::PolyLine,
                Some(self.state.selection),
                Message::RadioSelected,
                ).into();

        let rect: Element<Message> = 
            radio(
                "Rectangle",
                IpgCanvasWidget::Rectangle,
                Some(self.state.selection),
                Message::RadioSelected,
                ).into();

        let triangle: Element<Message> = 
            radio(
                "Triangle",
                IpgCanvasWidget::Triangle,
                Some(self.state.selection),
                Message::RadioSelected,
                ).into();

        let r_triangle: Element<Message> = 
            radio(
                "Right Triangle",
                IpgCanvasWidget::RightTriangle,
                Some(self.state.selection),
                Message::RadioSelected,
                ).into();

        let mode = self.state.canvas_mode.to_str();
        let canvas_mode: Element<Message> = text(format!("Mode = {}", mode)).into();

        let del_last: Element<Message> = 
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

        let colors: Element<Message> = 
            pick_list(
                color_opt, 
                self.state.selected_color_str.clone(), 
                Message::ColorSelected).into();

        let widths: Element<Message> = text(format!("widths = {}", 2.0)).into();

        let poly_input = if self.state.poly_points == 0 {
            ""
        } else {
            &self.state.poly_points.to_string()
        };
        let poly_pts_input: Element<Message> = 
            text_input("Poly Points", poly_input)
            .on_input(Message::PolyInput)
            .into();

        let edit: Element<Message> = 
             button("Change Mode")
                .on_press(Message::Edit)
                .into();

        let edit_next: Element<Message> = 
            button("Edit Next")
                .on_press(Message::EditNext)
                .into();
        
        let save: Element<Message> = 
            button("Save")
                .padding(5.0)
                .on_press(Message::Save)
                .into();

        let load: Element<Message>  = 
            button("Load")
                .padding(5.0)
                .on_press(Message::Load)
                .into();
        
        let load_save_row: Element<Message> = 
            row(vec![load, save])
                .spacing(5.0)
                .into();

        let instructions: Element<Message> = 
            text("Start:\n Select a curve.\n\nDraw:\nUse left mouse button, click and move move then click again.\n\nCancel Draw:\nHold down esc and press left mouse button to cancel drawing.").into();
         
        let col: Element<Message> = 
            column(vec![clear_btn, 
            biezer, 
            circle, 
            line,
            polygon,
            polyline,
            rect,
            triangle,
            r_triangle,
            canvas_mode,
            edit,
            edit_next,
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
    w.write(b"\n").expect("unable to append to buffer");
    w.flush().expect("unable to flush buffer");
    Ok(())
}

// iced Point does not derive any serialization 
// so had to use own version for saving data.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DrawCanvasPoint{
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawCanvasCurve {
    curve_type: IpgCanvasWidget,
    points: Vec<DrawCanvasPoint>,
    poly_points: usize,
    color: DrawColor,
    width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct DrawColor {
    /// Red component, 0.0 - 1.0
    pub r: f32,
    /// Green component, 0.0 - 1.0
    pub g: f32,
    /// Blue component, 0.0 - 1.0
    pub b: f32,
    /// Transparency, 0.0 - 1.0
    pub a: f32,
}

impl DrawColor {
    pub const fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> DrawColor {
        DrawColor { r, g, b, a }
    }
}

fn convert_to_iced_point_color(curves: Vec<DrawCanvasCurve>) -> Vec<DrawCurve> {
    let mut iced_curves = vec![];
    for curve in curves {
        let mut dc_points = vec![];
        for point in curve.points.iter() {
            dc_points.push(convert_to_iced_point(point.clone()));
        }
        let color = Color{ r: curve.color.r, g: curve.color.g, b: curve.color.b, a: curve.color.a };
        iced_curves.push(DrawCurve { curve_type: curve.curve_type, 
                                    points: dc_points, 
                                    poly_points: curve.poly_points, 
                                    color, 
                                    width: curve.width, 
                                    });
    }
    iced_curves
}

fn convert_to_draw_point_color(curves: &Vec<DrawCurve>) -> Vec<DrawCanvasCurve> {
    let mut ipg_curves = vec![];
    for curve in curves {
        let mut dp_points = vec![];
        for point in curve.points.iter() {
            dp_points.push(convert_to_draw_point(point.clone()));
        }

        let color = DrawColor { r: curve.color.r, g: curve.color.g, b: curve.color.b, a: curve.color.a };
        let width = curve.width;
        ipg_curves.push(DrawCanvasCurve { curve_type: curve.curve_type, 
                                            points: dp_points,
                                            poly_points: curve.poly_points, 
                                            color, 
                                            width,
                                        });
    }
    ipg_curves
}

fn convert_to_iced_point(ipg_point: DrawCanvasPoint) -> Point {
    Point { x: ipg_point.x, y: ipg_point.y }
}
fn convert_to_draw_point(point: Point) -> DrawCanvasPoint {
    DrawCanvasPoint { x: point.x, y: point.y }
}

