use ratzilla::ratatui::layout::Rect;
use ratzilla::ratatui::layout::{Constraint, Layout};
use ratzilla::ratatui::widgets::Block;
use ratzilla::ratatui::Frame;

use ratzilla::ratatui::text;
use ratzilla::ratatui::widgets::{Paragraph, Wrap};

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

pub fn render_logo_text(frame: &mut Frame, area: Rect) -> Rect {
    let chunks = Layout::vertical([Constraint::Max(10), Constraint::Min(50)])
        .margin(1)
        .split(area);

    let mut lines: Vec<text::Line> = vec![];
    for line in LOGO_RAW.lines() {
        lines.push(text::Line::from(line));
    }
    let block = Block::new();
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, chunks[0]);
    chunks[1]
}
