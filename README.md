# tola-ssg

(Still writing the document, releasing it shortly when I have some spare time)  

## Introduction

`tola`: static site generator for Typst-based blog.  
It handles the most tedious tasks unrelated to Typst itself.  

e.g.,  
- automatically extract embedded svg images for smaller size and faster loading
- slugify the paths && fragments for posts
- watch changes and recompile it 
- local server for previewing the generated size
- prevent users from typing repetitive command which user doesn't matter
- built-in tailwind-css support, out of the box
- deploy the generated site to github page (other provider is in plan)
- provide template file with a small kernal, so that most people can customize their own site easily (in plan)
- rss2.0 support
- sitemap.xml support

## Philosophy

The philosophy of `tola`:  
**keeping your focus on the content itself**  

`tola` helps you get closer!  
- A lightweight and minimal abstraction layer should be provided, which allows users to work without being locked into a rigid framework.
- If you can accomplish something in post more easily with Typst, then you’d better use it. (e.g.: [TODO](todo))
- Keep the core simple and maintainable, and enjoy the pure joy of writing.

## Examples

[My blog](https://kawayww.com):

![home](/screenshots/home.avif)  
![figure1](/screenshots/figure1.avif)  
![figure2](/screenshots/figure2.avif)  

## Installation

- `cargo install tola`
- install the binary in [release page](https://github.com/KawaYww/tola/releases).
- for nix users, a `flake.nix` already exists in the repo root, and you could use binary cache in [tola.cachix.org](https://tola.cachix.org):

```nix
# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    tola = {
      url = "github:kawayww/tola";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # ...
    # ...
  }
  # ...
  # ...
}
```

```nix
# configuration.nix
{ config, pkgs, inputs, ... }:

{
  nix.settings = {
    substituters = [
      "https://tola.cachix.org"
      # ...
      # ...
    ];
    trusted-public-keys = [
      "tola.cachix.org-1:5hMwVpNfWcOlq0MyYuU9QOoNr6bRcRzXBMt/Ua2NbgA="
      # ...
      # ...
    ];
    environment.systemPackages = with pkgs; [
      # ...
      # ...
    ] ++ [
      inputs.tola.packages.${pkgs.system}.default
    ];
  }
}
```

## Usage

- `tola -h`:  

```text
A static site generator for typst-based blog

Usage: tola [OPTIONS] <COMMAND>

Commands:
  init    Init a template site
  serve   Serve the site. Rebuild and reload on change automatically
  build   Deletes the output directory if there is one and rebuilds the site
  deploy  Deletes the output directory if there is one and rebuilds the site
  help    Print this message or the help of the given subcommand(s)

Options:
  -r, --root <ROOT>          root directory path
  -o, --output <OUTPUT>      Output directory path related to `root`
  -c, --content <CONTENT>    Content directory path related to `root`
  -a, --assets <ASSETS>      Assets directory path related to `root`
  -C, --config <CONFIG>      Config file path related to `root` [default: tola.toml]
  -m, --minify <MINIFY>      Minify the html content [possible values: true, false]
  -t, --tailwind <TAILWIND>  enable tailwindcss support [possible values: true, false]
  -h, --help                 Print help
  -V, --version              Print version
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
│   ├── index.typ
│   ├── programming.typ
├── templates
│   └── normal.typ
└── utils
    └── main.typ
```

Files under the `content/` directory are mapped to their respective routes:  
e.g., `content/posts/examples/aaa.typ` -> `http://127.0.0.1:8282/posts/examples/aaa`  
(`content/index.typ` will be specially compiled into `http://127.0.0.1:8282/index.html`)  

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


