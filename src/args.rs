use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub target: String,
    pub max_hops: u8,
    pub timeout: Duration,
    pub interval: Duration,
    pub snapshot: bool,
    pub cycles: u32,
}

impl Config {
    pub fn parse<I>(arguments: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut arguments = arguments.into_iter();
        let _program = arguments.next();
        let mut target = None;
        let mut max_hops = 30_u8;
        let mut timeout_ms = 500_u64;
        let mut interval_ms = 80_u64;
        let mut snapshot = false;
        let mut cycles = 1_u32;

        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "-h" | "--help" => return Err(usage()),
                "-V" | "--version" => return Err(format!("remtr {}", env!("CARGO_PKG_VERSION"))),
                "--snapshot" => snapshot = true,
                "--max-hops" => {
                    max_hops = parse_value(arguments.next(), "--max-hops")?;
                    if !(1..=64).contains(&max_hops) {
                        return Err("--max-hops must be between 1 and 64".into());
                    }
                }
                "--timeout-ms" => {
                    timeout_ms = parse_value(arguments.next(), "--timeout-ms")?;
                    if !(50..=10_000).contains(&timeout_ms) {
                        return Err("--timeout-ms must be between 50 and 10000".into());
                    }
                }
                "--interval-ms" => {
                    interval_ms = parse_value(arguments.next(), "--interval-ms")?;
                    if !(10..=10_000).contains(&interval_ms) {
                        return Err("--interval-ms must be between 10 and 10000".into());
                    }
                }
                "--cycles" => {
                    cycles = parse_value(arguments.next(), "--cycles")?;
                    if !(1..=1000).contains(&cycles) {
                        return Err("--cycles must be between 1 and 1000".into());
                    }
                }
                value if value.starts_with('-') => {
                    return Err(format!("unknown option: {value}\n\n{}", usage()));
                }
                value => {
                    if target.replace(value.to_owned()).is_some() {
                        return Err("only one target may be specified".into());
                    }
                }
            }
        }

        let target = target.ok_or_else(|| format!("missing TARGET\n\n{}", usage()))?;
        Ok(Self {
            target,
            max_hops,
            timeout: Duration::from_millis(timeout_ms),
            interval: Duration::from_millis(interval_ms),
            snapshot,
            cycles,
        })
    }
}

fn parse_value<T>(value: Option<String>, option: &str) -> Result<T, String>
where
    T: std::str::FromStr,
{
    value
        .ok_or_else(|| format!("missing value for {option}"))?
        .parse()
        .map_err(|_| format!("invalid value for {option}"))
}

pub fn usage() -> String {
    format!(
        "ReMTR {version}\n\nUsage: remtr [OPTIONS] TARGET\n\nOptions:\n  --snapshot           Print a finite report instead of opening the TUI\n  --cycles N           Probe cycles in snapshot mode [default: 1]\n  --max-hops N         Maximum TTL [default: 30]\n  --timeout-ms N       Per-probe timeout [default: 500]\n  --interval-ms N      Delay between probes [default: 80]\n  -h, --help           Print help\n  -V, --version        Print version",
        version = env!("CARGO_PKG_VERSION")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_snapshot_configuration() {
        let config = Config::parse(
            [
                "remtr",
                "--snapshot",
                "--cycles",
                "3",
                "--max-hops",
                "20",
                "example.com",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();

        assert_eq!(config.target, "example.com");
        assert_eq!(config.max_hops, 20);
        assert_eq!(config.cycles, 3);
        assert!(config.snapshot);
    }

    #[test]
    fn rejects_unsafe_hop_limit() {
        let error = Config::parse(
            ["remtr", "--max-hops", "255", "example.com"]
                .into_iter()
                .map(str::to_owned),
        )
        .unwrap_err();
        assert!(error.contains("between 1 and 64"));
    }
}
