// build.rs
use std::path::PathBuf;

fn main() {
    // Only rebuild if the tree-sitter-sql directory changes
    println!("cargo:rerun-if-changed=tree-sitter-sql");
    
    // Check if we should compile SQL support
    let sql_enabled = std::env::var("CARGO_FEATURE_SQL").is_ok();
    
    if !sql_enabled {
        println!("cargo:warning=SQL syntax highlighting is disabled. Enable with: cargo run --release --features sql");
        return;
    }
    
    // Compile tree-sitter-sql parser
    let sql_dir = PathBuf::from("tree-sitter-sql");
    let sql_src_dir = sql_dir.join("src");
    
    // Check if the directory exists
    if sql_src_dir.exists() {
        println!("cargo:warning=Building tree-sitter-sql parser...");
        
        cc::Build::new()
            .include(&sql_src_dir)
            .file(sql_src_dir.join("parser.c"))
            .file(sql_src_dir.join("scanner.c"))
            .warnings(false) // Disable warnings from generated code
            .compile("tree-sitter-sql");
            
        println!("cargo:rustc-link-lib=static=tree-sitter-sql");
        println!("cargo:warning=SQL syntax highlighting enabled!");
    } else {
        // If the directory doesn't exist, we'll provide instructions
        println!("cargo:warning=");
        println!("cargo:warning=tree-sitter-sql directory not found!");
        println!("cargo:warning=");
        println!("cargo:warning=To enable SQL syntax highlighting:");
        println!("cargo:warning=  1. git clone https://github.com/DerekStride/tree-sitter-sql.git");
        println!("cargo:warning=  2. cd tree-sitter-sql && git checkout gh-pages");
        println!("cargo:warning=  3. cd .. && cargo run --release");
        println!("cargo:warning=");
        println!("cargo:warning=SQL syntax highlighting will be disabled for now.");
        println!("cargo:warning=");
        
        // Define a cfg flag to disable SQL in the code
        println!("cargo:rustc-cfg=no_sql");
    }
}
