use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

/// CPU architectures supported by Rivet.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetArch {
    X86_64,
    Aarch64,
    Riscv64,
    Armv7,
    X86,
    Wasm32,
    #[serde(untagged)]
    Custom(String),
}

impl TargetArch {
    /// Detects the target architecture of the host machine at runtime/compile-time.
    pub fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            TargetArch::X86_64
        }
        #[cfg(target_arch = "aarch64")]
        {
            TargetArch::Aarch64
        }
        #[cfg(target_arch = "riscv64")]
        {
            TargetArch::Riscv64
        }
        #[cfg(target_arch = "arm")]
        {
            TargetArch::Armv7
        }
        #[cfg(target_arch = "x86")]
        {
            TargetArch::X86
        }
        #[cfg(target_arch = "wasm32")]
        {
            TargetArch::Wasm32
        }
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64",
            target_arch = "arm",
            target_arch = "x86",
            target_arch = "wasm32"
        )))]
        {
            TargetArch::Custom(std::env::consts::ARCH.to_string())
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            TargetArch::X86_64 => "x86_64",
            TargetArch::Aarch64 => "aarch64",
            TargetArch::Riscv64 => "riscv64",
            TargetArch::Armv7 => "armv7",
            TargetArch::X86 => "x86",
            TargetArch::Wasm32 => "wasm32",
            TargetArch::Custom(s) => s.as_str(),
        }
    }
}

impl fmt::Display for TargetArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TargetArch {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "x86_64" | "amd64" | "x64" => Ok(TargetArch::X86_64),
            "aarch64" | "arm64" => Ok(TargetArch::Aarch64),
            "riscv64" => Ok(TargetArch::Riscv64),
            "armv7" | "armv7l" | "armhf" => Ok(TargetArch::Armv7),
            "x86" | "i686" | "i386" => Ok(TargetArch::X86),
            "wasm32" => Ok(TargetArch::Wasm32),
            other if !other.is_empty() => Ok(TargetArch::Custom(other.to_string())),
            _ => Err(CoreError::InvalidTargetArch(s.to_string())),
        }
    }
}

/// Operating systems supported by Rivet.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetOs {
    Linux,
    MacOs,
    Windows,
    FreeBsd,
    NetBsd,
    OpenBsd,
    Veyra,
    #[serde(untagged)]
    Custom(String),
}

impl TargetOs {
    /// Detects the target operating system of the host machine.
    pub fn current() -> Self {
        #[cfg(target_os = "linux")]
        {
            TargetOs::Linux
        }
        #[cfg(target_os = "macos")]
        {
            TargetOs::MacOs
        }
        #[cfg(target_os = "windows")]
        {
            TargetOs::Windows
        }
        #[cfg(target_os = "freebsd")]
        {
            TargetOs::FreeBsd
        }
        #[cfg(target_os = "netbsd")]
        {
            TargetOs::NetBsd
        }
        #[cfg(target_os = "openbsd")]
        {
            TargetOs::OpenBsd
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        )))]
        {
            TargetOs::Custom(std::env::consts::OS.to_string())
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            TargetOs::Linux => "linux",
            TargetOs::MacOs => "macos",
            TargetOs::Windows => "windows",
            TargetOs::FreeBsd => "freebsd",
            TargetOs::NetBsd => "netbsd",
            TargetOs::OpenBsd => "openbsd",
            TargetOs::Veyra => "veyra",
            TargetOs::Custom(s) => s.as_str(),
        }
    }
}

impl fmt::Display for TargetOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TargetOs {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "linux" => Ok(TargetOs::Linux),
            "macos" | "darwin" | "osx" => Ok(TargetOs::MacOs),
            "windows" => Ok(TargetOs::Windows),
            "freebsd" => Ok(TargetOs::FreeBsd),
            "netbsd" => Ok(TargetOs::NetBsd),
            "openbsd" => Ok(TargetOs::OpenBsd),
            "veyra" => Ok(TargetOs::Veyra),
            other if !other.is_empty() => Ok(TargetOs::Custom(other.to_string())),
            _ => Err(CoreError::InvalidTargetOs(s.to_string())),
        }
    }
}

/// A target platform describing CPU architecture and Operating System.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Target {
    pub arch: TargetArch,
    pub os: TargetOs,
}

impl Target {
    /// Constructs a target platform.
    pub fn new(arch: TargetArch, os: TargetOs) -> Self {
        Self { arch, os }
    }

    /// Returns the host machine's target platform.
    pub fn host() -> Self {
        Self {
            arch: TargetArch::current(),
            os: TargetOs::current(),
        }
    }

    /// Checks if this target matches an optional arch filter and os filter.
    pub fn matches(&self, allowed_archs: &[TargetArch], allowed_os: &[TargetOs]) -> bool {
        let arch_match = allowed_archs.is_empty() || allowed_archs.contains(&self.arch);
        let os_match = allowed_os.is_empty() || allowed_os.contains(&self.os);
        arch_match && os_match
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.arch, self.os)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_arch_parsing() {
        assert_eq!("x86_64".parse::<TargetArch>().unwrap(), TargetArch::X86_64);
        assert_eq!("amd64".parse::<TargetArch>().unwrap(), TargetArch::X86_64);
        assert_eq!("aarch64".parse::<TargetArch>().unwrap(), TargetArch::Aarch64);
        assert_eq!("arm64".parse::<TargetArch>().unwrap(), TargetArch::Aarch64);
        assert_eq!("riscv64".parse::<TargetArch>().unwrap(), TargetArch::Riscv64);
    }

    #[test]
    fn test_target_os_parsing() {
        assert_eq!("linux".parse::<TargetOs>().unwrap(), TargetOs::Linux);
        assert_eq!("macos".parse::<TargetOs>().unwrap(), TargetOs::MacOs);
        assert_eq!("darwin".parse::<TargetOs>().unwrap(), TargetOs::MacOs);
        assert_eq!("veyra".parse::<TargetOs>().unwrap(), TargetOs::Veyra);
    }

    #[test]
    fn test_target_matching() {
        let target = Target::new(TargetArch::X86_64, TargetOs::Linux);
        
        // Empty filters match all
        assert!(target.matches(&[], &[]));

        // Matching filters
        assert!(target.matches(&[TargetArch::X86_64, TargetArch::Aarch64], &[TargetOs::Linux]));

        // Mismatched arch
        assert!(!target.matches(&[TargetArch::Aarch64], &[TargetOs::Linux]));

        // Mismatched os
        assert!(!target.matches(&[TargetArch::X86_64], &[TargetOs::MacOs]));
    }
}
