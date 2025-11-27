//! Sitemap generation.
//!
//! Generates sitemap.xml for SEO and search engine indexing.

use crate::{
    config::SiteConfig,
    log,
    utils::{build::collect_files, rss::get_guid_from_content_output_path},
};
use anyhow::{Result, anyhow};
use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};
use rayon::prelude::*;
use std::{
    fs,
    io::Cursor,
};

/// Build sitemap.xml if enabled in config
pub fn build_sitemap(config: &'static SiteConfig) -> Result<()> {
    if config.build.sitemap.enable {
        let sitemap = Sitemap::new(config)?;
        sitemap.write_to_file(config)?;
    }
    Ok(())
}

/// Represents a URL entry in the sitemap
struct SitemapUrl {
    loc: String,
}

/// Sitemap structure for generating sitemap.xml
pub struct Sitemap {
    urls: Vec<SitemapUrl>,
}

impl Sitemap {
    /// Create a new sitemap by collecting all content URLs
    pub fn new(config: &'static SiteConfig) -> Result<Self> {
        log!(true; "sitemap"; "generating sitemap started");

        let content_files = collect_files(
            &crate::utils::build::CONTENT_CACHE,
            &config.build.content,
            &|path| path.extension().is_some_and(|ext| ext == "typ"),
        )?;

        let urls: Vec<SitemapUrl> = content_files
            .par_iter()
            .filter_map(|path| {
                match get_guid_from_content_output_path(path, config) {
                    Ok(loc) => Some(SitemapUrl { loc }),
                    Err(e) => {
                        log!("sitemap"; "Failed to generate URL for {:?}: {}", path, e);
                        None
                    }
                }
            })
            .collect();

        Ok(Self { urls })
    }

    /// Convert sitemap to XML string
    fn to_xml(&self) -> Result<String> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        // XML declaration
        writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        // urlset element with namespace
        let mut urlset = BytesStart::new("urlset");
        urlset.push_attribute(("xmlns", "http://www.sitemaps.org/schemas/sitemap/0.9"));
        writer.write_event(Event::Start(urlset))?;

        // Write each URL entry
        for url in &self.urls {
            writer.write_event(Event::Start(BytesStart::new("url")))?;

            writer.write_event(Event::Start(BytesStart::new("loc")))?;
            writer.write_event(Event::Text(BytesText::new(&url.loc)))?;
            writer.write_event(Event::End(BytesEnd::new("loc")))?;

            writer.write_event(Event::End(BytesEnd::new("url")))?;
        }

        writer.write_event(Event::End(BytesEnd::new("urlset")))?;

        let xml_bytes = writer.into_inner().into_inner();
        let xml_string = String::from_utf8(xml_bytes)
            .map_err(|e| anyhow!("Failed to convert sitemap to string: {}", e))?;

        Ok(xml_string)
    }

    /// Write sitemap to file
    pub fn write_to_file(self, config: &'static SiteConfig) -> Result<()> {
        let xml = self.to_xml()?;
        let sitemap_path = config.build.sitemap.path.as_path();
        if let Some(parent) = sitemap_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(sitemap_path, xml)?;

        log!(true; "sitemap"; "sitemap written successfully to {}", sitemap_path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sitemap_xml_structure() {
        // Create a minimal sitemap with a test URL
        let sitemap = Sitemap {
            urls: vec![
                SitemapUrl { loc: "https://example.com/".to_string() },
                SitemapUrl { loc: "https://example.com/posts/hello-world".to_string() },
            ],
        };

        let xml = sitemap.to_xml().unwrap();

        // Verify XML structure
        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"));
        assert!(xml.contains("<url>"));
        assert!(xml.contains("<loc>https://example.com/</loc>"));
        assert!(xml.contains("<loc>https://example.com/posts/hello-world</loc>"));
        assert!(xml.contains("</url>"));
        assert!(xml.contains("</urlset>"));
    }

    #[test]
    fn test_empty_sitemap() {
        let sitemap = Sitemap { urls: vec![] };
        let xml = sitemap.to_xml().unwrap();

        assert!(xml.contains("<urlset"));
        assert!(xml.contains("</urlset>"));
        assert!(!xml.contains("<url>"));
    }
}
