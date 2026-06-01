use serde::{Serialize, Serializer};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProjectKind {
    Fullstack,
    RustWorker,
    StaticFrontend,
}

impl ProjectKind {
    pub(crate) fn parse(value: &str) -> std::result::Result<Self, String> {
        match value {
            "fullstack" => Ok(Self::Fullstack),
            "rust_worker" => Ok(Self::RustWorker),
            "static_frontend" => Ok(Self::StaticFrontend),
            _ => Err("kind must be fullstack, rust_worker, or static_frontend".to_owned()),
        }
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Fullstack => "fullstack",
            Self::RustWorker => "rust_worker",
            Self::StaticFrontend => "static_frontend",
        }
    }

    pub(crate) const fn sort_order(self) -> u8 {
        match self {
            Self::RustWorker => 0,
            Self::Fullstack => 1,
            Self::StaticFrontend => 2,
        }
    }

    pub(crate) const fn is_fullstack(self) -> bool {
        matches!(self, Self::Fullstack)
    }

    pub(crate) const fn is_static_frontend(self) -> bool {
        matches!(self, Self::StaticFrontend)
    }

    pub(crate) const fn is_rust_worker(self) -> bool {
        matches!(self, Self::RustWorker)
    }
}

impl Serialize for ProjectKind {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
