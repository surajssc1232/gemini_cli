use std::env;
use std::io::{self, Write};

// Make sure to import TagEnd here
use pulldown_cmark::{Event, Parser, Tag, TagEnd, CodeBlockKind};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

// ANSI color codes for formatting
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const ITALIC: &str = "\x1b[3m";
const GREEN: &str = "\x1b[32m";
const BLUE: &str = "\x1b[34m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const MAGENTA: &str = "\x1b[35m";
const STRIKETHROUGH: &str = "\x1b[9m";

fn format_code_block_with_border(code: &str, language: &str) -> String {
    let lang_tag = if language.is_empty() { "text".to_string() } else { language.to_string() };
    let lines: Vec<&str> = code.lines().collect();

    let max_code_display_len = lines.iter().map(|s| {
        let mut display_len = 0;
        let mut in_ansi_escape = false;
        for c in s.chars() {
            if c == '\x1b' {
                in_ansi_escape = true;
            } else if in_ansi_escape && c.is_alphabetic() {
                in_ansi_escape = false;
            } else if !in_ansi_escape {
                display_len += 1;
            }
        }
        display_len
    }).max().unwrap_or(0);

    let line_width = std::cmp::max(max_code_display_len, lang_tag.len()).max(40); // Minimum width of 40

    let mut result = String::new();

    let dot_char = "-"; // Or " " for a space-dash-space effect

    let total_tag_display_len = lang_tag.len() + 2; // +2 for spaces around the tag " tag "
    let dashes_around_tag_len = line_width.saturating_sub(total_tag_display_len);
    let dashes_left = dashes_around_tag_len / 2;
    let dashes_right = (dashes_around_tag_len + 1) / 2; // +1 for odd lengths

    result.push_str(&format!(
        "{}{}{}{}{}{}{}\n",
        DIM, // Set color to Dim (often grey)
        dot_char.repeat(dashes_left),
        " ", // Space before tag
        BOLD, // Make tag bold
        lang_tag,
        RESET, // Reset bold for the right dots
        dot_char.repeat(dashes_right)
    ));


    // Code content
    for line in lines {
        result.push_str(&format!("{}\n", line));
    }

    // Bottom dotted line
    result.push_str(&format!("{}{}{}\n", DIM, dot_char.repeat(line_width), RESET));

    result
}

/// Renders markdown text to the terminal with ANSI colors and formatting.
fn render_markdown(text: &str, syntax_set: &SyntaxSet, theme_set: &ThemeSet) {
    let parser = Parser::new(text);
    let mut code_buffer = String::new();
    let mut code_language = String::from("text");
    let mut in_code_block = false;
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut link_stack: Vec<String> = Vec::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => (),
                Tag::Heading { .. } => print!("{}{}", BOLD, GREEN),
                Tag::BlockQuote(_) => print!("{}│ {}", YELLOW, RESET),
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    if let CodeBlockKind::Fenced(lang) = kind {
                        code_language = lang.into_string();
                    }
                }
                Tag::List(start_num) => {
                    list_stack.push(start_num);
                }
                Tag::Item => {
                    let indent = "  ".repeat(list_stack.len().saturating_sub(1));
                    if let Some(Some(num)) = list_stack.last_mut() {
                         print!("\n{}{} {}. {}", indent, MAGENTA, num, RESET);
                         *num += 1;
                    } else {
                        print!("\n{}{} * {}", indent, MAGENTA, RESET);
                    }
                }
                Tag::Emphasis => print!("{}", ITALIC),
                Tag::Strong => print!("{}", BOLD),
                Tag::Strikethrough => print!("{}", STRIKETHROUGH),
                Tag::Link { dest_url, .. } => {
                    link_stack.push(dest_url.into_string());
                    print!("[");
                }
                _ => {}
            },
            // FIX: Use `TagEnd` for the `Event::End` match
            Event::End(tag) => match tag {
                TagEnd::Paragraph => print!("\n"),
                TagEnd::Heading(_) => print!("{}\n\n", RESET),
                TagEnd::BlockQuote => print!("\n"),
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    let syntax = syntax_set
                        .find_syntax_by_extension(&code_language)
                        .or_else(|| syntax_set.find_syntax_by_name(&code_language))
                        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

                    let theme = &theme_set.themes["base16-ocean.dark"];
                    let mut highlighter = HighlightLines::new(syntax, theme);
                    let mut highlighted = String::new();

                    for line in LinesWithEndings::from(&code_buffer) {
                        let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, syntax_set).unwrap_or_default();
                        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                        highlighted.push_str(&escaped);
                    }
                    
                    print!("{}", format_code_block_with_border(&highlighted.trim_end(), &code_language));

                    code_buffer.clear();
                    code_language = String::from("text");
                }
                TagEnd::List(_) => {
                    list_stack.pop();
                    if list_stack.is_empty() {
                         print!("\n");
                    }
                }
                TagEnd::Item => (),
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => print!("{}", RESET),
                TagEnd::Link => {
                    if let Some(url) = link_stack.pop() {
                        print!("]({}{}{})", BLUE, url, RESET);
                    } else {
                        // Fallback if something goes wrong with link stack
                        print!("]");
                    }
                },
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    code_buffer.push_str(&text);
                } else {
                    print!("{}", text);
                }
            }
            Event::Code(text) => print!("{}{}{}", CYAN, text, RESET),
            Event::HardBreak => print!("\n"),
            Event::SoftBreak => print!(" "),
            Event::Rule => println!("\n{}{} {}\n", DIM, "─".repeat(30), RESET),
            _ => {}
        }
    }
    io::stdout().flush().unwrap();
}


