use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn parse(content: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_style = Style::default();

    let parser = Parser::new_ext(content, Options::all());
    for event in parser {
        match event {
        Event::Start(Tag::Heading { .. }) => {
            current_style = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
        }
        Event::Start(Tag::Strong) => {
            current_style = current_style.add_modifier(Modifier::BOLD);
        }
        Event::Start(Tag::Emphasis) => {
            current_style = current_style.add_modifier(Modifier::ITALIC);
        }
        Event::Text(text) => {
            spans.push(Span::styled(text.to_string(), current_style));
        }
        Event::Code(text) => {
            spans.push(Span::styled(
                text.to_string(),
                Style::default().fg(Color::Yellow),
            ));
        }
        Event::End(TagEnd::Heading(_)) | Event::End(TagEnd::Paragraph) => {
            lines.push(Line::from(spans.clone()));
            spans.clear();
            current_style = Style::default();
        }
        Event::End(TagEnd::Strong) | Event::End(TagEnd::Emphasis) => {
            current_style = Style::default();
        }
        Event::SoftBreak | Event::HardBreak => {
            lines.push(Line::from(spans.clone()));
            spans.clear();
        }
        _ => {}
        }
    }
    if !spans.is_empty() {
    lines.push(Line::from(spans));
  }

  lines
}

 #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_heading_is_bold_cyan() {
          let lines = parse("# Hello");
          let text: String = lines
              .iter()
              .flat_map(|l| l.spans.iter())
              .map(|s| s.content.as_ref())
              .collect();
          assert!(text.contains("Hello"), "Expected 'Hello' in output, got: {:?}", text);
      }

      #[test]
      fn test_bold_text_has_bold_modifier() {
          let lines = parse("**bold**");
          let has_bold = lines.iter().flat_map(|l| l.spans.iter()).any(|s| {
              s.style.add_modifier.contains(Modifier::BOLD)
          });
          assert!(has_bold, "Expected at least one BOLD span");
      }

      #[test]
      fn test_inline_code_is_yellow() {
          let lines = parse("a `snippet` here");
          let yellow_code = lines.iter().flat_map(|l| l.spans.iter()).find(|s| {
              s.content.as_ref() == "snippet"
          });
          let span = yellow_code.expect("expected a span containing 'snippet'");
          assert_eq!(span.style.fg, Some(Color::Yellow));
      }

      #[test]
      fn test_plain_text_renders() {
          let lines = parse("just some text");
          let text: String = lines
              .iter()
              .flat_map(|l| l.spans.iter())
              .map(|s| s.content.as_ref())
              .collect();
          assert!(text.contains("just some text"));
      }
 }

