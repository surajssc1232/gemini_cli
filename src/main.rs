use std::env;
use std::io::{self, Write};

use pulldown_cmark::{Event, Parser, Tag, CodeBlockKind};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use bat::PrettyPrinter;
use textwrap::{wrap, Options};
use term_size;

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
const LIST_ITEM_BULLET: &str = "▶";

/// Renders markdown text to the terminal with ANSI colors and formatting.
fn render_markdown(text: &str) {
    let (cols, _rows) = term_size::dimensions().unwrap_or((80, 24));
    let wrap_width = cols / 2;
    let wrap_options = Options::new(wrap_width).word_separator(textwrap::WordSeparator::AsciiSpace);

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
                                Tag::Heading { .. } => print!("{}{}", BOLD, HEADING_COLOR),
                Tag::BlockQuote(_) => print!("{}│ {}", YELLOW, RESET),
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    if let CodeBlockKind::Fenced(lang) = kind {
                        code_language = lang.into_string();
                        if code_language.is_empty() {
                            code_language = String::from("txt");
                        }
                    } else {
                        code_language = String::from("txt"); // Default for indented code blocks
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
                        print!("\n{}{} {} {}", indent, MAGENTA, LIST_ITEM_BULLET, RESET);
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
            Event::End(tag) => match tag {
                                pulldown_cmark::TagEnd::Paragraph => print!("\n\n"),
                pulldown_cmark::TagEnd::Heading(_) => print!("{}\n\n", RESET),
                pulldown_cmark::TagEnd::BlockQuote => print!("\n\n"),
                pulldown_cmark::TagEnd::CodeBlock => {
                    in_code_block = false;
                    
                    // Use bat for code highlighting
                    PrettyPrinter::new()
                        .input_from_bytes(code_buffer.as_bytes())
                        .language(&code_language)
                        .line_numbers(true)
                        .grid(true)
                        .print()
                        .unwrap();
                    print!("\n\n"); // Add two newlines after code block
                    
                    code_buffer.clear();
                    code_language = String::from("text");
                }
                pulldown_cmark::TagEnd::List(_) => {
                    list_stack.pop();
                    if list_stack.is_empty() {
                         print!("\n");
                    }
                }
                pulldown_cmark::TagEnd::Item => (),
                pulldown_cmark::TagEnd::Emphasis | pulldown_cmark::TagEnd::Strong | pulldown_cmark::TagEnd::Strikethrough => print!("{}", RESET),
                pulldown_cmark::TagEnd::Link => {
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
                }
                else {
                    for line in wrap(text.as_ref(), &wrap_options).into_iter() {
                        print!("{}", line);
                    }
                }
            }
            Event::Code(text) => print!("  {}{}{} ", KEYWORD_COLOR, text, RESET),
            Event::HardBreak => print!("\n"),
            Event::SoftBreak => print!(" "),
            Event::Rule => println!("\n{}{} {}\n", DIM, "─".repeat(30), RESET),
            _ => {}
        }
    }
    io::stdout().flush().unwrap();
    println!("\n"); // This will ensure a single blank line after the entire response
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
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-lite-preview-06-17:generateContent?key={}",
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
    

    println!("{}╭─────────────────────────────────────────────╮{}", BLUE, RESET);
    println!("{}│             {}Gemini AI REPL v2.2{}             │{}", BLUE, BOLD, RESET, BLUE);
    println!("{}│   Type 'help' for commands or a prompt.     │{}", BLUE, RESET);
    println!("{}╰─────────────────────────────────────────────╯{}", BLUE, RESET);
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
                println!("  {}help{}      - Show this help message", KEYWORD_COLOR, RESET);
                println!("  {}clear{}     - Clear the terminal screen", KEYWORD_COLOR, RESET);
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