#[derive(Serialize)]
struct GeminiRequest { contents: Vec<Content> }
#[derive(Serialize)]
struct Content { parts: Vec<Part> }
#[derive(Serialize)]
struct Part { text: String }
#[derive(Deserialize)]
struct GeminiResponse { candidates: Vec<Candidate> }
#[derive(Deserialize)]
struct Candidate { content: ResponseContent }
#[derive(Deserialize)]
struct ResponseContent { parts: Vec<ResponsePart> }
#[derive(Deserialize)]
struct ResponsePart { text: String }

async fn send_to_gemini(
    client: &Client,
    api_key: &str,
    text: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash-latest:generateContent?key={}",
        api_key
    );
    let request_body = GeminiRequest { contents: vec![Content { parts: vec![Part { text: text.to_string() }] }] };
    let response = client.post(&url).json(&request_body).send().await?;
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("API Error: {}", error_text).into());
    }
    let gemini_response: GeminiResponse = response.json().await?;
    if let Some(candidate) = gemini_response.candidates.first() {
        if let Some(part) = candidate.content.parts.first() {
            return Ok(part.text.clone());
        }
    }
    Err("No response content found".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");
    let client = Client::new();
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();

    println!("{}╭─────────────────────────────────────────────╮{}", CYAN, RESET);
    println!("{}│             {}Gemini AI REPL v2.2{}             │{}", CYAN, BOLD, RESET, CYAN);
    println!("{}│   Type 'help' for commands or a prompt.   │{}", CYAN, RESET);
    println!("{}╰─────────────────────────────────────────────╯{}", CYAN, RESET);
    println!();

    loop {
        print!("{}> {}", MAGENTA, RESET);
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            eprintln!("{}Error reading input.{}", RED, RESET);
            break;
        }
        
        let input = input.trim();

        match input {
            "" => continue,
            "quit" | "exit" => {
                println!("{}Goodbye!{}", YELLOW, RESET);
                break;
            }
            "clear" => {
                print!("\x1b[2J\x1b[H");
                io::stdout().flush().unwrap();
                continue;
            }
            "help" => {
                // FIX: Added the missing `RESET` argument
                println!("{}Available Commands:{}", BOLD, RESET);
                println!("  {}help{}      - Show this help message", CYAN, RESET);
                println!("  {}clear{}     - Clear the terminal screen", CYAN, RESET);
                println!("  {}quit/exit{} - Exit the REPL", CYAN, RESET);
                println!("\nJust type any other message to chat with Gemini!");
                continue;
            }
            _ => {
                print!("\r{}Thinking...{}", YELLOW, RESET);
                io::stdout().flush().unwrap();

                match send_to_gemini(&client, &api_key, input).await {
                    Ok(response) => {
                        print!("\r{}\r", " ".repeat(15)); // Clear "Thinking..."
                        println!("{}Gemini:{}", BOLD, RESET);
                        render_markdown(&response, &syntax_set, &theme_set);
                    }
                    Err(e) => {
                        print!("\r{}\r", " ".repeat(15));
                        eprintln!("{}Error:{} {}", RED, RESET, e);
                    }
                }
            }
        }
    }

    Ok(())
}
