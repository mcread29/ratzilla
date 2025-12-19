use crate::state::{StateActions, StateMachineError};
use ratzilla::event::KeyCode;
use ratzilla::ratatui::style::{Color, Modifier, Style};
use ratzilla::ratatui::text::{self, Span};
use ratzilla::ratatui::widgets::{Block, Paragraph, Wrap};
use ratzilla::ratatui::Frame;
use tachyonfx::Duration;

const LOGO_RAW: &str = r#"
  _   _          ___   
 | | | |        / _ \  
 | |_| |_ _   _| | | | 
 | __| __| | | | | | | 
 | |_| |_| |_| | |_| | 
  \__|\__|\__, |\___/  
           __/ |       
          |___/        
"#;

pub struct IntroState {
    blink_speed: Duration,
    cursor_on: bool,
    last_blink: web_time::Instant,
    state_time: Duration,
}

impl IntroState {
    pub fn new(blink_speed: Duration) -> Self {
        Self {
            blink_speed,
            cursor_on: true,
            last_blink: web_time::Instant::now(),
            state_time: Duration::from_millis(0),
        }
    }
    pub fn create() -> Box<dyn StateActions> {
        Box::new(IntroState::new(Duration::from_millis(1000)))
    }
}

impl StateActions for IntroState {
    fn can_enter_state(&self) -> Result<bool, StateMachineError> {
        Ok(true)
    }

    fn enter_state(&self) -> Result<(), StateMachineError> {
        Ok(())
    }

    fn should_exit_state(&self) -> Result<&str, StateMachineError> {
        Ok("")
    }

    fn can_exit_state(&self) -> Result<bool, StateMachineError> {
        Ok(false)
    }

    fn exit_state(&self) -> Result<(), StateMachineError> {
        Ok(())
    }

    fn update_state(&mut self, elapsed: Duration) -> Result<(), StateMachineError> {
        let e = web_time::Instant::now().duration_since(self.last_blink);
        if e.as_millis() as u32 > self.blink_speed.as_millis() {
            self.cursor_on = !self.cursor_on;
            self.last_blink = web_time::Instant::now();
        }
        self.state_time += elapsed;
        Ok(())
    }

    fn render_state(&self, frame: &mut Frame) {
        let mut text: Vec<text::Line> = vec![];

        text.push(text::Line::from(Span::styled(
            format!("{:.2}s", self.state_time.as_secs_f32()),
            Style::default().fg(Color::LightRed),
        )));
        for line in LOGO_RAW.lines() {
            text.push(text::Line::from(line));
        }
        text.push(text::Line::from(""));
        text.push(text::Line::from(Span::styled(
            "press any key to continue",
            Style::default().fg(Color::LightRed),
        )));
        if self.cursor_on {
            text.last_mut().unwrap().push_span(Span::styled(
                " █",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        let block = Block::bordered().title(Span::styled(
            "",
            Style::default(), // .fg(Color::LightMagenta)
                              // .add_modifier(Modifier::BOLD),
        ));
        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, frame.area());
    }

    fn key_press(&mut self, key: KeyCode) -> Result<(), StateMachineError> {
        match key {
            KeyCode::Char('q') => {
                self.exit_state()?;
            }
            _ => {}
        }
        Ok(())
    }
}
