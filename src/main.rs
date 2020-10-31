#![allow(dead_code, unused_variables, unused_imports)]

extern crate cairo;
extern crate gtk;
extern crate gio;

use std::time::{SystemTime, Duration};
use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gio::prelude::*;

use gtk::{Window, WindowType, Button, DrawingArea};
use cairo::{FontSlant, FontWeight};

type Color = (f64, f64, f64);

struct HMS {
    overtime: bool,
    h: u64,
    m: u64,
    s: u64,
}

struct Timer {
    duration: Duration,
    started_at: SystemTime,
    pub end_at: SystemTime,
}

impl Timer {
    pub fn new(duration: Duration) -> Timer {
        let started_at = SystemTime::now();
        Timer {
            duration: duration,
            started_at: started_at,
            end_at: started_at + duration,
        }
    }

    pub fn restart(&mut self) {
        self.started_at = SystemTime::now();
        self.update_end_at();
    }

    pub fn set_duration(&mut self, new_duration: Duration) {
        self.duration = new_duration;
        self.update_end_at();
    }

    fn update_end_at(&mut self) {
        self.end_at = self.started_at + self.duration;
    }

    pub fn until_end_hms(&self) -> HMS {
        let now = SystemTime::now();
        let overtime = self.end_at < now;
        let mut until_dur = if overtime {
            now.duration_since(self.end_at).unwrap().as_secs()
        } else {
            self.end_at.duration_since(now).unwrap().as_secs()
        };
        let hours = until_dur / (60 * 60);
        until_dur = until_dur % (60 * 60);
        let minutes = until_dur / 60;
        until_dur = until_dur % 60;
        let seconds = until_dur;
        HMS { overtime: overtime, h: hours, m: minutes, s: seconds }
    }

}

struct CanvasSize { x: f64, y: f64 }

struct TimerUI {
    timer: Rc<RefCell<Timer>>,
    background_color: Color,
    inverted_background_color: Color,
    color: Color,
    inverted_color: Color,
    font_size_multiplier: f64,
    canvas_size: CanvasSize,
}

impl TimerUI {

    fn redraw(&mut self, drawing_area: &DrawingArea, cr: &cairo::Context) {
        self.canvas_size = CanvasSize {
            x: drawing_area.get_allocated_width() as f64,
            y: drawing_area.get_allocated_height() as f64,
        };

        let hms = self.timer.borrow().until_end_hms();

        self.draw(cr, hms);
    }

    fn draw(&self, cr: &cairo::Context, hms: HMS) {
        // TODO half second blink
        let inverted = hms.overtime && hms.s % 2 == 0;
        self.draw_background(cr, inverted);
        self.draw_time(cr, hms, inverted);
    }

    fn draw_background(&self, cr: &cairo::Context, inverted: bool) {
        if inverted {
            let bg_color = self.inverted_background_color;
            cr.set_source_rgb(bg_color.0, bg_color.1, bg_color.2);
            cr.paint();
        } else {
            let bg_color = self.background_color;
            cr.set_source_rgba(bg_color.0, bg_color.1, bg_color.2, 0.1);
            cr.set_operator(cairo::Operator::Source);
            cr.paint();
            cr.set_operator(cairo::Operator::Over);
        }
    }

    fn draw_time(&self, cr: &cairo::Context, hms: HMS, inverted: bool) {
        let color = if inverted {self.inverted_color} else {self.color};
        cr.set_source_rgb(color.0, color.1, color.2);

        let text = TimerUI::format_hms(hms);
        cr.select_font_face(
            "Block Digits",
            FontSlant::Normal,
            FontWeight::Normal
        );
        cr.set_font_size(self.canvas_size.y * self.font_size_multiplier);
        let extents = cr.text_extents(&text);
        let x = (self.canvas_size.x - extents.x_advance - extents.x_bearing) / 2.;
        let y = (self.canvas_size.y + extents.height) / 2.;
        cr.move_to(x, y);
        cr.show_text(&text);
    }

    fn format_hms(hms: HMS) -> String {
        let sign = if hms.overtime { "+" } else { "" };
        let mut hours = hms.h.to_string() + ":";
        if hms.h == 0 { hours = String::from("") };
        return format!("{}{}{:0>2}:{:0>2}", sign, hours, hms.m, hms.s);
    }

    pub fn refresh(area: &DrawingArea) {
        area.queue_draw();
    }

    pub fn create_drawing_area_for_window(
        window: &Window,
        rc_timer_ui: Rc<RefCell<TimerUI>>,
    ) -> DrawingArea {
        let area = DrawingArea::new();
        area.connect_draw({
            move |drawing_area, cr| {
                rc_timer_ui.borrow_mut().redraw(drawing_area, cr);
                gtk::Inhibit(false)
            }
        });
        window.add(&area);
        area
    }

}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let window = Window::new(WindowType::Toplevel);
    window.set_title("on screen countdown");
    window.set_default_size(550, 300);
    window.set_decorated(false);
    window.set_keep_above(true);

    let screen = {
        window.get_screen().expect("The application must be run with dispaly")
    };
    match screen.get_rgba_visual() {
        Some(visual) => window.set_visual(Some(&visual)),
        _ => println!("Transparent window not supported")
    }

    let timer = Timer::new(Duration::from_secs(15 * 60));
    let rc_timer = Rc::new(RefCell::new(timer));

    window.connect_key_release_event({
        let timer = Rc::clone(&rc_timer);
        move |_, event_key| {
            match event_key.get_keyval().to_unicode() {
                Some('r') | Some('R') => timer.borrow_mut().restart(),
                // Some('q') => println!("quit"),
                _ => ()
            }
            Inhibit(false)
        }
    });

    let timer_ui = TimerUI {
        color: (1., 1., 1.),
        inverted_color: (0., 0., 0.),
        background_color: (0.21, 0.2, 0.22),
        inverted_background_color: (0.9, 0.0, 0.1),
        timer: Rc::clone(&rc_timer),
        canvas_size: CanvasSize { x: 0., y: 0. },
        font_size_multiplier: 0.40,
    };
    let rc_timer_ui = Rc::new(RefCell::new(timer_ui));

    let area = TimerUI::create_drawing_area_for_window(
        &window,
        Rc::clone(&rc_timer_ui)
    );

    window.show_all();

    let sleep_dur = Duration::new(0, 50_000_000);
    loop {
        TimerUI::refresh(&area);
        gtk::main_iteration_do(false);

        std::thread::sleep(sleep_dur);
    }
}
