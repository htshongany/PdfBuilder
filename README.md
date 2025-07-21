# PDFBuilder  A Chill Coding Experiment with Rust

**PDFBuilder** started as a vibe coding project no pressure, just pure curiosity, learning, and creativity.

At first, I just wanted a fast and easy way to create PDF.  
Why? Because **LaTeX**, while powerful, always felt overly complex the documentation can be unclear, the syntax harsh, and the error messages frustrating. It just didn’t feel smooth for simple writing.

So I thought:  
*What if I could take Markdown, run it through a lightweight tool, and get a clean, styled PDF with minimal setup?*

I had already made a version in **Python**, but it felt too slow and heavy.  
**C**? Too low-level and not worth the memory management.  
That’s when I gave **Rust** a try i didn’t find the syntax intimidating, but rather hard to read and unusual. For example: I would have preferred it to be more explicit like other languages writing `function` instead of `fn` feels clearer to me I’m still learning, and the project isn’t finished, but I’m starting to see Rust as a serious second language after Python.

> ⚠ The project is still in development and **currently only works on Windows** it’s not polished yet just a learning milestone for now.

---

## What is PDFBuilder?

A simple CLI tool to convert **Markdown to PDF**, fast and with style. It uses a clean folder structure and supports custom CSS themes.

### Project Structure

- `config.yaml`: Main configuration (title, author, theme, source files…)
- `chapters/`: Your Markdown files
- `assets/`: Images and other static content
- `themes/`: Custom CSS styles

---

### Clone the project

```bash
git clone https://github.com/htshongany/PdfBuilder
cd PdfBuilder
```

###  Initialize a new project

```bash
cargo run -- init
```

### Build the PDF

```bash
cargo run -- build
```

### Watch for changes (auto-rebuild)

```bash
cargo run -- build --watch
```