# Tola

## Introduction

`Tola`: A static site generator for Typst-based blog.  

The `tola` command handles the most tedious tasks unrelated to Typst itself.  

e.g.,  
- watch changes in multiple files and recompile  
- provide a local server to preview  
- to prevent users from typing tedious/repetitive command options, like `typst compile --features html --format html --root ./  --font-path ./ xxx.typ xxx/index.html`
- run `typst compile ...` and `tailwindcss -i input.css ...` (`tola` comes with built-in Tailwind CSS support out of the box.) (developing in `v0.5.0`)
- deploy the generated blog to github-page/cloudflare-page/vercal-page (developing in `v0.5.0`)
- provide a small-kernel typst template file, so that most people can customize their own site easily  (developing )

*Note*: The release 0.5.0 is comming soon.  

## Philosophy

The philosophy of Tola:  
**Please just focus on Typst itself(`tola` helps you get it!)**  

## Examples

[My blog](https://kawayww.com):

![home](/screenshots/home.avif)  
![figure1](/screenshots/figure1.avif)  
![figure2](/screenshots/figure2.avif)  

## Installation

1. Typing `cargo install tola` to get `Tola`.
2. Install the binary in [release page](https://github.com/KawaYww/tola/releases).
3. For Nix users: A `flake.nix` already exists in the repo root.

## Usage

- `tola -h`:  

```text
A static site generator for typst-based blog

Usage: tola [OPTIONS] [COMMAND]

Commands:
  init    Init a template site
  serve   Serve the site. Rebuild and reload on change automatically
  build   Deletes the output directory if there is one and rebuilds the site
  deploy  Deletes the output directory if there is one and rebuilds the site
  help    Print this message or the help of the given subcommand(s)

Options:
  -r, --root <ROOT>
          root directory path [default: ./]
  -o, --output <OUTPUT>
          Output directory path related to `root_dor` [default: public]
  -c, --content <CONTENT>
          Content directory path related to `root_dor` [default: content]
  -a, --assets <ASSETS>
          Assets directory path related to `root_dor` [default: assets]
  -C, --config <CONFIG>
          Config file path related to `root_dor` [default: tola.toml]
  -m, --minify
          Minify the html content
      --tailwind-support

      --tailwind-command <TAILWIND_COMMAND>
          [default: tailwindcss]
  -h, --help
          Print help
  -V, --version
          Print version
```

You should keep the directory structure identical to the below:

```text
.
├── assets
│   ├── fonts
│   ├── iconfonts
│   ├── images
│   ├── scripts
│   ├── styles
├── content
│   ├── posts/
│   ├── categories/
│   ├── home.typ
│   ├── programming.typ
├── templates
│   └── normal.typ
└── utils
    └── main.typ
```

Files under the `content/` directory are mapped to their respective routes:  
e.g., `content/posts/examples/aaa.typ` -> `http://127.0.0.1:8282/posts/examples/aaa`  
(`content/home.typ` will be specially compiled into `http://127.0.0.1:8282/index.html`)  

```text
http://127.0.0.1:8000:
├── assets
│   ├── fonts
│   ├── iconfonts
│   ├── images
│   ├── scripts
│   └─ styles
├── posts/
├── categories/
└── index.html
```


