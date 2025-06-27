// build.rs
use std::path::PathBuf;

fn main() {
    // Only rebuild if the tree-sitter-sql directory changes
    println!("cargo:rerun-if-changed=tree-sitter-sql");
    
    // Compile tree-sitter-sql parser
    let sql_dir = PathBuf::from("tree-sitter-sql");
    let sql_src_dir = sql_dir.join("src");
    
    // Check if the directory exists
    if sql_src_dir.exists() {
        cc::Build::new()
            .include(&sql_src_dir)
            .file(sql_src_dir.join("parser.c"))
            .file(sql_src_dir.join("scanner.c"))
            .compile("tree-sitter-sql");
            
        println!("cargo:rustc-link-lib=static=tree-sitter-sql");
    } else {
        // If the directory doesn't exist, we'll provide instructions
        println!("cargo:warning=tree-sitter-sql directory not found. SQL syntax highlighting will be disabled.");
        println!("cargo:warning=To enable SQL highlighting, run:");
        println!("cargo:warning=  git clone https://github.com/DerekStride/tree-sitter-sql.git");
        println!("cargo:warning=  cd tree-sitter-sql && git checkout gh-pages");
        println!("cargo:warning=  cd .. && cargo build");
    }
}
