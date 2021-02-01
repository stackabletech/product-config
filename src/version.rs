use regex::Regex;
use thiserror::Error;

lazy_static! {
    static ref REGEX: Regex =
        Regex::new(r"(?P<major>[1-9]\d*).(?P<minor>[1-9]\d*).(?P<patch>[1-9]\d*)$").unwrap();
}

#[derive(PartialEq, PartialOrd, Debug)]
pub struct Version {
    pub major: usize,
    pub minor: usize,
    pub patch: usize,
}

#[derive(PartialEq, Error, Debug)]
pub enum VersionError {
    #[error("[{0}] does not match pattern 'x.y.z' !")]
    WrongVersionPattern(String),
}

impl Version {
    pub fn parse(version: &str) -> Result<Version, VersionError> {
        return match REGEX.captures(version) {
            None => Err(VersionError::WrongVersionPattern(version.to_string())),
            Some(caps) => Ok(Version {
                major: caps["major"].parse::<usize>().unwrap(),
                minor: caps["minor"].parse::<usize>().unwrap(),
                patch: caps["patch"].parse::<usize>().unwrap(),
            }),
        };
    }

    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", &self.major, &self.minor, &self.patch)
    }
}

#[cfg(test)]
mod tests {
    use crate::version::Version;

    #[test]
    fn test_parse() {
        let v1 = Version::parse("1.5.2");
        assert!(v1.is_ok());
        let v1 = v1.as_ref().unwrap();
        assert_eq!(v1.major == 1 && v1.minor == 5 && v1.patch == 2, true);

        let v2 = Version::parse("2.1.100");
        assert!(v2.is_ok());
        let v2 = v2.as_ref().unwrap();
        assert_eq!(v2.major == 2 && v2.minor == 1 && v2.patch == 100, true);

        let v3 = Version::parse("a.bc.2");
        assert!(v3.is_err());
    }

    #[test]
    fn test_eq_ord() {
        let v1 = Version {
            major: 1,
            minor: 200,
            patch: 1,
        };
        assert_eq!(v1.major == 1 && v1.minor == 200 && v1.patch == 1, true);

        let v2 = Version {
            major: 200,
            minor: 3,
            patch: 12,
        };
        assert_eq!(v2.major == 200 && v2.minor == 3 && v2.patch == 12, true);

        assert_eq!(v1 < v2, true);
        assert_eq!(v1 == v1, true);
        assert_eq!(v1 > v2, false);
    }

    #[test]
    fn test_to_string() {
        let v1 = Version {
            major: 1,
            minor: 200,
            patch: 1,
        };

        assert_eq!(v1.to_string() == "1.200.1", true);
    }
}
