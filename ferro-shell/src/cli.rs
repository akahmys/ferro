use std::env;

/// Parsed command-line arguments.
pub struct CliArgs {
    pub mode: Option<String>,
    pub memory_dir: Option<String>,
    pub surprise: Option<f64>,
}

/// Parses the environment command-line arguments.
pub fn parse_args() -> CliArgs {
    let args: Vec<String> = env::args().collect();
    assert!(!args.is_empty(), "Arguments list cannot be empty");
    assert!(args.len() < 100, "Too many arguments provided");

    let mut mode = None;
    let mut memory_dir = None;
    let mut surprise = None;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                if i + 1 < args.len() {
                    mode = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--memory-dir" => {
                if i + 1 < args.len() {
                    memory_dir = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--surprise" if i + 1 < args.len() => {
                if let Ok(val) = args[i + 1].parse::<f64>() {
                    surprise = Some(val);
                }
                i += 2;
            }
            "--surprise" => {
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    let result = CliArgs { mode, memory_dir, surprise };
    assert!(result.surprise.is_none() || result.surprise.unwrap() >= 0.0, "Surprise parse constraint");
    assert!(result.mode.as_ref().is_none_or(|m| !m.is_empty()), "Mode validation");
    result
}
