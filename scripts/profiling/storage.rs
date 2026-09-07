//! Optimized microbenchmark of the actual storage implementation, without
//! compiling or starting a webview. Run through `bun run profile:storage`.
#[allow(dead_code)]
#[path = "../../src-tauri/src/download/den.rs"]
mod den;

use den::{GalleryDen, SpiderInfo};
use std::{fs, hint::black_box, time::Instant};

fn measure(mut operation: impl FnMut(), repetitions: u32) -> f64 {
    for _ in 0..5 { operation(); }
    let started = Instant::now();
    for _ in 0..repetitions { operation(); }
    started.elapsed().as_secs_f64() * 1_000_000.0 / repetitions as f64
}

fn main() {
    let root = std::env::temp_dir().join(format!("exviewer-storage-profile-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let result = std::panic::catch_unwind(|| {
        println!("{{\"units\":\"microseconds per operation\",\"optimized\":true,\"fixtures\":[");
        for (case, pages) in [100, 1_000, 10_000].into_iter().enumerate() {
            let den = GalleryDen::open(&root, &pages.to_string()).unwrap();
            let mut info = SpiderInfo::new(1, "fixture", pages, 1, 40);
            for index in 0..pages {
                den.write_page(index, "jpg", b"fixture").unwrap();
                info.ptoken_map.insert(index, "0123456789abcdef0123456789abcdef".into());
            }
            let path = info.write_to_dir(den.path()).unwrap();
            let header = measure(|| { black_box(SpiderInfo::read_header(&path).unwrap()); }, 1_000);
            let scan = measure(|| { black_box(den.available_pages(pages)); }, 30);
            let lookup = measure(|| { black_box(den.find_page(pages / 2)); }, 1_000);
            println!("{}{{\"pages\":{pages},\"metadataBytes\":{},\"headerUs\":{header:.3},\"scanUs\":{scan:.3},\"pageLookupUs\":{lookup:.3}}}", if case == 0 { "" } else { "," }, fs::metadata(&path).unwrap().len());
        }
        println!("]}}");
    });
    fs::remove_dir_all(root).unwrap();
    if let Err(error) = result { std::panic::resume_unwind(error); }
}
