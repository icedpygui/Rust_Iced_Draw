//! This example showcases an interactive `Canvas` for drawing Bézier curves.
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use iced::keyboard::key;
use iced::widget::{button, column, container, radio, row, text, vertical_space};
use iced::{event, keyboard, Element, Event, Point, Subscription, Theme};

use serde::{Deserialize, Serialize};

mod draw_canvas;
use draw_canvas::{Choice, DrawCurve};

pub fn main() -> iced::Result {
    iced::application("Bezier Tool - Iced", Example::update, Example::view)
        .theme(|_| Theme::CatppuccinMocha)
        .subscription(Example::subscription)
        .antialiasing(true)
        .centered()
        .run()
}

#[derive(Default)]
struct Example {
    state: draw_canvas::State,
}

#[derive(Debug, Clone)]
enum Message {
    AddCurve(draw_canvas::DrawCurve),
    Clear,
    DeleteLast,
    Edit,
    RadioSelected(Choice),
    Event(Event),
    Load,
    Save,
}

impl Example {
    fn update(&mut self, message: Message) {
        match message {
            Message::AddCurve(curve) => {
                if self.state.curve_to_edit.is_some() {
                    self.state.curves[self.state.curve_to_edit.unwrap()] = curve;
                    self.state.edit_points = vec![curve.from, curve.to];
                    if curve.control.is_some() {
                        self.state.edit_points.push(curve.control.unwrap());
                    }
                } else {
                    self.state.curves.push(curve);
                    self.state.curve_to_edit = None;
                }
                
                self.state.request_redraw();
            }
            Message::Clear => {
                self.state = draw_canvas::State::default();
                self.state.curves.clear();
            }
            Message::DeleteLast => {
                if self.state.curves.is_empty() {
                    return
                }
                self.state.curves.remove(self.state.curves.len()-1);
                self.state.request_redraw();
            }
            Message::Edit => {
                if self.state.curves.is_empty() {
                    return
                }
                
                // first edit press sets to curve 0
                if self.state.curve_to_edit.is_none() {
                    self.state.curve_to_edit = Some(0);
                } else {
                    let mut edit = self.state.curve_to_edit.unwrap();
                    edit += 1;
                    if edit > self.state.curves.len()-1 {
                        self.state.curve_to_edit = None;
                    } else {
                        self.state.curve_to_edit = Some(edit);
                    }
                }
                self.state.selection = if self.state.curve_to_edit.is_some() {
                    let curve = self.state.curves[self.state.curve_to_edit.unwrap()];
                    self.state.edit_points = vec![curve.from, curve.to];
                    if curve.control.is_some() {
                        self.state.edit_points.push(curve.control.unwrap());
                    }
                    curve.curve_type
                } else {
                    Choice::None
                };
                
                self.state.request_redraw();
            }
            Message::RadioSelected(choice) => {
                self.state.selection = choice; 
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
                self.state.curves = convert_to_iced_point(curves);
                self.state.request_redraw();
            }
            Message::Save => {
                let path = Path::new("./resources/data.json");
                let curves = convert_to_ipg_point(&self.state.curves);
                let _ = save(path, &curves);
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {

        event::listen().map(Message::Event)

    }

    fn view(&self) -> Element<Message> {
        let clear_btn: Element<Message> = button("Clear")
                                            .on_press(Message::Clear)
                                            .into();

        let biezer_ctrl: Element<Message> = radio(
                                            "Beizer",
                                            Choice::Bezier,
                                            Some(self.state.selection),
                                            Message::RadioSelected,
                                            ).into();
        let circle_ctrl: Element<Message> = radio(
                                            "Circle",
                                            Choice::Circle,
                                            Some(self.state.selection),
                                            Message::RadioSelected,
                                            ).into();

        let line_ctrl: Element<Message> = radio(
                                            "Line",
                                            Choice::Line,
                                            Some(self.state.selection),
                                            Message::RadioSelected,
                                            ).into();
        let rect_ctrl: Element<Message> = radio(
                                            "Rectangle",
                                            Choice::Rectangle,
                                            Some(self.state.selection),
                                            Message::RadioSelected,
                                            ).into();

        let triangle_ctrl: Element<Message> = radio(
                                            "Triangle",
                                            Choice::Triangle,
                                            Some(self.state.selection),
                                            Message::RadioSelected,
                                            ).into();
        let r_triangle_ctrl: Element<Message> = radio(
                                            "Right Triangle",
                                            Choice::RightTriangle,
                                            Some(self.state.selection),
                                            Message::RadioSelected,
                                            ).into();

        let del_last: Element<Message> = button("Delete Last")
                                            .on_press(Message::DeleteLast)
                                            .into();

        let edit: Element<Message> = if self.state.curve_to_edit.is_some() {
            button("Edit Next")
                .on_press(Message::Edit)
                .into()
        } else {
             button("Edit Curve")
                .on_press(Message::Edit)
                .into()
        };
        
        let save: Element<Message> = button("Save")
                                    .padding(5.0)
                                    .on_press(Message::Save)
                                    .into();

        let load: Element<Message>  = button("Load")
                                    .padding(5.0)
                                    .on_press(Message::Load)
                                    .into();
        
        let load_save_row: Element<Message> = row(vec![load, save])
                                                .spacing(5.0)
                                                .into();

        let instructions: Element<Message> = text("Start:\n Select a curve.\n\nDraw:\nUse left mouse button, click and move move then click again.\n\nCancel Draw:\nHold down esc and press left mouse button to cancel drawing.").into();
         
        let col: Element<Message> = column(vec![clear_btn, 
                                                            biezer_ctrl, 
                                                            circle_ctrl, 
                                                            line_ctrl,
                                                            rect_ctrl,
                                                            triangle_ctrl,
                                                            r_triangle_ctrl,
                                                            del_last,
                                                            edit,
                                                            load_save_row,
                                                            vertical_space().height(50.0).into(),
                                                            instructions,
                                                            ])
                                                            .width(150.0)
                                                            .spacing(10.0)
                                                            .padding(10.0)
                                                            .into();

        

        let draw =  
            container(self.state.view(&self.state.curves)
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
pub struct IpgPoint{
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct IpgDrawCurve {
    curve_type: Choice,
    from: IpgPoint,
    to: IpgPoint,
    control: Option<IpgPoint>,
}

fn convert_to_iced_point(curves: Vec<IpgDrawCurve>) -> Vec<DrawCurve> {
    let mut iced_curves = vec![];
    for curve in curves {
        let from = to_point(curve.from);
        let to = to_point(curve.to);
        let control: Option<Point> = 
            match curve.control {
                Some(ctrl) => Some(to_point(ctrl)),
                None => None,
            };
        
        iced_curves.push(DrawCurve { curve_type: curve.curve_type, from, to, control });
    }
    iced_curves
}

fn convert_to_ipg_point(curves: &Vec<DrawCurve>) -> Vec<IpgDrawCurve> {
    let mut ipg_curves = vec![];
    for curve in curves {
        let from = to_ipg_point(curve.from);
        let to = to_ipg_point(curve.to);
        let control: Option<IpgPoint> = 
            match curve.control {
                Some(ctrl) => Some(to_ipg_point(ctrl)),
                None => None,
            };
        
        ipg_curves.push(IpgDrawCurve { curve_type: curve.curve_type, from, to, control });
    }
    ipg_curves
}

fn to_point(ipg_point: IpgPoint) -> Point {
    Point { x: ipg_point.x, y: ipg_point.y }
}
fn to_ipg_point(point: Point) -> IpgPoint {
    IpgPoint { x: point.x, y: point.y }
}

