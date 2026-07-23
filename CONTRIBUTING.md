# Contributing to hikari

## Getting Started

Make sure you have the latest stable version of Rust installed via [rustup](https://rustup.rs/). 

1. Fork the repository and clone it locally.
2. Run `cargo build` to verify the project compiles.
3. Run `cargo test` to ensure all base tests pass.

## Code Quality Standards

We want to keep our codebase clean, safe, and idiomatic. Before opening a Pull Request, please ensure you have run:

* **Formatting:** `cargo fmt --all`
* **Linting:** `cargo clippy --all -- -D warnings`
* **Testing:** `cargo test`

## AI-Assisted Development

We welcome the use of AI coding assistants (like GitHub Copilot, Claude, Cursor, ChatGPT) to help you write code faster. However, to maintain the quality and legal integrity of our codebase, we ask that you follow these guidelines based on the [Linux Kernel's AI policy](https://docs.kernel.org/process/coding-assistants.html):

### 1. Human Accountability (No AI Authors)
AI tools are assistants, not authors. As the human submitter, **you take full responsibility for the code you submit.** You must personally review, understand, and be able to explain every line of code in your contribution. 

### 2. Adherence to Project Standards
AI-generated code is held to the exact same standards as human-written code. The AI output must be idiomatic Rust, compile cleanly, pass `cargo clippy` without warnings, and include appropriate tests. Do not submit raw AI output without refining it.

### 3. License Compliance
You are responsible for ensuring that the AI tool did not introduce copyrighted, proprietary, or license-incompatible code into our repository. Ensure your contributions align with our project's open-source license.

### 4. Attribution
We value transparency and like to track how AI is helping shape the project. If you used an AI assistant to generate a significant portion of your contribution, please include an `Assisted-by:` tag at the end of your commit messages.

**Format:** `Assisted-by: <Tool Name>`

**Examples:**
* `Assisted-by: GitHub Copilot`
* `Assisted-by: Claude Code`

## Submitting a Pull Request

1. Create a new branch from `main` (e.g., `feature/add-new-route` or `fix/db-connection`).
2. Make your changes and commit them with clear, descriptive messages.
3. Push your branch and open a Pull Request.
4. If your PR resolves an open issue, link it in the description (e.g., "Closes #42").