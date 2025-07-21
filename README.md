# PDFBuilder - A Vibe Coding Journey into Rust

This project is more than just a tool; it's a "vibe coding" experiment and my first real dive into the Rust programming language. I'm documenting this journey and will be writing a detailed article about the experience. For now, you can follow my other writings on [Medium](https://medium.com/@sirehassan).

## The Story

I originally built a similar PDF generation tool in Python. While it worked, it felt heavy and slow. I wanted something fast, efficient, and robust.

My first thought was C, but it's been a while, and I wasn't thrilled about the idea of manual memory management. Then there was Rust. I'll be honest, its syntax seemed intimidating at first, and I had my doubts. But I decided to take the plunge.

This project is the result of that decision. It's my journey of learning Rust, overcoming my initial perceptions, and building something I'm proud of. I'm now seriously considering Rust as my go-to second language after Python.

## What is PDFBuilder?

PDFBuilder is a simple, fast, and command-line-based tool for creating beautiful PDFs from Markdown files. It's designed to be easy to use, highly configurable, and, most importantly, incredibly fast.

## How It Works

The tool is built around a simple directory structure and a central configuration file.

-   `config.yaml`: This is the heart of your project. You define the title, author, theme, source files, and other metadata here.
-   `chapters/`: This directory holds your Markdown files. The `source` key in your config file points to the main Markdown file that pulls in all the chapters.
-   `assets/`: Store your images and other static assets here.
-   `themes/`: Customize the look and feel of your PDF with custom CSS themes.

## Features

-   **Blazing Fast:** Built with Rust for optimal performance.
-   **Markdown to PDF:** Effortlessly convert your Markdown files into a polished PDF.
-   **Live Reload:** Use the `--watch` flag to automatically rebuild the PDF when you make changes to your source files.
-   **Theming:** Full support for custom CSS and syntax highlighting themes.
-   **Simple CLI:** An intuitive command-line interface to initialize and build your projects.

## Installation & Usage

**Prerequisites:**
You need to have the Rust toolchain (including `cargo`) installed.

**1. Install:**
Clone the repository and install the tool using cargo:
```bash
git clone <repository-url>
cd PdfBuilder
cargo install --path .
```

**2. Initialize a New Project:**
This command creates the necessary files and directories to get you started.
```bash
PdfBuilder init
```

**3. Build Your PDF:**
This command will read your `config.yaml`, process your Markdown files, and generate the final PDF.
```bash
PdfBuilder build
```

**4. Watch for Changes:**
For a seamless writing experience, use the `--watch` flag. The PDF will automatically be rebuilt every time you save a file.
```bash
PdfBuilder build --watch
```

## My Journey Continues

This project has been a fantastic learning experience. It has changed my perspective on Rust and has shown me that with a little bit of effort, you can build powerful and efficient tools.

Stay tuned for the full story on my Medium profile!
