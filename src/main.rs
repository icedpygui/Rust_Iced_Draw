//! This example showcases an interactive `Canvas` for drawing curves.
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use iced::theme::palette::Background;
use iced::widget::text::{LineHeight, Shaping};
use iced::widget::{button, column, container, 
    pick_list, radio, row, text_input};
use iced::{alignment, time, Color, Element, Font, Pixels, 
    Point, Radians, Subscription, Theme, Vector};
use iced::widget::container::Id;

use iced_aw::{color_picker, iced_fonts};
use serde::{Deserialize, Serialize};

mod draw_canvas;
mod colors;
mod path_builds;
mod helpers;

use draw_canvas::{get_draw_mode_and_status, get_widget_id, 
    set_widget_mode_or_status, Arc, Bezier, CanvasWidget, Circle, 
    DrawMode, DrawStatus, Ellipse, FreeHand, Line, PolyLine, Polygon, 
    RightTriangle, Text, Widget};



pub fn main() -> iced::Result {
    iced::application("Drawing Tool - Iced", CanvasDraw::update, CanvasDraw::view)
        .theme(|_| Theme::CatppuccinMocha)
        .subscription(CanvasDraw::subscription)
        .antialiasing(true)
        .font(iced_fonts::REQUIRED_FONT_BYTES)
        // .default_font(Font::MONOSPACE)
        .centered()
        .run()
}

#[derive(Default)]
struct CanvasDraw {
    canvas_state: draw_canvas::CanvasState,
    show_draw_color_picker: bool,
    show_canvas_color_picker: bool,
}

#[derive(Debug, Clone)]
enum Message {
    WidgetDraw(CanvasWidget),
    Clear,
    ModeSelected(String),
    RadioSelected(Widget),
    Load,
    Save,
    PolyInput(String),
    WidthInput(String),
    Tick,
    SelectDrawColor,
    SubmitDrawColor(Color),
    CancelDrawColor,
    SelectCanvasColor,
    SubmitCanvasColor(Color),
    CancelCanvasColor,
}

