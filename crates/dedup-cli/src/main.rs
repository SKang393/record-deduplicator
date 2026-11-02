use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() || (args.len() == 1 && (args[0] == "--help" || args[0] == "-h")) {
        println!(
            "record-deduplicator {}\n\nUsage: dedup [--help | --version]\n\n\
             Foundation build only. Import, matching, review, and export are not implemented.\n\
             See TASKS.md for the next implementation batch.",
            dedup_core::VERSION
        );
        return ExitCode::SUCCESS;
    }
    if args.len() == 1 && (args[0] == "--version" || args[0] == "-V") {
        println!("dedup {}", dedup_core::VERSION);
        return ExitCode::SUCCESS;
    }
    eprintln!("Unsupported arguments. This foundation build supports only --help and --version.");
    ExitCode::from(2)
}
