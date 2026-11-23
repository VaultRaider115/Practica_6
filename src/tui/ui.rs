use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::{
        Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph,
        canvas::{Canvas, Circle, Line},
    },
};
use std::{collections::VecDeque, f64::consts::TAU};
use std::{
    f32::consts::FRAC_PI_2,
    io::{Stdout, stdout},
};

use crate::{
    app::{App, AppError},
    motor::CurrentMa,
};

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    pub fn new() -> Result<Self, AppError> {
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub fn init(&self) -> Result<(), AppError> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, Hide, Clear(ClearType::All))?;
        Ok(())
    }

    pub fn restore(&self) -> Result<(), AppError> {
        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen, Show)?;
        Ok(())
    }

    pub fn draw(&mut self, app: &App) -> Result<(), AppError> {
        self.terminal.draw(|frame| {
            // Vertical split: left 70% | right 30%
            let [left, right] =
                Layout::horizontal([Constraint::Percentage(70), Constraint::Percentage(30)])
                    .areas(frame.area());

            // Right side split horizontally: top 70% / bottom 30%
            let [top_right, bottom_right] =
                Layout::vertical([Constraint::Percentage(70), Constraint::Percentage(30)])
                    .areas(right);

            Self::draw_left(frame, left, app);
            Self::draw_top_right(frame, top_right, app.current_history());
            Self::draw_bottom_right(frame, bottom_right, app);
        })?;
        Ok(())
    }

    /// Region 1: circle with 8 sticks outside + center ball with a stick.
    fn draw_left(frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default().title("Practica_6").borders(Borders::ALL);
        let inner = block.inner(area);

        // Braille = 2x4 dots per cell, so dots are ~square: circle stays round.
        let w = inner.width as f64 * 2.0;
        let h = inner.height as f64 * 4.0;
        let r = w.min(h) / 2.0 * 0.6;

        let current_degrees = app.angle_degrees();
        let angle = current_degrees.to_radians();

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

                let ball_r = r * 0.12;
                let needle_length = r * 0.8;
                let (dx, dy) = (
                    (angle + FRAC_PI_2 as f64).cos(),
                    (angle + FRAC_PI_2 as f64).sin(),
                );

                ctx.draw(&Circle {
                    x: 0.0,
                    y: 0.0,
                    radius: ball_r,
                    color: Color::Yellow,
                });

                ctx.draw(&Line {
                    x1: ball_r * dx,
                    y1: ball_r * dy,
                    x2: needle_length * dx,
                    y2: needle_length * dy,
                    color: Color::Yellow,
                });
            });

        frame.render_widget(canvas, area);
    }

    /// Region 2: top right.
    fn draw_top_right(frame: &mut Frame, area: Rect, history: &VecDeque<CurrentMa>) {
        let block = Block::default().title("Consumo").borders(Borders::ALL);

        let inner = block.inner(area);
        frame.render_widget(&block, area);

        if history.is_empty() {
            let paragraph = Paragraph::new("Esperando datos...").alignment(Alignment::Center);
            frame.render_widget(paragraph, inner);
        } else {
            let data: Vec<(f64, f64)> = history
                .iter()
                .enumerate()
                .map(|(index, current)| (index as f64, *current as f64))
                .collect();

            let dataset = Dataset::default()
                .name("Corriente")
                .data(&data)
                .graph_type(GraphType::Line);

            let x_axis = Axis::default()
                .title("Muestras")
                .style(Color::White)
                .bounds([0.0, 14.0])
                .labels(["0", "7", "14"]);

            let y_axis = Axis::default()
                .title("mA")
                .style(Color::White)
                .bounds([0.0, 250.0])
                .labels(["0", "100", "250"]);

            let chart = Chart::new(vec![dataset]).x_axis(x_axis).y_axis(y_axis);

            frame.render_widget(chart, inner);
        }
    }

    /// Region 3: bottom right.
    fn draw_bottom_right(frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default().title("Info").borders(Borders::ALL);
        frame.render_widget(&block, area);

        let current_degrees = app.angle_degrees();

        let paragraph =
            Paragraph::new(format!("Ángulo: {:.2}°", current_degrees)).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);

        let inner = block.inner(area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        let mut paragraphs: Vec<Paragraph> = Vec::new();
        paragraphs.extend([
            Paragraph::new("A/B -> Simple Step"),
            Paragraph::new("C/D -> Half Step"),
            Paragraph::new("E/F -> Double Step"),
            Paragraph::new("Q -> Exit"),
        ]);

        for (item, chunk) in paragraphs.into_iter().zip(chunks.iter()) {
            frame.render_widget(item, *chunk);
        }
    }
}
