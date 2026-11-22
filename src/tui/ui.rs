use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::Color,
    widgets::{
        Block, Borders,
        canvas::{Canvas, Circle, Line},
    },
};
use std::f64::consts::TAU;
use std::io::{Error, Stdout, stdout};

use crate::motor::MotorState;

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    pub fn new() -> Result<Self, std::io::Error> {
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub fn init(&self) -> Result<(), Error> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, Hide)?;
        Ok(())
    }

    pub fn restore(&self) -> Result<(), Error> {
        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen, Show)?;
        Ok(())
    }

    pub fn draw(&mut self, state: &MotorState) -> Result<(), Error> {
        self.terminal.draw(|frame| {
            // Vertical split: left 70% | right 30%
            let [left, right] =
                Layout::horizontal([Constraint::Percentage(70), Constraint::Percentage(30)])
                    .areas(frame.area());

            // Right side split horizontally: top 70% / bottom 30%
            let [top_right, bottom_right] =
                Layout::vertical([Constraint::Percentage(70), Constraint::Percentage(30)])
                    .areas(right);

            Self::draw_left(frame, left, state);
            Self::draw_top_right(frame, top_right, state);
            Self::draw_bottom_right(frame, bottom_right, state);
        })?;
        Ok(())
    }

    /// Region 1: circle with 8 sticks outside + center ball with a stick.
    fn draw_left(frame: &mut Frame, area: Rect, _state: &MotorState) {
        let block = Block::default().title("Practica_6").borders(Borders::ALL);
        let inner = block.inner(area);

        // Braille = 2x4 dots per cell, so dots are ~square: circle stays round.
        let w = inner.width as f64 * 2.0;
        let h = inner.height as f64 * 4.0;
        let r = w.min(h) / 2.0 * 0.6;

        let canvas = Canvas::default()
            .block(block)
            .x_bounds([-w / 2.0, w / 2.0])
            .y_bounds([-h / 2.0, h / 2.0])
            .paint(move |ctx| {
                ctx.draw(&Circle {
                    x: 0.0,
                    y: 0.0,
                    radius: r,
                    color: Color::Cyan,
                });

                // 8 sticks outside the circumference
                for k in 0..8 {
                    let a = TAU * k as f64 / 8.0;
                    ctx.draw(&Line {
                        x1: r * a.cos(),
                        y1: r * a.sin(),
                        x2: r * 1.3 * a.cos(),
                        y2: r * 1.3 * a.sin(),
                        color: Color::Cyan,
                    });
                }

                // Center ball + stick (pointing up; swap the angle for motor state later)
                ctx.draw(&Circle {
                    x: 0.0,
                    y: 0.0,
                    radius: r * 0.12,
                    color: Color::Yellow,
                });
                ctx.draw(&Line {
                    x1: 0.0,
                    y1: r * 0.12,
                    x2: 0.0,
                    y2: r * 0.8,
                    color: Color::Yellow,
                });
            });

        frame.render_widget(canvas, area);
    }

    /// Region 2: top right.
    fn draw_top_right(frame: &mut Frame, area: Rect, _state: &MotorState) {
        let block = Block::default().title("Region 2").borders(Borders::ALL);
        frame.render_widget(block, area);
    }

    /// Region 3: bottom right.
    fn draw_bottom_right(frame: &mut Frame, area: Rect, _state: &MotorState) {
        let block = Block::default().title("Region 3").borders(Borders::ALL);
        frame.render_widget(block, area);
    }
}
