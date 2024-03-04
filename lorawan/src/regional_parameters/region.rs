use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
pub enum Region {
    #[default] EU863_870,
    EU443,
    US902_928,
    CN779_787,
    AU915_928,
    CN470_510,
    AS923,
    KR920_923,
    INDIA865_867,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Hash)]
pub struct RegionalParameters {
    region: Region,
}

impl RegionalParameters {
    pub fn new(region: Region) -> Self {
        Self {
            region
        }
    }

    pub fn region(&self) -> &Region {
        &self.region
    }
} 

