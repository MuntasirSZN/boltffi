use std::process::{Command, Output};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JavaEightCompilation {
    Release,
    SourceAndTarget,
}

impl JavaEightCompilation {
    pub fn from_version_output(version: &str) -> Option<Self> {
        let release = version
            .split_whitespace()
            .find(|token| {
                token
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_digit())
            })?
            .split(['.', '-'])
            .filter_map(|component| component.parse::<u16>().ok())
            .collect::<Vec<_>>();
        let major = match release.as_slice() {
            [1, major, ..] => *major,
            [major, ..] => *major,
            [] => return None,
        };
        match major {
            8 => Some(Self::SourceAndTarget),
            9.. => Some(Self::Release),
            _ => None,
        }
    }

    fn configure(self, compiler: &mut Command) {
        match self {
            Self::Release => {
                compiler.args(["--release", "8"]);
            }
            Self::SourceAndTarget => {
                compiler.args(["-source", "8", "-target", "8"]);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavaCompiler {
    java_eight: JavaEightCompilation,
}

impl JavaCompiler {
    pub fn discover() -> Option<Self> {
        let version_output = Command::new("javac").arg("-version").output().ok()?;
        if !version_output.status.success() {
            return None;
        }
        let java_eight =
            JavaEightCompilation::from_version_output(&Self::combined_output(&version_output))?;

        Some(Self { java_eight })
    }

    pub fn configure_java_eight(&self, compiler: &mut Command) {
        self.java_eight.configure(compiler);
    }

    fn combined_output(output: &Output) -> String {
        let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
        combined.push(' ');
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
        combined
    }
}