impl CanvasDraw {
    fn update(&mut self, message: Message) {
        match message {
            Message::WidgetDraw(mut widget) => {
                let (draw_mode, draw_status) = get_draw_mode_and_status(&widget);
                // Since the text widget may have a blinking cursor, the only way to use a timer
                // is to use the main subscription one at this time, canvas lacks a time event.
                // Therefore, the pending has to return the curve also at each change so that
                // the curves can be updated.  The subscrition clears the cache at each tick.
                // A bit more efficient way would be to just have a text cache and just clear it.
                // Probable incorporated in a near future revision.
                match draw_status {
                    DrawStatus::Completed => {
                        widget = set_widget_mode_or_status(widget, Some(DrawMode::DrawAll), None);
                    },
                    DrawStatus::Delete => {
                        let id = get_widget_id(&widget);
                        self.canvas_state.curves.remove(&id);
                    },
                    DrawStatus::TextInProgress => {
                        // Since the text always returns a new curve or updated curve,
                        // a check for the first return is need to see if a text is present. 
                        let id = get_widget_id(&widget);
                        let present = self.canvas_state.curves.get(&id);
                        if present.is_none() {
                            self.canvas_state.curves.insert(id, widget.clone());
                        } else {
                            self.canvas_state.curves.entry(id).and_modify(|k| *k= widget.clone());
                        }
                    },
                    _ => (),
                }
                if draw_status != DrawStatus::TextInProgress {
                    if draw_mode == DrawMode::New {
                        let id = get_widget_id(&widget);
                        let widget = set_widget_mode_or_status(widget.clone(), Some(DrawMode::DrawAll), Some(DrawStatus::Completed));
                        self.canvas_state.curves.insert(id, widget);
                    } else {
                        // if not new must be in edit or rotate mode so modify.
                        let id = get_widget_id(&widget);
                        self.canvas_state.edit_widget_id = Some(id.clone());
                        self.canvas_state.curves.entry(id).and_modify(|k| *k= widget);
                    }
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
                // Have to  make sure and only use the timer event during
                // the text only.
                self.canvas_state.timer_event_enabled = false;
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
                    Widget::FreeHand => {
                        self.canvas_state.selected_radio_widget = Some(Widget::FreeHand);
                    }
                    Widget::Text => {
                        self.canvas_state.selected_radio_widget = Some(Widget::Text);
                        self.canvas_state.timer_event_enabled = true;
                    }
                    Widget::None => (),
                } 
            },
            Message::Tick => {
                self.canvas_state.elapsed_time += self.canvas_state.timer_duration;
                self.canvas_state.blink = !self.canvas_state.blink;
                self.canvas_state.request_redraw();
            },
            Message::Load => {
                let path = Path::new("./resources/data.json");
                let data = fs::read_to_string(path).expect("Unable to read file");
                let widgets = serde_json::from_str(&data).expect("Unable to parse");
                self.canvas_state.curves = import_widgets(widgets);
                self.canvas_state.request_redraw();
            },
            Message::Save => {
                let path = Path::new("./resources/data.json");
                let widgets = convert_to_export(&self.canvas_state.curves);
                let _ = save(path, &widgets);
            },
            Message::PolyInput(input) => {
                // little error checking
                self.canvas_state.selected_poly_points_str = input.clone();
                if !input.is_empty() {
                    self.canvas_state.selected_poly_points = input.parse().unwrap();
                } else {
                    self.canvas_state.selected_poly_points = 4; //default
                }
            },
            Message::WidthInput(input) => {
                // little error checking
                self.canvas_state.selected_width_str = input.clone();
                if !input.is_empty() {
                    self.canvas_state.selected_width = input.parse().unwrap();
                } else {
                    self.canvas_state.selected_width = 2.0; //default
                }
            },
            Message::SelectDrawColor => {
                self.show_draw_color_picker = true;
            },
            Message::SubmitDrawColor(color) => {
                self.canvas_state.selected_draw_color = color;
                self.show_draw_color_picker = false;
            },
            Message::CancelDrawColor => {
                self.show_draw_color_picker = false;
            },
            Message::SelectCanvasColor => {
                self.show_canvas_color_picker = true;
            },
            Message::SubmitCanvasColor(color) => {
                self.canvas_state.selected_canvas_color = color;
                self.show_canvas_color_picker = false;
                self.canvas_state.request_redraw();
            },
            Message::CancelCanvasColor => {
                self.show_canvas_color_picker = false;
            },
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

        let freehand = 
            radio(
                "FreeHand",
                Widget::FreeHand,
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

        let select_draw_color = 
            button("Draw Color")
                .padding(5.0)
                .on_press(Message::SelectDrawColor)
                .style(move|theme: &Theme, status| {   
                    get_button_styling(theme, status, self.canvas_state.selected_draw_color)  
                    });

        let select_canvas_color = 
            button("Canvas Color")
                .padding(5.0)
                .on_press(Message::SelectCanvasColor)
                .style(move|theme: &Theme, status| {   
                    get_button_styling(theme, status, self.canvas_state.selected_canvas_color)  
                    });
        
        let draw_color = color_picker(
            self.show_draw_color_picker,
            self.canvas_state.selected_draw_color,
            select_draw_color,
            Message::CancelDrawColor,
            Message::SubmitDrawColor,
        ).into();

        let canvas_color = color_picker(
            self.show_canvas_color_picker,
            self.canvas_state.selected_canvas_color,
            select_canvas_color,
            Message::CancelCanvasColor,
            Message::SubmitCanvasColor,
        ).into();

        let load_save_row = 
            row(vec![load, save])
                .spacing(5.0)
                .into();
            
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
            freehand,
            txt,
            mode,
            load_save_row,
            poly_pts_input,
            widths,
            draw_color,
            canvas_color,
            ])
            .width(175.0)
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

fn get_button_styling(theme: &Theme,
                        status: button::Status, 
                        bg_color: Color,
                        ) -> button::Style {

    let mut base_style = button::primary(theme, status);
    let mut hover_style = button::primary(theme, status);

    let background = Background::new(bg_color, Color::WHITE);

    base_style.background = Some(iced::Background::Color(bg_color));
    base_style.text_color = background.base.text;

    hover_style.background = Some(iced::Background::Color(background.strong.color));
    hover_style.text_color = background.weak.text;

    match status {
        button::Status::Active | button::Status::Pressed => base_style,
        button::Status::Hovered => hover_style,
        button::Status::Disabled => disabled(base_style),
    }
}

fn disabled(style: button::Style) -> button::Style {
    button::Style {
        background: style
            .background
            .map(|background| background.scale_alpha(0.5)),
        text_color: style.text_color.scale_alpha(0.5),
        ..style
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
    pub radius: f32,
    pub color: ExportColor,
    pub width: f32,
}

#[allow(clippy::redundant_closure)]
fn import_widgets(widgets: Vec<ExportWidget>) -> HashMap<Id, CanvasWidget> {
    
    let mut curves: HashMap<Id, CanvasWidget> = HashMap::new();

    for widget in widgets.iter() {
        let points: Vec<Point> = widget.points.iter().map(|p| convert_to_point(p)).collect();
        let other_point = convert_to_point(&widget.other_point);
        let color = convert_to_color(&widget.color);
        let width = widget.width;
        let draw_mode = DrawMode::DrawAll;
        let mid_point = if widget.name == Widget::Text {
            Point::default()
        } else {
            convert_to_point(&widget.mid_point)
        };
        
        match widget.name {
            Widget::None => {
            },
            Widget::Arc => {
                let id = Id::unique();
                let arc = Arc {
                    id: id.clone(),
                    points,
                    mid_point,
                    radius: widget.radius,
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
                    radius: widget.radius,
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
            Widget::FreeHand => {
                let id = Id::unique();
                let fh = FreeHand {
                    id: id.clone(),
                    points,
                    color,
                    width,
                    draw_mode,
                    status: DrawStatus::Completed,
                    completed: true,
                };
                curves.insert(id, CanvasWidget::FreeHand(fh));
            }
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
            radius,
            color, 
            width,
            content ,
            ) = 
            match widget {
                CanvasWidget::None => {
                    (Widget::None, &vec![], Point::default(), Point::default(), 0, 0.0, 0.0, Color::TRANSPARENT, 0.0, String::new())
                },
                CanvasWidget::Arc(arc) => {
                    let other_point = Point{ x: arc.start_angle.0, y: arc.end_angle.0 };
                    (Widget::Arc, &arc.points, arc.mid_point, other_point, 0, 0.0, arc.radius, arc.color, arc.width, String::new())
                },
                CanvasWidget::Bezier(bz) => {
                    (Widget::Bezier, &bz.points, bz.mid_point, Point::default(), 0, bz.degrees, 0.0, bz.color, bz.width, String::new())
                },
                CanvasWidget::Circle(cir) => {
                    (Widget::Circle, &vec![cir.circle_point], cir.center, cir.circle_point, 0, 0.0, cir.radius, cir.color, cir.width, String::new())
                },
                CanvasWidget::Ellipse(ell) => {
                    (Widget::Ellipse, &ell.points, ell.center, Point::default(), 0, ell.rotation.0, 0.0, ell.color, ell.width, String::new())
                },
                CanvasWidget::Line(ln) => {
                    (Widget::Line, &ln.points, ln.mid_point, Point::default(), 0, ln.degrees, 0.0, ln.color, ln.width, String::new())
                },
                CanvasWidget::Polygon(pg) => {
                    (Widget::Polygon, &pg.points, pg.mid_point, pg.pg_point, pg.poly_points, pg.degrees, 0.0, pg.color, pg.width, String::new())
                },
                CanvasWidget::PolyLine(pl) => {
                    (Widget::PolyLine, &pl.points, pl.mid_point, pl.pl_point, pl.poly_points, pl.degrees, 0.0, pl.color, pl.width, String::new())
                },
                CanvasWidget::RightTriangle(tr) => {
                    (Widget::RightTriangle, &tr.points, tr.mid_point, tr.tr_point, 3, tr.degrees, 0.0, tr.color, tr.width, String::new())
                },
                CanvasWidget::FreeHand(fh) => {
                    (Widget::FreeHand, &fh.points, Point::default(), Point::default(), 0, 0.0, 0.0, fh.color, fh.width, String::new())
                }
                CanvasWidget::Text(txt) => {
                    (Widget::Text, &vec![], Point::default(), txt.position, 3, txt.degrees, 0.0, txt.color, 0.0, txt.content.clone())
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
                radius, 
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
