//! `writer`: WriterProgram project folders from a terminal (src/cli.rs).

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(writer_mcp::cli::main(&args));
}
