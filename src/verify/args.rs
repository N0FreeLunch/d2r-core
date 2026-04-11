use std::collections::HashMap;
use std::ffi::OsString;
use std::env;

#[derive(Debug, Clone, PartialEq)]
pub enum ArgType {
    /// A positional argument (e.g., `file.txt`)
    Positional,
    /// A boolean flag (e.g., `--verbose` or `-v`)
    Flag,
    /// A key-value option (e.g., `--output out.json` or `-o out.json`)
    Option,
}

#[derive(Debug, Clone)]
pub struct ArgSpec {
    pub name: String,
    pub short: Option<char>,
    pub long: Option<String>,
    pub arg_type: ArgType,
    pub description: String,
    pub required: bool,
    pub env_var: Option<String>,
    pub default: Option<String>,
}

impl ArgSpec {
    pub fn positional(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            short: None,
            long: None,
            arg_type: ArgType::Positional,
            description: description.to_string(),
            required: true,
            env_var: None,
            default: None,
        }
    }

    pub fn flag(name: &str, short: Option<char>, long: Option<&str>, description: &str) -> Self {
        Self {
            name: name.to_string(),
            short,
            long: long.map(|s| s.to_string()),
            arg_type: ArgType::Flag,
            description: description.to_string(),
            required: false,
            env_var: None,
            default: None,
        }
    }

    pub fn option(name: &str, short: Option<char>, long: Option<&str>, description: &str) -> Self {
        Self {
            name: name.to_string(),
            short,
            long: long.map(|s| s.to_string()),
            arg_type: ArgType::Option,
            description: description.to_string(),
            required: false,
            env_var: None,
            default: None,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_env(mut self, env_var: &str) -> Self {
        self.env_var = Some(env_var.to_string());
        self
    }

    pub fn with_default(mut self, default: &str) -> Self {
        self.default = Some(default.to_string());
        self
    }
}

#[derive(Debug)]
pub struct ArgParser {
    program_name: String,
    specs: Vec<ArgSpec>,
}

#[derive(Debug)]
pub struct ParsedArgs {
    values: HashMap<String, String>,
    flags: HashMap<String, bool>,
}

impl ArgParser {
    pub fn new(program_name: &str) -> Self {
        Self {
            program_name: program_name.to_string(),
            specs: Vec::new(),
        }
    }

    pub fn add_spec(&mut self, spec: ArgSpec) {
        self.specs.push(spec);
    }

    pub fn parse(&self, args: Vec<OsString>) -> Result<ParsedArgs, String> {
        let mut values = HashMap::new();
        let mut flags = HashMap::new();
        let mut positional_idx = 0;
        let mut it = args.into_iter().peekable();

        while let Some(arg) = it.next() {
            let arg_str = arg.to_string_lossy();
            if arg_str.starts_with("--") {
                let long_name = &arg_str[2..];
                if let Some(spec) = self.specs.iter().find(|s| s.long.as_deref() == Some(long_name)) {
                    match spec.arg_type {
                        ArgType::Flag => {
                            flags.insert(spec.name.clone(), true);
                        }
                        ArgType::Option => {
                            if let Some(val) = it.next() {
                                values.insert(spec.name.clone(), val.to_string_lossy().to_string());
                            } else {
                                return Err(format!("Option --{} requires a value", long_name));
                            }
                        }
                        ArgType::Positional => unreachable!(),
                    }
                } else if long_name == "help" {
                    return Err(self.usage());
                } else {
                    return Err(format!("Unknown option --{}", long_name));
                }
            } else if arg_str.starts_with("-") && arg_str.len() > 1 {
                let short_name = arg_str.chars().nth(1).unwrap();
                if let Some(spec) = self.specs.iter().find(|s| s.short == Some(short_name)) {
                    match spec.arg_type {
                        ArgType::Flag => {
                            flags.insert(spec.name.clone(), true);
                        }
                        ArgType::Option => {
                            if let Some(val) = it.next() {
                                values.insert(spec.name.clone(), val.to_string_lossy().to_string());
                            } else {
                                return Err(format!("Option -{} requires a value", short_name));
                            }
                        }
                        ArgType::Positional => unreachable!(),
                    }
                } else if short_name == 'h' {
                    return Err(self.usage());
                } else {
                    return Err(format!("Unknown option -{}", short_name));
                }
            } else {
                // Positional
                if let Some(spec) = self.specs.iter().filter(|s| matches!(s.arg_type, ArgType::Positional)).nth(positional_idx) {
                    values.insert(spec.name.clone(), arg_str.to_string());
                    positional_idx += 1;
                } else {
                    return Err(format!("Unexpected positional argument: {}", arg_str));
                }
            }
        }

        // Apply env fallbacks and defaults, check required
        for spec in &self.specs {
            if matches!(spec.arg_type, ArgType::Flag) {
                if !flags.contains_key(&spec.name) {
                    flags.insert(spec.name.clone(), false);
                }
                continue;
            }

            if !values.contains_key(&spec.name) {
                if let Some(env_name) = &spec.env_var {
                    if let Ok(val) = env::var(env_name) {
                        values.insert(spec.name.clone(), val);
                    }
                }
            }

            if !values.contains_key(&spec.name) {
                if let Some(default) = &spec.default {
                    values.insert(spec.name.clone(), default.clone());
                }
            }

            if spec.required && !values.contains_key(&spec.name) {
                return Err(format!("Missing required argument: {}", spec.name));
            }
        }

        Ok(ParsedArgs { values, flags })
    }

    pub fn usage(&self) -> String {
        let mut usage = format!("Usage: {}", self.program_name);
        let mut options_txt = String::new();

        for spec in &self.specs {
            match spec.arg_type {
                ArgType::Positional => {
                    if spec.required {
                        usage.push_str(&format!(" <{}>", spec.name));
                    } else {
                        usage.push_str(&format!(" [{}]", spec.name));
                    }
                }
                _ => {}
            }
        }

        usage.push_str(" [options]\n\nOptions:\n");

        for spec in &self.specs {
            let mut opt = String::new();
            if let Some(short) = spec.short {
                opt.push_str(&format!("-{}, ", short));
            } else {
                opt.push_str("    ");
            }

            if let Some(long) = &spec.long {
                opt.push_str(&format!("--{}", long));
            }

            match spec.arg_type {
                ArgType::Option => opt.push_str(" <value>"),
                ArgType::Positional => continue,
                ArgType::Flag => {}
            }

            options_txt.push_str(&format!("  {:20} {}\n", opt, spec.description));
        }
        
        options_txt.push_str("  -h, --help           Show this help message\n");

        usage.push_str(&options_txt);
        usage
    }
}

impl ParsedArgs {
    pub fn get(&self, name: &str) -> Option<&String> {
        self.values.get(name)
    }

    pub fn is_set(&self, name: &str) -> bool {
        self.flags.get(name).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let mut parser = ArgParser::new("test");
        parser.add_spec(ArgSpec::positional("input", "input file"));
        parser.add_spec(ArgSpec::flag("verbose", Some('v'), Some("verbose"), "verbose output"));
        parser.add_spec(ArgSpec::option("output", Some('o'), Some("output"), "output file"));

        let args = vec![
            OsString::from("in.bin"),
            OsString::from("-v"),
            OsString::from("--output"),
            OsString::from("out.json"),
        ];

        let parsed = parser.parse(args).unwrap();
        assert_eq!(parsed.get("input").unwrap(), "in.bin");
        assert!(parsed.is_set("verbose"));
        assert_eq!(parsed.get("output").unwrap(), "out.json");
    }

    #[test]
    fn test_missing_required() {
        let mut parser = ArgParser::new("test");
        parser.add_spec(ArgSpec::positional("input", "input file"));
        
        let args = vec![];
        let result = parser.parse(args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing required argument"));
    }

    #[test]
    fn test_defaults_and_env() {
        let mut parser = ArgParser::new("test");
        parser.add_spec(ArgSpec::option("port", None, Some("port"), "port number").with_default("8080"));
        parser.add_spec(ArgSpec::option("host", None, Some("host"), "host name").with_env("TEST_HOST"));

        unsafe {
            env::set_var("TEST_HOST", "localhost");
        }
        
        let args = vec![];
        let parsed = parser.parse(args).unwrap();
        assert_eq!(parsed.get("port").unwrap(), "8080");
        assert_eq!(parsed.get("host").unwrap(), "localhost");
    }
}
