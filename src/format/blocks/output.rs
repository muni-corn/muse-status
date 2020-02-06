use super::super::Attention;
use crate::format::bit::Bit;

pub struct BlockOutput {
    pub block_name: String,
    pub body: Option<BlockOutputBody>,
}

impl BlockOutput {
    pub fn new(block_name: &str, body: Option<BlockOutputBody>) -> Self {
        Self {
            block_name: String::from(block_name),
            body,
        }
    }
}

pub enum BlockOutputBody {
    Nice(NiceOutput),
    SingleBit(Bit),
    Custom(String),
}

impl From<NiceOutput> for BlockOutputBody {
    fn from(n: NiceOutput) -> Self {
        Self::Nice(n)
    }
}

impl From<Bit> for BlockOutputBody {
    fn from(b: Bit) -> Self {
        Self::SingleBit(b)
    }
}

impl From<String> for BlockOutputBody {
    fn from(s: String) -> Self {
        Self::Custom(s)
    }
}

impl From<&str> for BlockOutputBody {
    fn from(s: &str) -> Self {
        Self::Custom(String::from(s))
    }
}

pub struct NiceOutput {
    pub icon: Option<char>,
    pub primary_text: String,
    pub secondary_text: Option<String>,
    pub attention: Attention,
}
