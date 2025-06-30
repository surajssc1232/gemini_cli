use std::env;
use std::io::{self, Write};

use bat::PrettyPrinter;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, Options as ParserOptions, HeadingLevel, TagEnd};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use term_size;
use textwrap::{wrap, Options};

// ANSI color codes for formatting
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const ITALIC: &str = "\x1b[3m";
const HEADING_COLOR: &str = "\x1b[38;5;40m"; // A vibrant green
const BLUE: &str = "\x1b[34m";
const KEYWORD_COLOR: &str = "\x1b[38;5;111m"; // A distinct blue/cyan
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const MAGENTA: &str = "\x1b[35m";
const STRIKETHROUGH: &str = "\x1b[9m";
const LIST_ITEM_BULLET: &str = "▸";

/// Renders markdown text to the terminal with ANSI colors and formatting.
fn render_markdown(text: &str) {
    let (cols, _rows) = term_size::dimensions().unwrap_or((80, 24));
    let wrap_width = (cols * 3 / 4).min(100); // Better width calculation
    let wrap_options = Options::new(wrap_width)
        .word_separator(textwrap::WordSeparator::AsciiSpace)
        .break_words(false);

    let parser = Parser::new_ext(text, ParserOptions::all()); // Enable all markdown extensions
    let mut code_buffer = String::new();
    let mut code_language = String::from("text");
    let mut in_code_block = false;
    let mut list_stack: Vec<(Option<u64>, usize)> = Vec::new(); // (start_num, indent_level)
    let mut link_stack: Vec<String> = Vec::new();
    let mut pending_newlines = 0;
    let mut at_line_start = true;
    let mut last_was_list_item = false;
    

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    if !at_line_start && !last_was_list_item {
                        pending_newlines = pending_newlines.max(1);
                    }
                    
                }
                Tag::Heading { level, .. } => {
                    flush_newlines(&mut pending_newlines, 2);
                    let header_prefix = match level {
                        HeadingLevel::H1 => "# ",
                        HeadingLevel::H2 => "## ",
                        HeadingLevel::H3 => "### ",
                        HeadingLevel::H4 => "#### ",
                        HeadingLevel::H5 => "##### ",
                        HeadingLevel::H6 => "###### ",
                    };
                    print!("{}{}{}", BOLD, HEADING_COLOR, header_prefix);
                    at_line_start = false;
                }
                Tag::BlockQuote(_) => {
                    flush_newlines(&mut pending_newlines, 1);
                    at_line_start = true;
                }
                Tag::CodeBlock(kind) => {
                    flush_newlines(&mut pending_newlines, 1);
                    in_code_block = true;
                    code_language = match kind {
                        CodeBlockKind::Fenced(lang) => {
                            let lang_str = lang.to_string();
                            if lang_str.is_empty() {
                                "txt".to_string()
                            } else {
                                lang_str
                            }
                        }
                        CodeBlockKind::Indented => "txt".to_string(),
                    };
                }
                Tag::List(start_num) => {
                    if !list_stack.is_empty() {
                        pending_newlines = pending_newlines.max(1);
                    } else {
                        flush_newlines(&mut pending_newlines, 1);
                    }
                    let indent_level = list_stack.len();
                    list_stack.push((start_num, indent_level));
                }
                Tag::Item => {
                    if !at_line_start {
                        print!("\n");
                    }

                    let current_level = list_stack.len().saturating_sub(1);
                    let indent = "  ".repeat(current_level);

                    if let Some((Some(num), _)) = list_stack.last_mut() {
                        print!("{}{}{:2}. {}", indent, MAGENTA, num, RESET);
                        *num += 1;
                    } else {
                        print!("{}{} {} {}", indent, MAGENTA, LIST_ITEM_BULLET, RESET);
                    }
                    at_line_start = false;
                    last_was_list_item = true;
                    pending_newlines = 0;
                }
                Tag::Emphasis => print!("{}", ITALIC),
                Tag::Strong => print!("{}{}", BOLD, YELLOW),
                Tag::Strikethrough => print!("{}", STRIKETHROUGH),
                Tag::Link { dest_url, .. } => {
                    link_stack.push(dest_url.to_string());
                    print!("{}[", BLUE);
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                
                TagEnd::Heading(_) => {
                    print!("{}", RESET);
                    pending_newlines = pending_newlines.max(2);
                    at_line_start = true;
                }
                TagEnd::BlockQuote => {
                    pending_newlines = pending_newlines.max(1);
                    at_line_start = true;
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    render_code_block(&code_buffer, &code_language);
                    code_buffer.clear();
                    code_language = String::from("text");
                    pending_newlines = pending_newlines.max(1);
                    at_line_start = true;
                }
                TagEnd::List(_) => {
                    list_stack.pop();
                    if list_stack.is_empty() {
                        pending_newlines = pending_newlines.max(1);
                        last_was_list_item = false;
                    }
                    at_line_start = true;
                }
                TagEnd::Item => {
                    // Don't add extra newlines here, handled by next item or list end
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                    print!("{}", RESET);
                }
                TagEnd::Link => {
                    if let Some(url) = link_stack.pop() {
                        print!("]({}{}{})", BLUE, url, RESET);
                    } else {
                        print!("]");
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    code_buffer.push_str(&text);
                } else {
                    flush_newlines(&mut pending_newlines, 0);
                    render_text(&text, &wrap_options, &list_stack, &mut at_line_start);
                }
            }
            Event::Code(text) => {
                print!("{}`{}`{}", KEYWORD_COLOR, text, RESET);
                at_line_start = false;
            }
            Event::HardBreak => {
                print!("\n");
                at_line_start = true;
            }
            Event::SoftBreak => {
                if !at_line_start {
                    print!(" ");
                }
            }
            Event::Rule => {
                flush_newlines(&mut pending_newlines, 1);
                println!("{}{}{}", DIM, "─".repeat(wrap_width.min(50)), RESET);
                pending_newlines = pending_newlines.max(1);
                at_line_start = true;
            }
            Event::Html(html) => {
                // Basic HTML tag stripping for cleaner output
                if !html.trim().is_empty() && !html.starts_with('<') {
                    flush_newlines(&mut pending_newlines, 0);
                    print!("{}", html);
                    at_line_start = false;
                }
            }
            _ => {}
        }
    }

    // Final cleanup
    if !at_line_start {
        print!("\n");
    }
    io::stdout().flush().unwrap();
}

