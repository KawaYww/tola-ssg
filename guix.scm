;;; GNU Guix package definition for tola
;;;
;;; This file can be used to build and install tola using GNU Guix:
;;;
;;;   guix build -f guix.scm
;;;   guix package -f guix.scm
;;;
;;; Or add it to your Guix configuration by referencing this channel.

(use-modules (guix packages)
             (guix download)
             (guix git-download)
             (guix build-system cargo)
             (guix licenses)
             (gnu packages crates-io)
             (gnu packages crates-graphics)
             (gnu packages crates-web)
             (gnu packages crates-crypto)
             (gnu packages crates-apple)
             (gnu packages crates-vcs)
             (gnu packages crates-tls)
             (gnu packages crates-windows)
             (gnu packages assembly)
             (gnu packages pkg-config)
             (gnu packages version-control))

(define-public tola
  (package
    (name "tola")
    (version "0.5.14")
    (source
     (origin
       (method git-fetch)
       (uri (git-reference
             (url "https://github.com/KawaYww/tola-ssg")
             (commit (string-append "v" version))))
       (file-name (git-file-name name version))
       (sha256
        (base32 "0000000000000000000000000000000000000000000000000000"))))
    (build-system cargo-build-system)
    (arguments
     `(#:cargo-inputs
       (("rust-anyhow" ,rust-anyhow-1)
        ("rust-axum" ,rust-axum-0.7)
        ("rust-chrono" ,rust-chrono-0.4)
        ("rust-clap" ,rust-clap-4)
        ("rust-colored" ,rust-colored-2)
        ("rust-crossterm" ,rust-crossterm-0.28)
        ("rust-dashmap" ,rust-dashmap-6)
        ("rust-educe" ,rust-educe-0.6)
        ("rust-gix" ,rust-gix-0.66)
        ("rust-lru" ,rust-lru-0.12)
        ("rust-minify-html" ,rust-minify-html-0.15)
        ("rust-notify" ,rust-notify-6)
        ("rust-quick-xml" ,rust-quick-xml-0.36)
        ("rust-rayon" ,rust-rayon-1)
        ("rust-ravif" ,rust-ravif-0.11)
        ("rust-regex" ,rust-regex-1)
        ("rust-resvg" ,rust-resvg-0.44)
        ("rust-rss" ,rust-rss-2)
        ("rust-serde" ,rust-serde-1)
        ("rust-serde-json" ,rust-serde-json-1)
        ("rust-shellexpand" ,rust-shellexpand-3)
        ("rust-slug" ,rust-slug-0.1)
        ("rust-thiserror" ,rust-thiserror-1)
        ("rust-tokio" ,rust-tokio-1)
        ("rust-toml" ,rust-toml-0.8)
        ("rust-tower-http" ,rust-tower-http-0.6)
        ("rust-urlencoding" ,rust-urlencoding-2)
        ("rust-usvg" ,rust-usvg-0.44)
        ("rust-which" ,rust-which-6)
        ("rust-portable-pty" ,rust-portable-pty-0.8))))
    (native-inputs
     (list nasm pkg-config))
    (home-page "https://github.com/KawaYww/tola-ssg")
    (synopsis "Static site generator for Typst-based blogs")
    (description
     "Tola is a static site generator designed for Typst-based blogs.
It handles tedious tasks unrelated to Typst itself, including:
@itemize
@item Automatic extraction of embedded SVG images for smaller size and faster loading
@item Slugification of paths and fragments for posts
@item Watching changes and recompiling automatically
@item Local server for previewing the generated site
@item Built-in TailwindCSS support
@item Deployment to GitHub Pages
@item RSS 2.0 support
@end itemize")
    (license expat)))

tola
