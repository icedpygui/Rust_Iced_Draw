//! This example showcases an interactive `Canvas` for drawing Bézier curves.
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use colors::{get_rgba_from_canvas_draw_color, DrawCanvasColor};
use iced::keyboard::key;
use iced::widget::text::{LineHeight, Shaping};
use iced::widget::{button, column, container, 
    pick_list, radio, row, text, text_input, vertical_space};
use iced::{alignment, event, keyboard, time, Color, Element, Event, Font, Pixels, Point, Radians, Subscription, Theme, Vector};
use iced::widget::container::Id;

use serde::{Deserialize, Serialize};

mod draw_canvas;
use draw_canvas::{get_draw_mode_and_status, get_widget_id, set_widget_mode_or_status, Arc, Bezier, CanvasWidget, Circle, DrawMode, DrawStatus, Ellipse, Line, PolyLine, Polygon, RightTriangle, Text, Widget};
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
    canvas_state: draw_canvas::CanvasState,
}

#[derive(Debug, Clone)]
enum Message {
    WidgetDraw(CanvasWidget),
    Clear,
    ModeSelected(String),
    RadioSelected(Widget),
    Event(Event),
    Load,
    Save,
    ColorSelected(String),
    PolyInput(String),
    WidthInput(String),
    Tick,
}