fn flush_newlines(pending: &mut usize, min_newlines: usize) {
    let newlines_to_print = (*pending).max(min_newlines);
    for _ in 0..newlines_to_print {
        print!("\n");
    }
    *pending = 0;
}

fn render_text(
    text: &str,
    wrap_options: &Options,
    list_stack: &[(Option<u64>, usize)],
    at_line_start: &mut bool,
) {
    let current_indent = if !list_stack.is_empty() {
        let indent_level = list_stack.len() - 1;
        "  ".repeat(indent_level + 1) // +1 for alignment with list marker
    } else {
        String::new()
    };

    let text = text.trim_start_matches('\n').trim_end_matches('\n');
    if text.is_empty() {
        return;
    }

    // Handle blockquote prefix
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if *at_line_start && !current_indent.is_empty() && i > 0 {
            print!("{}", current_indent);
        }

        // Wrap the line if it's too long
        let effective_width = wrap_options.width.saturating_sub(current_indent.len());
        let wrapped_lines = wrap(line, effective_width);

        for (j, wrapped_line) in wrapped_lines.iter().enumerate() {
            if j > 0 {
                print!("\n{}", current_indent);
            }
            print!("{}", wrapped_line);
        }

        if i < lines.len() - 1 {
            print!("\n");
            *at_line_start = true;
        } else {
            *at_line_start = false;
        }
    }
}

fn render_code_block(code: &str, language: &str) {
    if code.trim().is_empty() {
        return;
    }

    // Try to use bat for syntax highlighting, fallback to simple display
    match PrettyPrinter::new()
        .input_from_bytes(code.trim_end().as_bytes())
        .language(language)
        .line_numbers(true)
        .grid(true)
        .header(false)
        .rule(false)
        .print()
    {
        Ok(_) => {}
        Err(_) => {
            // Fallback: simple code block rendering
            println!("{}┌{}", DIM, "─".repeat(50));
            for line in code.lines() {
                println!("{}│{} {}", DIM, RESET, line);
            }
            println!("{}└{}{}", DIM, "─".repeat(50), RESET);
        }
    }
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}
#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}
#[derive(Serialize)]
struct Part {
    text: String,
}
#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}
#[derive(Deserialize)]
struct Candidate {
    content: ResponseContent,
}
#[derive(Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}
#[derive(Deserialize)]
struct ResponsePart {
    text: String,
}

async fn send_to_gemini(
    client: &Client,
    api_key: &str,
    text: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-lite-preview-06-17:generateContent?key={}",
        api_key
    );
    let request_body = GeminiRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: text.to_string(),
            }],
        }],
    };
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
                println!("{}Available Commands:{}", BOLD, RESET);
                println!(
                    "  {}help{}      - Show this help message",
                    KEYWORD_COLOR, RESET
                );
                println!(
                    "  {}clear{}     - Clear the terminal screen",
                    KEYWORD_COLOR, RESET
                );
                println!("  {}quit/exit{} - Exit the REPL", KEYWORD_COLOR, RESET);
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
                        render_markdown(&response);
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
