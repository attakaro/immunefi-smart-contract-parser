#[derive(Debug)]
pub struct ContractData {
    pub name: String,
    pub code: String
}

pub enum ContractType {
    Splitted,
    United
}

pub enum ParserMode {
    Single,
    Immunefi(String)
}