impl Example {
    fn update(&mut self, message: Message) {
        match message {
            Message::WidgetDraw(mut widget) => {
                
                let (draw_mode, draw_status) = get_draw_mode_and_status(&widget);

                if draw_mode == DrawMode::New {
                    let id = get_widget_id(&widget);
                    let widget = set_widget_mode_or_status(widget.clone(), Some(DrawMode::DrawAll), Some(DrawStatus::Completed));
                    self.canvas_state.curves.insert(id, widget);
                } else {
                    if draw_status == DrawStatus::Completed {
                        widget = set_widget_mode_or_status(widget, Some(DrawMode::DrawAll), None);
                    }
                    let id = get_widget_id(&widget);
                    self.canvas_state.edit_widget_id = Some(id.clone());
                    self.canvas_state.curves.entry(id).and_modify(|k| *k= widget);
                }

                self.canvas_state.request_redraw();
            }
            Message::Clear => {
                self.canvas_state.curves.clear();
                self.canvas_state = draw_canvas::CanvasState::default();
            }
            Message::ModeSelected(mode) => {
                let mode = DrawMode::to_enum(mode.clone());
                match mode {
                    DrawMode::DrawAll => {
                        self.canvas_state.draw_mode = DrawMode::DrawAll;
                    },
                    DrawMode::Edit => {
                        if self.canvas_state.curves.is_empty() {
                            return
                        }
                        self.canvas_state.draw_mode = DrawMode::Edit;
                    },
                    DrawMode::New => {
                        self.canvas_state.draw_mode = DrawMode::New;
                    },
                    DrawMode::Rotate => {
                        self.canvas_state.draw_mode = DrawMode::Rotate;
                    },
                }
                self.canvas_state.request_redraw();
            },
            Message::RadioSelected(choice) => {
                match choice {
                    Widget::Arc => {
                        self.canvas_state.selected_radio_widget = Some(Widget::Arc);
                    },
                    Widget::Bezier => {
                        self.canvas_state.selected_radio_widget = Some(Widget::Bezier);
                    },
                    Widget::Circle => {
                        self.canvas_state.selected_radio_widget = Some(Widget::Circle);
                    },
                    Widget::Ellipse => {
                        self.canvas_state.selected_radio_widget = Some(Widget::Ellipse);
                    },
                    Widget::Line => {
                        self.canvas_state.selected_radio_widget = Some(Widget::Line);
                    },
                    Widget::PolyLine => {
                        self.canvas_state.selected_radio_widget = Some(Widget::PolyLine);
                    },
                    Widget::Polygon => {
                        self.canvas_state.selected_radio_widget = Some(Widget::Polygon);
                    },
                    Widget::RightTriangle => {
                        self.canvas_state.selected_radio_widget = Some(Widget::RightTriangle);
                    },
                    Widget::Text => {
                        self.canvas_state.selected_radio_widget = Some(Widget::Text);
                    }
                    Widget::None => (),
                } 
            },
            Message::Event(Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            })) => { 
                self.canvas_state.escape_pressed = true;
            },
            Message::Event(Event::Keyboard(keyboard::Event::KeyReleased {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            })) => { 
                self.canvas_state.escape_pressed = false;
            },
            Message::Event(_) => (),
            Message::Tick => {
                self.canvas_state.elapsed_time += self.canvas_state.timer_duration;
                self.canvas_state.blink = !self.canvas_state.blink;
                self.canvas_state.request_redraw();
            }
            Message::Load => {
                let path = Path::new("./resources/data.json");
                let data = fs::read_to_string(path).expect("Unable to read file");
                let widgets = serde_json::from_str(&data).expect("Unable to parse");
                self.canvas_state.curves = import_widgets(widgets);
                self.canvas_state.request_redraw();
            }
            Message::Save => {
                let path = Path::new("./resources/data.json");
                let widgets = convert_to_export(&self.canvas_state.curves);
                let _ = save(path, &widgets);
            }
            Message::ColorSelected(color_str) => {
                let canvas_color: DrawCanvasColor = match color_str.as_str() {
                    "Primary" => DrawCanvasColor::PRIMARY,
                    "Secondary" => DrawCanvasColor::SECONDARY,
                    "Success" => DrawCanvasColor::SUCCESS,
                    "Danger" => DrawCanvasColor::DANGER,
                    _ => DrawCanvasColor::WHITE,
                };
                self.canvas_state.selected_color_str = Some(color_str);
                self.canvas_state.selected_color = Color::from(get_rgba_from_canvas_draw_color(canvas_color));
            },
            Message::PolyInput(input) => {
                // little error checking
                self.canvas_state.selected_poly_points_str = input.clone();
                if !input.is_empty() {
                    self.canvas_state.selected_poly_points = input.parse().unwrap();
                } else {
                    self.canvas_state.selected_poly_points = 4; //default
                }
            }
            Message::WidthInput(input) => {
                // little error checking
                self.canvas_state.selected_width_str = input.clone();
                if !input.is_empty() {
                    self.canvas_state.selected_width = input.parse().unwrap();
                } else {
                    self.canvas_state.selected_width = 2.0; //default
                }
                // if self.state.edit_widget_id.is_some() {
                //     set_edit_widget_width(self.state.edit_widget_id, self.state.selected_width);
                // }
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![];
        
        if self.canvas_state.timer_event_enabled {
            subscriptions.push(time::every(
                iced::time::Duration::from_millis(
                    self.canvas_state.timer_duration))
                    .map(|_| Message::Tick));
        }
        
        subscriptions.push(event::listen().map(Message::Event)) ;
        
        Subscription::batch(subscriptions)
        
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
                self.canvas_state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let bezier = 
            radio(
                "Bezier",
                Widget::Bezier,
                self.canvas_state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let circle = 
            radio(
                "Circle",
                Widget::Circle,
                self.canvas_state.selected_radio_widget,
                Message::RadioSelected,
                ).into();
        
        let elipse = 
            radio(
                "Ellipse",
                Widget::Ellipse,
                self.canvas_state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let line = 
            radio(
                "Line",
                Widget::Line,
                self.canvas_state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let polygon = 
            radio(
                "Polygon",
                Widget::Polygon,
                self.canvas_state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let polyline = 
            radio(
                "PolyLine",
                Widget::PolyLine,
                self.canvas_state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let r_triangle = 
            radio(
                "Right Triangle",
                Widget::RightTriangle,
                self.canvas_state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

        let txt = 
            radio(
                "Text",
                Widget::Text,
                self.canvas_state.selected_radio_widget,
                Message::RadioSelected,
                ).into();

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
                self.canvas_state.selected_color_str.clone(), 
                Message::ColorSelected).into();

        let widths = 
            text_input("Width(2.0)", 
                        &self.canvas_state.selected_width_str)
                .on_input(Message::WidthInput)
                .into();

        let poly_pts_input = 
            text_input("Poly Points(3)", 
                        &self.canvas_state.selected_poly_points_str)
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
                Some(self.canvas_state.draw_mode.string()), 
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
            bezier, 
            circle,
            elipse, 
            line,
            polygon,
            polyline,
            r_triangle,
            txt,
            mode,
            load_save_row,
            poly_pts_input,
            colors,
            widths,
            vertical_space().height(50.0).into(),
            instructions,
            ])
            .width(150.0)
            .spacing(10.0)
            .padding(10.0)
            .into();

        let draw =  
            container(self.canvas_state
                .view(&self.canvas_state.curves)
                .map(Message::WidgetDraw))
                .into();
        
        Element::from(row(vec![col, draw]))

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
    pub content: String,
    pub points: Vec<ExportPoint>,
    pub poly_points: usize,
    pub mid_point: ExportPoint,
    pub other_point: ExportPoint,
    pub rotation: f32,
    pub color: ExportColor,
    pub width: f32,
}

#[allow(clippy::redundant_closure)]
fn import_widgets(widgets: Vec<ExportWidget>) -> HashMap<Id, CanvasWidget> {
    
    let mut curves: HashMap<Id, CanvasWidget> = HashMap::new();

    for widget in widgets.iter() {
        let points: Vec<Point> = widget.points.iter().map(|p| convert_to_point(p)).collect();
        let mid_point = convert_to_point(&widget.mid_point);
        let other_point = convert_to_point(&widget.other_point);
        let color = convert_to_color(&widget.color);
        let width = widget.width;
        let draw_mode = DrawMode::DrawAll;
        let radius = mid_point.distance(points[1]);

        match widget.name {
            Widget::None => {
            },
            Widget::Arc => {
                let id = Id::unique();
                let arc = Arc {
                    id: id.clone(),
                    points,
                    mid_point,
                    radius,
                    color,
                    width,
                    start_angle: Radians(other_point.x),
                    end_angle: Radians(other_point.y),
                    draw_mode,
                    status: DrawStatus::Completed,
                };
                
                curves.insert(id, CanvasWidget::Arc(arc));
            },
            Widget::Bezier => {
                let id = Id::unique();
                let bz = Bezier {
                    id: id.clone(),
                    points: points,
                    mid_point,
                    color,
                    width,
                    degrees: widget.rotation,
                    draw_mode,
                    status: DrawStatus::Completed
                };
                
                curves.insert(id, CanvasWidget::Bezier(bz));
            },
            Widget::Circle => {
                let id = Id::unique();
                let cir = Circle {
                    id: id.clone(),
                    center: mid_point,
                    circle_point: convert_to_point(&widget.points[0]),
                    radius: widget.mid_point.distance(widget.points[0]),
                    color,
                    width,
                    draw_mode,
                    status: DrawStatus::Completed,
                };
                
                curves.insert(id, CanvasWidget::Circle(cir));
            },
            Widget::Ellipse => {
                let id = Id::unique();
                let vx = points[1].distance(points[0]);
                let vy = points[2].distance(points[0]);
                let ell = Ellipse {
                    id: id.clone(),
                    points,
                    center: convert_to_point(&widget.points[0]),
                    radii: Vector { x: vx, y: vy },
                    rotation: Radians(widget.rotation),
                    color,
                    width,
                    draw_mode,
                    status: DrawStatus::Completed,
                };
                
                curves.insert(id, CanvasWidget::Ellipse(ell));
            },
            Widget::Line => {
                let id = Id::unique();
                let ln = Line {
                    id: id.clone(),
                    points,
                    mid_point,
                    color,
                    width,
                    degrees: widget.rotation,
                    draw_mode,
                    status: DrawStatus::Completed,
                };
                curves.insert(id, CanvasWidget::Line(ln));
            },
            Widget::Polygon => {
                let id = Id::unique();
                let pg = Polygon {
                    id: id.clone(),
                    points,
                    poly_points: widget.poly_points,
                    mid_point,
                    pg_point: other_point,
                    color,
                    width,
                    degrees: widget.rotation,
                    draw_mode,
                    status: DrawStatus::Completed,
                };
                curves.insert(id, CanvasWidget::Polygon(pg));
            },
            Widget::PolyLine => {
                let id = Id::unique();
                let pl = PolyLine {
                    id: id.clone(),
                    points,
                    poly_points: widget.poly_points,
                    mid_point,
                    pl_point: other_point,
                    color,
                    width,
                    degrees: widget.rotation,
                    draw_mode,
                    status: DrawStatus::Completed,
                };
                curves.insert(id, CanvasWidget::PolyLine(pl));
            },
            Widget::RightTriangle => {
                let id = Id::unique();
                let tr = RightTriangle {
                    id: id.clone(),
                    points,
                    mid_point,
                    tr_point: other_point,
                    color,
                    width,
                    degrees: widget.rotation,
                    draw_mode,
                    status: DrawStatus::Completed,
                };
                curves.insert(id, CanvasWidget::RightTriangle(tr));
            },
            Widget::Text => {
                let id = Id::unique();
                let txt = Text {
                    id: id.clone(),
                    content: widget.content.clone(),
                    position: other_point,
                    color,
                    size: Pixels(16.0),
                    line_height: LineHeight::Relative(1.2),
                    font: Font::default(),
                    horizontal_alignment: alignment::Horizontal::Left,
                    vertical_alignment: alignment::Vertical::Top,
                    shaping: Shaping::Basic,
                    degrees: widget.rotation,
                    draw_mode,
                    status: DrawStatus::Completed,
                };
                curves.insert(id, CanvasWidget::Text(txt));
            }
        }
    }

    curves

}

fn convert_to_export(widgets: &HashMap<Id, CanvasWidget>) -> Vec<ExportWidget> {
     
    let mut export = vec![];

    for (_id, widget) in widgets.iter() {

        let (name, 
            points, 
            mid_point,
            other_point, 
            poly_points, 
            rotation,
            color, 
            width,
            content ,
            ) = 
            match widget {
                CanvasWidget::None => {
                    (Widget::None, &vec![], Point::default(), Point::default(), 0, 0.0, Color::TRANSPARENT, 0.0, String::new())
                },
                CanvasWidget::Arc(arc) => {
                    let other_point = Point{ x: arc.start_angle.0, y: arc.end_angle.0 };
                    (Widget::Arc, &arc.points, arc.mid_point, other_point, 0, 0.0, arc.color, arc.width, String::new())
                },
                CanvasWidget::Bezier(bz) => {
                    (Widget::Bezier, &bz.points, bz.mid_point, Point::default(), 0, bz.degrees, bz.color, bz.width, String::new())
                },
                CanvasWidget::Circle(cir) => {
                    (Widget::Circle, &vec![cir.circle_point], cir.center, cir.circle_point, 0, 0.0, cir.color, cir.width, String::new())
                },
                CanvasWidget::Ellipse(ell) => {
                    (Widget::Ellipse, &ell.points, ell.center, Point::default(), 0, ell.rotation.0, ell.color, ell.width, String::new())
                },
                CanvasWidget::Line(ln) => {
                    (Widget::Line, &ln.points, ln.mid_point, Point::default(), 0, ln.degrees, ln.color, ln.width, String::new())
                },
                CanvasWidget::Polygon(pg) => {
                    (Widget::Polygon, &pg.points, pg.mid_point, pg.pg_point, pg.poly_points, pg.degrees, pg.color, pg.width, String::new())
                },
                CanvasWidget::PolyLine(pl) => {
                    (Widget::PolyLine, &pl.points, pl.mid_point, pl.pl_point, pl.poly_points, pl.degrees, pl.color, pl.width, String::new())
                },
                CanvasWidget::RightTriangle(tr) => {
                    (Widget::RightTriangle, &tr.points, tr.mid_point, tr.tr_point, 3, tr.degrees, tr.color, tr.width, String::new())
                },
                CanvasWidget::Text(txt) => {
                    (Widget::Text, &vec![], Point::default(), txt.position, 3, txt.degrees, txt.color, 0.0, txt.content.clone())
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
                content,
                points: x_points,
                poly_points, 
                mid_point: x_mid_pt,
                other_point: x_other_point,
                rotation, 
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
