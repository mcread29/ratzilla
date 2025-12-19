use ratzilla::ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{self, Span},
    widgets::{
        Axis, BarChart, Block, Chart, Dataset, Gauge, LineGauge, List, ListItem, Paragraph,
        Sparkline, Wrap,
    },
    Frame,
};
use tachyonfx::Duration;

use crate::app::App;

pub fn draw(elapsed: Duration, frame: &mut Frame, app: &mut App) {
    draw_first_tab(frame, app, frame.area());

    let area = frame.area();
    app.effects
        .process_effects(elapsed, frame.buffer_mut(), area);
}

fn draw_first_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(9),
        Constraint::Min(8),
        Constraint::Length(7),
    ])
    .split(area);
    draw_gauges(frame, app, chunks[0]);
    draw_charts(frame, app, chunks[1]);
    draw_text(frame, chunks[2]);
}

fn draw_gauges(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Length(2),
    ])
    .margin(1)
    .split(area);
    let block = Block::bordered().title("Graphs");
    frame.render_widget(block, area);

    let label = format!("{:.2}%", app.progress * 100.0);
    let gauge = Gauge::default()
        .block(Block::new().title("Gauge:"))
        .gauge_style(
            Style::default()
                .fg(Color::LightMagenta)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC | Modifier::BOLD),
        )
        .use_unicode(app.enhanced_graphics)
        .label(label)
        .ratio(app.progress);
    frame.render_widget(gauge, chunks[0]);

    let sparkline = Sparkline::default()
        .block(Block::new().title("Sparkline:"))
        .style(Style::default().fg(Color::LightGreen))
        .data(&app.sparkline.points)
        .bar_set(if app.enhanced_graphics {
            symbols::bar::NINE_LEVELS
        } else {
            symbols::bar::THREE_LEVELS
        });
    frame.render_widget(sparkline, chunks[1]);

    let line_gauge = LineGauge::default()
        .block(Block::new().title("LineGauge:"))
        .filled_style(Style::default().fg(Color::LightMagenta))
        .line_set(if app.enhanced_graphics {
            symbols::line::THICK
        } else {
            symbols::line::NORMAL
        })
        .ratio(app.progress);
    frame.render_widget(line_gauge, chunks[2]);
}

#[allow(clippy::too_many_lines)]
fn draw_charts(frame: &mut Frame, app: &mut App, area: Rect) {
    let constraints = if app.show_chart {
        vec![Constraint::Percentage(50), Constraint::Percentage(50)]
    } else {
        vec![Constraint::Percentage(100)]
    };
    let chunks = Layout::horizontal(constraints).split(area);
    {
        let chunks = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[0]);
        {
            let chunks =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(chunks[0]);

            // Draw tasks
            let tasks: Vec<ListItem> = app
                .tasks
                .items
                .iter()
                .map(|i| ListItem::new(vec![text::Line::from(Span::raw(*i))]))
                .collect();
            let tasks = List::new(tasks)
                .block(Block::bordered().title("List"))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(tasks, chunks[0], &mut app.tasks.state);

            // Draw logs
            let info_style = Style::default().fg(Color::Green);
            let warning_style = Style::default().fg(Color::LightYellow);
            let error_style = Style::default().fg(Color::LightMagenta);
            let critical_style = Style::default().fg(Color::LightRed);
            let logs: Vec<ListItem> = app
                .logs
                .items
                .iter()
                .map(|&(evt, level)| {
                    let s = match level {
                        "ERROR" => error_style,
                        "CRITICAL" => critical_style,
                        "WARNING" => warning_style,
                        _ => info_style,
                    };
                    let content = vec![text::Line::from(vec![
                        Span::styled(format!("{level:<9}"), s),
                        Span::raw(evt),
                    ])];
                    ListItem::new(content)
                })
                .collect();
            let logs = List::new(logs).block(Block::bordered().title("List"));
            frame.render_stateful_widget(logs, chunks[1], &mut app.logs.state);
        }

        let barchart = BarChart::default()
            .block(Block::bordered().title("Bar Chart"))
            .data(&app.barchart)
            .bar_width(3)
            .bar_gap(2)
            .bar_set(if app.enhanced_graphics {
                symbols::bar::NINE_LEVELS
            } else {
                symbols::bar::THREE_LEVELS
            })
            .value_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::ITALIC),
            )
            .label_style(Style::default().fg(Color::Yellow))
            .bar_style(Style::default().fg(Color::LightGreen));
        frame.render_widget(barchart, chunks[1]);
    }
    if app.show_chart {
        let x_labels = vec![
            Span::styled(
                format!("{}", app.signals.window[0]),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "{}",
                (app.signals.window[0] + app.signals.window[1]) / 2.0
            )),
            Span::styled(
                format!("{}", app.signals.window[1]),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];
        let datasets = vec![
            Dataset::default()
                .name("data2")
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::White))
                .data(&app.signals.sin1.points),
            Dataset::default()
                .name("data3")
                .marker(if app.enhanced_graphics {
                    symbols::Marker::Braille
                } else {
                    symbols::Marker::Dot
                })
                .style(Style::default().fg(Color::LightCyan))
                .data(&app.signals.sin2.points),
        ];
        let chart = Chart::new(datasets)
            .block(
                Block::bordered().title(Span::styled(
                    "Chart",
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                )),
            )
            .x_axis(
                Axis::default()
                    .title("X Axis")
                    .style(Style::default().fg(Color::Gray))
                    .bounds(app.signals.window)
                    .labels(x_labels),
            )
            .y_axis(
                Axis::default()
                    .title("Y Axis")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([-20.0, 20.0])
                    .labels([
                        Span::styled("-20", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw("0"),
                        Span::styled("20", Style::default().add_modifier(Modifier::BOLD)),
                    ]),
            );
        frame.render_widget(chart, chunks[1]);
    }
}

fn draw_text(frame: &mut Frame, area: Rect) {
    let text = vec![
        text::Line::from("This is a paragraph with several lines. You can change style your text the way you want"),
        text::Line::from(""),
        text::Line::from(vec![
            Span::from("For example: "),
            Span::styled("under", Style::default().fg(Color::LightRed)),
            Span::raw(" "),
            Span::styled("the", Style::default().fg(Color::LightGreen)),
            Span::raw(" "),
            Span::styled("rainbow", Style::default().fg(Color::LightCyan)),
            Span::raw("."),
        ]),
        text::Line::from(vec![
            Span::raw("Oh and if you didn't "),
            Span::styled("notice", Style::default().add_modifier(Modifier::ITALIC)),
            Span::raw(" you can "),
            Span::styled("automatically", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled("wrap", Style::default().add_modifier(Modifier::REVERSED)),
            Span::raw(" your "),
            Span::styled("text", Style::default().add_modifier(Modifier::UNDERLINED)),
            Span::raw(".")
        ]),
        text::Line::from(
            "One more thing is that it should display unicode characters: 10€"
        ),
    ];
    let block = Block::bordered().title(Span::styled(
        "Footer",
        Style::default()
            .fg(Color::LightMagenta)
            .add_modifier(Modifier::BOLD),
    ));
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}
