use std::str::FromStr;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Default, clap::ValueEnum)]
pub enum Units {
    #[default]
    #[value(name = "human")]
    Human,
    #[value(name = "si")]
    Si,
}

impl FromStr for Units {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match () {
            _ if s.eq_ignore_ascii_case("human") => Ok(Units::Human),
            _ if s.eq_ignore_ascii_case("si") => Ok(Units::Si),
            _ => Err(format!("unknown units `{}` (expected `human` or `si`)", s)),
        }
    }
}
