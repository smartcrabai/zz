use chrono::Local;
use zzsleep::{parse_end_time, sleep_until, split_args};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    if raw_args.iter().any(|a| a == "-v" || a == "--version") {
        println!("zz {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let (quiet, time_args) = split_args(&raw_args);

    if time_args.is_empty() {
        eprintln!("Usage: zz <duration>");
        eprintln!("  zz 10          # 10 seconds");
        eprintln!("  zz 2h          # 2 hours");
        eprintln!("  zz 5m          # 5 minutes");
        eprintln!("  zz 30s         # 30 seconds");
        eprintln!("  zz 2h 5m       # 2 hours 5 minutes");
        eprintln!("  zz 5m 30s      # 5 minutes 30 seconds");
        eprintln!("  zz 1h 30m 45s  # 1 hour 30 minutes 45 seconds");
        eprintln!("  zz 12:30       # until 12:30 today (tomorrow if past)");
        eprintln!("  zz 12:30:45    # until 12:30:45 today (tomorrow if past)");
        eprintln!("  zz 20260220T123000+0900  # ISO 8601 with timezone");
        eprintln!("  zz 20260220T123000Z      # ISO 8601 UTC");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -q, --quiet    suppress progress bar");
        std::process::exit(1);
    }

    let now = Local::now();
    let end_time = match parse_end_time(&time_args, now) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    sleep_until(end_time, quiet).await;
}
