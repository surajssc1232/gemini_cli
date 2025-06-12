Gemini CLI

A simple command-line interface (CLI) application built with Rust that allows you to chat with the Gemini AI model directly from your terminal.
Features

    Interactive REPL: Engage in a conversation with Gemini AI in a continuous chat session.

    Markdown Rendering: Displays Gemini's responses, including code blocks, with proper syntax highlighting and formatting in the terminal.

    Basic Commands: Includes help, clear, quit, and exit commands for easy management.

Prerequisites

Before you begin, ensure you have the following installed:

    Rust and Cargo: If you don't have Rust and Cargo (Rust's package manager) installed, you can get them via rustup:

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

    Follow the on-screen instructions.

Installation

    Clone the Repository:
    First, clone the gemini_cli repository to your local machine:

    git clone <repository-url> # Replace with the actual repository URL if available
    cd gemini_cli

    If you don't have a repository URL, you can create a new project and place the main.rs file inside the src directory:

    cargo new gemini_cli
    cd gemini_cli
    # Place the main.rs file into src/main.rs

    Build the Project:
    Navigate into the cloned directory and build the project using Cargo:

    cargo build --release

    This command compiles the project in release mode, which optimizes the binary for performance. The compiled executable will be located in the target/release/ directory.

Configuration

The gemini_cli requires a Google Gemini API key to communicate with the Gemini model. You need to set this key as an environment variable.

Set the GEMINI_API_KEY Environment Variable:

    Linux/macOS:
    Add the following line to your shell's profile file (e.g., ~/.bashrc, ~/.zshrc, ~/.profile):

    export GEMINI_API_KEY="YOUR_GEMINI_API_KEY"

    Replace "YOUR_GEMINI_API_KEY" with your actual API key. After adding, source your profile file:

    source ~/.bashrc # Or ~/.zshrc, etc.

    Windows (Command Prompt):

    setx GEMINI_API_KEY "YOUR_GEMINI_API_KEY"

    Windows (PowerShell):

    $env:GEMINI_API_KEY="YOUR_GEMINI_API_KEY"

    For persistent setting, you might need to use system environment variables or add it to your PowerShell profile.

Usage

Once configured, you can run the gemini_cli from your terminal.

    Run the CLI:

    ./target/release/gemini_cli

    or if you are in the project root:

    cargo run --release

    Interact with Gemini:
    You will see a prompt >. Type your message and press Enter.

    ╭─────────────────────────────────────────────╮
    │             Gemini AI REPL v2.2             │
    │   Type 'help' for commands or a prompt.     │
    ╰─────────────────────────────────────────────╯

    > Hello Gemini, how are you?
    Thinking...
    Gemini:
    Hello! I'm doing well, thank you for asking! As an AI, I don't have feelings or physical states, but I'm always ready to assist you. How can I help you today?

    > Can you tell me a joke?
    Thinking...
    Gemini:
    Why don't scientists trust atoms?
    Because they make up everything!

    >

    Available Commands:

        help: Displays the list of available commands.

        clear: Clears the terminal screen.

        quit or exit: Exits the REPL.

Contributing

If you'd like to contribute to this project, please feel free to fork the repository and submit pull requests.
License

This project is licensed under the MIT License.
